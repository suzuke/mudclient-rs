---
name: research
description: |
  研究與調研 Skill。Findings 管理、知識持久化、技術調研記錄。
  進行複雜技術調研、API 規格查詢或外部文檔閱讀時使用。
license: MIT
compatibility: Pixiu Agent / Claude Code / Cursor
metadata:
  author: pixiu
  version: "1.0"
  category: workflow
---

<!--
Source: https://github.com/OthmanAdi/planning-with-files
Concept: 3-File Pattern (Specifically findings.md)
Audited By: Pixiu Agent
Date: 2026-01-30
-->

# 🔍 Research & Findings / 研究與調研

當進行複雜技術調研、API 規格查詢或外部文檔閱讀時，**禁止**僅將結果輸出於對話 (Volatile Memory)。
必須將有價值的資訊存入 `findings.md` (Persistent Memory)。

## 1. Findings 檔案結構
檔案位置：`<appDataDir>/brain/<conversation-id>/findings.md` (與 task.md 同層級)

```markdown
# 📚 Project Findings

## [YYYY-MM-DD] <Topic Name>
- **Source**: <URL>
- **Key Takeaways**:
  - <Point 1>
  - <Point 2>
- **Code Snippets**:
  ...
```

## 2. 何時使用？
*   **API 規格**: 當查詢 Stripe/AWS API 時，將 endpoint 結構寫入。
*   **技術選型**: 當比較 Library A vs B 時，將優缺點表格寫入。
*   **架構決策**: 當決定資料庫 Schema 時，將 ERD 描述寫入。

## 3. 與其他 Artifacts 的關係
*   **task.md**: 定義「要做什麼研究」。
*   **findings.md**: 儲存「研究發現了什麼」 (Knowledge)。
*   **walkthrough.md**: 記錄「根據發現做了什麼」 (Action)。
*   **implementation_plan.md**: 基於 Findings 制定「計畫」。

## 4. 自動化觸發
當執行 `search_web` 或 `read_url_content` 後，若內容超過 500 字或包含重要 spec，應主動更新 `findings.md`。
