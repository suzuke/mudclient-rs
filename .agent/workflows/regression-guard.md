---
description: 回歸風險檢查 — 偵測多版本修改後的邏輯回歸
---

# 🛡️ Regression Guard Workflow

## 何時使用
- 在 CI/CD 中自動執行回歸風險攔截
- 手動執行 `pixiu.regressionAudit` 命令進行本地檢查

## 檢查項目

### 1. mirrorParity（鏡像一致性）
- **適用**：Pixiu-internal 結構或自訂 `regression.mirrorPairs`
- **檢查**：比對來源與目標目錄的同名檔案內容是否一致
- **嚴重度**：critical（不一致時）

### 2. workflowReality（CI 命令合理性）
- **適用**：含 `.github/workflows/*.yml` 的專案
- **檢查**：root npm 命令（`npm ci`、`npm run build`）是否與 repo root 的 `package.json` 存在一致
- **嚴重度**：critical（root 無 package.json 卻跑 root npm）

### 3. testImpact（測試覆蓋影響）
- **適用**：Git 環境下的程式碼變更
- **檢查**：高風險檔案（公開 API 或大量刪除）變更是否伴隨測試檔更新
- **嚴重度**：high（無對應測試變更時）

## 設定

在 VS Code 設定中：
```json
{
  "pixiu.regressionGuard.enabled": true,
  "pixiu.regressionGuard.mode": "warn",
  "pixiu.regressionGuard.baseRef": "origin/main",
  "pixiu.regressionGuard.deletionThreshold": 30
}
```

## CI 整合

安裝 `regression-guard.yml` GitHub Action 模板，或在現有 CI 新增：
```yaml
- name: Run Regression Gate
  run: node scripts/regression-gate.js --mode=block --base=origin/main
```
