# Changelog

All notable changes to byok-stt are documented here.
Format loosely follows [Keep a Changelog](https://keepachangelog.com/); versions follow [SemVer](https://semver.org/).

## [0.1.3] - 2026-09-15

### Added
- First-run onboarding: launching without an API key opens Settings automatically with a welcome hint, instead of failing only after the first dictation.
- Transcription prompt (optional): a style hint sent with every request — honored as the Whisper `prompt` field by OpenAI, Groq and self-hosted endpoints, and as a style instruction for chat audio models. Settings warns when the selected combination cannot honor it (OpenRouter transcription models currently ignore `prompt`).
- Qwen ASR: `qwen/qwen3-asr-1.7b` added to the OpenRouter model suggestions.
- Traditional Chinese output (optional): converts simplified results to traditional before pasting. Embeds OpenCC (Apache-2.0) phrase + character data, so ambiguous characters resolve by context (头发→頭髮, 出发→出發). No external files needed.
- Start with Windows (optional): per-user autostart entry (HKCU Run), toggled in Settings. No admin required.
- Full Traditional Chinese and Simplified Chinese READMEs.

### Fixed
- Models named `*-asr` (e.g. `qwen/qwen3-asr-1.7b`) are routed to `/audio/transcriptions` instead of `chat/completions` — OpenRouter rejects the latter with HTTP 400.
- The indicator bubble can no longer be covered by later shell surfaces (Start menu, search, IME flyouts): its TOPMOST z-order is re-asserted on every state change and every ~2 s.

### Changed
- Prompt field is a 3-line editor with a vertical scrollbar.
- Settings screenshot moved next to its Quick-start step; indicator-bubble image captioned.

## [0.1.2] - 2026-09-13

### Added
- Provider presets: OpenRouter, OpenAI, Groq, Google Gemini, Mistral, ElevenLabs + Custom (editable base URL and model).
- Configurable hotkey (modifier + key) with per-OS defaults; max recording length (5–3600 s) with a watchdog.
- Draggable always-on indicator bubble (optional); tray left-click toggles recording; right-click menus.
- Single-instance handoff and automatic release of stuck modifier keys before pasting.
- Exe icon embedded at build time; `scripts/install.ps1` (per-user install + Start Menu shortcut).

### Changed
- Statically linked CRT — the released exe runs on clean Windows with zero runtime dependencies.
- GitHub Actions release workflow builds and attaches the zip on tag push.

## [0.1.1] - 2026-09-13

### Added
- Release automation via GitHub Actions (tag push → Windows build → zip → GitHub Release).

## [0.1.0] - 2026-09-13

### Added
- Initial release: hold-the-hotkey push-to-talk dictation for Windows (BYOK).
- OpenAI-compatible speech-to-text via `/audio/transcriptions` (file models) or `chat/completions` (inline audio).
- Text pasted into the focused window and copied to the clipboard.
- System tray icon with state colors, settings window with persistent config, and an on-screen indicator bubble (recording / transcribing states).
