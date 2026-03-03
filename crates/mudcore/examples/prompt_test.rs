//! Prompt 邊界測試工具
//!
//! 連線到 MUD 伺服器，自動登入後送出 look，觀察每個 TCP chunk 的結尾是否有 \r\n
//!
//! 用法: cargo run -p mudcore --example prompt_test -- <host> <port> <user> <pass>

use std::env;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};

const IAC: u8 = 255;

fn analyze_chunk(data: &[u8], chunk_num: usize) {
    let mut text_bytes = Vec::new();
    let mut has_ga = false;
    let mut i = 0;
    while i < data.len() {
        if data[i] == IAC && i + 1 < data.len() {
            let cmd = data[i + 1];
            if (250..=254).contains(&cmd) && i + 2 < data.len() {
                i += 3;
                continue;
            } else if cmd == 250 {
                while i < data.len() {
                    if i + 1 < data.len() && data[i] == IAC && data[i + 1] == 240 {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
                continue;
            } else {
                if cmd == 249 { has_ga = true; }
                i += 2;
                continue;
            }
        }
        text_bytes.push(data[i]);
        i += 1;
    }

    let ends_crlf = text_bytes.ends_with(b"\r\n");
    let ends_lf = text_bytes.ends_with(b"\n");

    // 顯示最後幾個 bytes 的 hex 以便確認
    let tail_len = 20.min(text_bytes.len());
    let tail_hex: Vec<String> = text_bytes[text_bytes.len()-tail_len..]
        .iter().map(|b| format!("{:02x}", b)).collect();

    let text = String::from_utf8_lossy(&text_bytes);
    let lines: Vec<&str> = text.split('\n').collect();
    let last_nonempty: Vec<&str> = lines.iter().rev()
        .filter(|l| !l.trim().is_empty())
        .take(3).copied().collect::<Vec<_>>().into_iter().rev().collect();

    println!("━━━ chunk {} ━━━ {} raw, {} text, GA={}",
        chunk_num, data.len(), text_bytes.len(), has_ga);

    for line in &last_nonempty {
        let trimmed = line.trim();
        // 安全截斷：確保在 char boundary
        let display = if trimmed.len() > 120 {
            match trimmed.char_indices().take_while(|(i, _)| *i <= 120).last() {
                Some((i, c)) => &trimmed[..i + c.len_utf8()],
                None => trimmed,
            }
        } else {
            trimmed
        };
        println!("  │ {:?}", display);
    }

    println!("  │ tail hex: [{}]", tail_hex.join(" "));

    if !ends_lf && !text_bytes.is_empty() {
        println!("  └─▶ NO trailing \\n → PROMPT ◀─");
    } else if ends_crlf {
        println!("  └─ ends \\r\\n");
    } else if ends_lf {
        println!("  └─ ends \\n");
    }
    println!();
}

async fn send(writer: &mut tokio::net::tcp::OwnedWriteHalf, cmd: &str) {
    println!(">>> 送出: {:?}", cmd);
    let data = format!("{}\r\n", cmd);
    let _ = writer.write_all(data.as_bytes()).await;
}

async fn drain(reader: &mut tokio::net::tcp::OwnedReadHalf, chunk_num: &mut u32, wait_secs: u64) {
    let mut buf = vec![0u8; 8192];
    let deadline = tokio::time::Instant::now() + Duration::from_secs(wait_secs);
    loop {
        match timeout(
            deadline.saturating_duration_since(tokio::time::Instant::now()),
            reader.read(&mut buf)
        ).await {
            Ok(Ok(n)) if n > 0 => {
                *chunk_num += 1;
                analyze_chunk(&buf[..n], *chunk_num as usize);
            }
            _ => break,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 5 {
        eprintln!("用法: {} <host> <port> <user> <pass>", args[0]);
        std::process::exit(1);
    }

    let host = &args[1];
    let port: u16 = args[2].parse()?;
    let user = &args[3];
    let pass = &args[4];

    println!("=== Prompt \\n 邊界測試 ===");
    println!("連線到 {}:{}\n", host, port);

    let stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
    println!("已連線\n");

    let (mut reader, mut writer) = stream.into_split();
    let mut chunk_num = 0u32;

    // Phase 1: 登入畫面
    println!("─── Phase 1: 登入畫面 ───");
    drain(&mut reader, &mut chunk_num, 3).await;

    // Phase 2: 帳號
    println!("─── Phase 2: 帳號 ───");
    send(&mut writer, user).await;
    drain(&mut reader, &mut chunk_num, 2).await;

    // Phase 3: 密碼
    println!("─── Phase 3: 密碼 ───");
    send(&mut writer, pass).await;
    drain(&mut reader, &mut chunk_num, 5).await;

    // Phase 4: 空行 (確保進入遊戲)
    println!("─── Phase 4: 空行 ───");
    send(&mut writer, "").await;
    sleep(Duration::from_secs(1)).await;
    drain(&mut reader, &mut chunk_num, 3).await;

    // Phase 5: look
    println!("─── Phase 5: look ───");
    send(&mut writer, "look").await;
    drain(&mut reader, &mut chunk_num, 3).await;

    // Phase 6: 第二次 look
    println!("─── Phase 6: look (2nd) ───");
    send(&mut writer, "look").await;
    drain(&mut reader, &mut chunk_num, 3).await;

    // Phase 7: 移動 (如果有出口)
    println!("─── Phase 7: north ───");
    send(&mut writer, "n").await;
    drain(&mut reader, &mut chunk_num, 3).await;

    // Phase 8: look after move
    println!("─── Phase 8: look after move ───");
    send(&mut writer, "look").await;
    drain(&mut reader, &mut chunk_num, 3).await;

    println!("\n=== 測試完成 ===");
    Ok(())
}
