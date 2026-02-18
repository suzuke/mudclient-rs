---
name: typescript-patterns
description: |
  TypeScript 開發模式與最佳實踐 Skill。型別安全、泛型、React 模式、錯誤處理。
  TypeScript、React、Next.js、Node.js 專案開發時自動載入。
license: MIT
compatibility: Pixiu Agent / Claude Code / Cursor
metadata:
  author: pixiu
  version: "1.0"
  category: patterns
---

# 📘 TypeScript 開發模式

## 啟動時機

- TypeScript 專案開發
- React/Next.js 開發
- Node.js 後端開發

## 核心原則

### 1. 型別安全優先
充分利用 TypeScript 的型別系統，避免使用 `any`。

```typescript
// ✅ Good: 嚴格型別
interface User {
  id: string;
  name: string;
  email: string;
  createdAt: Date;
}

function getUser(id: string): Promise<User | null> {
  return db.users.findUnique({ where: { id } });
}

// ❌ Bad: 使用 any
function getUser(id: any): Promise<any> {
  return db.users.findUnique({ where: { id } });
}
```

### 2. 不可變性 (Immutability)
優先使用不可變的資料結構。

```typescript
// ✅ Good: 使用展開運算符
const updatedUser = { ...user, name: "New Name" };

// ❌ Bad: 直接修改
user.name = "New Name";
```

### 3. 函式式思維
優先使用純函式和宣告式程式設計。

```typescript
// ✅ Good: 宣告式
const activeUsers = users
  .filter(user => user.isActive)
  .map(user => user.name);

// ❌ Bad: 命令式
const activeUsers = [];
for (let i = 0; i < users.length; i++) {
  if (users[i].isActive) {
    activeUsers.push(users[i].name);
  }
}
```

## 型別定義模式

### Interface vs Type
```typescript
// Interface: 可擴展，適合物件定義
interface User {
  id: string;
  name: string;
}

interface Admin extends User {
  permissions: string[];
}

// Type: 適合聯合型別、元組、複雜型別
type Status = "pending" | "active" | "inactive";
type Result<T> = { success: true; data: T } | { success: false; error: string };
```

### 泛型 (Generics)
```typescript
// 通用的 API 回應處理
interface ApiResponse<T> {
  data: T;
  meta: {
    total: number;
    page: number;
  };
}

async function fetchApi<T>(url: string): Promise<ApiResponse<T>> {
  const response = await fetch(url);
  return response.json();
}

// 使用
const users = await fetchApi<User[]>("/api/users");
```

### Utility Types
```typescript
// 常用工具型別
type PartialUser = Partial<User>;           // 所有屬性可選
type RequiredUser = Required<User>;         // 所有屬性必填
type ReadonlyUser = Readonly<User>;         // 所有屬性唯讀
type UserName = Pick<User, "name">;         // 選取部分屬性
type UserWithoutId = Omit<User, "id">;      // 排除部分屬性
type UserRecord = Record<string, User>;     // 物件字典
```

## 錯誤處理模式

### 型別安全的錯誤處理
```typescript
// Result 型別模式
type Result<T, E = Error> =
  | { ok: true; value: T }
  | { ok: false; error: E };

function parseJSON<T>(json: string): Result<T> {
  try {
    return { ok: true, value: JSON.parse(json) };
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e : new Error(String(e)) };
  }
}

// 使用
const result = parseJSON<User>(jsonString);
if (result.ok) {
  console.log(result.value.name);
} else {
  console.error(result.error.message);
}
```

### 自訂錯誤類別
```typescript
class AppError extends Error {
  constructor(
    message: string,
    public readonly code: string,
    public readonly statusCode: number = 500
  ) {
    super(message);
    this.name = "AppError";
  }
}

class NotFoundError extends AppError {
  constructor(resource: string) {
    super(`${resource} not found`, "NOT_FOUND", 404);
  }
}

class ValidationError extends AppError {
  constructor(message: string) {
    super(message, "VALIDATION_ERROR", 400);
  }
}
```

## React 模式

### 元件型別
```typescript
// 函式元件
interface ButtonProps {
  label: string;
  onClick: () => void;
  variant?: "primary" | "secondary";
  disabled?: boolean;
}

const Button: React.FC<ButtonProps> = ({
  label,
  onClick,
  variant = "primary",
  disabled = false,
}) => {
  return (
    <button
      className={`btn btn-${variant}`}
      onClick={onClick}
      disabled={disabled}
    >
      {label}
    </button>
  );
};
```

### 型別安全的事件處理
```typescript
// 表單事件
const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
  setValue(e.target.value);
};

// 點擊事件
const handleClick = (e: React.MouseEvent<HTMLButtonElement>) => {
  e.preventDefault();
  submit();
};

// 表單提交
const handleSubmit = (e: React.FormEvent<HTMLFormElement>) => {
  e.preventDefault();
  processForm();
};
```

### 自訂 Hook 型別
```typescript
function useLocalStorage<T>(
  key: string,
  initialValue: T
): [T, (value: T) => void] {
  const [storedValue, setStoredValue] = useState<T>(() => {
    try {
      const item = window.localStorage.getItem(key);
      return item ? JSON.parse(item) : initialValue;
    } catch {
      return initialValue;
    }
  });

  const setValue = (value: T) => {
    setStoredValue(value);
    window.localStorage.setItem(key, JSON.stringify(value));
  };

  return [storedValue, setValue];
}
```

## 專案結構

```
src/
├── components/
│   ├── common/
│   │   ├── Button.tsx
│   │   └── index.ts
│   └── features/
├── hooks/
├── lib/
├── services/
├── types/
│   ├── api.ts
│   └── models.ts
└── utils/
```

## 快速參考

| 情境 | 最佳實踐 |
|------|----------|
| 可空值 | `string \| null` 而非 `string?` |
| 物件定義 | 優先使用 `interface` |
| 聯合型別 | 使用 `type` |
| 陣列型別 | `string[]` 而非 `Array<string>` |
| 函式型別 | `(arg: T) => R` |
| 非空斷言 | 謹慎使用 `!` |

## 避免的反模式

- ❌ 過度使用 `any`
- ❌ 忽略編譯器警告
- ❌ 使用 `// @ts-ignore`
- ❌ 過度使用型別斷言 `as`
- ❌ 忽略 null/undefined 檢查
