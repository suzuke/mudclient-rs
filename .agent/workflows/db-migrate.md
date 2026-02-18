---
description: 資料庫遷移與 Rollback 規範
version: 1.0.0
---

# DB Migrate Workflow - 資料庫遷移規範

trigger: 任務涉及「migration」、「遷移」、「schema」、「rollback」、「seed」關鍵字時自動載入

---

## 核心原則

1. **每個 migration 都必須可回滾**
2. **先測試再執行**（staging → production）
3. **大表操作要特別小心**（可能鎖表）
4. **永不在 migration 中修改資料**（用 data migration）

---

## Migration 檔案規範

### 檔案命名

```
migrations/
├── 20240127_001_create_users.sql
├── 20240127_002_add_email_index.sql
├── 20240128_001_create_orders.sql
└── 20240128_002_add_status_to_orders.sql
```

格式：`YYYYMMDD_NNN_description.sql`

### 檔案內容

```sql
-- Migration: 20240127_001_create_users
-- Description: 建立用戶表
-- Author: developer@example.com

-- ========== UP ==========
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);

-- ========== DOWN ==========
DROP INDEX IF EXISTS idx_users_email;
DROP TABLE IF EXISTS users;
```

---

## Phase 1: 建立 Migration

### 手動建立

```bash
# 建立新檔案
touch migrations/$(date +%Y%m%d)_001_description.sql
```

### 框架命令

```bash
# Node.js (Prisma)
npx prisma migrate dev --name add_user_table

# Node.js (TypeORM)
npx typeorm migration:create src/migrations/AddUserTable

# Django
python manage.py makemigrations --name add_user_table

# Rails
rails generate migration AddUserTable

# Go (golang-migrate)
migrate create -ext sql -dir migrations add_user_table
```

---

## Phase 2: 測試 Migration

### 本地測試

```bash
# 1. 執行 UP
npm run migrate:up
# 或
python manage.py migrate

# 2. 驗證結果
psql -c "\\d users"

# 3. 執行 DOWN（確認可回滾）
npm run migrate:down
# 或
python manage.py migrate myapp 0001

# 4. 再次執行 UP（確認可重複執行）
npm run migrate:up
```

### Docker 隔離測試

```bash
# 建立臨時測試資料庫
docker run -d --name test-db -e POSTGRES_PASSWORD=test postgres:15

# 執行 migration
DATABASE_URL=postgresql://postgres:test@localhost:5432/postgres npm run migrate

# 清理
docker rm -f test-db
```

---

## Phase 3: 部署 Migration

### 標準流程

```bash
# 1. 備份資料庫（必要！）
pg_dump -U postgres -d mydb -F c -f backup_$(date +%Y%m%d_%H%M%S).dump

# 2. 在 staging 環境測試
DATABASE_URL=$STAGING_DB npm run migrate

# 3. 驗證 staging
# ... 手動或自動化測試 ...

# 4. 在 production 執行
DATABASE_URL=$PRODUCTION_DB npm run migrate

# 5. 驗證 production
# ... 監控錯誤日誌 ...
```

### CI/CD 自動執行

```yaml
# .github/workflows/deploy.yml
deploy:
  steps:
    - name: Backup Database
      run: |
        pg_dump $DATABASE_URL > backup.sql
        
    - name: Run Migrations
      run: npm run migrate
      
    - name: Verify
      run: npm run test:integration
```

---

## Phase 4: Rollback 策略

### 自動回滾

```bash
# 回滾最後一個 migration
npm run migrate:down
# 或
python manage.py migrate myapp 0012_previous

# 回滾到特定版本
npx prisma migrate resolve --rolled-back 20240127_001
```

### 手動回滾（緊急情況）

```bash
# 1. 停止應用程式（避免寫入新資料）
# 2. 從備份還原
pg_restore -U postgres -d mydb -c backup_20240127_143000.dump
# 3. 更新 migration 狀態表
```

### 回滾檢查清單

- [ ] 已停止寫入流量
- [ ] 已確認備份可用
- [ ] 已通知團隊
- [ ] 已記錄回滾原因

---

## Seed Data 管理

### 檔案結構

```
seeds/
├── 01_roles.sql           # 基礎角色
├── 02_admin_user.sql      # 管理員帳號
├── 03_categories.sql      # 分類資料
└── 99_dev_data.sql        # 開發測試資料（僅限 dev）
```

### Seed 範例

```sql
-- seeds/01_roles.sql
INSERT INTO roles (name, permissions) VALUES
    ('admin', '{"all": true}'),
    ('user', '{"read": true, "write": false}')
ON CONFLICT (name) DO NOTHING;
```

### 執行 Seed

```bash
# 執行所有 seed（不包含開發資料）
for f in seeds/[0-8]*.sql; do psql -d mydb -f "$f"; done

# 開發環境（包含測試資料）
for f in seeds/*.sql; do psql -d mydb -f "$f"; done
```

---

## 大表操作注意事項

### 可能鎖表的操作

| 操作 | 風險 | 解決方案 |
|------|------|----------|
| `ALTER TABLE ADD COLUMN` | 短暫鎖表 | 使用 `CONCURRENTLY`（如可用） |
| `CREATE INDEX` | 長時間鎖表 | `CREATE INDEX CONCURRENTLY` |
| `ALTER TABLE ADD CONSTRAINT` | 鎖表 | 分步驟執行 |
| 大量 DELETE/UPDATE | 長時間鎖定 | 批次處理 |

### 安全的新增欄位

```sql
-- ❌ 危險：可能鎖表
ALTER TABLE orders ADD COLUMN status VARCHAR(50) DEFAULT 'pending';

-- ✅ 安全：分兩步
ALTER TABLE orders ADD COLUMN status VARCHAR(50);
UPDATE orders SET status = 'pending' WHERE status IS NULL;
-- 分批次更新
```

### 安全的建立索引

```sql
-- ❌ 危險：鎖表
CREATE INDEX idx_orders_created ON orders(created_at);

-- ✅ 安全：PostgreSQL
CREATE INDEX CONCURRENTLY idx_orders_created ON orders(created_at);
```

---

## Migration 檢查清單

### 撰寫時

- [ ] 有 UP 和 DOWN
- [ ] 描述清楚（註解說明用途）
- [ ] 冪等性（可重複執行）
- [ ] 考慮大表影響

### 測試時

- [ ] 本地 UP 成功
- [ ] 本地 DOWN 成功
- [ ] 再次 UP 成功
- [ ] Staging 測試通過

### 部署時

- [ ] 已備份 Production
- [ ] Staging 已驗證
- [ ] 有回滾計畫
- [ ] 團隊已通知

---

## 常見錯誤

### 錯誤 1：沒有 DOWN

```sql
-- ❌ 只有 UP，無法回滾
CREATE TABLE users (...);
```

### 錯誤 2：不可逆操作

```sql
-- ⚠️ DROP COLUMN 無法完全還原
ALTER TABLE users DROP COLUMN phone;

-- 建議：先軟刪除，確認無問題再真正刪除
-- Step 1: 標記棄用
-- Step 2: 等待 2 週
-- Step 3: 真正刪除
```

### 錯誤 3：在 Migration 中修改資料

```sql
-- ❌ 錯誤：Migration 不應修改資料
UPDATE users SET role = 'admin' WHERE email = 'admin@example.com';

-- ✅ 正確：使用獨立的 data migration 或 seed
```

---

**原則**：Migration 就像 Git commit——每一步都應該是可追蹤、可回滾的。
