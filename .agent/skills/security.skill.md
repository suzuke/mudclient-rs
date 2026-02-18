---
name: security
description: |
  安全性基線 Skill。防 SQL Injection、XSS、CSRF、權限控管、加密規範。
  任務涉及「安全」、「認證」、「授權」、「API」、「XSS」、「SQL Injection」關鍵字時自動載入。
license: MIT
compatibility: Pixiu Agent / Claude Code / Cursor
metadata:
  author: pixiu
  version: "1.0"
  category: security
---

# Security Skill - 安全性基線

> **用途**：防 SQL Injection、XSS、CSRF、權限控管、加密規範

---

## 核心原則

1. **永不信任用戶輸入**：所有輸入必須驗證
2. **最小權限原則**：只給必要的權限
3. **縱深防禦**：多層安全機制（前端驗證 + 後端驗證 + 資料庫約束）
4. **預設安全**：安全配置應為預設值，非選項

---

## API Key 管理

### 基本規則

- ✅ 所有 API Key 必須放 `.env`
- ❌ 絕不寫死在程式碼中
- ✅ 提供 `.env.example` 範本（不含真實 key）
- ✅ 定期輪替 Key（每季至少 1 次）

### 範例

#### .env

```bash
# ❌ 不要 commit 此檔案
STRIPE_SECRET_KEY=sk_live_xxxxxxxxxxxxx
DATABASE_URL=postgresql://user:pass@localhost/mydb
JWT_SECRET=your-super-secret-jwt-key-change-this
```

#### .env.example

```bash
# ✅ 提交此檔案供團隊參考
STRIPE_SECRET_KEY=sk_live_your_stripe_secret_key
DATABASE_URL=postgresql://user:password@localhost:5432/dbname
JWT_SECRET=your-jwt-secret-key-at-least-32-characters
```

#### .gitignore

```
.env
.env.local
.env.*.local
```

---

## 防止 SQL Injection

### 原則

- ✅ 使用參數化查詢（Prepared Statements）
- ❌ 永不拼接 SQL 字串

### 錯誤示範（❌ 危險）

```javascript
// Node.js
const userId = req.params.id;
const query = `SELECT * FROM users WHERE id = ${userId}`; // ❌ 危險！
db.query(query);
```

```python
# Python
user_id = request.args.get('id')
query = f"SELECT * FROM users WHERE id = {user_id}"  # ❌ 危險！
cursor.execute(query)
```

### 正確示範（✅ 安全）

#### Node.js (PostgreSQL)

```javascript
const userId = req.params.id;
const query = 'SELECT * FROM users WHERE id = $1'; // ✅ 參數化
const result = await db.query(query, [userId]);
```

#### Python (Django ORM)

```python
user_id = request.GET.get('id')
user = User.objects.get(id=user_id)  # ✅ ORM 自動處理
```

#### Python (Raw SQL)

```python
user_id = request.args.get('id')
query = "SELECT * FROM users WHERE id = %s"  # ✅ 參數化
cursor.execute(query, (user_id,))
```

---

## 防止 XSS (Cross-Site Scripting)

### 原則

- ✅ 跳脫 HTML 輸出
- ✅ 使用 Content Security Policy (CSP)
- ❌ 永不使用 `dangerouslySetInnerHTML` / `innerHTML`（除非內容已消毒）

### 錯誤示範（❌ 危險）

```javascript
// React
const userName = props.userName;
return <div dangerouslySetInnerHTML={{ __html: userName }} />; // ❌ 危險！

// Vanilla JS
element.innerHTML = userInput; // ❌ 危險！
```

### 正確示範（✅ 安全）

#### React

```javascript
const userName = props.userName;
return <div>{userName}</div>; // ✅ React 自動跳脫
```

#### Vanilla JS

```javascript
element.textContent = userInput; // ✅ 安全
// 或使用 DOMPurify
import DOMPurify from 'dompurify';
element.innerHTML = DOMPurify.sanitize(userInput);
```

#### Express.js (設定 CSP)

