# Deploy & Rollback Workflow - 部署與回滾
version: 1.0.0

trigger: 任務涉及「部署」、「回滾」、「rollback」、「release」關鍵字時自動載入

---

## Phase 1: 部署前檢查

### 必須完成項目
- [ ] 所有測試通過（`npm test` / `pytest` / `go test`）
- [ ] 程式碼已合併至主分支
- [ ] `.env.example` 與 `.env` 同步（但不包含敏感值）
- [ ] 資料庫 migrations 已測試（包含 up/down）
- [ ] 依賴套件無安全漏洞（`npm audit` / `pip-audit` / `go mod verify`）

### 版本號更新
遵循 [Semantic Versioning](https://semver.org/)：
- **MAJOR**（1.0.0 → 2.0.0）：破壞性變更
- **MINOR**（1.0.0 → 1.1.0）：新增功能，向後相容
- **PATCH**（1.0.0 → 1.0.1）：Bug 修復

更新位置：
- `package.json` (version)
- `docs/ChangeLog.md`
- Git tag: `git tag -a v1.2.0 -m "Release 1.2.0"`

---

## Phase 2: 備份策略

### 資料庫備份

#### PostgreSQL
```bash
# 備份
pg_dump -U postgres -d mydb -F c -f backup_$(date +%Y%m%d_%H%M%S).dump

# 還原
pg_restore -U postgres -d mydb -c backup_20240115_143000.dump
```

#### MySQL
```bash
# 備份
mysqldump -u root -p mydb > backup_$(date +%Y%m%d_%H%M%S).sql

# 還原
mysql -u root -p mydb < backup_20240115_143000.sql
```

#### MongoDB
```bash
# 備份
mongodump --db mydb --out backup_$(date +%Y%m%d_%H%M%S)

# 還原
mongorestore --db mydb backup_20240115_143000/mydb
```

### Docker 容器備份
```bash
# 建立當前容器快照
docker commit my_container backup_image:$(date +%Y%m%d)

# 若需回滾
docker stop my_container
docker run -d --name my_container backup_image:20240115
```

### 檔案系統備份
```bash
# 備份上傳目錄
tar -czf uploads_backup_$(date +%Y%m%d).tar.gz /var/www/uploads/

# 還原
tar -xzf uploads_backup_20240115.tar.gz -C /var/www/
```

---

## Phase 3: 部署執行

### 方案 A：手動部署（適合小型專案）

```bash
# 1. 拉取最新代碼
git pull origin main

# 2. 安裝依賴
npm install --production
# 或
pip install -r requirements.txt

# 3. 執行資料庫遷移
npm run migrate
# 或
python manage.py migrate

# 4. 建置前端資源
npm run build

# 5. 重啟服務
pm2 restart app
# 或
systemctl restart myapp
```

### 方案 B：Docker 部署

```bash
# 1. 建置新映像
docker build -t myapp:1.2.0 .

# 2. 停止舊容器（保留用於回滾）
docker stop myapp_old || true
docker rename myapp myapp_old || true

# 3. 啟動新容器
docker run -d \
  --name myapp \
  --env-file .env \
  -p 3000:3000 \
  myapp:1.2.0

# 4. 健康檢查（等待 30 秒）
sleep 30
curl -f http://localhost:3000/health || exit 1
```

### 方案 C：Kubernetes 部署

```bash
# 1. 更新 deployment
kubectl set image deployment/myapp myapp=myapp:1.2.0

# 2. 監控部署進度
kubectl rollout status deployment/myapp

# 3. 若失敗自動回滾
kubectl rollout undo deployment/myapp
```

### 方案 D：平台即服務（Vercel/Netlify/Zeabur）

```bash
# Vercel
vercel --prod

# Netlify
netlify deploy --prod

# Zeabur (通常透過 Git 自動觸發)
git push origin main
```

---

## Phase 4: 部署後驗證

### 自動化健康檢查
```bash
#!/bin/bash
# health-check.sh

BASE_URL="https://myapp.com"

# 1. 首頁可訪問
curl -f $BASE_URL || exit 1

# 2. API 健康檢查
curl -f $BASE_URL/api/health || exit 1

# 3. 資料庫連線正常
curl -f $BASE_URL/api/db-check || exit 1

echo "✅ 所有健康檢查通過"
```

### 手動驗證清單
- [ ] 首頁正常載入
- [ ] 登入功能正常
- [ ] 關鍵 API 端點回應正常
- [ ] 資料庫連線正常
- [ ] 靜態資源（CSS/JS/圖片）載入正常
- [ ] 日誌無異常錯誤
- [ ] 效能正常（API 回應 < 500ms）

---

## Phase 5: 回滾策略

### 緊急回滾決策樹
```
部署後出現問題
├─ 影響 < 5% 用戶？
│  └─ 是 → 發布 Hotfix（優先修復）
└─ 能在 30 分鐘內修復？
   └─ 否 → **立即回滾**
```

### 回滾方案 A：Git 回滾

```bash
# 1. 回滾至上一個 commit
git revert HEAD --no-edit

# 2. 推送變更
git push origin main

# 3. 重新部署
# （依照 Phase 3 的部署流程）
```

### 回滾方案 B：Docker 回滾

```bash
# 1. 停止當前容器
docker stop myapp

# 2. 啟動備份容器
docker start myapp_old
docker rename myapp_old myapp

# 3. 驗證
curl -f http://localhost:3000/health
```

### 回滾方案 C：Kubernetes 回滾

```bash
# 1. 查看歷史版本
kubectl rollout history deployment/myapp

# 2. 回滾至上一個版本
kubectl rollout undo deployment/myapp

# 3. 或回滾至特定版本
kubectl rollout undo deployment/myapp --to-revision=2
```

### 回滾方案 D：資料庫 Migration 回滾

```bash
# Node.js (Sequelize/TypeORM)
npm run migrate:undo

# Django
python manage.py migrate myapp 0012_previous_migration

# Rails
rails db:rollback

# 或手動還原備份
psql -U postgres -d mydb < backup_20240115_143000.sql
```

---

## 回滾後驗證

- [ ] API 健康檢查回傳 200
- [ ] 資料庫連線正常
- [ ] 關鍵功能可用（登入、核心業務流程）
- [ ] 日誌無異常錯誤
- [ ] 效能恢復正常（API 回應 < 500ms）
- [ ] 用戶回報問題減少

---

## 部署文件範本

### docs/Deployment.md

```markdown
# 部署文件

## 環境需求
- Node.js: 20.x
- PostgreSQL: 15.x
- Redis: 7.x

## 環境變數
見 `.env.example`

## 部署步驟
1. 備份資料庫（見 deploy.md Phase 2）
2. 執行部署腳本：`./scripts/deploy.sh`
3. 執行健康檢查：`./scripts/health-check.sh`

## 回滾步驟
見 `.agent/workflows/deploy.md` Phase 5

## 緊急聯絡人
- DevOps: devops@company.com
- 資料庫管理員: dba@company.com
```

---

## 部署檢查清單（完整版）

### 部署前
- [ ] 所有測試通過
- [ ] 版本號已更新
- [ ] ChangeLog 已更新
- [ ] .env.example 同步
- [ ] 資料庫備份完成
- [ ] 依賴套件無安全漏洞

### 部署中
- [ ] Git tag 已建立
- [ ] 部署腳本執行無錯誤
- [ ] Migration 執行成功

### 部署後
- [ ] 健康檢查通過
- [ ] 關鍵功能驗證通過
- [ ] 效能符合預期
- [ ] 日誌無異常
- [ ] 通知團隊部署完成

### 若失敗
- [ ] 立即執行回滾
- [ ] 記錄失敗原因
- [ ] 修復問題後重新部署

---

## 常見部署問題與解決方案

### 問題 1：Migration 失敗導致服務無法啟動
**解決**：
```bash
# 回滾 migration
npm run migrate:undo
# 修正問題後重試
npm run migrate
```

### 問題 2：新版本 API 不相容導致客戶端錯誤
**解決**：
- 短期：立即回滾
- 長期：實作 API 版本控制（`/api/v1`, `/api/v2`）

### 問題 3：部署後效能下降
**解決**：
```bash
# 檢查資料庫查詢
npm run analyze-queries
# 檢查記憶體使用
docker stats
```

---

**原則**：部署應該是無聊且可預測的。若部署讓你緊張，代表流程需要改進。