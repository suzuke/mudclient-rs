---
name: naming-convention
description: |
  命名規範 Skill。語意化命名、檔案結構規範、變數/函式命名規則。
  新增檔案、函式、變數、元件時自動參考。
license: MIT
compatibility: Pixiu Agent / Claude Code / Cursor
metadata:
  author: pixiu
  version: "1.0"
  category: patterns
---

# Naming Convention Skill - 命名規範

> **用途**：語意化命名、檔案結構規範、變數/函式命名規則

---

## 通用規則

| 類型 | 格式 | 範例 |
|------|------|------|
| 檔案名（JS/TS） | kebab-case | `user-profile.js` |
| 檔案名（React 元件） | PascalCase | `UserProfile.tsx` |
| 目錄名 | kebab-case | `user-management/` |
| 函式名 | camelCase | `getUserById()` |
| 變數名 | camelCase | `userName` |
| 常數 | SCREAMING_SNAKE | `MAX_RETRIES` |
| 類別名 | PascalCase | `UserService` |
| 介面名（TS） | PascalCase + I 前綴（可選） | `User` 或 `IUser` |
| 型別名（TS） | PascalCase | `UserType` |
| CSS Class | kebab-case | `.user-avatar` |
| Tailwind Class | 標準格式 | `bg-blue-500` |
| API 路由 | kebab-case | `/api/user-profile` |
| DB 欄位 | snake_case | `created_at` |
| 環境變數 | SCREAMING_SNAKE | `DATABASE_URL` |

---

## 語意化命名

### 動詞前綴

| 動作 | 前綴 | 範例 |
|------|------|------|
| 取得單筆 | `get` | `getUser(id)` |
| 取得多筆 | `get`, `list`, `fetch` | `getUsers()`, `listOrders()` |
| 建立 | `create`, `add` | `createOrder()` |
| 更新 | `update`, `set` | `updateProfile()` |
| 刪除 | `delete`, `remove` | `deleteUser()` |
| 檢查布林 | `is`, `has`, `can`, `should` | `isAdmin()`, `hasPermission()` |
| 轉換 | `to`, `from`, `parse`, `format` | `toJSON()`, `parseDate()` |
| 驗證 | `validate`, `check` | `validateEmail()` |
| 初始化 | `init`, `setup` | `initDatabase()` |
| 重置 | `reset`, `clear` | `resetForm()` |

### 布林變數命名

```javascript
// ✅ 正確
const isLoading = true;
const hasError = false;
const canEdit = user.role === 'admin';
const shouldRefresh = Date.now() > lastUpdate + 60000;

// ❌ 錯誤
const loading = true;      // 不明確是否為布林
const error = false;       // 可能誤認為錯誤物件
const edit = true;         // 是動詞還是狀態？
```

---

## React / Vue 元件命名

### 檔案結構

```
components/
├── common/                 # 通用元件
│   ├── Button.tsx
│   ├── Input.tsx
│   └── Modal.tsx
├── layout/                 # 版面元件
│   ├── Header.tsx
│   ├── Sidebar.tsx
│   └── Footer.tsx
└── features/               # 功能模組元件
    └── user/
        ├── UserList.tsx
        ├── UserCard.tsx
        └── UserForm.tsx
```

### 元件命名規則

```javascript
// ✅ 正確
export function UserProfileCard() { ... }    // PascalCase
export function useUserData() { ... }        // Hook: use 前綴
export const UserContext = createContext();  // Context: 後綴

// ❌ 錯誤
export function userProfileCard() { ... }    // 小寫開頭
export function UserData() { ... }           // Hook 沒有 use 前綴
```

---

## API 路由命名

### RESTful 規則

| 操作 | HTTP Method | 路由 | 範例 |
|------|-------------|------|------|
| 取得列表 | GET | `/resource` | `GET /users` |
| 取得單筆 | GET | `/resource/:id` | `GET /users/123` |
| 建立 | POST | `/resource` | `POST /users` |
| 更新 | PUT/PATCH | `/resource/:id` | `PUT /users/123` |
| 刪除 | DELETE | `/resource/:id` | `DELETE /users/123` |

### 複合資源

```
GET  /users/123/orders          # 取得用戶的訂單
POST /users/123/orders          # 為用戶建立訂單
GET  /users/123/orders/456      # 取得特定訂單
```

### 動作端點（非 CRUD）

```
POST /users/123/activate        # 啟用用戶
POST /orders/456/cancel         # 取消訂單
POST /auth/login                # 登入
POST /auth/logout               # 登出
```

---

## 資料庫欄位命名

### 通用欄位

| 欄位 | 說明 |
|------|------|
| `id` | 主鍵（UUID 或自增） |
| `created_at` | 建立時間 |
| `updated_at` | 更新時間 |
| `deleted_at` | 軟刪除時間 |
| `created_by` | 建立者 ID |

### 關聯欄位

```sql
-- 外鍵命名：<關聯表>_id
user_id     -- 關聯 users 表
order_id    -- 關聯 orders 表
category_id -- 關聯 categories 表
```

### 布林欄位

```sql
-- 使用 is_ 或 has_ 前綴
is_active
is_verified
has_subscription
can_edit
```

---

## 測試檔案命名

| 類型 | 格式 | 範例 |
|------|------|------|
| 單元測試 | `*.test.js` 或 `*.spec.js` | `formatDate.test.js` |
| 整合測試 | `*.integration.test.js` | `users.integration.test.js` |
| E2E 測試 | `*.e2e.js` 或 `*.spec.js` | `login.e2e.js` |

---

## 命名檢查清單

- [ ] 名稱能描述其用途（不需註解即可理解）
- [ ] 避免縮寫（除非是業界通用：id, url, api）
- [ ] 避免單字母變數（除 i, j 用於迴圈）
- [ ] 布林值使用 is/has/can/should 前綴
- [ ] 函式使用動詞開頭
- [ ] 類別使用名詞

---

**原則**：好的命名是最好的文件。程式碼應該「自我說明」(self-documenting)。
