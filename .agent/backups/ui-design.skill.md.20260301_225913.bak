---
name: ui-design
description: |
  UI/UX 設計規範 Skill。色彩系統、間距規範、響應式設計、無障礙設計。
  任務涉及「UI」、「介面」、「元件」、「樣式」關鍵字時自動載入。
license: MIT
compatibility: Pixiu Agent / Claude Code / Cursor
metadata:
  author: pixiu
  version: "1.0"
  category: frontend
---

# UI Design Skill - UI/UX 設計規範

> **用途**：色彩系統、間距規範、響應式設計、無障礙設計

---

## 設計系統 (Design System as Code)
> Source: nextlevelbuilder/ui-ux-pro-max-skill

### 1. 核心原則 (SSOT)
為了避免樣式碎片化，專案必須維護一個 **Design System Master File**。
所有 UI 開發必須先定義此檔案，再撰寫程式碼。

### 2. 檔案結構
- **`design-system/MASTER.md`**: 全域視覺規範 (Global Source of Truth)。包含色票、字體、圓角、陰影、主要元件樣式。
- **`design-system/pages/<page-name>.md`** (Optional): 頁面特定的覆蓋規則 (Overrides)。

### 3. 工作流 (Workflow)
1. **Define**: 產生或更新 `MASTER.md`。
2. **Override**: 若某頁面有特殊需求，建立 `pages/xxx.md`。
3. **Implement**: 寫 Code 時，強制參考 `MASTER.md` 的變數與 Class。

---

## 設計原則
1. **一致性**：嚴格遵守 `MASTER.md` 定義的 Token。
2. **層級性**：Page Override > Global Master > Default Rules。
3. **簡潔性**：移除不必要的元素。
4. **回饋性**：每個操作都有視覺回饋。
5. **可及性**：支援鍵盤操作與螢幕閱讀器。

---

## 色彩系統

### 語意色彩

| 用途 | CSS 變數 | 說明 |
|------|----------|------|
| 主色 | `--color-primary` | 品牌色、主要按鈕 |
| 成功 | `--color-success` | 成功訊息、確認 |
| 警告 | `--color-warning` | 警告訊息 |
| 錯誤 | `--color-error` | 錯誤訊息、刪除 |
| 資訊 | `--color-info` | 提示訊息 |

### 中性色階

```css
:root {
  --gray-50: #fafafa;
  --gray-100: #f5f5f5;
  --gray-200: #e5e5e5;
  --gray-300: #d4d4d4;
  --gray-400: #a3a3a3;
  --gray-500: #737373;
  --gray-600: #525252;
  --gray-700: #404040;
  --gray-800: #262626;
  --gray-900: #171717;
}
```

### 深色模式

```css
@media (prefers-color-scheme: dark) {
  :root {
    --bg-primary: var(--gray-900);
    --text-primary: var(--gray-100);
  }
}
```

---

## 間距系統

### 8px 網格

| Token | 值 | 用途 |
|-------|-----|------|
| `--space-1` | 4px | 緊湊間距 |
| `--space-2` | 8px | 元素內間距 |
| `--space-3` | 12px | 小區塊間距 |
| `--space-4` | 16px | 標準間距 |
| `--space-6` | 24px | 區塊間距 |
| `--space-8` | 32px | 大區塊間距 |
| `--space-12` | 48px | 頁面區塊 |

---

## 字體系統

### 字體堆疊

```css
:root {
  --font-sans: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
  --font-mono: 'Fira Code', 'Consolas', monospace;
}
```

### 字體大小

| Token | 大小 | 行高 | 用途 |
|-------|------|------|------|
| `--text-xs` | 12px | 1.5 | 標籤、註解 |
| `--text-sm` | 14px | 1.5 | 輔助文字 |
| `--text-base` | 16px | 1.5 | 內文 |
| `--text-lg` | 18px | 1.4 | 小標題 |
| `--text-xl` | 20px | 1.4 | 標題 |
| `--text-2xl` | 24px | 1.3 | 大標題 |
| `--text-3xl` | 30px | 1.2 | 頁面標題 |

---

## 元件規範

### 按鈕

```css
.btn {
  padding: var(--space-2) var(--space-4);
  border-radius: 6px;
  font-weight: 500;
  transition: all 0.2s;
}

.btn-primary {
  background: var(--color-primary);
  color: white;
}

.btn-primary:hover {
  filter: brightness(1.1);
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
```

### 按鈕狀態

| 狀態 | 說明 |
|------|------|
| Default | 預設樣式 |
| Hover | 滑鼠懸停 |
| Active | 點擊中 |
| Focus | 鍵盤聚焦 |
| Disabled | 禁用 |
| Loading | 載入中（顯示 spinner） |

### 輸入框

```css
.input {
  padding: var(--space-2) var(--space-3);
  border: 1px solid var(--gray-300);
  border-radius: 6px;
  font-size: var(--text-base);
}

.input:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px rgba(var(--color-primary-rgb), 0.1);
}

.input-error {
  border-color: var(--color-error);
}
```

---

## 響應式設計

### 斷點

| 斷點 | 寬度 | 用途 |
|------|------|------|
| `sm` | 640px | 手機橫向 |
| `md` | 768px | 平板 |
| `lg` | 1024px | 小筆電 |
| `xl` | 1280px | 桌機 |
| `2xl` | 1536px | 大螢幕 |

### Mobile First

```css
/* 手機優先 */
.container {
  padding: var(--space-4);
}

/* 平板以上 */
@media (min-width: 768px) {
  .container {
    padding: var(--space-8);
  }
}
```

---

## 無障礙設計 (A11y)

### 對比度

- 正常文字：至少 4.5:1
- 大文字（18px+）：至少 3:1

### Focus 樣式

```css
/* 永遠保留 focus 樣式 */
:focus-visible {
  outline: 2px solid var(--color-primary);
  outline-offset: 2px;
}
```

### ARIA 標籤

```html
<!-- 圖示按鈕需要 aria-label -->
<button aria-label="關閉">
  <svg>...</svg>
</button>

<!-- Loading 狀態 -->
<button aria-busy="true" aria-label="載入中...">
  <Spinner />
</button>
```

### 鍵盤操作

| 操作 | 按鍵 |
|------|------|
| 確認 | Enter |
| 取消 | Escape |
| 導航 | Tab / Shift+Tab |
| 選擇 | Space |
| 方向 | Arrow Keys |

---

## 動效規範

### 過渡時間

| 類型 | 時間 | 用途 |
|------|------|------|
| 快速 | 100-150ms | Hover 效果 |
| 標準 | 200-300ms | 大多數過渡 |
| 慢速 | 300-500ms | 複雜動畫 |

### Easing

```css
:root {
  --ease-in-out: cubic-bezier(0.4, 0, 0.2, 1);
  --ease-out: cubic-bezier(0, 0, 0.2, 1);
  --ease-in: cubic-bezier(0.4, 0, 1, 1);
}
```

### 減少動態

```css
@media (prefers-reduced-motion: reduce) {
  * {
    animation: none !important;
    transition-duration: 0.01ms !important;
  }
}
```

---

## 檢查清單

- [ ] 色彩對比符合 WCAG AA
- [ ] 所有互動元素有 focus 樣式
- [ ] 圖示按鈕有 aria-label
- [ ] 支援鍵盤操作
- [ ] 響應式設計（Mobile First）
- [ ] 支援深色模式
- [ ] Loading 狀態有視覺回饋

---

**原則**：好的 UI 是隱形的。使用者不應該「注意到」介面，而是專注於完成任務。
