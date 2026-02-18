---
name: api-design
description: |
  API 設計規範 Skill。RESTful 規範、版本控制、統一回應格式、分頁規範。
  任務涉及「API」、「REST」、「GraphQL」、「端點」關鍵字時自動載入。
license: MIT
compatibility: Pixiu Agent / Claude Code / Cursor
metadata:
  author: pixiu
  version: "1.0"
  category: backend
---

# API Design Skill - API 設計規範

> **用途**：RESTful 規範、版本控制、統一回應格式、分頁規範

---

## RESTful 規範

### HTTP 方法

| 方法 | 用途 | 範例 |
|------|------|------|
| GET | 讀取資源 | `GET /users/123` |
| POST | 建立資源 | `POST /users` |
| PUT | 完整更新 | `PUT /users/123` |
| PATCH | 部分更新 | `PATCH /users/123` |
| DELETE | 刪除資源 | `DELETE /users/123` |

### 狀態碼

| 狀態碼 | 用途 |
|--------|------|
| 200 | 成功（GET/PUT/PATCH） |
| 201 | 建立成功（POST） |
| 204 | 刪除成功（DELETE） |
| 400 | 請求格式錯誤 |
| 401 | 未認證 |
| 403 | 無權限 |
| 404 | 資源不存在 |
| 409 | 資源衝突 |
| 422 | 驗證失敗 |
| 429 | 請求過於頻繁 |
| 500 | 伺服器錯誤 |

---

## API 版本控制

### 方式選擇

| 方式 | 範例 | 優點 | 缺點 |
|------|------|------|------|
| URL 路徑 | `/api/v1/users` | 簡單直觀 | URL 較長 |
| Header | `Accept: application/vnd.api+json;version=1` | URL 乾淨 | 不易測試 |
| Query | `/api/users?version=1` | 易於切換 | 易被忽略 |

**建議**：使用 URL 路徑（`/api/v1/`），最直觀且易於文件化。

---

## 統一回應格式

### 成功回應

```json
{
  "success": true,
  "data": {
    "id": 123,
    "name": "餐廳A"
  },
  "meta": {
    "timestamp": "2024-01-15T10:30:00Z"
  }
}
```

### 錯誤回應

```json
{
  "success": false,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "電子郵件格式不正確",
    "field": "email",
    "details": []
  }
}
```

### 列表回應（含分頁）

```json
{
  "success": true,
  "data": [...],
  "meta": {
    "page": 1,
    "perPage": 20,
    "total": 150,
    "totalPages": 8
  }
}
```

---

## 分頁規範

### 查詢參數

```
GET /api/v1/orders?page=2&perPage=20&sort=-createdAt
```

| 參數 | 說明 |
|------|------|
| `page` | 頁碼（從 1 開始） |
| `perPage` | 每頁筆數（預設 20，最大 100） |
| `sort` | 排序欄位（`-` 前綴為降序） |

### 游標分頁（大量資料）

```
GET /api/v1/orders?cursor=abc123&limit=20
```

---

## 篩選與搜尋

### 篩選

```
GET /api/v1/orders?status=pending&createdAt[gte]=2024-01-01
```

| 運算符 | 說明 |
|--------|------|
| `[eq]` | 等於（預設） |
| `[ne]` | 不等於 |
| `[gt]` | 大於 |
| `[gte]` | 大於等於 |
| `[lt]` | 小於 |
| `[lte]` | 小於等於 |
| `[in]` | 包含於 |

### 搜尋

```
GET /api/v1/products?q=咖啡
```

---

## 關聯資源

### 嵌套路由

```
GET /api/v1/restaurants/123/menus        # 取得餐廳的菜單
POST /api/v1/restaurants/123/orders      # 為餐廳建立訂單
```

### Expand 參數（減少 N+1）

```
GET /api/v1/orders/456?expand=customer,items
```

---

## API 文件

### OpenAPI 3.0 範本

```yaml
openapi: 3.0.0
info:
  title: 餐飲 SaaS API
  version: 1.0.0

paths:
  /api/v1/restaurants:
    get:
      summary: 取得餐廳列表
      parameters:
        - name: page
          in: query
          schema:
            type: integer
      responses:
        '200':
          description: 成功
```

---

## 檢查清單

- [ ] 使用正確的 HTTP 方法
- [ ] 回傳適當的狀態碼
- [ ] 統一回應格式
- [ ] 有版本控制
- [ ] 列表有分頁
- [ ] 有 OpenAPI 文件
- [ ] 敏感端點有認證

---

**原則**：好的 API 應該是「自解釋」的。看 URL 就知道做什麼，看回應就知道結果。
