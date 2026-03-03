---
description: 更新日誌生成規範
version: 1.0.0
---

# Changelog Workflow - 更新日誌規範

trigger: 任務涉及「changelog」、「更新日誌」、「版本紀錄」關鍵字時自動載入

---

## 標準格式

遵循 [Keep a Changelog](https://keepachangelog.com/) 規範：

```markdown
# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added
- 新功能描述

### Changed
- 變更描述

### Deprecated
- 即將棄用的功能

### Removed
- 已移除的功能

### Fixed
- Bug 修復

### Security
- 安全性修復

## [1.0.0] - 2024-01-27

### Added
- 初始版本發布
- 用戶註冊與登入功能
- 基礎 API 端點
```

---

## 分類說明

| 分類 | 說明 | 範例 |
|------|------|------|
| **Added** | 新增功能 | 新增 OAuth 登入 |
| **Changed** | 既有功能的變更 | 改善載入速度 |
| **Deprecated** | 即將移除的功能 | 棄用 v1 API |
| **Removed** | 已移除的功能 | 移除舊版儀表板 |
| **Fixed** | Bug 修復 | 修復登入失敗問題 |
| **Security** | 安全性修復 | 修補 XSS 漏洞 |

---

## 撰寫原則

### 1. 面向用戶

```markdown
# ❌ 錯誤：開發者視角
- Refactored UserService to use dependency injection

# ✅ 正確：用戶視角
- 改善登入頁面載入速度
```

### 2. 提供上下文

```markdown
# ❌ 錯誤：太簡略
- Fixed bug

# ✅ 正確：說明影響
- Fixed: 修復 Safari 瀏覽器無法上傳圖片的問題 (#156)
```

### 3. 連結相關資源

```markdown
- Added: 支援 Google OAuth 登入 ([#123](https://github.com/xxx/issues/123))
- Security: 修補 CVE-2024-1234 ([詳情](https://nvd.nist.gov/xxx))
```

---

## 自動生成

### 使用 conventional-changelog

```bash
# 安裝
npm install -g conventional-changelog-cli

# 生成 changelog
conventional-changelog -p angular -i CHANGELOG.md -s

# 生成從頭開始的完整 changelog
conventional-changelog -p angular -i CHANGELOG.md -s -r 0
```

### 使用 standard-version

```bash
# 安裝
npm install --save-dev standard-version

# 發布新版本（自動更新 changelog）
npx standard-version

# 預覽變更（不實際執行）
npx standard-version --dry-run
```

### GitHub Actions 自動生成

```yaml
# .github/workflows/changelog.yml
name: Generate Changelog

on:
  push:
    tags:
      - 'v*'

jobs:
  changelog:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
          
      - name: Generate Changelog
        uses: TriPSs/conventional-changelog-action@v4
        with:
          github-token: ${{ secrets.GITHUB_TOKEN }}
          output-file: 'CHANGELOG.md'
```

---

## 範例 CHANGELOG.md

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- 新增深色模式支援

## [1.2.0] - 2024-01-27

### Added
- 支援 Google OAuth 登入 ([#123](https://github.com/xxx/issues/123))
- 新增報表匯出為 PDF 功能 ([#145](https://github.com/xxx/issues/145))

### Changed
- 改善首頁載入速度，FCP 從 2.5s 降至 1.2s

### Fixed
- 修復 Safari 瀏覽器無法上傳圖片的問題 ([#156](https://github.com/xxx/issues/156))
- 修復登入後偶爾白屏的問題 ([#158](https://github.com/xxx/issues/158))

### Security
- 更新依賴套件以修復 CVE-2024-1234

## [1.1.0] - 2024-01-15

### Added
- 基礎報表功能
- 用戶偏好設定

### Fixed
- 修復密碼重設郵件未發送的問題

## [1.0.0] - 2024-01-01

### Added
- 用戶註冊與登入
- 基礎 CRUD API
- 初始 UI 介面

---

[Unreleased]: https://github.com/xxx/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/xxx/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/xxx/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/xxx/releases/tag/v1.0.0
```

---

## Changelog 檢查清單

- [ ] 使用正確的分類（Added/Changed/Fixed 等）
- [ ] 描述面向用戶（非開發者）
- [ ] 有連結到相關 Issue/PR
- [ ] 版本號符合 Semantic Versioning
- [ ] 日期格式正確（YYYY-MM-DD）
- [ ] 最新版本在最上方

---

**原則**：Changelog 是給「用戶」看的，不是給開發者看的。用戶關心「這次更新對我有什麼影響」。
