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

/// Transcribe raw WAV bytes through any OpenAI-compatible provider.
/// File-transcription style models (whisper/voxtral/*-transcribe/scribe) use
/// the audio transcriptions endpoint; other audio-capable models use
/// chat/completions with inline base64 input_audio.
pub fn transcribe_wav(wav: &[u8]) -> Result<String, String> {
    let cfg = config::load();
    if cfg.api_key.trim().is_empty() {
        return Err("API key not configured. Open tray menu > Settings.".into());
    }
    let m = cfg.model.to_lowercase();
    if ["whisper", "voxtral", "transcribe", "scribe"]
        .iter()
        .any(|k| m.contains(k))
    {
        transcribe_transcriptions(wav, &cfg)
    } else {
        transcribe_chat(wav, &cfg)
    }
}

fn agent() -> Result<ureq::Agent, String> {
    Ok(ureq::AgentBuilder::new()
        .tls_connector(std::sync::Arc::new(
            native_tls::TlsConnector::new().map_err(|e| format!("TLS init failed: {e}"))?,
        ))
        .build())
}

fn auth_headers(req: ureq::Request) -> ureq::Request {
    let cfg = config::load();
    req.set("Authorization", &format!("Bearer {}", cfg.api_key.trim()))
        .set("X-Title", "BYOK-STT")
}

fn transcribe_chat(wav: &[u8], cfg: &config::Config) -> Result<String, String> {
    let b64 = base64::engine::general_purpose::STANDARD.encode(wav);
    let body = serde_json::json!({
        "model": cfg.model,
        "messages": [{
            "role": "user",
            "content": [
                {"type": "text", "text": "Transcribe this audio. Output only the transcription text, nothing else."},
                {"type": "input_audio", "input_audio": {"data": b64, "format": "wav"}}
            ]
        }]
    });

    let (chat_url, _) = endpoints(&cfg.api_base);
    let resp = auth_headers(agent()?.post(&chat_url))
        .timeout(Duration::from_secs(30))
        .send_json(body)
        .map_err(|e| http_err(e))?;

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
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"audio.wav\"\r\nContent-Type: audio/wav\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(wav);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let (_, stt_url) = endpoints(&cfg.api_base);
    let resp = auth_headers(agent()?.post(&stt_url))
        .set(
            "Content-Type",
            &format!("multipart/form-data; boundary={boundary}"),
        )
        .timeout(Duration::from_secs(30))
        .send_bytes(&body)
        .map_err(|e| http_err(e))?;

    let json: serde_json::Value = resp.into_json().map_err(|e| format!("bad JSON: {e}"))?;
    let text = json["text"]
        .as_str()
        .ok_or_else(|| format!("unexpected response shape: {}", truncate(&json.to_string(), 400)))?;
    Ok(text.trim().to_string())
}

fn http_err(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, r) => {
            let body = r.into_string().unwrap_or_default();
            format!("HTTP {code}: {}", truncate(&body, 400))
        }
        other => format!("request failed: {other}"),
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
