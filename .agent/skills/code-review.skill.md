---
name: code-review
description: |
  程式碼審查 Skill。安全性檢查、效能審查、可維護性評估、錯誤處理檢查。
  任務涉及「review」、「審查」、「檢查」、「PR」、「merge」關鍵字時自動載入。
license: MIT
compatibility: Pixiu Agent / Claude Code / Cursor
metadata:
  author: pixiu
  version: "1.0"
  category: workflow
---

# Code Review Skill - 程式碼審查

> **用途**：安全性檢查、效能審查、可維護性評估、錯誤處理檢查

---

## 自動審查清單

### 🔒 安全性

- [ ] 無硬編碼 API Key / 密碼 / Token
- [ ] 使用參數化查詢（防 SQL Injection）
- [ ] HTML 輸出已跳脫（防 XSS）
- [ ] 敏感操作有權限檢查
- [ ] 使用 HTTPS
- [ ] Cookie 設定 httpOnly, secure, sameSite

### ⚡ 效能

- [ ] 無 N+1 查詢
- [ ] 大列表有分頁或虛擬滾動
- [ ] 圖片有 lazy loading
- [ ] API 呼叫有 debounce/throttle
- [ ] 重複計算有快取

### 🧹 可維護性

- [ ] 函式長度 < 50 行
- [ ] 單一職責（一個函式只做一件事）
- [ ] 命名語意清晰（見 naming-convention.skill.md）
- [ ] 無 magic numbers（使用常數）
- [ ] 無重複程式碼（DRY 原則）

### ⚠️ 錯誤處理

- [ ] try-catch 包裹外部呼叫
- [ ] 有友善的錯誤訊息
- [ ] Log 記錄足夠診斷問題
- [ ] 有 Fallback UI

### 📝 文件

- [ ] 複雜邏輯有註解
- [ ] 公開 API 有文件
- [ ] README 保持更新

### 🧪 測試

- [ ] 關鍵邏輯有單元測試
- [ ] 覆蓋率 ≥ 70%（關鍵模組 ≥ 90%）

---

## 審查回饋格式

```markdown
## 📋 Code Review 結果

### ✅ 做得好
- 錯誤處理完善
- 命名清晰易懂

### ⚠️ 建議改進
| 位置 | 問題 | 建議 |
|------|------|------|
| `api/users.js:45` | N+1 查詢 | 使用 JOIN 或 eager loading |
| `components/List.tsx:12` | 無虛擬滾動 | 列表超過 100 項應使用 react-window |

### 🚨 必須修正
| 位置 | 問題 | 風險 |
|------|------|------|
| `config.js:3` | 硬編碼 API Key | 安全風險：Key 可能洩漏 |

### 📊 整體評分
- 安全性：⭐⭐⭐⭐⭐
- 效能：⭐⭐⭐⭐☆
- 可維護性：⭐⭐⭐⭐⭐
- 測試覆蓋：⭐⭐⭐☆☆
```

---

## 常見問題與修正建議

### 問題 1：巢狀地獄

```javascript
// ❌ 錯誤
if (user) {
  if (user.role === 'admin') {
    if (user.isActive) {
      // 業務邏輯
    }
  }
}

// ✅ 正確：Early Return
if (!user) return;
if (user.role !== 'admin') return;
if (!user.isActive) return;
// 業務邏輯
```

### 問題 2：過長函式

```javascript
// ❌ 錯誤：100+ 行的函式
function processOrder(order) {
  // 驗證
  // 計算價格
  // 處理庫存
  // 發送通知
  // 記錄日誌
  // ...
}

// ✅ 正確：拆分成小函式
function processOrder(order) {
  validateOrder(order);
  const total = calculateTotal(order);
  updateInventory(order.items);
  sendConfirmation(order);
  logOrderEvent(order);
}
```

### 問題 3：Magic Numbers

```javascript
// ❌ 錯誤
if (user.loginAttempts > 5) {
  lockAccount(user);
}

// ✅ 正確
const MAX_LOGIN_ATTEMPTS = 5;
if (user.loginAttempts > MAX_LOGIN_ATTEMPTS) {
  lockAccount(user);
}
```

### 問題 4：回調地獄

```javascript
// ❌ 錯誤
getUser(userId, (user) => {
  getOrders(user.id, (orders) => {
    processOrders(orders, (result) => {
      sendNotification(result, () => {
        console.log('Done');
      });
    });
  });
});

// ✅ 正確：使用 async/await
const user = await getUser(userId);
const orders = await getOrders(user.id);
const result = await processOrders(orders);
await sendNotification(result);
console.log('Done');
```

---

## PR 描述模板

```markdown
## 📝 變更說明
簡述這個 PR 做了什麼

## 🎯 關聯 Issue
Closes #123

## 📋 變更類型
- [ ] 新功能
- [ ] Bug 修復
- [ ] 重構
- [ ] 文件更新
- [ ] 測試

## 🧪 測試方式
1. 步驟一
2. 步驟二
3. 預期結果

## 📸 截圖（若為 UI 變更）
Before | After
-------|------
![](before.png) | ![](after.png)

## ✅ 自我檢查
- [ ] 程式碼符合專案規範
- [ ] 無 console.log / debugger
- [ ] 測試通過
- [ ] 文件已更新（若需要）
```

---

**原則**：Code Review 的目的是「改善程式碼」，而非「批評作者」。保持建設性與尊重。
