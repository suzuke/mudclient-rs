---
name: verification-before-completion
description: |
  交付前驗證 Skill。禁止在未驗證的情況下宣告「完成」，要求以證據確認修改有效且無回歸。
  適用於任何已修改程式碼、設定或文件的任務。
license: MIT
compatibility: Pixiu Agent / Claude Code / Cursor
metadata:
  author: pixiu
  version: "1.1.0"
  category: workflow
  tags: [verification, testing, quality, completion]
---

<!--
Source: https://github.com/obra/superpowers
Commit: e16d611eee14ac4c3253b4bf4c55a98d905c2e64
Concept: verification-before-completion
Audited By: Pixiu Agent
Date: 2026-02-16
-->

# ✅ Verification Before Completion / 完成前驗證

任何修改都要先驗證，再回報完成。
若無法驗證，必須明確說明缺口與風險。

## Iron Law / 鐵律

```
未經新鮮驗證證據，不得宣稱完成。
NO COMPLETION CLAIMS WITHOUT FRESH VERIFICATION EVIDENCE.
```

如果你在**這次對話**中沒有執行驗證命令，就不能宣稱它通過。

## Gate Function / 閘門流程（5 步驟）

在宣稱任何狀態或表達滿意之前：

1. **識別**：哪個命令能證明這個宣稱？
2. **執行**：執行**完整**的驗證命令（新鮮、完整執行）
3. **閱讀**：完整讀取輸出，檢查 exit code，計算失敗數
4. **確認**：輸出是否支持你的宣稱？
   - 若**否**：陳述實際狀態並附上證據
   - 若**是**：陳述結論並**附上證據**
5. **然後才能宣稱**

> 跳過任何一步 = 欺騙，不是驗證。

## Common Failures / 常見宣稱與所需證據

| 宣稱 | 需要什麼證據 | 不夠充分的 |
|------|-------------|-----------|
| 測試通過 | 測試命令輸出：0 failures | 之前的執行、「應該會過」 |
| Lint 通過 | Linter 輸出：0 errors | 只跑部分檢查、推測 |
| Build 成功 | Build 命令：exit 0 | Lint 通過不代表能編譯 |
| Bug 已修復 | 原始症狀測試：通過 | 改了程式碼就假設修好了 |
| 回歸測試有效 | Red-Green 循環已驗證 | 測試只通過一次 |
| 需求已滿足 | 逐條核對 checklist | 測試通過不等於需求滿足 |

## Red Flags / 危險信號 — 立即停下

- 使用「應該」、「大概」、「看起來」等字眼
- 在驗證前表達滿意（「太好了！」、「完成！」、「搞定！」）
- 即將 commit / push / PR 但未驗證
- 依賴部分驗證
- 想著「就這一次」
- 任何暗示成功但**未執行驗證**的措辭

## Rationalization Prevention / 反合理化

| 藉口 | 現實 |
|------|------|
| 「改完應該可以了」 | **跑一次驗證** |
| 「我很有信心」 | 信心 ≠ 證據 |
| 「就這一次」 | 沒有例外 |
| 「Lint 過了」 | Lint ≠ 編譯 ≠ 功能正確 |
| 「我累了」 | 疲勞不是藉口 |
| 「部分檢查就夠了」 | 部分驗證什麼都不能證明 |

## 驗證順序

1. 先驗證改動最直接的功能
2. 再驗證高風險邊界（錯誤路徑、空值、權限）
3. 最後做快速回歸（相關模組）

## 無法驗證時的回報格式

```markdown
## Verification Status
- Performed: (已完成的檢查)
- Not performed: (未做的檢查)
- Reason: (為何無法驗證)
- Risk: (可能影響)
- Next action: (建議下一步)
```

## 底線

**驗證沒有捷徑。** 執行命令、閱讀輸出、然後才能宣稱結果。這是不可協商的。
