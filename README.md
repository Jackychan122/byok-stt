<div align="center">

<img src="assets/app_256.png" width="110" alt="byok-stt icon"/>

# byok-stt

**Push-to-talk voice typing for Windows — bring your own AI key.**

Hold the hotkey, speak, release — your words appear as text.

[![GitHub release](https://img.shields.io/github/v/release/Jackychan122/byok-stt)](https://github.com/Jackychan122/byok-stt/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-blue)

English | [繁體中文（香港）](README.zh-HK.md) | [简体中文](README.zh-CN.md)

<img src="docs/demo-quickstart.gif" width="383" alt="byok-stt in action: the bubble turns yellow while transcribing, then the text appears"/>

</div>

---

## Why byok-stt?

Most voice-typing tools lock you into one cloud service. **byok-stt works with
any OpenAI-compatible speech-to-text provider** — OpenRouter, OpenAI, Groq,
Google Gemini, Mistral, ElevenLabs, or your own self-hosted endpoint. You bring
the API key; the app does the rest. Audio never touches disk, and the whole
thing is a single ~1 MB executable.

## Features

- **Global hotkey** — hold it anywhere, dictate, release → text is pasted into
  whatever app has focus (and copied to the clipboard). Default `Ctrl+Win`
  (Windows), `Cmd+Space` (macOS), `Ctrl+Space` (Linux) — fully customizable.
- **Any provider** — presets for OpenRouter, OpenAI, Groq, Google Gemini,
  Mistral and ElevenLabs, plus an editable base URL for anything else that
  speaks the OpenAI format.
- **On-screen bubble** — a small indicator that only appears while you dictate
  (red) or transcribe (yellow spinner). Draggable, optional always-on gray mode,
  left-click to start/stop, right-click for settings.
- **Tray icon** — state colors, left-click toggles recording, right-click menu.
- **Safety nets** — configurable max recording length, single-instance
  handoff, and automatic release of stuck modifier keys.
- **Private by design** — audio lives in RAM only and goes straight to the
  provider you chose. Nothing is stored, nothing is telemetry'd.
- **Tiny** — one static executable, ~10 MB RAM.

## Quick start

1. Download `byok-stt-v*-windows-x64.zip` from
   [**Releases**](https://github.com/Jackychan122/byok-stt/releases) and unzip it.
2. Double-click **`byok-stt.exe`** — a microphone icon appears in the system tray.
3. Right-click the tray icon → **Settings**:

   <img src="docs/settings.png" width="466" alt="Settings window"/>

4. Paste your API key, pick a provider and model → **Save**.
   (Get a key: [OpenRouter](https://openrouter.ai/keys) ·
   [OpenAI](https://platform.openai.com/api-keys) ·
   [Groq](https://console.groq.com/keys) ·
   [Gemini](https://aistudio.google.com/apikey) — any OpenAI-compatible
   endpoint works via **Custom**.)
5. Hold **Ctrl+Win**, speak, release. ✨ The bubble shows the state at a
   glance — gray while idle:

<div align="center">
<img src="docs/bubble-idle.png" width="106" alt="Idle bubble (gray)"/><br/>
<i>The indicator bubble: gray while idle, red while recording, yellow while transcribing.</i>
</div>

## Settings reference

| Setting | Default | Notes |
|---|---|---|
| Provider | OpenRouter | Presets: OpenAI, Groq, Gemini, Mistral, ElevenLabs, Custom |
| API base URL | per provider | Any OpenAI-compatible endpoint |
| API key | — | Stored locally in `%APPDATA%\byok-stt\config.json` |
| Model | per provider | Editable dropdown with suggestions |
| Prompt (optional) | empty | Style hint sent with every transcription — e.g. colloquial Cantonese. Honored by OpenAI, Groq and self-hosted Whisper endpoints; passed as a style instruction to chat audio models. OpenRouter's transcription endpoint currently accepts but ignores it. |
| Hotkey | Ctrl+Win | Modifier (Ctrl/Alt/Shift/Win) + key (Space, A–Z, 0–9, F1–F12) |
| Keep bubble always visible | off | Gray idle bubble pinned on screen |
| Max recording (s) | 120 | 5–3600; a watchdog stops long sessions |

## Providers & models

| Provider | Key page | Suggested model |
|---|---|---|
| OpenRouter | https://openrouter.ai/keys | `openai/whisper-large-v3-turbo` |
| OpenAI | https://platform.openai.com/api-keys | `gpt-4o-mini-transcribe` |
| Groq | https://console.groq.com/keys | `whisper-large-v3-turbo` |
| Google Gemini | https://aistudio.google.com/apikey | `gemini-2.5-flash-lite` |
| Mistral | https://console.mistral.ai/apikeys | `voxtral-small-24b-2507` |
| ElevenLabs | https://elevenlabs.io/app/settings/api-keys | `scribe_v1` |

File-style models (`whisper`, `voxtral`, `*-transcribe`, `scribe`) are sent to
`{base}/audio/transcriptions`; everything else goes through
`{base}/chat/completions` with inline audio.

## Usage

| Action | Result |
|---|---|
| Hold the hotkey + speak | Recording starts (red bubble) |
| Release keys | Transcribes (yellow spinner) → pastes text |
| Tray / bubble left-click | Start / stop recording |
| Drag the bubble | Move it (position is remembered) |
| Bubble right-click | Settings menu |
| Tray icon right-click | Settings, Exit |
| Launch while already running | Balloon reminder (single instance) |

## Install from source

Prerequisites: [Rust](https://rustup.rs/) on Windows 10/11.

```powershell
git clone https://github.com/Jackychan122/byok-stt.git
cd byok-stt
powershell -ExecutionPolicy Bypass -File scripts\install.ps1
```

The script builds the release binary, installs it to `%LOCALAPPDATA%\byok-stt`
and creates a Start Menu shortcut. No admin rights needed.

## Platform support

Windows 10/11 is fully supported today. The hotkey/config layer is
OS-independent (per-OS defaults: macOS `Cmd+Space`, Linux `Ctrl+Space`), and
macOS/Linux backends are on the roadmap — they require different global-hook,
audio-capture and paste implementations (contributions welcome).

## Privacy

- **Audio is never written to disk** — it lives in RAM, goes straight to the
  provider you configured, and is dropped immediately after transcription.
- Config, logs and the bubble position live in `%APPDATA%\byok-stt\`.
- No telemetry, no analytics, no phone-home.

## Troubleshooting

- **Nothing gets pasted** — check the tray balloon or
  `%APPDATA%\byok-stt\log.txt` for the error (bad key, no network, …).
- **Windows key feels "stuck"** — the app force-releases Win/Shift/Alt after
  every session; tap `Win` once if it ever happens mid-bug.
- **App icon missing in search** — re-run `scripts\install.ps1` to refresh the
  Start Menu shortcut.

## License

[MIT](LICENSE) © byok-stt contributors
