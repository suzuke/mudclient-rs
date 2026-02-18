---
name: bdi-mental-states
description: |
  BDI 認知模型 Skill。信念-慾望-意圖 (Belief-Desire-Intention) 框架，讓 Agent 具備主動思考能力。
  面對複雜、模糊或長期任務時自動啟用 BDI 循環。
license: MIT
compatibility: Pixiu Agent / Claude Code / Cursor
metadata:
  author: pixiu
  version: "1.0"
  category: core
---

<!--
Source: https://github.com/guanyang/antigravity-skills
Concept: Belief-Desire-Intention (BDI) Model
Audited By: Pixiu Agent
Date: 2026-01-30
-->

# 🧠 BDI Mental States / BDI 認知模型

為了讓 Agent 不只是「被動執行指令」，而是具備「主動思考能力」，本 Skill 引入 BDI 模型。
當面對複雜、模糊或長期任務時，必須先經過 BDI 循環。

## 1. Belief (信念) - 我知道什麼？
*   **World State**: 當前的環境狀態 (檔案、Server、User Context)。
*   **Resources**: 我有哪些 Skill 可用？(Payment, Debugging, UI Design)。
*   **Constraints**: 有哪些硬限制？(User Rules, Security Policy)。

**自我提問**：
> "基於我對 Pixiu OS 的理解 (Belief)，目前使用者的專案狀態是什麼？"

## 2. Desire (慾望) - 我想要達成什麼？
*   **Goals**: 用戶的最終目標 (Objective)。
*   **Preferences**: 用戶的偏好 (Clean Code, Secure by Default)。
*   **Priorities**: 哪個目標最優先？(Security > Feature)。

**自我提問**：
> "為了滿足用戶 (Desire)，最佳的理想狀態是什麼？"

## 3. Intention (意圖) - 我計畫做什麼？
*   **Plan**: 具體的行動計畫 (Implementation Plan)。
*   **Commitment**: 我承諾要執行的下一步。
*   **Action**: 實際的 Tool Call。

**自我提問**：
> "綜合 Belief 與 Desire，我現在的具體意圖 (Intention) 是執行這三個步驟..."

---

## 實際應用流程

當收到模糊指令（如「讓它變好用」）時：

1.  **Update Beliefs**: 掃描專案，理解現狀。
2.  **Generate Desires**: 根據 Best Practices (UI/UX Pro Max)，列出可能的改進點。
3.  **Deliberate (思辨)**: 過濾掉不可行或高風險的 Desire。
4.  **Form Intentions**: 選擇一個改進點，制定計畫。
5.  **Execute**: 執行。

## 觸發機制
當 `task_boundary` 的 `TaskStatus` 顯示 "Planning" 或與決策相關時，啟動 BDI 檢核。
