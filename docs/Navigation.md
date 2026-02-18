# 導航與地圖系統使用手冊

本文件說明 MUD 客戶端的導航 (`MudNav`) 與地圖探索 (`MudExplorer`) 功能，特別是針對 **Room ID 整合** 的使用方式。

---

## 目錄

1. [MudNav - 基礎導航與錄製](#mudnav---基礎導航與錄製)
2. [MudExplorer - 自動地圖探索](#mudexplorer---自動地圖探索)
3. [Room ID 機制說明](#room-id-機制說明)

---

## MudNav - 基礎導航與錄製

`MudNav` 是負責執行固定路徑的核心模組。

### 載入
通常由其他腳本自動載入。手動載入命令：
```lua
/lua dofile("scripts/modules/MudNav.lua")
```

### 1. 路徑格式

MudNav 現在支援兩種路徑格式：

#### A. 純字串模式 (傳統/簡易)
適用於簡單、無風險的移動。
```lua
local path = "n;e;3s;w"
MudNav.walk(path, callback)
```
- **優點**：撰寫快速。
- **缺點**：如果移動失敗（例如被傳送走、Lag 沒走到），腳本**不會知道**，繼續傻傻地執行下一步，容易導致迷路。

#### B. ID 驗證模式 (推薦/安全)
適用於關鍵任務（如 Boss 戰跑圖、危險區域）。
```lua
local path = {
    {cmd="n", id="8f2a..."}, -- 系統會驗證是否到達 ID 為 8f2a... 的房間
    {cmd="e", id="1b3d..."},
}
MudNav.walk(path, callback)
```
- **優點**：**絕對安全**。如果 ID 不對（走錯路），腳本會**立即停止**並報錯，防止意外。
- **缺點**：路徑寫法複雜（但可以用錄製器自動產生）。

### 2. 自動路徑錄製器

您不需要手動查詢 Room ID。使用錄製器可以自動生成帶有 ID 驗證的程式碼。

**使用步驟：**

1.  **開始錄製**：
    ```lua
    /lua MudNav.record_start()
    ```
    *(系統顯示：🔴 開始錄製路徑)*

2.  **在遊戲中移動**：
    使用您的慣用方式移動（鍵盤、別名皆可）。系統會自動記錄您的移動指令與到達的房間 ID。

3.  **停止錄製**：
    到達目的地後：
    ```lua
    /lua MudNav.record_stop()
    ```
    *(系統顯示：⏹️ 錄製結束，並列出程式碼)*

4.  **使用程式碼**：
    將系統輸出的 table 複製貼上到您的 Lua 腳本中即可。

---

## MudExplorer - 自動地圖探索

`MudExplorer` 用於未知區域的自動開圖與遍歷。

### 功能特色

- **自動繪圖**：利用 DFS (深度優先搜尋) 演算法遍歷所有房間。
- **Room ID 追蹤**：[NEW] 使用房間 ID 來識別位置，即使在「迷宮」或「迴圈」地圖中也不會鬼打牆。
- **智慧門鎖處理**：自動嘗試打開關閉的門。

### 指令

| 指令 | 說明 |
|------|------|
| `MudExplorer.explore(callback)` | 開始探索當前區域 |
| `MudExplorer.stop()` | 停止探索 |
| `MudExplorer.status()` | 顯示目前探索進度（房間數、深度） |
| `MudExplorer.config.max_laps = N` | 設定最大探索圈數（預設 5） |

### 使用範例 (腳本中)

```lua
-- 開始探索，尋找 "神秘商人"
MudExplorer.config.target = "神秘商人"
MudExplorer.explore(function(found, line)
    if found then
        mud.print("找到了！就在這裡！")
    else
        mud.print("找遍了整個區域都沒看到。")
    end
end)
```

---

## Room ID 機制說明

**Room ID 是什麼？**
它是根據房間的「名稱 + 敘述 + 出口」計算出的唯一雜湊值 (SHA256 Hash)。

**為什麼需要它？**
傳統 MUD 機器人只靠 `(x,y)` 座標導航。但在 MUD 中，房間的空間結構往往是不合邏輯的（例如：向北走一步，再向南一步，可能回不到原點）。
使用 Room ID 可以讓我們確切知道「我現在到底在哪裡」，而不僅僅是「我覺得我在 (x,y)」。

> [!IMPORTANT]
> **雙胞胎房間**：如果有兩個房間的名稱、敘述和出口完全一模一樣，它們會有相同的 ID。
> 這通常不影響導航，但在極少數情況下可能會讓機器人誤以為回到了原點。MudExplorer 已包含邏輯處理此類情況。
