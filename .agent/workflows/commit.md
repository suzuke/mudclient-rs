---
description: Git Commit 規範與 Conventional Commits
version: 1.0.0
---

# Commit Workflow - Git 提交規範

trigger: 任務涉及「commit」、「提交」、「PR」、「merge」、「版本」關鍵字時自動載入

---

## Conventional Commits 格式

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Type（必填）

| Type | 說明 | 版本變化 |
|------|------|----------|
| `feat` | 新功能 | MINOR |
| `fix` | Bug 修復 | PATCH |
| `docs` | 文件更新 | - |
| `style` | 程式碼格式（不影響邏輯） | - |
| `refactor` | 重構（不新增功能/不修 bug） | - |
| `perf` | 效能優化 | PATCH |
| `test` | 測試相關 | - |
| `chore` | 建置工具、依賴更新 | - |
| `ci` | CI/CD 設定 | - |
| `build` | 打包相關 | - |
| `revert` | 回滾 commit | PATCH |

### Scope（選填）

描述影響範圍，如：`api`、`ui`、`auth`、`db`

### Subject（必填）

- 使用祈使句（如：「add」而非「added」）
- 首字母小寫
- 結尾不加句號
- 最多 50 字元

---

## 範例

### 基本範例

```bash
feat(auth): add Google OAuth login

fix(api): handle null response from payment gateway

docs: update README with installation steps

refactor(ui): extract Button component from Form
```

### 完整範例（含 body 與 footer）

```bash
feat(payment): add Stripe subscription support

- Implement monthly and yearly subscription plans
- Add webhook handler for subscription events
- Store subscription status in user profile

Closes #123
BREAKING CHANGE: Payment API response format changed
```

---

## Breaking Changes

當有破壞性變更時：

```bash
# 方法 1：在 type 後加 !
feat(api)!: change user endpoint response format

# 方法 2：在 footer 加 BREAKING CHANGE
feat(api): change user endpoint response format

BREAKING CHANGE: user.name changed to user.fullName
```

---

## Commit Message 檢查清單

- [ ] Type 正確（feat/fix/docs...）
- [ ] Subject 使用祈使句
- [ ] Subject 長度 ≤ 50 字元
- [ ] 如有破壞性變更，已標註 BREAKING CHANGE
- [ ] 如有關聯 Issue，已在 footer 註明（如 `Closes #123`）

---

## Git Hooks 自動檢查

### 安裝 commitlint

```bash
npm install --save-dev @commitlint/cli @commitlint/config-conventional
```

### commitlint.config.js

```javascript
module.exports = {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'type-enum': [2, 'always', [
      'feat', 'fix', 'docs', 'style', 'refactor', 
      'perf', 'test', 'chore', 'ci', 'build', 'revert'
    ]],
    'subject-max-length': [2, 'always', 72],
  }
};
```

### .husky/commit-msg

```bash
#!/bin/sh
npx --no -- commitlint --edit $1
```

---

## PR / Merge 規範

### PR 標題

使用與 commit 相同的 Conventional Commits 格式：

```
feat(auth): add password reset functionality
```

### Merge 策略

| 策略 | 適用情境 |
|------|----------|
| **Squash Merge** | 功能分支 → main（推薦） |
| **Merge Commit** | release 分支 → main |
| **Rebase** | 同步 main 到功能分支 |

### 分支命名

```
feat/add-login-page
fix/payment-null-error
docs/update-api-docs
refactor/user-service
```

---

## 版本號更新時機

| Commit Type | 版本變化 | 範例 |
|-------------|----------|------|
| `fix` | PATCH | 1.0.0 → 1.0.1 |
| `feat` | MINOR | 1.0.0 → 1.1.0 |
| `BREAKING CHANGE` | MAJOR | 1.0.0 → 2.0.0 |

---

## 常見錯誤

```bash
# ❌ 錯誤：type 錯誤
update: fix login bug          # 沒有 "update" 這個 type

# ✅ 正確
fix(auth): resolve login null pointer exception

# ❌ 錯誤：使用過去式
feat: added new feature        # 應該用祈使句

# ✅ 正確
feat: add new feature

# ❌ 錯誤：太長且無意義
fix: fix bug                   # 沒說明修了什麼

# ✅ 正確
fix(cart): prevent negative quantity on checkout
```

---

## 自動化工具

### standard-version（自動版本號 + changelog）

```bash
npm install --save-dev standard-version

# 執行版本發布
npx standard-version
```

### semantic-release（CI/CD 自動發布）

```bash
npm install --save-dev semantic-release

# 在 CI 中自動：
# 1. 分析 commit 決定版本
# 2. 更新 package.json
# 3. 生成 changelog
# 4. 建立 Git tag
# 5. 發布到 npm（如需要）
```

---

**原則**：好的 commit message 就是未來的你送給自己的禮物。半年後 debug 時會感謝當初寫清楚。
