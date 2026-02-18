# AI Skill 觸發規則

version: 1.1.0
trigger: always_on
alwaysApply: true

---

## 目的

此規則讓 AI 在對話中偵測到特定關鍵字時，主動建議用戶啟用與下載相關的 Skills 或 Workflows。

---

## 觸發規則

當對話中出現以下關鍵字時，AI **應**在回覆中加入對應的建議：

### 🧠 0. 核心優化 (Context & Cognition)
> 防止 AI 變笨、遺忘或喪失動機。

| 關鍵字 | 建議 Skill | 原因 |
|--------|------------|------|
| 變笨、忘記、遺忘、忘了、前後文 | `context-optimization.skill.md` | 上下文衰退 (Degradation) 防止 |
| 慢、卡頓、token、context window | `context-optimization.skill.md` | Token 預算管理 |
| 總結、summary、回顧 | `context-optimization.skill.md` | 記憶壓縮 |
| 思考、計畫、plan、意圖、intention | `bdi-mental-states.skill.md` | 啟動 BDI 深度思考模型 |
| 為什麼、動機、reasoning、reason | `bdi-mental-states.skill.md` | 解釋 AI 行為背後的動機 |

### 🎨 3. UI/UX 與前端設計
> 請參考 `ui-design.skill.md`。

| 關鍵字 | 建議 Skill | 原因 |
|--------|------------|------|
| UI、UX、介面、設計、樣式、CSS、Style | `ui-design.skill.md` | 載入 Design System 規範 |
| RWD、手機版、響應式 | `ui-design.skill.md` | 使用 Mobile First 原則 |
| 無障礙、a11y、讀屏器 | `ui-design.skill.md` | 符合 WCAG 標準 |
| Tailwind、class、樣式庫 | `ui-design.skill.md` | 強制參考 MASTER.md 定義 |
| 設計系統、Design System、主題、Theme | `ui-design.skill.md` | 啟動 Design System Workflow |

### 🔐 3.1 資安與權限 (High Risk)
> 涉及金錢交易，需強制合規。

| 關鍵字 | 建議 Skill | 原因 |
|--------|------------|------|
| 金流、支付、payment、stripe、ecpay、綠界、藍新 | `payment.skill.md` + `security.skill.md` | PCI DSS 合規、防止資料外洩 |
| 訂閱、subscription、recurring、invoice、發票 | `payment.skill.md` | 交易一致性、冪等性設計 |
| 錢包、wallet、點數、credit | `payment.skill.md` | 小數點精度問題 |

### 🎨 2. 前端開發
> 確保 UI/UX 品質與無障礙設計。

| 關鍵字 | 建議 Skill | 原因 |
|--------|------------|------|
| UI、UX、介面、設計、樣式、CSS、Tailwind | `ui-design.skill.md` | 響應式設計、視覺一致性 |
| RWD、手機版、mobile、component | `ui-design.skill.md` | 組件化規範 |
| 無障礙、a11y、aria | `ui-design.skill.md` | 網頁親和力標準 |

### 🔒 3. 認證 / 權限
> 核心安全防護。

| 關鍵字 | 建議 Skill | 原因 |
|--------|------------|------|
| 認證、登入、auth、JWT、OAuth、SSO | `security.skill.md` | 防止 Session 劫持、Token 安全 |
| 權限、permission、RBAC、ACL、middleware | `security.skill.md` | 最小權限原則 |

### 🌐 4. API 設計
> 確保介面一致性與可維護性。

| 關鍵字 | 建議 Skill | 原因 |
|--------|------------|------|
| API、端點、REST、GraphQL、Webhook | `api-design.skill.md` | RESTful 規範、版本控制 |
| 第三方串接、integration、json | `api-design.skill.md` | 錯誤處理、重試機制 |

### 🗄️ 5. 資料庫
> 資料結構與正規化。

| 關鍵字 | 建議 Skill | 原因 |
|--------|------------|------|
| 資料庫、database、schema、migration | `db-schema.skill.md` + `db-migrate.md` | 正規化、索引策略、遷移流程 |
| 多租戶、multi-tenant、SaaS | `db-schema.skill.md` | RLS 設計、租戶隔離 |
| SQL、query、join | `performance.skill.md` | 防止 N+1、效能優化 |

