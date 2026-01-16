# Rust Code Style Rule

## 背景
確保所有 Rust 代碼遵循簡潔且低抽象的原則。

## 指令
- 優先使用 `?` 運算符處理錯誤。
- 除非絕對必要，否則避免使用 `trait` 的複雜層次。
- 函數長度應保持在 30 行以內。

## 範例
```rust
// 好的範例
fn read_data(path: &str) -> Result<String> {
    Ok(fs::read_to_string(path)?)
}
```
