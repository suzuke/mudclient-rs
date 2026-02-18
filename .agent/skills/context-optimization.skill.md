---
name: context-optimization
description: |
  上下文優化 Skill。專注力管理、Token 節省、漸進式揭露、記憶卸載。
  防止 AI 在長對話中出現注意力稀釋或遺忘。
license: MIT
compatibility: Pixiu Agent / Claude Code / Cursor
metadata:
  author: pixiu
  version: "1.0"
  category: core
---

<!--
Source: https://github.com/muratcankoylan/Agent-Skills-for-Context-Engineering
Concept: Context Degradation & Progressive Disclosure
Audited By: Pixiu Agent
Date: 2026-01-30
-->

# 🧠 Context Optimization / 上下文優化

為了防止 AI 在長對話中出現「注意力稀釋 (Attention Dilution)」或「遺忘 (Context Degradation)」，必須遵守以下優化策略。

## 1. 漸進式揭露 (Progressive Disclosure)
不要一次讀取所有檔案。
*   **原則**：只讀取當前任務**絕對必要**的檔案。
*   **操作**：使用 `grep` 或 `ls` 先確認範圍，再使用 `view_file` 讀取特定段落。
*   **禁止**：禁止使用 `cat` 一次讀取整個巨型專案結構。

## 2. 外部記憶卸載 (Memory Offloading)
將狀態儲存於 Artifacts，而非依賴對話歷史。
*   **Task State**: 使用 `task.md` 記錄當前進度與 TODO。
*   **Verification**: 使用 `walkthrough.md` 記錄測試結果。
*   **Design**: 使用 `implementation_plan.md` 記錄設計決策。
*   **好處**：即使 Context Window 滿了，只要讀取 Artifacts 就能恢復狀態。

## 3. 週期性壓縮 (Periodic Compression)
當對話超過 20 輪，或完成一個大任務 (Task Boundary) 時：
1.  **更新 Summary**：在 `task_boundary` 中提供精簡但完整的摘要。
2.  **清理噪音**：忽略之前的錯誤嘗試或 verbose 輸出，只關注最終結果。

## 4. 檔案閱讀優化
*   **大檔案 (>500行)**：
    *   先讀 `view_file_outline` (如有) 或只讀檔頭/介面定義。
    *   使用 `grep` 搜尋關鍵字。
    *   只 `view_file` 相關的函數區塊 (StartLine - EndLine)。

## 5. Token 預算意識
*   **System Prompt** 是昂貴的，不要重複輸出 System Rules。
*   **Tool Output** 如果過長 (如 `git log` 1000行)，必須重新執行並加上 `-n 20` 限制。
