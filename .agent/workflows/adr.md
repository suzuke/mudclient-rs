---
version: 1.0.0
description: 如何記錄架構決策 (ADR)
---

# Architecture Decision Records (ADR)

> 💡 **核心精神**：拒絕「鬼打牆」。把為什麼 (Why) 記錄下來，未來的 AI 或新進員工就不會重複提出已被否決的方案。

## 什麼時候需要寫 ADR？

當你的決定符合以下任一特徵時：

1. **不可逆**：一旦選了就很難回頭（如：選定 DB、選定 Framework）。
2. **有爭議**：有 A/B 兩派意見，最終選了 A。
3. **有代價**：雖然解決了問題，但引入了新問題（如：引入 Microservices 解決了部署問題，但增加了運維複雜度）。

## ADR 標準格式

每個 ADR 應包含五大要素：

1. **標題 (Title)**：簡短描述決定 (e.g., `001-use-postgresql-as-primary-db`)
2. **狀態 (Status)**：
   - `Proposed` (提議中)
   - `Accepted` (已採納)
   - `Deprecated` (已棄用)
3. **背景 (Context)**：我們面臨什麼問題？有哪些限制？
4. **決策 (Decision)**：我們選了什麼方案？為什麼選它？
5. **後果 (Consequences)**：這個決定會帶來什麼好處？有什麼壞處（技術債）？

## 自動化工具

使用 `scripts/record-adr.sh` 來快速生成 ADR 樣板：

```bash
./scripts/record-adr.sh "Use Redis for Caching"
```

## 範例：001-use-postgresql.md

```markdown
# 001. 使用 PostgreSQL 作為主要資料庫

Date: 2024-01-27
Status: Accepted

## Context
我們需要一個支援完整 ACID 交易且能處理複雜關聯查詢的資料庫。專案初期預算有限，需開源方案。

## Decision
我們決定使用 **PostgreSQL 16**。

替代方案：
- MySQL：對 JSON 支援度較弱，複雜 Join 效能不如 PG。
- MongoDB：不支援 ACID，不適合我們的金融業務。

## Consequences
- ✅ 優點：強大的 JSONB 支援，未來可擴充 NoSQL 用途。
- ⚠️ 風險：團隊成員對 PL/pgSQL 較不熟悉，需學習成本。
```
