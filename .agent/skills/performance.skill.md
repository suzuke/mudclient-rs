---
name: performance
description: |
  效能基線 Skill。N+1 查詢優化、索引策略、快取設計、Web Vitals。
  任務涉及「效能」、「優化」、「慢」、「卡頓」、「loading」關鍵字時自動載入。
license: MIT
compatibility: Pixiu Agent / Claude Code / Cursor
metadata:
  author: pixiu
  version: "1.0"
  category: performance
---

# Performance Skill - 效能基線

> **用途**：N+1 查詢優化、索引策略、快取設計、Web Vitals

---

## 核心指標

### Web Vitals (前端)

- **FCP (First Contentful Paint)**：首次內容繪製 < 1.5s
- **LCP (Largest Contentful Paint)**：最大內容繪製 < 2.5s
- **TTI (Time to Interactive)**：可互動時間 < 3.5s
- **CLS (Cumulative Layout Shift)**：累積版面配置位移 < 0.1
- **FID (First Input Delay)**：首次輸入延遲 < 100ms

### API 回應時間 (後端)

- **P50**（中位數）：< 200ms
- **P95**（95% 用戶）：< 500ms
- **P99**（99% 用戶）：< 1s

### 資料庫查詢

- **單一查詢**：< 100ms
- **複雜查詢（多表 JOIN）**：< 500ms
- **避免 N+1 查詢**

---

## 前端效能優化

### 1. 程式碼分割 (Code Splitting)

#### React (Lazy Loading)

```javascript
import { lazy, Suspense } from 'react';

// ❌ 錯誤：全部載入
import Dashboard from './Dashboard';
import UserProfile from './UserProfile';

// ✅ 正確：按需載入
const Dashboard = lazy(() => import('./Dashboard'));
const UserProfile = lazy(() => import('./UserProfile'));

function App() {
  return (
    <Suspense fallback={<div>載入中...</div>}>
      <Routes>
        <Route path="/dashboard" element={<Dashboard />} />
        <Route path="/profile" element={<UserProfile />} />
      </Routes>
    </Suspense>
  );
}
```

#### Next.js (Dynamic Import)

```javascript
import dynamic from 'next/dynamic';

const HeavyChart = dynamic(() => import('./HeavyChart'), {
  loading: () => <p>載入圖表中...</p>,
  ssr: false, // 禁用服務端渲染（若元件依賴瀏覽器 API）
});
```

### 2. 圖片優化

```javascript
// ❌ 錯誤：載入原始大小
<img src="/hero.jpg" alt="Hero" />

// ✅ 正確：使用 WebP + 響應式圖片
<picture>
  <source srcset="/hero.webp" type="image/webp" />
  <source srcset="/hero.jpg" type="image/jpeg" />
  <img 
    src="/hero.jpg" 
    alt="Hero"
    loading="lazy"
    width="800"
    height="600"
  />
</picture>
```

### 3. 虛擬滾動（大量列表）

```javascript
// ❌ 錯誤：渲染 10,000 項
{items.map(item => <ListItem key={item.id} {...item} />)}

// ✅ 正確：使用 react-window
import { FixedSizeList } from 'react-window';

<FixedSizeList
  height={600}
  itemCount={items.length}
  itemSize={50}
  width="100%"
>
  {({ index, style }) => (
    <div style={style}>
      <ListItem {...items[index]} />
    </div>
  )}
</FixedSizeList>
```

### 4. Debounce / Throttle（防抖/節流)

```javascript
import { debounce } from 'lodash';

// ❌ 錯誤：每次輸入都發 API
<input onChange={(e) => searchAPI(e.target.value)} />

// ✅ 正確：延遲 300ms 後才發送
const debouncedSearch = debounce((value) => {
  searchAPI(value);
}, 300);

<input onChange={(e) => debouncedSearch(e.target.value)} />
```

---

## 後端效能優化

### 1. 資料庫索引

```sql
-- ❌ 錯誤：無索引（全表掃描）
SELECT * FROM users WHERE email = 'test@example.com';

-- ✅ 正確：建立索引
CREATE INDEX idx_users_email ON users(email);
```

### 2. 避免 N+1 查詢

#### 錯誤示範（❌ N+1 問題）

```javascript
// 查詢所有用戶（1 次查詢）
const users = await User.findAll();

// 為每個用戶查詢訂單（N 次查詢）
for (const user of users) {
  const orders = await Order.findAll({ where: { userId: user.id } });
  user.orders = orders;
}
```

#### 正確示範（✅ 使用 JOIN）

```javascript
// 一次查詢包含關聯資料
const users = await User.findAll({
  include: [{ model: Order }]
});
```

### 3. 快取策略

#### Redis 快取

```javascript
const redis = require('redis');
const client = redis.createClient();

async function getUserProfile(userId) {
  // 1. 先查快取
  const cached = await client.get(`user:${userId}`);
  if (cached) return JSON.parse(cached);

  // 2. 快取未命中，查資料庫
  const user = await db.query('SELECT * FROM users WHERE id = $1', [userId]);

  // 3. 寫入快取（TTL 1 小時）
  await client.setEx(`user:${userId}`, 3600, JSON.stringify(user));

  return user;
}
```

#### HTTP 快取 Header

```javascript
app.get('/api/users/:id', async (req, res) => {
  const user = await getUserById(req.params.id);
  
  res.set('Cache-Control', 'public, max-age=300'); // 快取 5 分鐘
  res.json(user);
});
```

### 4. 連接池（Connection Pooling）

