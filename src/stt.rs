use std::time::Duration;

use base64::Engine;

use crate::config;

fn endpoints(base: &str) -> (String, String) {
    let base = base.trim_end_matches('/');
    (
        format!("{base}/chat/completions"),
        format!("{base}/audio/transcriptions"),
    )
}

/// True when the configured model routes through the audio transcriptions
/// endpoint (file-style upload) rather than chat/completions. Covers the
/// file-model families (whisper, voxtral, scribe), OpenAI's *-transcribe
/// models and ASR-named models (e.g. qwen3-asr).
pub fn uses_transcriptions_endpoint(model: &str) -> bool {
    let m = model.to_lowercase();
    ["whisper", "voxtral", "transcribe", "scribe", "asr"]
        .iter()
        .any(|k| m.contains(k))
}

/// Transcribe raw WAV bytes through any OpenAI-compatible provider.
pub fn transcribe_wav(wav: &[u8]) -> Result<String, String> {
    let cfg = config::load();
    if cfg.api_key.trim().is_empty() {
        return Err("API key not configured. Open tray menu > Settings.".into());
    }
    let mut text = if uses_transcriptions_endpoint(&cfg.model) {
        transcribe_transcriptions(wav, &cfg)?
    } else {
        transcribe_chat(wav, &cfg)?
    };
    if cfg.convert_to_traditional {
        text = crate::zh::s2t(&text);
    }
    Ok(text)
}

fn agent() -> Result<ureq::Agent, String> {
    Ok(ureq::AgentBuilder::new()
        .tls_connector(std::sync::Arc::new(
            native_tls::TlsConnector::new().map_err(|e| format!("TLS init failed: {e}"))?,
        ))
        // Connection establishment (TCP + TLS) is capped separately from the
        // request/response timeout, so an unreachable host fails fast instead
        // of waiting out the full processing window.
        .timeout_connect(CONNECT_TIMEOUT)
        .build())
}

/// Establishing the connection should take well under a second on any
/// working route; 8 s catches a black-holed route long before the OS default.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
/// One retry covers transient blips (route hiccups, 5xx, rate limits).
const MAX_ATTEMPTS: usize = 2;
const RETRY_BACKOFF: Duration = Duration::from_millis(500);

/// Retry once on network errors, 429 and 5xx; never on client errors or
/// malformed requests. Returns a human-friendly message on final failure.
fn send_retry(
    host: &str,
    mut attempt: impl FnMut() -> Result<ureq::Response, ureq::Error>,
) -> Result<ureq::Response, String> {
    let mut last: Option<ureq::Error> = None;
    for i in 0..MAX_ATTEMPTS {
        if i > 0 {
            std::thread::sleep(RETRY_BACKOFF);
        }
        match attempt() {
            Ok(r) => return Ok(r),
            Err(e) if is_retryable(&e) => last = Some(e),
            Err(e) => return Err(http_err(host, e)),
        }
    }
    Err(http_err(host, last.expect("one attempt ran")))
}

fn is_retryable(e: &ureq::Error) -> bool {
    match e {
        ureq::Error::Status(code, _) => *code == 429 || (500..600).contains(code),
        ureq::Error::Transport(t) => !matches!(
            t.kind(),
            ureq::ErrorKind::BadHeader
                | ureq::ErrorKind::InvalidUrl
                | ureq::ErrorKind::UnknownScheme
        ),
    }
}

fn auth_headers(req: ureq::Request) -> ureq::Request {
    let cfg = config::load();
    req.set("Authorization", &format!("Bearer {}", cfg.api_key.trim()))
        .set("X-Title", "BYOK-STT")
}

