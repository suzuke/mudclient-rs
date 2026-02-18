---
name: writing-plans
description: |
  大型任務規劃 Skill。先寫可執行計畫再動手，避免直接開工造成返工與漏項。
  適用於跨檔案修改、重構、多步驟交付、風險較高任務。
license: MIT
compatibility: Pixiu Agent / Claude Code / Cursor
metadata:
  author: pixiu
  version: "1.1.0"
  category: workflow
  tags: [planning, execution, milestones, risk]
---

<!--
Source: https://github.com/obra/superpowers
Commit: e16d611eee14ac4c3253b4bf4c55a98d905c2e64
Concept: writing-plans
Audited By: Pixiu Agent
Date: 2026-02-16
-->

# 🧭 Writing Plans / 撰寫執行計畫

當任務超過「單檔小改」，先寫計畫再實作。
目標是降低返工、提前暴露風險、讓用戶可快速決策。

## 何時必須啟用

- 跨 2 個以上檔案或模組
- 需要遷移/重構/版本升級
- 有相依順序（先做 A 才能做 B）
- 驗證成本高（測試、部署、資料變更）

## 計畫輸出格式

計畫必須包含以下區塊：

1. 目標與完成條件（Definition of Done）
2. 假設與限制（Assumptions / Constraints）
3. 實作步驟（按依賴順序）
4. 驗證方案（每步如何驗證）
5. 風險與回滾方案（至少列出高風險項）

## 細粒度任務拆分

**每個步驟應是 2-5 分鐘可完成的單一動作：**

- 「寫失敗測試」 — 一個步驟
- 「執行測試，確認失敗」 — 一個步驟
- 「實作最小程式碼讓測試通過」 — 一個步驟
- 「執行測試，確認通過」 — 一個步驟
- 「Commit」 — 一個步驟

## Task Structure 範本（完整版）

````markdown
### Task N: [元件名稱]

**檔案：**
- 建立: `exact/path/to/file.ts`
- 修改: `exact/path/to/existing.ts:123-145`
- 測試: `tests/exact/path/to/test.ts`

**Step 1: 寫失敗測試**

```typescript
describe('specificBehavior', () => {
  it('should return expected result', () => {
    const result = targetFunction(input);
    expect(result).toBe(expected);
  });
});
```

**Step 2: 執行測試，確認失敗**

執行: `npm test -- --testPathPattern=test.ts -t "specificBehavior"`
預期: FAIL — `targetFunction is not defined`

**Step 3: 實作最小程式碼**

```typescript
export function targetFunction(input: string): string {
  return expected;
}
```

**Step 4: 執行測試，確認通過**

執行: `npm test -- --testPathPattern=test.ts -t "specificBehavior"`
預期: PASS ✅

**Step 5: Commit**

```bash
git add tests/path/test.ts src/path/file.ts
git commit -m "feat: 新增 specificBehavior 功能"
```
````

## 實作規則

1. 每個步驟要可執行、可驗證，避免抽象描述（❌「加入驗證邏輯」→ ✅ 完整程式碼）
2. 精確檔案路徑（不可用「相關檔案」模糊帶過）
3. 驗證命令要包含預期結果（不可只寫「跑測試」）
4. 先做低風險探查，再做高風險修改
5. 若過程中發現新風險，必須更新計畫再繼續
6. 若需求改變，先同步差異，再調整步驟
7. DRY、YAGNI、頻繁 Commit

## 簡版範本

```markdown
## Plan
1) Goal:
2) Done when:
3) Assumptions / constraints:
4) Steps:
   - Step A (檔案: xxx, 驗證: xxx, 預期: xxx)
   - Step B (檔案: xxx, 驗證: xxx, 預期: xxx)
5) Risks & rollback:
   - Risk -> Mitigation
```