```javascript
// ❌ 錯誤：每次請求建立新連線
const client = new Client({ connectionString: process.env.DATABASE_URL });
await client.connect();
const result = await client.query('SELECT * FROM users');
await client.end();

// ✅ 正確：使用連接池
const { Pool } = require('pg');
const pool = new Pool({
  connectionString: process.env.DATABASE_URL,
  max: 20, // 最多 20 個連線
  idleTimeoutMillis: 30000,
});

const result = await pool.query('SELECT * FROM users');
```

---

## API 呼叫優化

### 1. Timeout 設定

```javascript
// ❌ 錯誤：無 timeout（可能永久等待）
const response = await fetch(url);

// ✅ 正確：設定 30 秒 timeout
const response = await fetch(url, {
  signal: AbortSignal.timeout(30000)
});
```

### 2. Retry 機制（指數退避）

```javascript
async function fetchWithRetry(url, maxRetries = 3) {
  for (let i = 0; i < maxRetries; i++) {
    try {
      const res = await fetch(url, {
        signal: AbortSignal.timeout(30000)
      });

      if (res.ok) return res;
      
      // 5xx 錯誤才重試，4xx 不重試
      if (res.status >= 500) throw new Error('Server error');
      return res;
    } catch (error) {
      if (i === maxRetries - 1) throw error;
      
      // 指數退避：1s, 2s, 4s
      await new Promise(r => setTimeout(r, Math.pow(2, i) * 1000));
    }
  }
}
```

### 3. 並行請求

```javascript
// ❌ 錯誤：序列執行（總時間 = 3s）
const users = await fetchUsers();    // 1s
const posts = await fetchPosts();    // 1s
const comments = await fetchComments(); // 1s

// ✅ 正確：並行執行（總時間 = 1s）
const [users, posts, comments] = await Promise.all([
  fetchUsers(),
  fetchPosts(),
  fetchComments()
]);
```

---

## 效能測量

### 前端測量

#### Web Vitals (React)

```javascript
import { onCLS, onFID, onFCP, onLCP, onTTFB } from 'web-vitals';

onCLS(console.log);
onFID(console.log);
onFCP(console.log);
onLCP(console.log);
onTTFB(console.log);
```

#### Performance API

```javascript
// 測量 API 呼叫時間
performance.mark('api-start');
await fetchData();
performance.mark('api-end');

performance.measure('api-call', 'api-start', 'api-end');
const measure = performance.getEntriesByName('api-call')[0];
console.log(`API 呼叫耗時：${measure.duration}ms`);
```

### 後端測量

#### Express.js (Response Time)

```javascript
app.use((req, res, next) => {
  const start = Date.now();
  
  res.on('finish', () => {
    const duration = Date.now() - start;
    console.log(`${req.method} ${req.path} - ${duration}ms`);
    
    // 若超過 500ms 發出警告
    if (duration > 500) {
      logger.warn(`Slow request: ${req.method} ${req.path} (${duration}ms)`);
    }
  });
  
  next();
});
```

---

## 效能分析工具

### 前端

- **Lighthouse**（Chrome DevTools）：整體效能評分
- **React DevTools Profiler**：找出慢元件
- **webpack-bundle-analyzer**：分析打包檔案大小

```bash
# 分析 bundle 大小
npm install --save-dev webpack-bundle-analyzer
npx webpack-bundle-analyzer dist/stats.json
```

### 後端

- **Node.js Profiler**：找出慢函式
- **Database Query Analyzer**：分析慢查詢

```bash
# Node.js 效能分析
node --prof app.js
node --prof-process isolate-*.log > processed.txt
```

```sql
-- PostgreSQL 慢查詢分析
EXPLAIN ANALYZE SELECT * FROM users WHERE email = 'test@example.com';
```

---

## 效能檢查清單

### 開發階段

- [ ] 大型列表使用虛擬滾動
- [ ] 圖片使用 lazy loading
- [ ] 第三方套件按需載入
- [ ] API 呼叫有 debounce/throttle
- [ ] 資料庫查詢有索引
- [ ] 避免 N+1 查詢

### 部署前

- [ ] Lighthouse 評分 ≥ 90
- [ ] API P95 < 500ms
- [ ] 資料庫查詢 < 100ms
- [ ] 靜態資源啟用壓縮（gzip/brotli）
- [ ] CDN 設定正確

### 監控中

- [ ] 設定效能警報（API 超過 1s）
- [ ] 定期檢視慢查詢日誌
- [ ] 追蹤 Core Web Vitals 趨勢

---

## 常見效能問題與解決方案

### 問題 1：首頁載入慢

**診斷**：Lighthouse 分析  
**解決**：

1. 程式碼分割（React.lazy）
2. 圖片優化（WebP + lazy loading）
3. 啟用 CDN
4. 減少第三方腳本

### 問題 2：API 回應慢

**診斷**：檢查慢查詢日誌  
**解決**：

1. 資料庫索引
2. Redis 快取
3. 減少 JOIN 深度
4. 使用連接池

### 問題 3：記憶體洩漏

**診斷**：Chrome Memory Profiler  
**解決**：

1. 清理 event listeners
2. 清理 timers (setTimeout/setInterval)
3. 避免全域變數累積資料

---

## 效能影響評估範本

```markdown
## 📊 效能影響評估

### 變更前
- API 回應時間（P95）：450ms
- FCP：1.8s
- 資料庫查詢數：5 次

### 變更後
- API 回應時間（P95）：380ms
- FCP：1.2s
- 資料庫查詢數：2 次

### 改善幅度
- API：-15.6%
- FCP：-33.3%
- 查詢數：-60%

### 測試環境
- 本地 Docker (8GB RAM, 4 cores)
- 模擬 100 並發用戶
```

---

**原則**：效能優化是持續的過程，而非一次性任務。先測量，再優化，最後驗證。