```javascript
const helmet = require('helmet');
app.use(helmet.contentSecurityPolicy({
  directives: {
    defaultSrc: ["'self'"],
    scriptSrc: ["'self'", "https://trusted-cdn.com"],
    styleSrc: ["'self'", "'unsafe-inline'"], // 僅在必要時
  }
}));
```

---

## 防止 CSRF (Cross-Site Request Forgery)

### 原則

- ✅ 使用 CSRF Token
- ✅ 檢查 `Referer` / `Origin` Header
- ✅ SameSite Cookie 屬性

### 實作範例

#### Express.js

```javascript
const csrf = require('csurf');
const csrfProtection = csrf({ cookie: true });

app.post('/api/transfer', csrfProtection, (req, res) => {
  // 自動驗證 CSRF token
  res.json({ success: true });
});
```

#### Django

```python
# settings.py
MIDDLEWARE = [
    'django.middleware.csrf.CsrfViewMiddleware',  # ✅ 預設啟用
]

# views.py
from django.views.decorators.csrf import csrf_protect

@csrf_protect
def transfer_money(request):
    # 自動驗證 CSRF token
    pass
```

#### Cookie 設定

```javascript
res.cookie('sessionId', sessionId, {
  httpOnly: true,      // 防止 JS 讀取
  secure: true,        // 僅 HTTPS
  sameSite: 'strict',  // 防止 CSRF
});
```

---

## 密碼安全

### 原則

- ✅ 使用 bcrypt / argon2 / scrypt
- ❌ 永不使用 MD5 / SHA1（已被破解）
- ✅ 密碼最小長度 ≥ 8 字元
- ✅ 建議包含大小寫、數字、特殊符號

### 實作範例

#### Node.js (bcrypt)

```javascript
const bcrypt = require('bcrypt');

// 註冊時
const saltRounds = 10;
const hashedPassword = await bcrypt.hash(plainPassword, saltRounds);
await db.query('INSERT INTO users (email, password) VALUES ($1, $2)', [email, hashedPassword]);

// 登入時
const user = await db.query('SELECT * FROM users WHERE email = $1', [email]);
const isValid = await bcrypt.compare(plainPassword, user.password);
```

#### Python (Django)

```python
from django.contrib.auth.hashers import make_password, check_password

# 註冊時
hashed_password = make_password(plain_password)
user = User.objects.create(email=email, password=hashed_password)

# 登入時
is_valid = check_password(plain_password, user.password)
```

---

## CORS 配置

### 原則

- ❌ 生產環境禁止 `Access-Control-Allow-Origin: *`
- ✅ 明確指定允許的來源

### 錯誤示範（❌ 危險）

```javascript
app.use((req, res, next) => {
  res.header('Access-Control-Allow-Origin', '*'); // ❌ 任何網站都可存取
  next();
});
```

### 正確示範（✅ 安全）

#### Express.js

```javascript
const cors = require('cors');

const corsOptions = {
  origin: ['https://myapp.com', 'https://admin.myapp.com'],
  credentials: true, // 允許帶 cookie
};

app.use(cors(corsOptions));
```

#### Nginx

```nginx
add_header Access-Control-Allow-Origin "https://myapp.com";
add_header Access-Control-Allow-Credentials "true";
```

---

## JWT (JSON Web Token) 安全

### 原則

- ✅ 使用強密鑰（≥ 256 bits）
- ✅ 設定合理過期時間（≤ 15 分鐘 for access token）
- ✅ 使用 Refresh Token 機制
- ❌ 不在 JWT 中儲存敏感資料（密碼、信用卡號）

### 實作範例

#### Node.js (jsonwebtoken)

```javascript
const jwt = require('jsonwebtoken');

// 生成 token
const accessToken = jwt.sign(
  { userId: user.id, role: user.role },
  process.env.JWT_SECRET,
  { expiresIn: '15m' } // 15 分鐘過期
);

const refreshToken = jwt.sign(
  { userId: user.id },
  process.env.JWT_REFRESH_SECRET,
  { expiresIn: '7d' } // 7 天過期
);

// 驗證 token
try {
  const decoded = jwt.verify(token, process.env.JWT_SECRET);
  req.user = decoded;
} catch (err) {
  return res.status(401).json({ error: 'Invalid token' });
}
```

