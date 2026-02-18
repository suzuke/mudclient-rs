---
description: 初始化專案文檔結構
---

# 初始化文檔流程

1. **建立 docs 目錄**
    * 確認 `docs/` 目錄存在。

2. **建立 RoadMap.md**
    * 路徑: `docs/RoadMap.md`
    * 內容: 記錄功能規劃、待辦事項、里程碑。

3. **建立 Architecture.md**
    * 路徑: `docs/Architecture.md`
    * 內容: 記錄專案架構、目錄結構、技術選型。

4. **建立 API.md**
    * 路徑: `docs/API.md`
    * 內容: 記錄 API 介面規格 (若有)。

5. **建立 Contributing.md**
    * 路徑: `docs/Contributing.md`
    * 內容: 貢獻指南、開發規範。

6. **建立 Deployment.md**
    * 路徑: `docs/Deployment.md`
    * 內容: 部署步驟、環境變數設定。

7. **建立 Env.md**
    * 路徑: `docs/Env.md`
    * 內容: 環境變數詳細說明。

8. **建立 ChangeLog.md**
    * 路徑: `docs/ChangeLog.md`
    * 內容: 版本變更記錄 (格式參照 `commit.md` 規則)。

9. **建立 TestAccounts.md**
    * 路徑: `docs/TestAccounts.md`
    * 內容: 測試用的帳號密碼 (請勿提交真實機敏資料)。

10. **檢查 .gitignore**
    * 確保 `docs/TestAccounts.md` 被忽略 (或確保內容不含敏感資訊)。

11. **模組文件規範 (Module README)**
    * 若模組包含獨立登入/資料隔離邏輯，必須在 `modules/[name]/README.md` 中附上：
        * **測試帳號表** (Default Test Accounts)
        * **檔案結構說明**
        * **功能與權限說明**
