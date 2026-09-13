<div align="center">

<img src="assets/app_256.png" width="110" alt="byok-stt icon"/>

# byok-stt

**Windows 按鍵說話語音輸入工具 — 帶上你自己的 AI 金鑰。**

按住快捷鍵、說話、放開 — 文字自動出現。

[![GitHub release](https://img.shields.io/github/v/release/Jackychan122/byok-stt)](https://github.com/Jackychan122/byok-stt/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-blue)

[English](README.md) | 繁體中文 | [简体中文](README.zh-CN.md)

<img src="docs/demo-quickstart.gif" width="383" alt="byok-stt 操作示範：轉寫時氣泡轉為黃色，完成後文字出現"/>

</div>

---

## 為什麼選擇 byok-stt？

大部分語音輸入工具都把你鎖定在單一雲端服務。**byok-stt 兼容任何
OpenAI 格式的語音轉文字服務** — OpenRouter、OpenAI、Groq、Google Gemini、
Mistral、ElevenLabs，甚至你自架的端點。你只需帶上 API 金鑰，其他交給程式。
音訊永不落盤，整個程式只是一個約 1 MB 的執行檔。

## 功能

- **全域快捷鍵** — 在任何地方按住、說話、放開 → 文字自動貼上當前視窗
  （並複製到剪貼簿）。預設 `Ctrl+Win`（Windows）、`Cmd+Space`（macOS）、
  `Ctrl+Space`（Linux），可完全自訂。
- **任意服務提供商** — 內建 OpenRouter、OpenAI、Groq、Google Gemini、
  Mistral、ElevenLabs 預設，也可自行編輯 API 網址接上任何 OpenAI 相容端點。
- **螢幕指示氣泡** — 只在錄音（紅色）或轉寫（黃色旋轉）時出現的小圓點。
  可拖曳移動、可設定常駐顯示；左鍵開始／停止錄音，右鍵開啟設定。
- **系統列圖示** — 狀態顏色變化，左鍵切換錄音，右鍵選單。
- **安全防護** — 可設定最長錄音時間、單一實例提示、自動釋放卡住的修飾鍵。
- **隱私優先** — 音訊只存在於記憶體，直接送往你選擇的服務。
  不儲存、不追蹤、不上報。
- **輕量** — 單一執行檔，約 10 MB 記憶體。

## 快速開始

1. 到 [**Releases**](https://github.com/Jackychan122/byok-stt/releases) 下載
   `byok-stt-v*-windows-x64.zip` 並解壓縮。
2. Double-click **`byok-stt.exe`** — 系統列會出現麥克風圖示。
3. 右鍵點托盤圖示 → **Settings**：

   <img src="docs/settings.png" width="466" alt="設定視窗"/>

4. 貼上你的 API key、選擇 Provider 與 Model → **Save**。
   （取得金鑰：[OpenRouter](https://openrouter.ai/keys) ·
   [OpenAI](https://platform.openai.com/api-keys) ·
   [Groq](https://console.groq.com/keys) ·
   [Gemini](https://aistudio.google.com/apikey) — 任何 OpenAI 相容端點都可
   選 **Custom** 自行填入網址。）
5. 按住 **Ctrl+Win** 說話，放開 — 文字自動貼上 ✨ 氣泡會顯示目前狀態
   （灰色＝閒置）：

<div align="center">
<img src="docs/bubble-idle.png" width="106" alt="閒置氣泡（灰色）"/><br/>
<i>指示氣泡：灰色＝閒置、紅色＝錄音中、黃色＝轉寫中。</i>
</div>

## 設定說明

| 設定 | 預設值 | 說明 |
|---|---|---|
| Provider | OpenRouter | 預設：OpenAI、Groq、Gemini、Mistral、ElevenLabs、Custom |
| API base URL | 依 Provider | 任何 OpenAI 相容端點 |
| API key | — | 儲存於本機 `%APPDATA%\byok-stt\config.json` |
| Model | 依 Provider | 可編輯下拉選單，附建議清單 |
| Hotkey | Ctrl+Win | 修飾鍵（Ctrl/Alt/Shift/Win）+ 按鍵（Space、A–Z、0–9、F1–F12）|
| 常駐顯示氣泡 | 關 | 灰色閒置氣泡固定在螢幕上 |
| 最長錄音（秒）| 120 | 5–3600；看門狗會自動停止過長錄音 |

## Provider 與模型

| Provider | 金鑰申請頁 | 建議模型 |
|---|---|---|
| OpenRouter | https://openrouter.ai/keys | `openai/whisper-large-v3-turbo` |
| OpenAI | https://platform.openai.com/api-keys | `gpt-4o-mini-transcribe` |
| Groq | https://console.groq.com/keys | `whisper-large-v3-turbo` |
| Google Gemini | https://aistudio.google.com/apikey | `gemini-2.5-flash-lite` |
| Mistral | https://console.mistral.ai/apikeys | `voxtral-small-24b-2507` |
| ElevenLabs | https://elevenlabs.io/app/settings/api-keys | `scribe_v1` |

檔案型模型（`whisper`、`voxtral`、`*-transcribe`、`scribe`）會送往
`{base}/audio/transcriptions`；其他模型走 `{base}/chat/completions`
夾帶音訊。

## 操作說明

| 操作 | 效果 |
|---|---|
| 按住快捷鍵 + 說話 | 開始錄音（紅色氣泡）|
| 放開按鍵 | 轉寫（黃色旋轉）→ 自動貼上文字 |
| 托盤／氣泡左鍵 | 開始／停止錄音 |
| 拖曳氣泡 | 移動位置（自動記憶）|
| 氣泡右鍵 | 設定選單 |
| 托盤圖示右鍵 | Settings、Exit |
| 重複啟動程式 | 彈出「已在運行」提示（單一實例）|

## 從原始碼建置

需求：Windows 10/11 上的 [Rust](https://rustup.rs/)。

```powershell
git clone https://github.com/Jackychan122/byok-stt.git
cd byok-stt
powershell -ExecutionPolicy Bypass -File scripts\install.ps1
```

腳本會建置執行檔、安裝到 `%LOCALAPPDATA%\byok-stt` 並建立開始功能表捷徑。
不需要系統管理員權限。

## 平台支援

目前完整支援 Windows 10/11。快捷鍵與設定層與作業系統無關
（各平台預設：macOS `Cmd+Space`、Linux `Ctrl+Space`），macOS/Linux
後端在規劃中 — 需要不同的全域鉤子、音訊擷取與貼上實作
（歡迎貢獻）。

## 隱私

- **音訊永不寫入磁碟** — 只存在於記憶體，直接送往你設定的服務，
  轉寫完成立即釋放。
- 設定、日誌與氣泡位置儲存在 `%APPDATA%\byok-stt\`。
- 沒有遙測、沒有分析、沒有回傳。

## 疑難排解

- **沒有文字貼上** — 查看托盤通知或 `%APPDATA%\byok-stt\log.txt`
  的錯誤訊息（金鑰錯誤、網路問題等）。
- **Windows 鍵好像「卡住」** — 程式會在每次結束後自動強制釋放
  Win/Shift/Alt；若真的遇到，按一下 `Win` 鍵即可復位。
- **搜尋找不到圖示** — 重新執行 `scripts\install.ps1` 重新整理
  開始功能表捷徑。

## 授權

[MIT](LICENSE) © byok-stt contributors
