# byok-stt

Push-to-talk voice typing for Windows. Hold **Ctrl+Win**, speak, release — your
words are typed into whatever app has focus. **Bring your own key**: works with
any OpenAI-compatible speech-to-text provider (OpenRouter, OpenAI, Groq, Google
Gemini, Mistral, ElevenLabs, or your own endpoint).

A small on-screen bubble appears only while recording (red) or transcribing
(amber spinner). No cloud account, no telemetry — one Rust binary, ~10 MB RAM.

## Features

- **Global hotkey**: hold the hotkey anywhere, dictate, release → text is
  pasted automatically (and copied to the clipboard). Default `Ctrl+Win`
  (Windows), `Cmd+Space` (macOS), `Ctrl+Space` (Linux) — fully customizable
  in Settings (modifier + key)
- **Provider presets**: pick a provider, paste your API key, choose a model —
  or point at any OpenAI-compatible endpoint with the editable base URL
- **On-screen indicator bubble**: draggable, position remembered, gray when
  idle, red while recording, amber spinner while transcribing. Left-click
  toggles recording, right-click opens a Settings menu; an option keeps it
  always visible
- **Tray icon** with state colors (idle / recording / transcribing):
  left-click toggles recording, right-click opens the menu (Settings, Exit)
- **Safety limits**: configurable maximum recording length (default 120 s) so
  a forgotten session can never run forever
- **Privacy**: audio lives in RAM only, is sent straight to the provider you
  configured, and is never written to disk
- **Lightweight**: single exe, ~10 MB RAM, no background indexer

## Getting an API key

| Provider | Key page | Suggested model |
|---|---|---|
| OpenRouter | https://openrouter.ai/keys | `openai/whisper-large-v3-turbo` |
| OpenAI | https://platform.openai.com/api-keys | `gpt-4o-mini-transcribe` |
| Groq | https://console.groq.com/keys | `whisper-large-v3-turbo` |
| Google Gemini | https://aistudio.google.com/apikey | `gemini-2.5-flash-lite` |
| Mistral | https://console.mistral.ai/apikeys | `voxtral-small-24b-2507` |
| ElevenLabs | https://elevenlabs.io/app/settings/api-keys | `scribe_v1` |

Any other OpenAI-compatible service works too: choose **Custom** in Settings
and fill in the base URL (e.g. `http://localhost:8080/v1` for a local server).

## Install (from source)

Prerequisites: [Rust](https://rustup.rs/) (MSVC toolchain) on Windows 10/11.

```powershell
git clone https://github.com/<you>/byok-stt.git
cd byok-stt
powershell -ExecutionPolicy Bypass -File scripts\install.ps1
```

The install script builds the release binary, copies it to
`%LOCALAPPDATA%\byok-stt\`, and creates a Start Menu shortcut so the app shows
up in Windows search. No admin rights needed.

Prefer manual?

```powershell
cargo build --release
.\scripts\add-start-menu-shortcut.ps1   # optional: Start Menu entry
.\target\release\byok-stt.exe           # launch
```

### First run

1. Launch **byok-stt** from the Start Menu (or the exe directly).
2. A microphone icon appears in the system tray.
3. Right-click the tray icon → **Settings** → paste your API key, pick a
   provider and model → **Save**.
4. Hold **Ctrl+Win**, speak, release.

## Using it

| Action | Result |
|---|---|
| Hold the hotkey + speak | Recording starts (red bubble) |
| Release keys | Transcribes (amber spinner) → pastes text |
| Tray / bubble left-click | Start / stop recording |
| Drag the bubble | Move it (position is remembered) |
| Bubble right-click | Settings menu |
| Tray icon right-click | Settings, Exit |
| Launch while already running | Balloon reminder (single instance) |

Whisper/voice-file style models (`whisper`, `voxtral`, `*-transcribe`,
`scribe`) are sent to `{base}/audio/transcriptions`; everything else goes
through `{base}/chat/completions` with inline audio.

## Settings reference

| Setting | Default | Notes |
|---|---|---|
| Provider | OpenRouter | Presets for OpenAI, Groq, Gemini, Mistral, ElevenLabs, Custom |
| API base URL | per provider | Any OpenAI-compatible endpoint |
| API key | — | Stored locally in `%APPDATA%\byok-stt\config.json` |
| Model | per provider | Editable, with suggestions per provider |
| Hotkey | Ctrl+Win | Modifier (Ctrl/Alt/Shift/Win) + key (Space, A-Z, 0-9, F1-F12) |
| Keep bubble always visible | off | Gray idle bubble pinned on screen |
| Max recording (s) | 120 | 5-3600; a watchdog stops long sessions |

## Platform support

Windows 10/11 is fully supported today. The hotkey/config layer is
OS-independent (per-OS defaults: macOS `Cmd+Space`, Linux `Ctrl+Space`), and
macOS/Linux backends are on the roadmap — they require different global-hook,
audio-capture and paste implementations (contributions welcome).

## Troubleshooting

- **Nothing gets pasted** — check the tray balloon or
  `%APPDATA%\byok-stt\log.txt` for the error (bad key, no network, ...).
- **Windows key feels "stuck"** (typing opens shortcuts) — the app force-
  releases Win/Shift/Alt after every session; tap `Win` once if it ever
  happens mid-bug.
- **Wrong language recognized** — pick a model that matches your language;
  transcription quality depends on the provider.

## Data & privacy

- Config: `%APPDATA%\byok-stt\config.json` (API key, model, base URL)
- Indicator position: `%APPDATA%\byok-stt\indicator_pos.json`
- Log: `%APPDATA%\byok-stt\log.txt`
- Audio: **RAM only** — never persisted

## License

MIT — see [LICENSE](LICENSE).
