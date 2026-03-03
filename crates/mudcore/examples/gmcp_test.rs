//! GMCP 協商測試工具
//!
//! 連線到 MUD 伺服器，測試 GMCP 是否可用，並嘗試取得房間資訊。
//!
//! 用法: cargo run -p mudcore --example gmcp_test -- <host> <port>

use std::env;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

const IAC: u8 = 255;
const SB: u8 = 250;
const SE: u8 = 240;
const WILL: u8 = 251;
const WONT: u8 = 252;
const DO: u8 = 253;
const DONT: u8 = 254;
const GA: u8 = 249;
const GMCP: u8 = 201;

fn cmd_name(cmd: u8) -> &'static str {
    match cmd {
        WILL => "WILL", WONT => "WONT", DO => "DO", DONT => "DONT", _ => "???",
    }
}

fn opt_name(opt: u8) -> String {
    match opt {
        0 => "BINARY".into(), 1 => "ECHO".into(), 3 => "SGA".into(),
        24 => "TTYPE".into(), 31 => "NAWS".into(),
        85 => "MCCP1".into(), 86 => "MCCP2".into(), 87 => "MCCP3".into(),
        201 => "GMCP".into(),
        _ => format!("OPT({})", opt),
    }
}

#[derive(Debug)]
enum Evt {
    Cmd(u8, u8),
    Sub(u8, Vec<u8>),
}

fn parse(input: &[u8]) -> (Vec<u8>, Vec<Evt>, usize) {
    let mut data = Vec::new();
    let mut events = Vec::new();
    let mut i = 0;
    let mut consumed = 0;

    while i < input.len() {
        if input[i] != IAC {
            data.push(input[i]);
            i += 1;
            consumed = i;
            continue;
        }
        if i + 1 >= input.len() { break; }
        let c = input[i + 1];
        if c == IAC { data.push(IAC); i += 2; consumed = i; continue; }
        match c {
            WILL | WONT | DO | DONT => {
                if i + 2 >= input.len() { break; }
                events.push(Evt::Cmd(c, input[i + 2]));
                i += 3; consumed = i;
            }
            SB => {
                if i + 2 >= input.len() { break; }
                let opt = input[i + 2];
                let mut sub = Vec::new();
                let mut j = i + 3;
                let mut found = false;
                while j + 1 < input.len() {
                    if input[j] == IAC && input[j + 1] == SE {
                        events.push(Evt::Sub(opt, sub));
                        i = j + 2; consumed = i; found = true; break;
                    }
                    sub.push(input[j]);
                    j += 1;
                }
                if !found { break; }
            }
            GA => { i += 2; consumed = i; }
            _ => { i += 2; consumed = i; }
        }
    }
    (data, events, consumed)
}

async fn read_events(
    stream: &mut TcpStream,
    raw_buf: &mut Vec<u8>,
    secs: u64,
    server_will_gmcp: &mut bool,
    gmcp_data: &mut Vec<String>,
) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(secs);
    let mut buf = [0u8; 4096];

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() { break; }

        match timeout(remaining, stream.read(&mut buf)).await {
            Ok(Ok(0)) => { println!("[連線關閉]"); break; }
            Ok(Ok(n)) => {
                raw_buf.extend_from_slice(&buf[..n]);
                let (text, events, consumed) = parse(raw_buf);
                raw_buf.drain(..consumed);

                if !text.is_empty() {
                    let (decoded, _, _) = encoding_rs::BIG5.decode(&text);
                    let s = decoded.to_string();
                    if !s.trim().is_empty() {
                        for line in s.lines().take(5) {
                            println!("[TEXT] {}", &line[..line.len().min(120)]);
                        }
                        if s.lines().count() > 5 {
                            println!("[TEXT] ... ({} lines total)", s.lines().count());
                        }
                    }
                }

                for evt in &events {
                    match evt {
                        Evt::Cmd(c, opt) => {
                            println!("[TELNET] IAC {} {}", cmd_name(*c), opt_name(*opt));
                            if *c == WILL && *opt == GMCP {
                                *server_will_gmcp = true;
                                println!("  >>> 伺服器支援 GMCP! <<<");
                            }
                            if *c == WONT && *opt == GMCP {
                                println!("  >>> 伺服器拒絕 GMCP (WONT) <<<");
                            }
                        }
                        Evt::Sub(opt, data) => {
                            let decoded = String::from_utf8_lossy(data);
                            println!("[SUBNEG] {} : {}", opt_name(*opt), decoded);
                            if *opt == GMCP {
                                gmcp_data.push(decoded.to_string());
                            }
                        }
                    }
                }
            }
            Ok(Err(e)) => { eprintln!("[讀取錯誤] {}", e); break; }
            Err(_) => break,
        }
    }
}

