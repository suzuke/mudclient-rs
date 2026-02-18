---
name: tech-stack
description: |
  技術棧自動偵測 Skill。自動偵測前端/後端框架、資料庫、部署環境、套件管理器。
  新專案初始化或任務開始時自動載入。
license: MIT
compatibility: Pixiu Agent / Claude Code / Cursor
metadata:
  author: pixiu
  version: "1.0"
  category: workflow
---

# Tech Stack Skill - 技術棧自動偵測

> **用途**：自動偵測前端/後端框架、資料庫、部署環境、套件管理器

---

## 偵測規則（優先順序）

### 1. 前端框架

| 檔案特徵 | 技術 |
|----------|------|
| `next.config.*` | Next.js |
| `nuxt.config.*` | Nuxt.js |
| `vite.config.*` | Vite |
| `angular.json` | Angular |
| `package.json` 含 `react` | React |
| `package.json` 含 `vue` | Vue.js |
| `package.json` 含 `svelte` | Svelte |

### 2. 後端框架

| 檔案特徵 | 技術 |
|----------|------|
| `requirements.txt` 含 `django` | Django |
| `requirements.txt` 含 `fastapi` | FastAPI |
| `requirements.txt` 含 `flask` | Flask |
| `package.json` 含 `express` | Express.js |
| `package.json` 含 `nestjs` | NestJS |
| `go.mod` | Go |
| `Cargo.toml` | Rust |
| `pom.xml` | Java (Maven) |
| `build.gradle` | Java/Kotlin (Gradle) |

### 3. 資料庫

| 檔案特徵 | 技術 |
|----------|------|
| `docker-compose.yml` 含 `postgres` | PostgreSQL |
| `docker-compose.yml` 含 `mysql` | MySQL |
| `docker-compose.yml` 含 `mongo` | MongoDB |
| `docker-compose.yml` 含 `redis` | Redis |
| `prisma/schema.prisma` | Prisma ORM |

### 4. 部署環境

| 檔案特徵 | 技術 |
|----------|------|
| `vercel.json` | Vercel |
| `netlify.toml` | Netlify |
| `Dockerfile` | Docker |
| `docker-compose.yml` | Docker Compose |
| `zeabur.json` | Zeabur |
| `fly.toml` | Fly.io |
| `.github/workflows/*.yml` | GitHub Actions |

### 5. 套件管理器

| 檔案特徵 | 技術 |
|----------|------|
| `pnpm-lock.yaml` | pnpm |
| `yarn.lock` | Yarn |
| `package-lock.json` | npm |
| `bun.lockb` | Bun |
| `poetry.lock` | Poetry (Python) |
| `Pipfile.lock` | Pipenv (Python) |

---

## 版本偵測

### Node.js 專案

```bash
# 從 package.json 讀取
jq '.engines.node' package.json

# 從 .nvmrc 讀取
cat .nvmrc
```

### Python 專案

```bash
# 從 pyproject.toml 讀取
grep 'python' pyproject.toml

# 從 runtime.txt 讀取
cat runtime.txt
```

---

## 輸出格式

偵測完成後，輸出：

```markdown
🔍 專案技術棧偵測結果

| 類別 | 技術 | 版本 | 來源 |
|------|------|------|------|
| 前端 | Next.js | 14.x | next.config.js |
| 後端 | - | 純前端 | - |
| 資料庫 | PostgreSQL | 15.x | docker-compose.yml |
| 部署 | Vercel | - | vercel.json |
| 套件管理 | pnpm | 8.x | pnpm-lock.yaml |

⚠️ 不確定項目（需確認）：
- Node.js 版本未明確指定，建議新增 `.nvmrc`
```

---

## 偵測失敗處理

若無法偵測，詢問用戶（最多 3 個問題）：

```markdown
❓ 無法自動偵測技術棧，請回答以下問題：

1. 前端框架？
   A. React  B. Vue  C. Angular  D. Svelte  E. 其他/無

2. 後端語言？
   A. Node.js  B. Python  C. Go  D. Java  E. 其他/無

3. 資料庫？
   A. PostgreSQL  B. MySQL  C. MongoDB  D. SQLite  E. 其他/無
```

---

**原則**：不猜測，偵測不到就問。錯誤的假設比多問一個問題更浪費時間。
