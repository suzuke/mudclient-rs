---
name: payment
description: |
  金流支付開發 Skill。PCI DSS 合規、交易安全、冪等性、Webhook 安全、金額精準度。
  開發涉及金流、支付、信用卡處理或任何交易功能時必須遵守。
license: MIT
compatibility: Pixiu Agent / Claude Code / Cursor
metadata:
  author: pixiu
  version: "1.0"
  category: security
---

# 💳 Payment / 金流開發規範

開發涉及金流、支付、信用卡處理或任何交易功能時，必須嚴格遵守本規範。

## ⚠️ PCI DSS 核心原則（絕對遵守）

1.  **不落帝（Do not store card data）**
    *   **禁止** 將信用卡號 (PAN)、驗證碼 (CVV/CVC) 存入資料庫或 Log。
    *   僅允許儲存 Token 或遮罩後的卡號（如 `************1234`）。
2.  **傳輸加密**
    *   所有金流相關 API 呼叫必須使用 HTTPS (TLS 1.2+)。

## 🔒 交易安全性設計

### 1. 冪等性 (Idempotency)
所有扣款、退款 API 必須支援冪等性，防止因網路超時導致的重複扣款。
*   **作法**: 使用 `ideponmency_key` 或 `order_id` 作為唯一識別。

### 2. 也是性 (Consistency)
涉及金額變動的操作，必須使用資料庫交易 (Database Transaction)。
```php
// ✅ 正確範例 (Laravel)
DB::transaction(function () use ($order) {
    $order->status = 'paid';
    $order->save();
    $user->wallet->decrement($order->amount);
});
```

### 3. 金額精準度
*   **禁止** 使用 `float` 或 `double` 儲存金額。
*   **必須** 使用 `DECIMAL` (如 `DECIMAL(10,2)`) 或以「分」為單位的 `INTEGER`。

## 🛡️ Webhook 安全

第三方支付的回調 (Webhook) 必須經過嚴格驗證：
1.  **簽章驗證 (Signature Verification)**：必須驗證 request hash 是否與金鑰簽署結果相符。
2.  **檢查金額與狀態**：不要只相信 `status=success`，必須查詢資料庫確認該筆訂單金額是否正確。
3.  **回應 ACK**：處理成功回傳 200/OK，失敗回傳 4xx/5xx 以觸發重試。

## 📝 敏感資料日誌 (Log Redaction)

在記錄 Log 時，必須自動脫敏敏感欄位。

*   **敏感欄位清單**:
    *   `card_number`, `pan`, `cvv`, `cvc`
    *   `password`, `secret`, `token`
*   **脫敏方式**:
    ```json
    // ❌ 錯誤
    "card": "4242424242424242"
    
    // ✅ 正確
    "card": "************4242"
    ```
