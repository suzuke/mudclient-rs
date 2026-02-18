---
description: 版本發布與 GitHub Release 流程
version: 1.0.0
---

# Release Workflow - 版本發布規範

trigger: 任務涉及「release」、「發布」、「版本」、「tag」關鍵字時自動載入

---

## 版本號規範 (Semantic Versioning)

```
MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]

範例：
1.0.0         → 正式版
1.0.1         → Patch 修復
1.1.0         → 新功能
2.0.0         → 破壞性變更
1.0.0-alpha   → 預覽版
1.0.0-beta.1  → Beta 測試
1.0.0-rc.1    → 候選發布
```

| 變更類型 | 版本變化 | 範例 |
|----------|----------|------|
| Bug 修復 | PATCH | 1.0.0 → 1.0.1 |
| 新功能（向後相容） | MINOR | 1.0.0 → 1.1.0 |
| 破壞性變更 | MAJOR | 1.0.0 → 2.0.0 |

---

## Phase 1: 發布前檢查

### 必要條件

- [ ] 所有測試通過
- [ ] 無未解決的 blocking issues
- [ ] CHANGELOG.md 已更新
- [ ] 版本號已更新（package.json/pyproject.toml 等）
- [ ] 文檔已同步更新
- [ ] 依賴套件無安全漏洞

### Chrome Extension 特定檢查
- [ ] manifest.json 版本號已更新 (`package.json` 可選)
- [ ] `permissions` 最小化檢查
- [ ] 打包排除 `.git`, `.env`, `node_modules`
- [ ] 測試 Load Unpacked 功能正常

### 版本號更新位置

```bash
# Node.js
package.json → "version": "1.2.0"

# Python
pyproject.toml → version = "1.2.0"
setup.py → version="1.2.0"

# Go
version.go → const Version = "1.2.0"
```

### 📱 行動端與原生平台 (Native Platforms)

除了 `package.json`，請務必更新原生專案設定檔：

#### iOS (`Info.plist`)
- **CFBundleShortVersionString**: 對應 `MAJOR.MINOR.PATCH` (如 1.2.0)
- **CFBundleVersion**: 對應建置號碼 (如 123)
- **工具建議**: 使用 `agvtool` 自動同步
  ```bash
  xcrun agvtool new-marketing-version 1.2.0
  xcrun agvtool next-version -all
  ```

#### Android (`build.gradle`)
- **versionName**: 對應 `MAJOR.MINOR.PATCH` (如 "1.2.0")
- **versionCode**: 整數遞增 (如 123)
  ```gradle
  android {
      defaultConfig {
          versionCode 123
          versionName "1.2.0"
      }
  }
  ```

#### Flutter (`pubspec.yaml`)
- **version**: 格式 `version: 1.2.0+123` (版本號+建置號)

---

## Phase 2: 建立 Git Tag

### 標準流程

```bash
# 1. 確保在 main 分支且最新
git checkout main
git pull origin main

# 2. 建立 annotated tag
git tag -a v1.2.0 -m "Release 1.2.0: 新增支付功能"

# 3. 推送 tag
git push origin v1.2.0
```

### Tag 命名規範

| 格式 | 範例 | 說明 |
|------|------|------|
| `v[VERSION]` | `v1.2.0` | 正式版本（推薦） |
| `[VERSION]-beta` | `1.2.0-beta.1` | 測試版 |
| `[VERSION]-rc` | `1.2.0-rc.1` | 候選發布 |

---

## Phase 3: GitHub Release

### 手動建立

1. 前往 GitHub → Releases → Draft a new release
2. 選擇 tag
3. 填寫 Release title 和 Release notes
4. 若為預覽版，勾選 "This is a pre-release"
5. 點擊 "Publish release"

### Release Notes 格式

```markdown
## 🚀 v1.2.0 (2024-01-27)

### ✨ 新功能
- 支援 Google OAuth 登入 (#123)
- 新增報表匯出功能 (#145)

### 🐛 Bug 修復
- 修復登入後白屏問題 (#156)
- 修復 Safari 相容性問題 (#158)

### ⚠️ 破壞性變更
- API `/users` 回應格式變更，詳見 Migration Guide

### 📝 其他
- 更新依賴套件
- 改善文件

---

**完整更新日誌**: https://github.com/xxx/compare/v1.1.0...v1.2.0
```

---

## Phase 4: 自動化發布

### GitHub Actions 自動發布

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Build
        run: npm run build
      
      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          generate_release_notes: true
          files: |
            dist/*.js
            dist/*.map
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### semantic-release 自動化

```bash
npm install --save-dev semantic-release

# .releaserc.json
{
  "branches": ["main"],
  "plugins": [
    "@semantic-release/commit-analyzer",
    "@semantic-release/release-notes-generator",
    "@semantic-release/changelog",
    "@semantic-release/npm",
    "@semantic-release/github",
    "@semantic-release/git"
  ]
}
```

---

## Phase 5: 發布後驗證

### 檢查清單

- [ ] GitHub Release 頁面正確顯示
- [ ] npm/PyPI 套件已發布（如適用）
- [ ] Docker Image 已推送（如適用）
- [ ] 文件網站已更新
- [ ] 監控無異常

### 通知團隊

```markdown
🎉 v1.2.0 已發布！

主要變更：
- 新增 Google OAuth
- 修復登入問題

Release Notes: https://github.com/xxx/releases/tag/v1.2.0
```

---

## 回滾策略

### 發現嚴重問題時

```bash
# 1. 刪除 GitHub Release（保留 tag 作為紀錄）
# 2. 發布 hotfix
git checkout -b hotfix/1.2.1
# ... 修復問題 ...
git checkout main
git merge hotfix/1.2.1
git tag -a v1.2.1 -m "Hotfix: 修復 xxx 問題"
git push origin v1.2.1
```

---

## 發布檢查清單（完整版）

### 發布前

- [ ] 決定版本號（MAJOR/MINOR/PATCH）
- [ ] 更新 CHANGELOG.md
- [ ] 更新版本號（package.json 等）
- [ ] 所有測試通過
- [ ] PR 已合併至 main

### 發布中

- [ ] 建立 Git tag
- [ ] 推送 tag 到遠端
- [ ] 建立 GitHub Release
- [ ] 填寫 Release Notes

### 發布後

- [ ] 驗證 Release 頁面
- [ ] 驗證套件發布（npm/PyPI）
- [ ] 通知團隊
- [ ] 監控無異常

---

**原則**：發布應該是可預測且可重複的。如果每次發布都讓你緊張，代表流程需要更多自動化。
