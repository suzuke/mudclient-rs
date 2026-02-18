# Testing Workflow - 測試規範
version: 1.0.0

trigger: 任務涉及「測試」、「單元測試」、「整合測試」、「E2E」關鍵字時自動載入

---

## 測試金字塔

```
     /\    E2E (10%)
    /  \   Integration (30%)
   /____\  Unit (60%)
```

**原則**：單元測試為主，整合測試為輔，E2E 測試僅覆蓋關鍵流程

---

## 單元測試 (Unit Tests)

### 觸發條件
- 新增/修改純函式
- 新增工具類別或業務邏輯函式

### 覆蓋率要求
- 一般功能：≥ 70%
- 關鍵業務邏輯：≥ 90%

### 範例（JavaScript/TypeScript）

```javascript
// utils/formatDate.test.js
describe('formatDate', () => {
  test('應將 ISO 時間轉為 YYYY-MM-DD', () => {
    expect(formatDate('2024-01-15T10:30:00Z')).toBe('2024-01-15');
  });

  test('應處理無效輸入', () => {
    expect(formatDate(null)).toBe('');
    expect(formatDate('invalid')).toBe('');
  });

  test('應處理時區', () => {
    expect(formatDate('2024-01-15T23:30:00+08:00')).toBe('2024-01-15');
  });
});
```

### 範例（Python）

```python
# tests/test_format_date.py
import pytest
from utils.date import format_date

def test_format_iso_date():
    assert format_date('2024-01-15T10:30:00Z') == '2024-01-15'

def test_handle_invalid_input():
    assert format_date(None) == ''
    assert format_date('invalid') == ''

def test_handle_timezone():
    assert format_date('2024-01-15T23:30:00+08:00') == '2024-01-15'
```

---

## 整合測試 (Integration Tests)

### 觸發條件
- API 新增/修改
- 資料庫 Schema 變更
- 第三方服務整合

### 必含項目
- ✅ 成功案例
- ❌ 錯誤處理
- 🔢 邊界值測試

### 範例（Node.js + Express）

```javascript
// tests/integration/users.test.js
const request = require('supertest');
const app = require('../../app');
const db = require('../../db');

describe('POST /api/users', () => {
  beforeEach(async () => {
    await db.users.deleteMany({}); // 清理測試資料
  });

  test('應成功建立用戶', async () => {
    const res = await request(app)
      .post('/api/users')
      .send({ email: 'test@example.com', password: 'SecurePass123!' });
    
    expect(res.status).toBe(201);
    expect(res.body.email).toBe('test@example.com');
  });

  test('應拒絕重複 email', async () => {
    await request(app).post('/api/users').send({ email: 'test@example.com', password: 'pass' });
    
    const res = await request(app)
      .post('/api/users')
      .send({ email: 'test@example.com', password: 'pass2' });
    
    expect(res.status).toBe(400);
    expect(res.body.error).toMatch(/已存在/);
  });

  test('應驗證 email 格式', async () => {
    const res = await request(app)
      .post('/api/users')
      .send({ email: 'invalid-email', password: 'pass' });
    
    expect(res.status).toBe(400);
  });
});
```

### 範例（Python + FastAPI）

```python
# tests/integration/test_users.py
from fastapi.testclient import TestClient
from app.main import app
from app.database import get_db

client = TestClient(app)

def setup_function():
    db = next(get_db())
    db.execute("DELETE FROM users")
    db.commit()

def test_create_user_success():
    response = client.post("/api/users", json={
        "email": "test@example.com",
        "password": "SecurePass123!"
    })
    assert response.status_code == 201
    assert response.json()["email"] == "test@example.com"

def test_reject_duplicate_email():
    client.post("/api/users", json={"email": "test@example.com", "password": "pass"})
    
    response = client.post("/api/users", json={"email": "test@example.com", "password": "pass2"})
    assert response.status_code == 400
    assert "已存在" in response.json()["detail"]
```

---

## E2E 測試 (End-to-End Tests)

### 觸發條件
- 關鍵用戶流程變更（註冊、登入、付款）

### 工具選擇
- **Playwright**（推薦，支援多瀏覽器）
- **Cypress**（適合 Web App）

### 原則
- 使用 `data-testid` 選擇器（避免 CSS class 變動）
- 避免硬編碼等待時間（使用 `waitFor`）
- 測試資料獨立（每次測試前清理）

### 範例（Playwright）

```javascript
// tests/e2e/user-registration.spec.js
const { test, expect } = require('@playwright/test');

test.describe('用戶註冊流程', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:3000/register');
  });

  test('應成功註冊新用戶', async ({ page }) => {
    await page.fill('[data-testid="email-input"]', 'test@example.com');
    await page.fill('[data-testid="password-input"]', 'SecurePass123!');
    await page.click('[data-testid="register-button"]');

    await expect(page.locator('[data-testid="success-message"]')).toBeVisible();
    await expect(page).toHaveURL(/\/dashboard/);
  });

  test('應顯示 email 已存在錯誤', async ({ page }) => {
    // 假設已有此 email
    await page.fill('[data-testid="email-input"]', 'existing@example.com');
    await page.fill('[data-testid="password-input"]', 'pass');
    await page.click('[data-testid="register-button"]');

    await expect(page.locator('[data-testid="error-message"]')).toContainText('已存在');
  });
});
```

---

## 測試交付檢查清單

每次提交測試時，必須確認：

- [ ] 測試檔案路徑與原始碼對應（如 `src/utils/date.js` → `tests/utils/date.test.js`）
- [ ] 包含正向 + 反向案例
- [ ] 非同步操作正確處理（`async/await`, `.then/.catch`）
- [ ] 測試資料隔離（`beforeEach/afterEach` 清理）
- [ ] 不依賴外部 API（使用 mock）
- [ ] 測試通過且無 flaky tests（執行 3 次都通過）

---

## 測試框架指令速查

### JavaScript/TypeScript
```bash
# Jest
npm test
npm test -- --coverage

# Playwright
npx playwright test
npx playwright test --headed  # 顯示瀏覽器
```

### Python
```bash
# pytest
pytest
pytest --cov=app tests/  # 覆蓋率報告
pytest -v  # 詳細輸出
```

### Go
```bash
go test ./...
go test -v ./...
go test -cover ./...
```

---

## 常見錯誤與解決方案

### 問題 1：測試執行順序影響結果
**症狀**：單獨執行通過，批次執行失敗  
**原因**：共享狀態未清理  
**解決**：
```javascript
beforeEach(async () => {
  await db.collection.deleteMany({});
});
```

### 問題 2：非同步測試超時
**症狀**：`Error: Timeout of 2000ms exceeded`  
**原因**：未正確等待 Promise  
**解決**：
```javascript
// ❌ 錯誤
test('fetch data', () => {
  fetchData().then(data => expect(data).toBe('result'));
});

// ✅ 正確
test('fetch data', async () => {
  const data = await fetchData();
  expect(data).toBe('result');
});
```

### 問題 3：測試依賴外部 API
**症狀**：外部服務掛掉導致測試失敗  
**原因**：未 mock 外部依賴  
**解決**：
```javascript
// Jest Mock
jest.mock('axios');
axios.get.mockResolvedValue({ data: { id: 1 } });
```

---

## 何時不需要寫測試

- 簡單的 getter/setter
- 框架自動生成的代碼
- 僅做數據轉發的中間層
- 一次性腳本（遷移腳本、種子資料）

---

**原則**：測試應該讓你更有信心，而非成為負擔。優先測試業務邏輯和易出錯的邏輯。