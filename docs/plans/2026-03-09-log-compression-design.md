# Log Auto-Compression Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** App 啟動時自動 gzip 壓縮超過 7 天的 log 檔，節省磁碟空間。

**Architecture:** 在 `crates/mudcore/src/logger.rs` 新增 `compress_old_logs()` 公開函數，在 `MudApp::new()` 初始化時呼叫。使用 `flate2` crate 做 gzip 壓縮。

**Tech Stack:** Rust, flate2 (gzip)

---

### Task 1: Add flate2 dependency

**Files:**
- Modify: `crates/mudcore/Cargo.toml`

**Step 1: Add flate2 to mudcore dependencies**

在 `[dependencies]` 區塊加入：
```toml
flate2 = "1"
```

**Step 2: Verify it compiles**

Run: `cargo check -p mudcore`
Expected: compiles without errors

**Step 3: Commit**

```bash
git add crates/mudcore/Cargo.toml Cargo.lock
git commit -m "deps: add flate2 for log compression"
```

---

### Task 2: Write failing test for compress_old_logs

**Files:**
- Modify: `crates/mudcore/src/logger.rs`

**Step 1: Write the test**

在 `logger.rs` 的 `#[cfg(test)] mod tests` 區塊末尾加入：

```rust
#[test]
fn test_compress_old_logs() {
    use std::time::{Duration, SystemTime};

    let temp_dir = std::env::temp_dir().join("test_compress_logs");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    // 建立一個「舊」log 檔
    let old_file = temp_dir.join("old_log.txt");
    fs::write(&old_file, "old log content").unwrap();
    // 將修改時間設為 8 天前
    let eight_days_ago = SystemTime::now() - Duration::from_secs(8 * 24 * 3600);
    filetime::set_file_mtime(&old_file, filetime::FileTime::from_system_time(eight_days_ago)).unwrap();

    // 建立一個「新」log 檔
    let new_file = temp_dir.join("new_log.txt");
    fs::write(&new_file, "new log content").unwrap();

    // 建立一個已壓縮的檔案（不應被處理）
    let gz_file = temp_dir.join("already.txt.gz");
    fs::write(&gz_file, "fake gz").unwrap();

    // 執行壓縮
    compress_old_logs(temp_dir.to_str().unwrap(), 7);

    // 舊檔案應被壓縮
    assert!(!old_file.exists(), "old .txt should be removed");
    assert!(temp_dir.join("old_log.txt.gz").exists(), ".gz should exist");

    // 新檔案不變
    assert!(new_file.exists(), "new .txt should remain");
    assert!(!temp_dir.join("new_log.txt.gz").exists());

    // 已壓縮檔不變
    assert!(gz_file.exists());

    // 驗證解壓內容正確
    let gz_data = fs::read(temp_dir.join("old_log.txt.gz")).unwrap();
    let mut decoder = flate2::read::GzDecoder::new(&gz_data[..]);
    let mut decoded = String::new();
    std::io::Read::read_to_string(&mut decoder, &mut decoded).unwrap();
    assert_eq!(decoded, "old log content");

    // 清理
    let _ = fs::remove_dir_all(&temp_dir);
}
```

**Step 2: Add filetime dev-dependency**

在 `crates/mudcore/Cargo.toml` 加入：
```toml
[dev-dependencies]
filetime = "0.2"
```

**Step 3: Run test to verify it fails**

Run: `cargo test -p mudcore test_compress_old_logs`
Expected: FAIL — `compress_old_logs` 函數不存在

**Step 4: Commit**

```bash
git add crates/mudcore/src/logger.rs crates/mudcore/Cargo.toml
git commit -m "test: add failing test for compress_old_logs"
```

---

### Task 3: Implement compress_old_logs

**Files:**
- Modify: `crates/mudcore/src/logger.rs`

**Step 1: Add the function**

在 `impl Drop for Logger` 之前加入：

```rust
/// 壓縮指定目錄中超過 `days` 天的 .txt log 檔為 .txt.gz
pub fn compress_old_logs(log_dir: &str, days: u64) {
    let Ok(entries) = fs::read_dir(log_dir) else {
        return;
    };
    let cutoff = std::time::SystemTime::now()
        - std::time::Duration::from_secs(days * 24 * 3600);

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(ext) = path.extension() else { continue };
        if ext != "txt" {
            continue;
        }
        let Ok(meta) = path.metadata() else { continue };
        let Ok(modified) = meta.modified() else { continue };
        if modified > cutoff {
            continue;
        }

        // 壓縮
        let gz_path = path.with_extension("txt.gz");
        match compress_file(&path, &gz_path) {
            Ok(()) => {
                let _ = fs::remove_file(&path);
                tracing::info!("compressed log: {}", path.display());
            }
            Err(e) => {
                tracing::warn!("failed to compress {}: {}", path.display(), e);
                let _ = fs::remove_file(&gz_path); // 清理不完整的 gz
            }
        }
    }
}

fn compress_file(src: &Path, dst: &Path) -> Result<(), LogError> {
    use flate2::write::GzEncoder;
    use flate2::Compression;

    let input = fs::read(src)?;
    let out_file = File::create(dst)?;
    let mut encoder = GzEncoder::new(out_file, Compression::default());
    encoder.write_all(&input)?;
    encoder.finish()?;
    Ok(())
}
```

**Step 2: Add flate2 import at top of file**

在 `use` 區塊中 `flate2` 只在函數內部用，不需要 top-level import。已在函數內 `use`。

**Step 3: Run tests**

Run: `cargo test -p mudcore test_compress_old_logs`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/mudcore/src/logger.rs
git commit -m "feat: implement compress_old_logs for gzip compression of old logs"
```

---

### Task 4: Call compress_old_logs on app startup

**Files:**
- Modify: `crates/mudgui/src/app/mod.rs`

**Step 1: Add call in MudApp::new()**

在 `MudApp::new()` 函數開頭（`let global_config = ...` 之前）加入：

```rust
mudcore::logger::compress_old_logs("logs", 7);
```

**Step 2: Verify it compiles**

Run: `cargo check -p mudgui`
Expected: compiles

**Step 3: Commit**

```bash
git add crates/mudgui/src/app/mod.rs
git commit -m "feat: auto-compress logs older than 7 days on startup"
```

---

### Task 5: Compress existing old logs (one-time cleanup)

**Step 1: Run the app or test manually**

啟動 app 一次，確認 `logs/` 中舊的 `.txt` 被壓縮為 `.txt.gz`。

**Step 2: Verify**

Run: `ls logs/*.gz | head -5`
Expected: 看到 `.txt.gz` 檔案

Run: `ls logs/*.txt | wc -l`
Expected: 數量大幅減少（只剩 7 天內的）