fn transcribe_chat(wav: &[u8], cfg: &config::Config) -> Result<String, String> {
    let b64 = base64::engine::general_purpose::STANDARD.encode(wav);
    // Chat audio models follow instructions, so the prompt becomes an
    // explicit style hint (e.g. colloquial Cantonese output).
    let hint = cfg.stt_prompt.trim();
    let instr = if hint.is_empty() {
        "Transcribe this audio. Output only the transcription text, nothing else.".to_string()
    } else {
        format!(
            "Transcribe this audio. Style hint: {hint}\nOutput only the transcription text, nothing else."
        )
    };
    let body = serde_json::json!({
        "model": cfg.model,
        "messages": [{
            "role": "user",
            "content": [
                {"type": "text", "text": instr},
                {"type": "input_audio", "input_audio": {"data": b64, "format": "wav"}}
            ]
        }]
    });

    let (chat_url, _) = endpoints(&cfg.api_base);
    let ag = agent()?;
    let resp = send_retry(host_of(&chat_url), || {
        auth_headers(ag.clone().post(&chat_url))
            .timeout(Duration::from_secs(30))
            .send_json(&body)
    })?;

    let json: serde_json::Value = resp.into_json().map_err(|e| format!("bad JSON: {e}"))?;
    let text = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| format!("unexpected response shape: {}", truncate(&json.to_string(), 400)))?;
    Ok(text.trim().trim_matches('"').trim().to_string())
}

fn transcribe_transcriptions(wav: &[u8], cfg: &config::Config) -> Result<String, String> {
    let boundary = "----byokstt7f3a91c2";
    let mut body = Vec::with_capacity(wav.len() + 512);
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"model\"\r\n\r\n{}\r\n",
            cfg.model
        )
        .as_bytes(),
    );
    // Whisper-style `prompt` field: honored by OpenAI/Groq/self-hosted
    // endpoints; OpenRouter accepts but ignores it (harmless).
    let p = cfg.stt_prompt.trim();
    if !p.is_empty() {
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"prompt\"\r\n\r\n{p}\r\n"
            )
            .as_bytes(),
        );
    }
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"audio.wav\"\r\nContent-Type: audio/wav\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(wav);
    let (_, stt_url) = endpoints(&cfg.api_base);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let ag = agent()?;
    let resp = send_retry(host_of(&stt_url), || {
        auth_headers(ag.clone().post(&stt_url))
            .set(
                "Content-Type",
                &format!("multipart/form-data; boundary={boundary}"),
            )
            .timeout(Duration::from_secs(30))
            .send_bytes(&body)
    })?;

    let json: serde_json::Value = resp.into_json().map_err(|e| format!("bad JSON: {e}"))?;
    let text = json["text"]
        .as_str()
        .ok_or_else(|| format!("unexpected response shape: {}", truncate(&json.to_string(), 400)))?;
    Ok(text.trim().to_string())
}

/// Host part of a URL, for error messages.
fn host_of(url: &str) -> &str {
    url.split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or(url)
}

/// Actionable error text: the balloon is the only thing most users will see.
fn http_err(host: &str, e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(401, r) | ureq::Error::Status(403, r) => format!(
            "{host} rejected the API key (HTTP {}). Open Settings and paste a valid key.",
            r.status()
        ),
        ureq::Error::Status(429, _) => {
            format!("{host} rate limit reached (HTTP 429). Wait a moment and try again.")
        }
        ureq::Error::Status(code, r) => {
            let body = r.into_string().unwrap_or_default();
            format!("{host} returned HTTP {code}: {}", truncate(&body, 300))
        }
        ureq::Error::Transport(t) => match t.kind() {
            ureq::ErrorKind::Io | ureq::ErrorKind::ConnectionFailed => format!(
                "Cannot reach {host}: the connection timed out or was dropped — the \
                 request never reached the server. Check your internet connection, \
                 VPN or proxy; this is a network route issue, not a server outage."
            ),
            ureq::ErrorKind::Dns => {
                format!("Cannot resolve {host}. Check your DNS or internet connection.")
            }
            _ => format!("Network error contacting {host}: {t}"),
        },
    }
}

/// Load a WAV file from disk and transcribe it (used by --test-transcribe).
pub fn transcribe_file(path: &str) -> Result<String, String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| format!("cannot open {path}: {e}"))?;
    let spec = reader.spec();
    if spec.sample_format != hound::SampleFormat::Int || spec.bits_per_sample != 16 {
        return Err("test file must be 16-bit PCM WAV".into());
    }
    let samples: Vec<i16> = reader
        .samples::<i16>()
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    let wav = crate::audio::encode_wav(&samples, spec)?;
    transcribe_wav(&wav)
}

fn truncate(s: &str, n: usize) -> &str {
    match s.char_indices().nth(n) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}
