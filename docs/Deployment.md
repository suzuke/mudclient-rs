# 部署指南 (Deployment Guide)

本文件說明如何編譯、打包與發布 `mudclient-rs`。

## 1. 編譯需求

*   **Rust Toolchain**: 建議使用 stable channel。
*   **Make** (選用): 用於執行 Makefile 指令 (如有)。
*   **Platform Dependencies**:
    *   **Linux**: `libxcb`, `libssl`, `fontconfig`
    *   **macOS**: Xcode Command Line Tools
    *   **Windows**: MSVC Toolchain

## 2. 建置 (Build)

### 開發版本 (Debug)
快速編譯，包含除錯資訊。

```bash
cargo build -p mudgui
```

### 發布版本 (Release)
最佳化執行效能，檔案較小。

```bash
cargo build -p mudgui --release
```

產出檔案位於: `target/release/mudgui` (或 `.exe`)。

## 3. 打包 (Packaging)

發布時，除了執行檔外，還需要包含必要的資源文件。標準發布包結構：

```
mudclient-vX.Y.Z/
├── mudgui          (執行檔)
├── scripts/        (腳本目錄)
├── docs/           (文件目錄)
├── README.md
└── LICENSE
```

### GitHub Actions 自動化

專案已設定 `.github/workflows/build.yml`，當推送到 `main` 分支或建立 Tag 時會自動觸發：

1.  **Check**: 執行 `cargo check` 與 `cargo test`。
2.  **Build**: 在 ubuntu-latest, macos-latest, windows-latest 進行編譯。
3.  **Artifact**: 上傳編譯後的執行檔與資源。
4.  **Release**: (僅 Tag) 自動建立 GitHub Release 並上傳資產。

## 4. 部署注意事項

*   **macOS Gatekeeper**: 由於未簽章，首次執行可能需要右鍵點選 `Open` 並允許執行。
*   **路徑問題**: 建議在終端機執行或確保工作目錄正確，以便程式讀取 `scripts/`。
