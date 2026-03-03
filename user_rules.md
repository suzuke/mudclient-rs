# 📜 User Rules — 專案最高憲法

> **此檔案是 Pixiu Agent 的核心：專案級 AI 行為規範。**
> 無論使用 Cursor、Windsurf、Copilot 或 Gemini，此規則都將被強制執行。
> 全域規則（如 `~/.cursor/rules`）可能被忽略，但專案級規則不會。

---

## 🔒 硬閘門（最高優先）

- **最小改動原則**：只改達成目標所需最小範圍，嚴禁「順便重構」
- **白名單變更**：只修改指定路徑，未提供白名單時必須先詢問
- **高風險操作需確認**：刪檔 / 大規模重構 / DB schema / 新增套件，一律先說明風險並等待同意
- **禁止擴張需求**：不得自行重構、抽設定檔、加套件、加新頁面

---

## 🛡️ 安全規範

- 敏感資料放 `.env`，加入 `.gitignore`
- `.gitignore` 必含：`.env`、`node_modules/`、`dist/`、`.DS_Store`
- 禁止硬編碼 API Key、密碼、Token
- 禁止執行危險終端指令（`rm -rf`、`format`、`drop database`）

---

## 📐 程式碼風格

- 變數命名：camelCase
- 元件命名：PascalCase
- CSS 類名：kebab-case
- 使用 ES6+ 語法
- 關鍵 UI 與第三方呼叫必須 try-catch
- 單一模組錯誤不可導致全站停止

---

## 📝 文件規範

- `.md` 文件放 `docs/`（README.md 除外）
- 功能變更時須提醒同步更新相關文件（RoadMap、CHANGELOG）

---

## 🤖 AI 行為約束

- **零猜測政策**：runtime / framework / DB 版本一律不得猜測，必須從專案檔偵測
- **問句 = 討論**：句尾含「？」時，只回答與提出方案，不得直接改檔
- **語言**：所有回覆使用繁體中文（除非專案規範要求英文）

---

> 💡 **提示**：此為 Pixiu 預設模板，請根據您的專案需求修改。
> 使用 `Pixiu: 更新上下文` 指令讓所有 AI 重新載入此規則。
