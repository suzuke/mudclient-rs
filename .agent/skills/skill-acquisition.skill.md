---
name: skill-acquisition
description: 缺少 Skill/Workflow 時的「智慧查找 + 安全審核」規則
license: MIT
compatibility: Pixiu Agent
metadata:
  author: pixiu
  version: "1.0.0"
  category: general
  tags: [security, audit, sop, external]
---

# 🔍 Skill Acquisition / 外部技能獲取

當本地 `.agent/skills/` 缺少所需技能時，請遵循此規則進行查找與審核。

## 1. 查找策略 (Discovery)
1.  **本地優先**：先檢查 `docs/` 或 `.agent/` 是否有類似文件。
2.  **官方來源**：優先參考官方文檔或 Antigravity 官方庫。
3.  **可信來源**：GitHub Stars > 100，且近期有更新。

## 2. 安全審核 (Security Audit)
對任何外部 Skill/Workflow 進行嚴格審查：
*   **✅ 允許**：純文字流程、Prompt、檢查清單。
*   **⚠️ 需降權**：`npm install -g` -> `npm install`。
*   **⛔ 禁止**：
    *   執行未知腳本 (`curl | bash`).
    *   讀取敏感資料 (`.env`, `id_rsa`).
    *   上傳代碼到未知伺服器。

## 3. 內化 (Internalization)
*   不要直接複製貼上。
*   根據專案 `tech-stack` 與 `security` 規則重寫。
*   確保符合 Pixiu YAML Frontmatter 格式。

## 4. 溯源 (Traceability)
在檔案頭部註明來源：
```markdown
<!--
Source: url
Audited By: Pixiu Agent
Date: YYYY-MM-DD
-->
```
