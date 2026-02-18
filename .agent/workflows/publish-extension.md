---
description: VS Code Extension 發布流程與文件同步規範
version: 1.0.0
---

# Publish Extension Workflow - 發布與同步規範

trigger: 任務涉及「發布 extension」、「readme 同步」、「圖片破圖」時自動載入

---

## 核心原則：Single Source of Truth

為了避免維護多份文件，我們採用 **「根目錄為主，建置時同步」** 的策略：

| 文件 | 唯一真理 (Source) | 發布產物 (Artifact) | 處理方式 |
|------|-------------------|---------------------|----------|
| **README.md** | `./README.md` | `./vscode-extension/README.md` | Build 時自動複製 |
| **CHANGELOG.md** | `./CHANGELOG.md` | `./vscode-extension/CHANGELOG.md` | Build 時自動複製 |
| **Images** | `./docs/images/` | `./vscode-extension/docs/images/` | Build 時整包複製 |

---

## 同步策略 (Sync Strategy)

### 1. 圖片處理 (Image Bundling)

為了確保圖片在 **Marketplace** (Web) 與 **Local Preview** (VS Code, Offline) 都能正常顯示，我們**不再使用** GitHub Raw URL 替換法，而是採用 **本地打包策略**。

- **來源**: `docs/`
- **目標**: `vscode-extension/docs/`
- **Markdown 寫法**: `![Label](docs/images/demo.png)` (使用相對路徑)

這樣做的好處是：
- 本地預覽 (Preview) 可直接讀取檔案。
- Marketplace 會讀取 `.vsix` 包內的圖片。
- 離線安裝也能看到圖片。

### 2. 建置腳本 (`build-extension.sh`)

所有同步邏輯必須寫在 `scripts/build-extension.sh` 中：

```bash
# 範例
echo "📄 Syncing README..."
cp "$ROOT_DIR/README.md" "$EXTENSION_DIR/README.md"

echo "🖼️ Bundling docs..."
rm -r "$EXTENSION_DIR/docs"
cp -r "$ROOT_DIR/docs" "$EXTENSION_DIR/"
```

---

## Git 與 VSCE 配置 (關鍵！)

這是最容易出錯的部分，必須精確配置以達成：
1. **Git** 忽略複製的檔案 (避免重複提交)。
2. **VSCE** 打包這些被 Git 忽略的檔案 (因為發布需要)。

### `.gitignore` (vscode-extension/)

**忽略** 自動產生的檔案：

```gitignore
README.md
CHANGELOG.md
docs/
```

### `.vscodeignore` (vscode-extension/)

**強制包含 (Whitelisting)** 這些檔案進入 `.vsix`：

```gitignore
# 預設忽略所有
*

# 顯式包含 (!代表不忽略)
!README.md
!CHANGELOG.md
!docs/
!out/
!package.json
!icon.png
```

> ⚠️ **注意**：`.vscodeignore` 的優先級高於 `.gitignore`，但在 `vsce package` 時，如果檔案在 `.gitignore` 內，預設是不會被打包的，除非在 `.vscodeignore` 中用 `!` 顯式宣告。

---

## 發布檢查清單

- [ ] `ROOT/README.md` 引用圖片使用相對路徑 `docs/images/xxx.png`
- [ ] 執行 `sh scripts/build-extension.sh`
- [ ] 檢查 `vscode-extension/docs/` 是否存在
- [ ] 檢查 `vscode-extension/README.md` 是否更新
- [ ] 若要預覽：在 VS Code 開啟 `vscode-extension/README.md` 按 `Ctrl+K V`

---

## 常見問題 (FAQ)

### Q: 為什麼 Private Repo 的圖片在 Marketplace 網頁上破圖？

**A**: Marketplace 網站會嘗試從你的 GitHub Repo 抓取圖片。若 Repo 是 Private，外部無法存取，圖片就會 404。

**解決方案**：

| 方案 | 適用情境 | 說明 |
|------|----------|------|
| **相對路徑 + 打包 (推薦)** | 大多數情況 | 保持 `docs/images/xxx.png` 相對路徑，建置時將 `docs/` 複製到 Extension。**VS Code 安裝後可正常顯示**，但 Marketplace 網頁上不行。 |
| **公開圖床** | 需要 Marketplace 預覽 | 將圖片上傳到 Imgur / 公開 Repo / CDN，然後在 README 使用絕對 URL。 |
| **Repo 設為 Public** | 開源專案 | 直接公開 Repo，所有連結自動可存取。 |

> 💡 **建議**：若您是企業內部工具或不在意 Marketplace 網頁預覽，採用「相對路徑 + 打包」最簡單，零額外維護成本。

