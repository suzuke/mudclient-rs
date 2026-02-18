---
name: error-handling
description: |
  錯誤處理規範 Skill。統一錯誤格式、前端 Error Boundary、後端全域錯誤處理、Fallback 策略。
  任務涉及「錯誤」、「Exception」、「try-catch」、「失敗」、「error」關鍵字時自動載入。
license: MIT
compatibility: Pixiu Agent / Claude Code / Cursor
metadata:
  author: pixiu
  version: "1.0"
  category: patterns
---

# Error Handling Skill - 錯誤處理規範

> **用途**：統一錯誤格式、前端 Error Boundary、後端全域錯誤處理、Fallback 策略

---

## 核心原則

1. **用戶友善訊息** vs **開發者 Log**
   - 前端：顯示「操作失敗，請稍後再試」
   - 後端 Log：詳細錯誤堆疊 + 請求資訊

2. **永不洩漏內部資訊**
   - ❌ 錯誤：`res.json({ error: err.message })`
   - ✅ 正確：`res.json({ error: '伺服器錯誤' })`

3. **分層處理**
   - UI 層：顯示 Toast / Error State / Error Boundary
   - API 層：統一錯誤格式
   - 資料層：Transaction Rollback

---

## 統一錯誤格式

### API 錯誤回應

```json
{
  "success": false,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "電子郵件格式不正確",
    "field": "email"
  }
}
```

### 常用錯誤碼

| Code | HTTP Status | 說明 |
|------|-------------|------|
| `VALIDATION_ERROR` | 400 | 輸入驗證失敗 |
| `UNAUTHORIZED` | 401 | 未登入 |
| `FORBIDDEN` | 403 | 無權限 |
| `NOT_FOUND` | 404 | 資源不存在 |
| `CONFLICT` | 409 | 資源衝突（如重複 email） |
| `RATE_LIMITED` | 429 | 請求過於頻繁 |
| `INTERNAL_ERROR` | 500 | 伺服器錯誤 |

---

## 前端錯誤處理

### React Error Boundary

```javascript
class ErrorBoundary extends React.Component {
  state = { hasError: false };

  static getDerivedStateFromError(error) {
    return { hasError: true };
  }

  componentDidCatch(error, errorInfo) {
    // 記錄到監控服務
    logErrorToService(error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      return <ErrorFallback onRetry={() => this.setState({ hasError: false })} />;
    }
    return this.props.children;
  }
}
```

### Vue Error Handler

```javascript
app.config.errorHandler = (err, vm, info) => {
  console.error('Vue Error:', err);
  logErrorToService(err, info);
};
```

### API 呼叫封裝

```javascript
async function apiCall(url, options = {}) {
  try {
    const res = await fetch(url, {
      ...options,
      signal: AbortSignal.timeout(30000),
    });

    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new ApiError(data.error?.message || '請求失敗', res.status, data.error?.code);
    }

    return await res.json();
  } catch (err) {
    if (err.name === 'AbortError') {
      throw new ApiError('請求超時，請檢查網路連線', 0, 'TIMEOUT');
    }
    throw err;
  }
}

class ApiError extends Error {
  constructor(message, status, code) {
    super(message);
    this.status = status;
    this.code = code;
  }
}
```

---

## 後端錯誤處理

### Express.js 全域錯誤處理

```javascript
// 自訂錯誤類別
class AppError extends Error {
  constructor(message, statusCode, code) {
    super(message);
    this.statusCode = statusCode;
    this.code = code;
    this.isOperational = true;
  }
}

// 全域錯誤中介軟體（放在所有路由之後）
app.use((err, req, res, next) => {
  // 記錄錯誤
  logger.error({
    message: err.message,
    stack: err.stack,
    url: req.url,
    method: req.method,
    userId: req.user?.id,
  });

  // 回應用戶
  if (err.isOperational) {
    res.status(err.statusCode).json({
      success: false,
      error: { code: err.code, message: err.message }
    });
  } else {
    res.status(500).json({
      success: false,
      error: { code: 'INTERNAL_ERROR', message: '伺服器錯誤，請稍後再試' }
    });
  }
});
```

### FastAPI (Python)

```python
from fastapi import HTTPException
from fastapi.responses import JSONResponse

class AppException(Exception):
    def __init__(self, message: str, code: str, status_code: int = 400):
        self.message = message
        self.code = code
        self.status_code = status_code

@app.exception_handler(AppException)
async def app_exception_handler(request, exc):
    return JSONResponse(
        status_code=exc.status_code,
        content={"success": False, "error": {"code": exc.code, "message": exc.message}}
    )

@app.exception_handler(Exception)
async def general_exception_handler(request, exc):
    logger.error(f"Unhandled error: {exc}")
    return JSONResponse(
        status_code=500,
        content={"success": False, "error": {"code": "INTERNAL_ERROR", "message": "伺服器錯誤"}}
    )
```

---

## Fallback 策略

| 情境 | Fallback 方案 |
|------|---------------|
| API 失敗 | 顯示「暫無資料」+ 重試按鈕 |
| 圖片載入失敗 | 顯示預設圖片 |
| 第三方服務失敗 | 顯示「功能暫時無法使用」 |
| 致命錯誤 | 導向錯誤頁面 + 自動回報 |

---

## 錯誤處理檢查清單

- [ ] 所有 API 呼叫有 try-catch
- [ ] 所有 Promise 有 .catch 或 await 包在 try 內
- [ ] 用戶看到的錯誤訊息友善且不洩漏技術細節
- [ ] 後端有全域錯誤處理中介軟體
- [ ] 錯誤有記錄到 Log 或監控服務
- [ ] 有 Fallback UI（如 Error State、Empty State）

---

**原則**：錯誤是一定會發生的，重點是「優雅地處理」而非「避免所有錯誤」。
