---
name: brainstorming
description: |
  腦力激盪 Skill。Socratic 式需求探索：一次只問一個問題，逐步釐清需求後提出 2-3 方案比較。
  適用於需求探索、方案選型、產品與技術方向決策。
license: MIT
compatibility: Pixiu Agent / Claude Code / Cursor
metadata:
  author: pixiu
  version: "1.1.0"
  category: workflow
  tags: [brainstorming, ideation, decision, options]
---

<!--
Source: https://github.com/obra/superpowers
Commit: e16d611eee14ac4c3253b4bf4c55a98d905c2e64
Concept: brainstorming
Audited By: Pixiu Agent
Date: 2026-02-16
-->

# 💡 Brainstorming / 腦力激盪

目標不是立刻選答案，而是透過結構化對話，先理解問題全貌，再擴大解法空間，最終系統化收斂。

## Hard Gate / 硬閘門

> ⚠️ **設計未經用戶核准，不得開始實作。**
> （此規則與 user_rules「先對齊理解＋多方案」一致，不重複定義。）

無論任務看起來多「簡單」，都必須先完成本流程。
「簡單」的任務正是未經檢驗的假設最容易造成浪費的地方。

## 工作流程（Socratic 式）

### Phase 1：探索脈絡

- 先檢查專案現況（檔案結構、文件、近期 commit）
- 理解問題背景與約束

### Phase 2：逐一提問

- **一次只問一個問題**，不要一次丟出多個問題
- 優先使用選擇題（比開放式更容易回答）
- 聚焦於：目的、限制、成功標準
- 若一個主題需要進一步探索，拆成多個問題

### Phase 3：提出方案

- 提出 **2-3 個不同方案**，涵蓋不同取捨方向
- 每個方案必須包含：
  - 優點 / 代價 / 風險 / 適用情境
- 先說明推薦方案及推薦理由
- 標明假設與取捨

### Phase 4：分段呈現設計

- 設計按章節呈現，每一段後確認用戶是否同意再繼續
- 涵蓋：架構、元件、資料流、錯誤處理、測試策略
- 複雜度低的章節用幾句話說清楚即可
- 準備好回頭修正（用戶可能要求調整）

### Phase 5：文件化與交接

- 將核准的設計寫入文件（如 `docs/` 下）
- Commit 設計文件
- 進入實作階段

## 評估矩陣（簡版）

```markdown
| 方案 | 影響 | 複雜度 | 成本 | 風險 | 可逆性 | 備註 |
|------|------|--------|------|------|--------|------|
|      |      |        |      |      |        |      |
```

## 行為規則

- 一次只問一個問題
- 選擇題優先於開放式問題
- YAGNI：從設計中移除不必要的功能
- 不把單一偏好包裝成唯一正解
- 分段呈現、逐段核准
- 優先給可落地的下一步（POC、實驗、驗證點）
