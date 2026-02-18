---
name: python-patterns
description: |
  Python 開發模式與最佳實踐 Skill。型別提示、錯誤處理、Context Manager、裝飾器模式。
  Python、Django、FastAPI、Flask 專案開發時自動載入。
license: MIT
compatibility: Pixiu Agent / Claude Code / Cursor
metadata:
  author: pixiu
  version: "1.0"
  category: patterns
---

# 🐍 Python 開發模式

## 啟動時機

- Python 專案開發
- Django/FastAPI/Flask 開發
- 撰寫 Python 腳本

## 核心原則

### 1. 可讀性至上 (Readability Counts)
Python 優先考慮可讀性。程式碼應該明顯且易於理解。

```python
# ✅ Good: 清晰可讀
def get_active_users(users: list[User]) -> list[User]:
    """Return only active users from the provided list."""
    return [user for user in users if user.is_active]

# ❌ Bad: 聰明但令人困惑
def get_active_users(u):
    return [x for x in u if x.a]
```

### 2. 明確優於隱含 (Explicit is Better Than Implicit)
避免魔法；清楚表達程式碼的作用。

```python
# ✅ Good: 明確配置
import logging

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)

# ❌ Bad: 隱藏的副作用
import some_module
some_module.setup()  # 這做了什麼？
```

### 3. EAFP 原則
寧可請求原諒，也不要事先請求許可。

```python
# ✅ Good: EAFP 風格
def get_value(dictionary: dict, key: str) -> Any:
    try:
        return dictionary[key]
    except KeyError:
        return default_value

# ❌ Bad: LBYL 風格
def get_value(dictionary: dict, key: str) -> Any:
    if key in dictionary:
        return dictionary[key]
    else:
        return default_value
```

## 型別提示 (Type Hints)

### 基本型別註解
```python
def greet(name: str) -> str:
    return f"Hello, {name}"

def process_items(items: list[int]) -> dict[str, int]:
    return {"count": len(items), "sum": sum(items)}
```

### 現代型別提示 (Python 3.9+)
```python
from typing import Optional, Union

# 使用 | 代替 Union
def process(value: int | str) -> None:
    pass

# 使用 X | None 代替 Optional[X]
def find_user(user_id: int) -> User | None:
    pass
```

## 錯誤處理模式

### 具體異常處理
```python
# ✅ Good: 具體異常
try:
    result = risky_operation()
except ValueError as e:
    logger.error(f"Invalid value: {e}")
    raise
except ConnectionError as e:
    logger.error(f"Connection failed: {e}")
    return fallback_value

# ❌ Bad: 捕獲所有異常
try:
    result = risky_operation()
except Exception:
    pass  # 靜默失敗
```

### 自訂異常層級
```python
class AppError(Exception):
    """Application base exception."""
    pass

class ValidationError(AppError):
    """Validation failed."""
    pass

class NotFoundError(AppError):
    """Resource not found."""
    pass
```

## Context Managers

### 資源管理
```python
# ✅ Good: 使用 context manager
with open("file.txt", "r") as f:
    content = f.read()

# ❌ Bad: 手動管理
f = open("file.txt", "r")
try:
    content = f.read()
finally:
    f.close()
```

### 自訂 Context Manager
```python
from contextlib import contextmanager

@contextmanager
def timer(name: str):
    start = time.time()
    try:
        yield
    finally:
        elapsed = time.time() - start
        logger.info(f"{name} took {elapsed:.2f}s")
```

## Data Classes

```python
from dataclasses import dataclass, field
from typing import Optional

@dataclass
class User:
    id: int
    name: str
    email: str
    is_active: bool = True
    roles: list[str] = field(default_factory=list)

    def __post_init__(self):
        if not self.email or "@" not in self.email:
            raise ValueError("Invalid email")
```

## 裝飾器模式

```python
import functools
import time

def retry(max_attempts: int = 3, delay: float = 1.0):
    def decorator(func):
        @functools.wraps(func)
        def wrapper(*args, **kwargs):
            for attempt in range(max_attempts):
                try:
                    return func(*args, **kwargs)
                except Exception as e:
                    if attempt == max_attempts - 1:
                        raise
                    time.sleep(delay)
            return None
        return wrapper
    return decorator

@retry(max_attempts=3, delay=0.5)
def fetch_data(url: str) -> dict:
    # 實作
    pass
```

## 專案結構

```
my_project/
├── src/
│   └── my_package/
│       ├── __init__.py
│       ├── core/
│       ├── api/
│       └── utils/
├── tests/
│   ├── __init__.py
│   ├── unit/
│   └── integration/
├── pyproject.toml
├── requirements.txt
└── README.md
```

## 快速參考: Python 慣用法

| 情境 | 慣用法 |
|------|--------|
| 檢查空集合 | `if not my_list:` |
| 交換變數 | `a, b = b, a` |
| 多重賦值 | `x = y = z = 0` |
| 串列拼接 | `''.join(strings)` |
| 字典合併 | `{**dict1, **dict2}` |
| 條件表達式 | `x if condition else y` |

## 避免的反模式

- ❌ 使用 `from module import *`
- ❌ 使用可變預設參數
- ❌ 忽略異常 (`except: pass`)
- ❌ 在迴圈中修改迭代對象
- ❌ 使用全域變數
