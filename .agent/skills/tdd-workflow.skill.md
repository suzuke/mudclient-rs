---
name: tdd-workflow
description: |
  測試驅動開發 (TDD) Skill。Red-Green-Refactor 循環、覆蓋率要求、測試類型劃分。
  開發新功能、修改程式碼、重構或修復 Bug 時使用。
license: MIT
compatibility: Pixiu Agent / Claude Code / Cursor
metadata:
  author: pixiu
  version: "1.0"
  category: workflow
---

# 🧪 TDD 工作流程 (Test-Driven Development)

## 啟動時機

- 開發新功能時
- 修改現有程式碼時
- 重構程式碼時
- 修復 Bug 時

## 核心原則

### 1. 測試先於程式碼
永遠先寫測試，再寫實作。這確保你的程式碼是為了滿足測試而存在。

### 2. 覆蓋率要求
- **最低目標**: 80% 程式碼覆蓋率
- **理想目標**: 90%+ 程式碼覆蓋率
- **必須 100%**: 關鍵業務邏輯

### 3. 測試類型

| 類型 | 用途 | 比例 |
|------|------|------|
| 單元測試 | 測試單一函式/模組 | 70% |
| 整合測試 | 測試模組間互動 | 20% |
| E2E 測試 | 測試使用者旅程 | 10% |

## TDD 工作流程步驟

### Step 1: 寫使用者旅程 (RED)
```markdown
# 使用者旅程: 登入功能
1. 使用者輸入 email 和密碼
2. 系統驗證憑證
3. 成功則回傳 JWT，失敗則回傳錯誤訊息
```

### Step 2: 產生測試案例 (RED)
```typescript
describe('AuthService', () => {
  describe('login', () => {
    it('should return JWT when credentials are valid', async () => {
      const result = await authService.login('user@example.com', 'password123');
      expect(result.token).toBeDefined();
      expect(result.token).toMatch(/^eyJ/);
    });

    it('should throw error when email is invalid', async () => {
      await expect(
        authService.login('invalid@example.com', 'password')
      ).rejects.toThrow('Invalid credentials');
    });

    it('should throw error when password is wrong', async () => {
      await expect(
        authService.login('user@example.com', 'wrongpassword')
      ).rejects.toThrow('Invalid credentials');
    });
  });
});
```

### Step 3: 執行測試 (應該失敗)
```bash
npm test -- --watch
# 預期: 所有測試失敗 (因為還沒實作)
```

### Step 4: 實作程式碼 (GREEN)
```typescript
class AuthService {
  async login(email: string, password: string): Promise<{ token: string }> {
    const user = await this.userRepository.findByEmail(email);
    if (!user) throw new Error('Invalid credentials');
    
    const isValid = await bcrypt.compare(password, user.passwordHash);
    if (!isValid) throw new Error('Invalid credentials');
    
    return { token: this.jwtService.sign({ userId: user.id }) };
  }
}
```

### Step 5: 再次執行測試 (應該通過)
```bash
npm test
# 預期: 所有測試通過 ✅
```

### Step 6: 重構 (REFACTOR)
在測試保護下進行重構，確保測試持續通過。

### Step 7: 驗證覆蓋率
```bash
npm run test:coverage
# 確認覆蓋率 >= 80%
```

## 測試模式

### 單元測試模式 (Jest/Vitest)
```typescript
describe('Calculator', () => {
  let calculator: Calculator;

  beforeEach(() => {
    calculator = new Calculator();
  });

  it('should add two numbers', () => {
    expect(calculator.add(2, 3)).toBe(5);
  });

  it('should handle negative numbers', () => {
    expect(calculator.add(-1, 1)).toBe(0);
  });
});
```

### API 整合測試模式
```typescript
describe('POST /api/users', () => {
  it('should create a new user', async () => {
    const response = await request(app)
      .post('/api/users')
      .send({ email: 'test@example.com', password: 'password123' });
    
    expect(response.status).toBe(201);
    expect(response.body.id).toBeDefined();
  });
});
```

### E2E 測試模式 (Playwright)
```typescript
test('user can login successfully', async ({ page }) => {
  await page.goto('/login');
  await page.fill('[data-testid="email"]', 'user@example.com');
  await page.fill('[data-testid="password"]', 'password123');
  await page.click('[data-testid="submit"]');
  
  await expect(page).toHaveURL('/dashboard');
});
```

## 常見錯誤避免

### ❌ 錯誤: 測試實作細節
```typescript
// 不要這樣做
expect(service.privateMethod).toHaveBeenCalled();
```

### ✅ 正確: 測試使用者可見行為
```typescript
// 這樣做
expect(result.status).toBe('success');
```

## 最佳實踐

1. **獨立測試**: 每個測試應該獨立運行，不依賴其他測試
2. **明確命名**: 測試名稱應該清楚描述測試內容
3. **單一職責**: 每個測試只測試一件事
4. **避免 Magic Numbers**: 使用常數或明確的值
5. **Mock 外部服務**: 不要在單元測試中呼叫真實 API
