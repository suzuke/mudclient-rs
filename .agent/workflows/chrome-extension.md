---
description: Chrome Extension (Manifest V3) 開發與發布規範
version: 1.0.0
---

# Chrome Extension Development Workflow

trigger: 專案包含 `manifest.json` 時自動建議

---

## 核心原則

1.  **Manifest V3 優先**：所有新開發必須遵循 V3 規範（無遠端程式碼執行、Service Worker 背景運作）。
2.  **最小權限原則 (Least Privilege)**：`permissions` 與 `host_permissions` 僅宣告必要項目，避免審核被拒。
3.  **版本同步**：`manifest.json` 的 `version` 必須與 `package.json` (若有) 及 Git Tag 一致。

---

## 開發規範 (Development Standards)

### 1. 目錄結構
建議將擴充功能原始碼放在 `src` 或 `extension` 目錄，避免將整個 Repo 根目錄直接作為擴充功能載入點（除非是極簡易專案）。

```
my-extension/
├── manifest.json      # 核心設定
├── _locales/          # 多語系 (i18n)
├── icons/             # 圖示 (16/32/48/128)
├── src/               # 程式碼
│   ├── background/    # Service Workers
│   ├── content/       # Content Scripts
│   └── popup/         # UI 邏輯
└── assets/            # 靜態資源
```

### 2. Manifest V3 檢查清單
- [ ] **background**: 使用 `service_worker` 而非 `scripts`。
- [ ] **permissions**: 移除 `activeTab` 以外的不必要權限。
- [ ] **CSP**: `content_security_policy` 不可允許 `unsafe-eval`。
- [ ] **Remote Code**: 禁止載入外部 JS (如 CDN 上的 jQuery)，必須打包在套件內。

### 3. 圖示要求
Web Store 強制要求提供以下尺寸 (PNG 格式)：
- `16x16` (擴充功能列)
- `32x32` (Windows 高 DPI)
- `48x48` (擴充功能管理頁)
- `128x128` (安裝與商店頁面)

---

## 發布準備 (Release Preparation)

### 1. 更新版本號
在發布前，必須手動更新 `manifest.json` 中的版本號：

```json
{
  "version": "1.2.3",
  "version_name": "1.2.3 beta" // 可選，僅供顯示
}
```

### 2. 更新日誌 (Changelog)
必須同步更新 `CHANGELOG.md`，記錄本次版本的變更內容。

### 3. 打包 (Packaging)
**嚴禁**將開發檔案打包進 `.zip`。使用 `zip`指令排除干擾檔：

```bash
# 範例打包指令
zip -r extension.zip . -x "*.git*" "*.env*" "*.vscode*" "node_modules/*" "*.DS_Store" "src/*"
# 注意：若有 build step (如 Webpack/Vite)，應只打包 `dist/` 目錄
```

> 💡 **建議**：撰寫 `build.sh` 自動執行打包，避免手動錯誤。

---

## 審核常見拒絕原因 (Rejection Risks)

1.  **權限過大**：要求了 `tabs` 或 `all_urls` 但功能上看不出必要性。
2.  **單一用途**：擴充功能必須有單一且明確的用途，不可是 "工具箱" 且各功能不相關。
3.  **遠端程式碼**：載入外部 JS 會直接被拒 (V3 核心規定)。
4.  **使用者隱私**：若收集 User Data，必須在 Dashboard 填寫隱私權政策 URL 並誠實宣告。

---

## 發布檢查清單

- [ ] `manifest.json` 版本號已更新 (vX.Y.Z)
- [ ] `CHANGELOG.md` 已記錄變更
- [ ] 所有 `console.log` 調試訊息已移除
- [ ] 用 `zip` 打包並排除了 `.git`, `node_modules`
- [ ] 本地載入 (Load Unpacked) 測試功能正常
- [ ] 確認 Service Worker 在閒置後喚醒功能正常 (V3 常見 Bug)
