---
name: visualization
description: |
  視覺化思考 Skill。架構圖、Canvas、Mermaid 流程圖。
  當文字不足以表達複雜架構、流程或關聯時使用。
license: MIT
compatibility: Pixiu Agent / Claude Code / Cursor
metadata:
  author: pixiu
  version: "1.0"
  category: documentation
---

<!--
Source: https://github.com/kepano/obsidian-skills
Concept: JSON Canvas (.canvas) & Visual Thinking
Audited By: Pixiu Agent
Date: 2026-01-30
-->

# 🎨 Visualization / 視覺化思考

當文字不足以表達複雜架構、流程或關聯時，**禁止**僅使用文字描述。
應優先使用視覺化工具來呈現 "Big Picture"。

## 1. 工具選擇策略

| 情境 | 推薦工具 | 檔案格式 |
|------|----------|----------|
| **流程圖 / 時序圖** | Mermaid | `.md` (code block) |
| **系統架構 / 拓撲圖** | Obsidian Canvas | `.canvas` |
| **腦力激盪 / 關係圖** | Obsidian Canvas | `.canvas` |
| **資料庫 ERD** | Mermaid | `.md` (code block) |

## 2. JSON Canvas (.canvas) 規範
Obsidian Canvas 是一個無限畫布格式。當需要產生 `.canvas` 檔案時，請遵循以下結構：
*   **Nodes**: 可以是 Text, File, Link, Group。
*   **Edges**: 用於連接 Nodes，表示依賴或流向。
*   **位置**: 確保節點不重疊 (x, y 座標需計算)。

## 3. Mermaid 最佳實踐
*   **方向**: 預設使用 `TD` (Top-Down) 或 `LR` (Left-Right)。
*   **樣式**: 避免過度依賴 CSS，保持原生可讀性。
*   **子圖**: 使用 `subgraph` 來群組化模組。

## 4. 應用場景
*   **Implementation Plan**: 在設計階段，附上 Mermaid 流程圖。
*   **Architecture Review**: 產生 `architecture.canvas` 來展示 Microservices 關係。
*   **Root Cause Analysis**: 使用 Canvas 畫出因果關係魚骨圖。
