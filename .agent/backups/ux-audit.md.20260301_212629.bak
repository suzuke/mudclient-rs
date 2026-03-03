---
description: 如何進行 UX 審計與文件更新
---

# UX Audit & Documentation Workflow

當進行 UI 修改、新增功能或修復前端 Bug 時，請遵循此流程以確保 `docs/UX-Flows.md` 保持最新。

## 1. 變更識別 (Identify Changes)
確認本次提交是否涉及以下區域：
- HTML 結構變更 (Layout)
- CSS 樣式調整 (Visual)
- JS 互動邏輯 (Interaction/State)

## 2. 驗證現有流程 (Verify Existing Flows)
執行 `docs/UX-Flows.md` 中的相關條目：
- [ ] 既有操作是否仍有效？
- [ ] 視覺回饋是否改變？

## 3. 更新文件 (Update Documentation)
- **若修正現有行為**：直接修改 `docs/UX-Flows.md` 對應段落。
- **若新增功能**：
  1. 複製 `.agent/templates/UX-FLOW.md` 內容。
  2. 填寫新功能的觸發與邏輯。
  3. 將其 append 到 `docs/UX-Flows.md` 的適當區塊。

## 4. 交付檢查 (Delivery Check)
在 Pull Request 或 Commit Message 中標註：
> 📝 UX Docs Updated: [Yes/No]

---
**Tip**: 使用 Agent 時，可指令 `/ux-audit` 讓 AI 自動閱讀當前程式碼並建議 UX 文件更新。