---

## 輸入驗證

### 原則

- ✅ 前端驗證（UX）+ 後端驗證（安全）
- ✅ 使用白名單（允許清單），而非黑名單
- ✅ 驗證資料類型、長度、格式

### 實作範例

#### Express.js (express-validator)

```javascript
const { body, validationResult } = require('express-validator');

app.post('/api/users',
  // 驗證規則
  body('email').isEmail().normalizeEmail(),
  body('password').isLength({ min: 8 }).matches(/^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)/),
  body('age').optional().isInt({ min: 0, max: 120 }),
  
  (req, res) => {
    const errors = validationResult(req);
    if (!errors.isEmpty()) {
      return res.status(400).json({ errors: errors.array() });
    }
    
    // 處理請求
  }
);
```

#### Python (Pydantic)

```python
from pydantic import BaseModel, EmailStr, validator

class UserCreate(BaseModel):
    email: EmailStr
    password: str
    age: int | None = None

    @validator('password')
    def password_strength(cls, v):
        if len(v) < 8:
            raise ValueError('密碼至少 8 字元')
        if not any(c.isupper() for c in v):
            raise ValueError('密碼必須包含大寫字母')
        return v

    @validator('age')
    def age_range(cls, v):
        if v is not None and (v < 0 or v > 120):
            raise ValueError('年齡必須在 0-120 之間')
        return v
```

---

## Rate Limiting

### 原則

- ✅ 防止暴力破解（登入、註冊）
- ✅ 防止 DDoS 攻擊
- ✅ 防止 API 濫用

### 實作範例

#### Express.js (express-rate-limit)

```javascript
const rateLimit = require('express-rate-limit');

const loginLimiter = rateLimit({
  windowMs: 15 * 60 * 1000, // 15 分鐘
  max: 5, // 最多 5 次嘗試
  message: '嘗試次數過多，請 15 分鐘後再試',
});

app.post('/api/login', loginLimiter, (req, res) => {
  // 處理登入
});
```

#### Nginx

```nginx
limit_req_zone $binary_remote_addr zone=login:10m rate=5r/m;

location /api/login {
    limit_req zone=login burst=3 nodelay;
}
```

---

## 安全檢查清單

### 開發階段

- [ ] 所有 API Key 放 `.env`
- [ ] `.env` 已加入 `.gitignore`
- [ ] 使用參數化查詢（防 SQL Injection）
- [ ] HTML 輸出已跳脫（防 XSS）
- [ ] 使用 CSRF Token
- [ ] 密碼使用 bcrypt/argon2
- [ ] 敏感操作有 Rate Limiting

### 部署前

- [ ] 執行 `npm audit` / `pip-audit` 檢查漏洞
- [ ] CORS 僅允許信任的來源
- [ ] HTTPS 已啟用（生產環境）
- [ ] Cookie 設定 `httpOnly`, `secure`, `sameSite`
- [ ] CSP Header 已設定
- [ ] 錯誤訊息不洩漏內部資訊

### 定期檢查

- [ ] API Key 輪替（每季）
- [ ] 依賴套件更新
- [ ] 安全日誌審查
- [ ] 滲透測試（每年）

---

## 常見錯誤與解決方案

### 錯誤 1：錯誤訊息洩漏內部資訊

```javascript
// ❌ 錯誤
catch (err) {
  res.status(500).json({ error: err.message }); // 可能洩漏 SQL 結構
}

// ✅ 正確
catch (err) {
  logger.error(err); // 記錄到後端
  res.status(500).json({ error: '伺服器錯誤，請稍後再試' });
}
```

### 錯誤 2：Session Fixation

```javascript
// ❌ 錯誤：登入後未重新生成 session
app.post('/login', (req, res) => {
  req.session.userId = user.id; // 舊 session 未銷毀
});

// ✅ 正確
app.post('/login', (req, res) => {
  req.session.regenerate(err => {
    req.session.userId = user.id;
  });
});
```

---

**原則**：安全性不是「加上去的功能」，而是設計的一部分。從第一行程式碼就要考慮安全。
