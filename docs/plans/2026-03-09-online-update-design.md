# 線上更新系統設計

**狀態**：Roadmap（待實作）
**日期**：2026-03-09

## 需求

1. **App 自動更新**：啟動時檢查 GitHub Releases，有新版彈通知，用戶點擊後開瀏覽器下載
2. **Lua 腳本更新**：從同一 GitHub repo 的 Release assets 下載 `scripts.zip`，更新官方腳本
3. **目錄分離**：`scripts/` 放官方腳本，`scripts/user/` 放用戶自訂（更新不觸及）
4. **安裝方式**：先簡單的來 — 開瀏覽器下載 DMG/ZIP，用戶手動安裝

## 方案：自建輕量更新模組（GitHub Releases API）

### App 更新
- 啟動後 3 秒非同步檢查 `GET https://api.github.com/repos/{owner}/{repo}/releases/latest`
- semver 比較 `tag_name` vs `env!("CARGO_PKG_VERSION")`
- 有新版 → egui 頂部橫幅通知 +「下載」按鈕 +「忽略此版本」
- 下載 = `open::that(release_url)` 開啟瀏覽器
- 每 24 小時最多檢查一次

### 腳本更新
- CI 額外打包 `scripts.zip`（含 `manifest.json`）上傳為 Release asset
- `manifest.json` 記錄每個官方腳本的 SHA256 hash
- 比對新舊 manifest，只覆蓋有變更的檔案
- 不動 `scripts/user/` 目錄

### 設定檔 `data/update_config.json`
```json
{
  "last_check": "2026-03-09T12:00:00Z",
  "check_interval_hours": 24,
  "ignored_versions": [],
  "auto_update_scripts": true,
  "update_enabled": true
}
```

### 新增程式碼
- `crates/mudgui/src/updater.rs` — 版本檢查、下載、腳本解壓
- `crates/mudgui/src/app/mod.rs` — UI 橫幅
- `.github/workflows/release.yml` — 加 `scripts.zip` asset

### 新增依賴
- `semver`, `reqwest`(已有), `open`, `sha2`, `zip`

### 不做
- 不做 binary 自動替換（macOS 簽名問題）
- 不做差量更新
- 不做自建更新伺服器
