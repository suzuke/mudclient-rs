---
name: db-schema
description: |
  資料庫設計規範 Skill。命名規範、索引策略、多租戶設計、Migration 規範。
  任務涉及「資料庫」、「Schema」、「ERD」、「Migration」關鍵字時自動載入。
license: MIT
compatibility: Pixiu Agent / Claude Code / Cursor
metadata:
  author: pixiu
  version: "1.0"
  category: backend
---

# DB Schema Skill - 資料庫設計規範

> **用途**：命名規範、索引策略、多租戶設計、Migration 規範

---

## 設計原則

1. **正規化優先**：減少資料重複，確保一致性
2. **適度反正規化**：讀取頻繁的資料可適度冗餘
3. **索引策略**：WHERE、JOIN、ORDER BY 欄位建立索引
4. **軟刪除**：使用 `deleted_at` 而非直接刪除

---

## 通用欄位規範

### 每張表必備欄位

| 欄位 | 類型 | 說明 |
|------|------|------|
| `id` | UUID / BIGINT | 主鍵 |
| `created_at` | TIMESTAMP | 建立時間 |
| `updated_at` | TIMESTAMP | 更新時間 |

### 選用欄位

| 欄位 | 類型 | 說明 |
|------|------|------|
| `deleted_at` | TIMESTAMP | 軟刪除時間 |
| `created_by` | UUID | 建立者 |
| `updated_by` | UUID | 更新者 |

---

## 命名規範

| 類型 | 格式 | 範例 |
|------|------|------|
| 表名 | snake_case 複數 | `users`, `order_items` |
| 欄位名 | snake_case | `first_name`, `created_at` |
| 外鍵 | `<關聯表單數>_id` | `user_id`, `restaurant_id` |
| 索引 | `idx_<表>_<欄位>` | `idx_users_email` |
| 唯一索引 | `uniq_<表>_<欄位>` | `uniq_users_email` |

---

## 索引策略

### 必須建立索引

- 外鍵欄位
- WHERE 條件常用欄位
- ORDER BY 欄位
- 唯一性約束欄位

### 複合索引順序

遵循「選擇性高 → 低」原則：

```sql
-- ✅ 正確：status 選擇性低，created_at 選擇性高
CREATE INDEX idx_orders_status_created ON orders(status, created_at);

-- 查詢能使用索引
SELECT * FROM orders WHERE status = 'pending' AND created_at > '2024-01-01';
```

### 避免過度索引

- 每個索引都有寫入成本
- 監控未使用的索引並移除

---

## 多租戶設計

### 方案比較

| 方案 | 說明 | 優點 | 缺點 |
|------|------|------|------|
| **共享表** | 所有租戶同一表，用 `tenant_id` 區分 | 簡單、成本低 | 需注意隔離 |
| **Schema 隔離** | 每租戶獨立 schema | 隔離性好 | 維護複雜 |
| **資料庫隔離** | 每租戶獨立資料庫 | 完全隔離 | 成本最高 |

### 共享表設計（推薦）

```sql
CREATE TABLE restaurants (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,  -- 租戶識別
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    
    -- 每張表都要有 tenant_id 索引
    INDEX idx_restaurants_tenant (tenant_id)
);

-- 所有查詢都必須帶 tenant_id
SELECT * FROM restaurants WHERE tenant_id = ? AND id = ?;
```

### Row Level Security (PostgreSQL)

```sql
-- 啟用 RLS
ALTER TABLE restaurants ENABLE ROW LEVEL SECURITY;

-- 建立策略
CREATE POLICY tenant_isolation ON restaurants
    USING (tenant_id = current_setting('app.tenant_id')::uuid);
```

---

## 關聯設計

### 一對多

```sql
-- 一個餐廳有多個菜單
CREATE TABLE menus (
    id UUID PRIMARY KEY,
    restaurant_id UUID NOT NULL REFERENCES restaurants(id),
    name VARCHAR(255) NOT NULL
);
```

### 多對多

```sql
-- 訂單與餐點的多對多關係
CREATE TABLE order_items (
    id UUID PRIMARY KEY,
    order_id UUID NOT NULL REFERENCES orders(id),
    menu_item_id UUID NOT NULL REFERENCES menu_items(id),
    quantity INT NOT NULL,
    price DECIMAL(10,2) NOT NULL,
    
    UNIQUE(order_id, menu_item_id)
);
```

---

## Migration 規範

### 檔案命名

```
migrations/
├── 20240115_001_create_users.sql
├── 20240115_002_create_restaurants.sql
└── 20240116_001_add_phone_to_users.sql
```

### Migration 內容

```sql
-- Up
ALTER TABLE users ADD COLUMN phone VARCHAR(20);
CREATE INDEX idx_users_phone ON users(phone);

-- Down
DROP INDEX idx_users_phone;
ALTER TABLE users DROP COLUMN phone;
```

### 原則

- ✅ 每個 migration 都要有 rollback
- ✅ 避免鎖表操作（大表加欄位用 `CONCURRENTLY`）
- ❌ 不在 migration 中修改資料（另開 seed/data migration）

---

## 效能注意事項

### 避免 N+1

```sql
-- ❌ 錯誤：N+1 查詢
SELECT * FROM orders WHERE user_id = 1;
-- 然後對每個 order 執行
SELECT * FROM order_items WHERE order_id = ?;

-- ✅ 正確：JOIN 或子查詢
SELECT o.*, oi.*
FROM orders o
LEFT JOIN order_items oi ON o.id = oi.order_id
WHERE o.user_id = 1;
```

### 大表分頁

```sql
-- ❌ 避免 OFFSET（效能差）
SELECT * FROM orders ORDER BY id LIMIT 20 OFFSET 10000;

-- ✅ 使用游標分頁
SELECT * FROM orders WHERE id > 'last_seen_id' ORDER BY id LIMIT 20;
```

---

## 檢查清單

- [ ] 每張表有 id, created_at, updated_at
- [ ] 外鍵欄位有索引
- [ ] 多租戶表有 tenant_id 並建立索引
- [ ] Migration 有 up 和 down
- [ ] 避免 N+1 查詢
- [ ] 敏感欄位已加密

---

**原則**：資料庫設計決定系統上限。寧可前期多花時間設計，也不要後期痛苦重構。
