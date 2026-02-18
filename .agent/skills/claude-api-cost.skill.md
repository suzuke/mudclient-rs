---
name: claude-api-cost
description: |
  Claude API 成本優化 Skill。節省 50-90% API 費用，使用 Batch API、Prompt Caching、Extended Thinking。
  任務涉及「Claude API」、「Anthropic API」、「API 費用」、「成本優化」、「批次處理」關鍵字時自動載入。
license: MIT
compatibility: Pixiu Agent / Claude Code / Cursor
metadata:
  author: pixiu
  version: "1.0"
  category: optimization
  source: sstklen/claude-api-cost-optimization
---

# Claude API Cost Optimization Skill - API 成本優化

> **用途**：節省 50-90% Claude API 費用，使用 Batch API、Prompt Caching、Extended Thinking 三大官方技術

---

## 快速參考

| 技巧 | 節省 | 適用時機 |
|------|------|---------|
| **Batch API** | 50% | 任務可等待最多 24 小時 |
| **Prompt Caching** | 90% | 重複的 system prompt（>1K tokens） |
| **Extended Thinking** | ~80% | 複雜推理任務 |
| **Batch + Cache** | ~95% | 批次任務 + 共享 context |

---

## 決策流程圖

```
是否緊急？
├── 是 → 使用一般 API
└── 否 → 可以等 24 小時？
    ├── 是 → 使用 Batch API（省 50%）
    └── 否 → 繼續往下

有重複的 system prompt？
├── 是（>1K tokens）→ 使用 Prompt Caching（省 90%）
└── 否 → 繼續往下

是複雜推理任務？
├── 是 → 使用 Extended Thinking
└── 否 → 使用一般 API
```

---

## 1. Batch API（省 50%）

### 適用場景

- ✅ 批次翻譯
- ✅ 每日內容生成
- ✅ 夜間報表處理
- ❌ 即時對話
- ❌ 需要立即回應

### 程式碼範例

```python
import anthropic

client = anthropic.Anthropic()

batch = client.messages.batches.create(
    requests=[
        {
            "custom_id": "task-001",
            "params": {
                "model": "claude-sonnet-4-5",
                "max_tokens": 1024,
                "messages": [{"role": "user", "content": "Task 1"}]
            }
        },
        {
            "custom_id": "task-002",
            "params": {
                "model": "claude-sonnet-4-5",
                "max_tokens": 1024,
                "messages": [{"role": "user", "content": "Task 2"}]
            }
        }
    ]
)

# 等待完成（最多 24h，通常 <1h）
for result in client.messages.batches.results(batch.id):
    print(f"{result.custom_id}: {result.result.message.content[0].text}")
```

### 限制

- 每批次最多 100,000 筆請求或 256MB
- 結果保留 29 天
- 大多數在 1 小時內完成

### 🔥 重要發現

**批次越大越快！** 實測 294 筆比 10 筆還早完成 53 分鐘。Anthropic 會優先處理大批次。

---

## 2. Prompt Caching（省 90%）

### 適用場景

- ✅ 長 system prompt（>1K tokens）
- ✅ 重複的指令
- ✅ RAG 大量 context
- ❌ Prompt < 1,024 tokens（無法快取）
- ❌ 經常變動的 prompt

### 程式碼範例

```python
import anthropic

client = anthropic.Anthropic()

response = client.messages.create(
    model="claude-sonnet-4-5",
    max_tokens=1024,
    system=[
        {
            "type": "text",
            "text": "你的長 system prompt（必須 >1024 tokens）...",
            "cache_control": {"type": "ephemeral"}  # ← 啟用快取！
        }
    ],
    messages=[{"role": "user", "content": "用戶問題"}]
)

# 第一次呼叫：Cache write（+25% 成本）
# 後續呼叫：Cache read（-90% 成本！）
```

### 價格對照

| 類型 | Sonnet 價格 | 與原價比較 |
|------|-------------|-----------|
| 原價 | $3/MTok | 基準 |
| Cache write | $3.75/MTok | +25%（首次） |
| Cache read | $0.30/MTok | **-90%** |

### 快取規則

- 最小門檻：1,024 tokens（Sonnet）、4,096 tokens（Opus/Haiku 4.5）
- TTL：5 分鐘（使用時刷新）或 1 小時（額外成本）
- 失效：快取內容有任何變動

---

## 3. Extended Thinking（省 ~80%）

### 適用場景

- ✅ 複雜程式碼架構設計
- ✅ 策略規劃
- ✅ 數學推理
- ✅ 複雜 debug
- ❌ 簡單問答
- ❌ 翻譯

### 程式碼範例

```python
response = client.messages.create(
    model="claude-sonnet-4-5",
    max_tokens=16000,
    thinking={
        "type": "enabled",
        "budget_tokens": 10000  # 思考預算
    },
    messages=[{
        "role": "user",
        "content": "設計一個最佳化的架構..."
    }]
)

for block in response.content:
    if block.type == "thinking":
        print("🧠 思考過程:", block.thinking)
    elif block.type == "text":
        print("📝 答案:", block.text)
```

