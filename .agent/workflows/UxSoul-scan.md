---
description: 啟動 UX 靈魂掃描 (Soul Extraction)
---

# UX Scan Soul Workflow

此工作流會呼叫 `UxSoul-extractor` Skill，對指定檔案進行深度互動邏輯分析。

## 使用方式
`/UxSoul-scan [檔案路徑]`

## 執行步驟

1. **確認目標**:
   - 檢查使用者是否提供了目標檔案路徑。
   - 若未提供，詢問使用者要掃描哪個 UI 元件。

2. **載入技能**:
   - 讀取 `.agent/skills/UxSoul-extractor.skill.md` 以獲取分析規則。

3. **執行掃描**:
   - 讀取目標檔案內容。
   - (Optional) 讀取關聯的 CSS/Less/Sass 檔案以獲取動畫數據。
   - 根據 Skill 中的「Core Capabilities」進行分析。

4. **生成報告**:
   - 在 `docs/ux-manuals/` 下建立或更新 `UxSoul-[檔名].md`。
   - 確保內容包含「靈魂意圖 (Design Intent)」區塊。

5. **通知用戶**:
   - 顯示報告連結並請求檢閱。