### 🚀 6. 部署與維運
> 線上環境穩定性。

| 關鍵字 | 建議 Workflow | 原因 |
|--------|------------|------|
| 部署、deploy、上線、release、CI/CD | `deploy.md` + `release.md` | 部署檢核點、回滾策略 |
| Docker、container、k8s、image | `deploy.md` | 容器安全、環境變數管理 |
| 環境變數、env、config | `tech-stack.skill.md` | 敏感配置管理 |

### 🧪 7. 測試與品質
> 確保程式碼穩定性。

| 關鍵字 | 建議 Workflow/Skill | 原因 |
|--------|---------------------|------|
| 測試、test、unit test、e2e、jest | `testing.md` | 測試覆蓋率、測試案例設計 |
| 重構、refactor、優化、clean code | `code-review.skill.md` | 程式碼品質、命名規範 |
| 命名、naming、變數 | `naming-convention.skill.md` | 命名一致性 |

### 🐛 8. 除錯與錯誤處理
> 提升系統韌性。

| 關鍵字 | 建議 Skill | 原因 |
|--------|------------|------|
| 錯誤、error、exception、bug、crash、報錯 | `debugging.skill.md` + `error-handling.skill.md` | 啟動 4 階段除錯流程 |
| log、日誌、monitor、trace、root cause | `debugging.skill.md` | Log 分析與根因追蹤 |
| fix、修復、壞了、not working | `debugging.skill.md` | 避免 Trial and Error |

### 🛠️ 9. Git 操作
> 版本控制規範。

| 關鍵字 | 建議 Workflow | 原因 |
|--------|------------|------|
| commit、提交、PR、merge | `commit.md` | Conventional Commits、分支策略 |
| 版本、version、tag | `release.md` | 版本號管理 (SemVer) |

### 🔍 10. 外部技能查找 (New Skill)
> 本地無對應規則時，啟動安全查找流程。

| 關鍵字 | 建議 Workflow | 原因 |
|--------|------------|------|
| 查找、search、research、找一下、新功能 | `skill-acquisition.md` + `research.skill.md` | 安全審核 + Findings 記錄 |
| 怎麼做、如何做、how to | `skill-acquisition.md` | 獲取最佳實踐 |
| 外部、external、source | `skill-acquisition.md` | 來源溯源 |
| 規格、spec、文檔、docs | `research.skill.md` | API 規格記錄 |

### 🎨 11. 視覺化思考 (New Skill)
> 使用 Mermaid 或 Canvas 進行架構視覺化。

| 關鍵字 | 建議 Skill | 原因 |
|--------|------------|------|
| 畫圖、diagram、流程圖、flowchart | `visualization.skill.md` | 使用 Mermaid 繪製流程 |
| 架構圖、拓撲圖、topology、structure | `visualization.skill.md` | 使用 Canvas 繪製架構 |
| 關係圖、mindmap、腦圖 | `visualization.skill.md` | 複雜關係可視化 |

---

## AI 行為指引

1.  **優先順序**：安全性相關 (Payment/Security) > 功能性相關。
2.  **不要過度提示**：每個對話 Session 中，針對同一類 Skill 最多提示一次，避免煩人。
3.  **已安裝檢查**：如果 `.agent/skills/` 或 `.agent/workflows/` 中已有該檔案，**不要建議下載**，而是說：「根據已安裝的 `xxx.skill.md`，建議...」。

---

## 範例對話

**用戶**：「我想做金流功能，串接藍新」

**AI 回覆**：
> 💡 **安全提示**：偵測到「金流」需求。
> 根據 **PCI DSS 資安規範**，建議啟用 `payment.skill.md` 來確保交易安全。
>
> **指令**：
> `curl -sSL https://raw.githubusercontent.com/MDFireLin/Pixiu-Agent-3280/main/.pixiu3280os/skills/payment.skill.md -o .agent/skills/payment.skill.md`