### 價格

- Input：$3/MTok
- Thinking output：~$3/MTok（較便宜！）
- Final output：$15/MTok

---

## 組合技巧：Batch + Cache

```python
# 批次請求 + 共享 context
batch = client.messages.batches.create(
    requests=[
        {
            "custom_id": f"task-{i}",
            "params": {
                "model": "claude-sonnet-4-5",
                "max_tokens": 1024,
                "system": [{
                    "type": "text",
                    "text": "共享的 system prompt...",
                    "cache_control": {"type": "ephemeral", "ttl": "1h"}
                }],
                "messages": [{"role": "user", "content": f"Task {i}"}]
            }
        }
        for i in range(100)
    ]
)
```

**提示**：批次任務建議用 1 小時 TTL（因為可能執行超過 5 分鐘）

---

## 成本計算範例

### 每日影片腳本生成（30 支）

| 項目 | Tokens | 價格 | 成本 |
|------|--------|------|------|
| System prompt（快取） | 2,000 | $0.30/MTok | $0.0006 |
| User input × 30 | 15,000 | $1.50/MTok（batch） | $0.0225 |
| Output × 30 | 30,000 | $7.50/MTok（batch） | $0.225 |
| **每日總計** | | | **$0.25** |
| 未優化 | | | $1.50 |
| **節省** | | | **83%** |

---

## 常見錯誤

| 錯誤 | 解決方案 |
|------|---------|
| 快取 <1K tokens | 不會快取；增加更多 context |
| 5 分鐘快取過期 | 使用 1h TTL 或保持請求持續 |
| 變動快取內容 | 將靜態內容分開 |
| 期望 batch 立即完成 | 允許最多 24 小時 |

---

## 🔥 真實案例：294 支影片

### Token 明細

| Token 類型 | 數量 | 成本 |
|------------|------|------|
| Input（無快取） | 365,624 | $0.55 |
| Cache write（1h） | 106,920 | $0.32 |
| Cache read | 416,988 | $0.06 |
| Output | 611,412 | $4.59 |
| **總計** | **1,500,944** | **$5.52** |

### 對照

| 方式 | 成本 | 每筆請求 |
|------|------|---------|
| 標準 API | $11.04 | $0.0376 |
| **Batch API** | **$5.52** | **$0.0188** |
| **節省** | **50%** | |

### 💡 重要發現

**圖片/影片任務只省 ~14%，不是 90%！**

原因：圖片佔 85% tokens，只有 system prompt（15%）可快取。
```
Input 組成：
├── System Prompt：~15% → ✅ 可快取（省 90%）
└── Image Data：~85% → ❌ 無法快取

實際節省：15% × 90% = ~14%
```

---

## 💰 省錢報告模板

實作優化後，請顯示此報告給用戶：

```
╔══════════════════════════════════════════════════════════════╗
║  💰 CLAUDE API 省錢報告                                       ║
╠══════════════════════════════════════════════════════════════╣
║  📊 使用的技巧:                                               ║
║     ☑️ Batch API (-50%)                                       ║
║     ☑️ Prompt Caching (-90%)                                  ║
║     ☐ Extended Thinking (-80%)                               ║
║                                                              ║
║  💵 總計:                                                     ║
║     原價：$0.111                                              ║
║     優化後：$0.042                                            ║
║     節省：$0.069 (62%)                                        ║
║                                                              ║
║  📅 長期預估:                                                 ║
║     每日節省：$2.07                                           ║
║     每月節省：$62.10                                          ║
║     每年節省：$745.20 🎊                                      ║
╚══════════════════════════════════════════════════════════════╝
```

### 簡化版報告

```
💰 省錢報告：使用 Prompt Caching 後，預估省下 $0.05/次 (90%)
   📅 每日 100 次 = 省 $5/天 = $150/月 = $1,800/年 🎉
```

---

## 官方文件

- [Prompt Caching](https://docs.anthropic.com/en/docs/build-with-claude/prompt-caching)
- [Batch Processing](https://docs.anthropic.com/en/docs/build-with-claude/batch-processing)
- [Extended Thinking](https://docs.anthropic.com/en/docs/build-with-claude/extended-thinking)

---

## 效能檢查清單

### 開發階段

- [ ] 識別可批次處理的任務
- [ ] 識別可快取的 system prompt（>1K tokens）
- [ ] 複雜推理任務考慮 Extended Thinking
- [ ] API 呼叫有 timeout 和 retry

### 部署前

- [ ] 測量優化前後成本差異
- [ ] 確認 batch 任務的等待時間可接受
- [ ] 快取 TTL 設定正確

### 監控中

- [ ] 追蹤 API 費用趨勢
- [ ] 監控 cache hit rate
- [ ] 定期檢視 batch 完成時間

---

**原則**：先測量，再優化，最後驗證。省錢報告是必須的！