async fn send_gmcp(stream: &mut TcpStream, payload: &[u8]) {
    let mut pkt = vec![IAC, SB, GMCP];
    pkt.extend_from_slice(payload);
    pkt.extend_from_slice(&[IAC, SE]);
    stream.write_all(&pkt).await.ok();
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("用法: {} <host> <port>", args[0]);
        std::process::exit(1);
    }
    let host = &args[1];
    let port: u16 = args[2].parse().expect("port 必須是數字");

    println!("=== GMCP 協商測試 ===");
    println!("目標: {}:{}\n", host, port);

    let addr = format!("{}:{}", host, port);
    let mut stream = match timeout(Duration::from_secs(10), TcpStream::connect(&addr)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => { eprintln!("連線失敗: {}", e); return; }
        Err(_) => { eprintln!("連線逾時"); return; }
    };
    stream.set_nodelay(true).ok();
    println!("已連線!\n");

    let mut raw_buf = Vec::new();
    let mut server_will_gmcp = false;
    let mut gmcp_data: Vec<String> = Vec::new();

    // 階段 1: 讀取伺服器初始協商
    println!("--- 階段 1: 伺服器初始協商 (5s) ---");
    read_events(&mut stream, &mut raw_buf, 5, &mut server_will_gmcp, &mut gmcp_data).await;

    // 階段 2: 主動送 IAC DO GMCP
    println!("\n--- 階段 2: 送出 IAC DO GMCP ---");
    stream.write_all(&[IAC, DO, GMCP]).await.ok();
    println!("已送出: IAC DO GMCP");
    read_events(&mut stream, &mut raw_buf, 5, &mut server_will_gmcp, &mut gmcp_data).await;

    // 階段 3: 如果 GMCP 可用，請求 room info
    if server_will_gmcp {
        println!("\n--- 階段 3: GMCP 握手 & 請求 Room ---");

        send_gmcp(&mut stream, b"Core.Hello { \"client\": \"mudclient-rs\", \"version\": \"0.1\" }").await;
        println!("送出: Core.Hello");

        send_gmcp(&mut stream, b"Core.Supports.Set [ \"Room 1\", \"Char 1\" ]").await;
        println!("送出: Core.Supports.Set [\"Room 1\", \"Char 1\"]");

        read_events(&mut stream, &mut raw_buf, 8, &mut server_will_gmcp, &mut gmcp_data).await;
    }

    // 結論
    println!("\n========================================");
    println!("  測試結論");
    println!("========================================");
    println!("伺服器 WILL GMCP: {}", if server_will_gmcp { "YES" } else { "NO" });
    println!("收到 GMCP 資料: {} 筆", gmcp_data.len());
    for (i, d) in gmcp_data.iter().enumerate() {
        println!("  [{}] {}", i, d);
    }
    if !server_will_gmcp {
        println!("\n此伺服器不支援 GMCP。房間 ID 必須依賴 content hash。");
    } else if gmcp_data.is_empty() {
        println!("\n伺服器接受 GMCP 但未送出資料（可能需要登入後才會送 Room 資訊）。");
    } else {
        println!("\nGMCP 資料可用!");
    }
}
