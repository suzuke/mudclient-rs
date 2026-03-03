//! mudcore 整合測試
//!
//! 驗證跨模組協作的完整工作流程

use mudcore::*;
use mudcore::telnet::{
    IAC, TelnetCommand, TelnetEvent, TelnetOption,
    GmcpMessage, parse_telnet_data, generate_refusal,
};

// ============================================================================
// Telnet 協定完整流程
// ============================================================================

#[test]
fn telnet_mixed_stream_with_gmcp() {
    // 模擬一個包含文字、Telnet 命令、GMCP 子協商的完整資料流
    let mut stream: Vec<u8> = Vec::new();

    // 1. 伺服器發送 WILL SGA
    stream.extend_from_slice(&[IAC, TelnetCommand::Will as u8, TelnetOption::SuppressGoAhead.as_byte()]);
    // 2. 一些文字
    stream.extend_from_slice(b"Welcome to MUD!\r\n");
    // 3. WILL GMCP
    stream.extend_from_slice(&[IAC, TelnetCommand::Will as u8, TelnetOption::Gmcp.as_byte()]);
    // 4. GMCP 子協商: Room.Info
    let gmcp_payload = b"Room.Info {\"id\":1,\"name\":\"Town Square\"}";
    stream.push(IAC);
    stream.push(TelnetCommand::Sb as u8);
    stream.push(TelnetOption::Gmcp.as_byte());
    stream.extend_from_slice(gmcp_payload);
    stream.push(IAC);
    stream.push(TelnetCommand::Se as u8);
    // 5. 更多文字
    stream.extend_from_slice(b"You are standing in the town square.");

    let (data, events, consumed) = parse_telnet_data(&stream);

    // 驗證文字資料
    assert_eq!(
        String::from_utf8_lossy(&data),
        "Welcome to MUD!\r\nYou are standing in the town square."
    );

    // 驗證事件
    assert_eq!(events.len(), 3);
    assert_eq!(events[0], TelnetEvent::Command(TelnetCommand::Will, TelnetOption::SuppressGoAhead));
    assert_eq!(events[1], TelnetEvent::Command(TelnetCommand::Will, TelnetOption::Gmcp));

    // 驗證 GMCP 子協商
    if let TelnetEvent::Subnegotiation(opt, sub_data) = &events[2] {
        assert_eq!(*opt, TelnetOption::Gmcp);
        let msg = GmcpMessage::parse(sub_data).unwrap();
        assert_eq!(msg.package, "Room.Info");
        assert!(msg.data.as_deref().unwrap().contains("Town Square"));
    } else {
        panic!("Expected GMCP Subnegotiation event");
    }

    assert_eq!(consumed, stream.len());
}

#[test]
fn telnet_incomplete_sequence_preserves_remainder() {
    let stream = [b'H', b'i', IAC, TelnetCommand::Will as u8];
    let (data, events, consumed) = parse_telnet_data(&stream);

    assert_eq!(data, b"Hi");
    assert!(events.is_empty());
    assert_eq!(consumed, 2);
}

#[test]
fn telnet_negotiation_responses() {
    // WILL SGA → DO
    let resp = generate_refusal(TelnetCommand::Will, TelnetOption::SuppressGoAhead);
    assert_eq!(resp, vec![IAC, TelnetCommand::Do as u8, TelnetOption::SuppressGoAhead.as_byte()]);

    // WILL GMCP → DO
    let resp = generate_refusal(TelnetCommand::Will, TelnetOption::Gmcp);
    assert_eq!(resp, vec![IAC, TelnetCommand::Do as u8, TelnetOption::Gmcp.as_byte()]);

    // DO MCCP2 → WONT
    let resp = generate_refusal(TelnetCommand::Do, TelnetOption::Mccp2);
    assert_eq!(resp, vec![IAC, TelnetCommand::Wont as u8, TelnetOption::Mccp2.as_byte()]);

    // WILL ECHO → 空（不回應）
    let resp = generate_refusal(TelnetCommand::Will, TelnetOption::Echo);
    assert!(resp.is_empty());
}

// ============================================================================
// GMCP
// ============================================================================

#[test]
fn gmcp_complex_json_roundtrip() {
    let json = r#"{"hp":100,"mp":50,"moves":200,"level":10,"class":"Warrior"}"#;
    let msg = GmcpMessage {
        package: "Char.Vitals".into(),
        data: Some(json.into()),
    };
    let bytes = msg.to_bytes();
    let parsed = GmcpMessage::parse(&bytes).unwrap();
    assert_eq!(parsed.package, "Char.Vitals");
    assert_eq!(parsed.data.as_deref(), Some(json));
}

#[test]
fn gmcp_package_without_data() {
    let msg = GmcpMessage { package: "Core.Ping".into(), data: None };
    let bytes = msg.to_bytes();
    assert_eq!(bytes, b"Core.Ping");
    let parsed = GmcpMessage::parse(&bytes).unwrap();
    assert!(parsed.data.is_none());
}

// ============================================================================
// Alias
// ============================================================================

#[test]
fn alias_expansion_with_parameters() {
    let mut manager = AliasManager::new();
    // Alias::new(name, pattern, replacement)
    manager.add(Alias::new("atk", "atk $1", "kill $1;cast 'magic missile' $1"));
    let result = manager.process("atk goblin");
    assert_eq!(result, "kill goblin;cast 'magic missile' goblin");
}

#[test]
fn alias_priority_longest_match_first() {
    let mut manager = AliasManager::new();
    manager.add(Alias::new("go", "go", "walk north"));
    manager.add(Alias::new("go_dir", "go $1", "walk $1"));

    let result = manager.process("go south");
    assert_eq!(result, "walk south");
}

// ============================================================================
// Trigger
// ============================================================================

#[test]
fn trigger_pattern_types() {
    let mut manager = TriggerManager::new();

    let mut t1 = Trigger::new("hp_warn", TriggerPattern::Contains("HP:".into()));
    t1.actions.push(TriggerAction::SendCommand("heal".into()));
    manager.add(t1);

    let mut t2 = Trigger::new("room_detect", TriggerPattern::StartsWith("[Room]".into()));
    t2.actions.push(TriggerAction::SendCommand("look".into()));
    manager.add(t2);

    // Contains 匹配
    let matches = manager.process("Your HP: 50/100 looks low.");
    assert!(matches.iter().any(|(t, _)| t.name == "hp_warn"));

    // StartsWith 匹配
    let matches = manager.process("[Room] Town Square");
    assert!(matches.iter().any(|(t, _)| t.name == "room_detect"));

    // 不匹配
    let matches = manager.process("Nothing special here.");
    assert!(matches.is_empty());
}

#[test]
fn trigger_regex_captures() {
    let mut manager = TriggerManager::new();
    let mut t = Trigger::new("damage", TriggerPattern::Regex(r"You deal (\d+) damage to (.+)\.".into()));
    t.actions.push(TriggerAction::SendCommand("echo hit $1 on $2".into()));
    manager.add(t);

    let matches = manager.process("You deal 42 damage to dragon.");
    assert_eq!(matches.len(), 1);
    let (_, m) = &matches[0];
    // captures 只含捕獲組（skip(1)），不含 full match
    assert_eq!(m.captures[0], "42");
    assert_eq!(m.captures[1], "dragon");
}

#[test]
fn disabled_trigger_does_not_fire() {
    let mut manager = TriggerManager::new();
    let mut t = Trigger::new("greeting", TriggerPattern::Contains("Hello".into()));
    t.enabled = false;
    t.actions.push(TriggerAction::SendCommand("wave".into()));
    manager.add(t);

    let matches = manager.process("Hello world!");
    assert!(matches.is_empty());
}

// ============================================================================
// Encoding (Big5 ↔ UTF-8)
// ============================================================================

#[test]
fn big5_utf8_roundtrip_chinese_text() {
    let original = "你好世界！歡迎來到武林MUD";
    let encoded = encode_big5(original);
    let decoded = decode_big5(&encoded);
    assert_eq!(decoded, original);
}

#[test]
fn big5_utf8_mixed_content() {
    let original = "HP: 100 生命值: 滿";
    let encoded = encode_big5(original);
    let decoded = decode_big5(&encoded);
    assert_eq!(decoded, original);
}

// ============================================================================
// Window routing
// ============================================================================

#[test]
fn window_message_routing() {
    let mut wm = WindowManager::new();

    let sub = SubWindow::new("combat", "戰鬥").with_capacity(200);
    wm.add_window(sub);

    let msg = WindowMessage::new("You hit the goblin!");
    wm.route_message("combat", msg);

    let sub = wm.get("combat").unwrap();
    assert_eq!(sub.messages().count(), 1);

    let main = wm.main_window();
    assert_eq!(main.message_count(), 0);
}

#[test]
fn main_window_always_exists() {
    let wm = WindowManager::new();
    let _ = wm.main_window(); // 不 panic
}

// ============================================================================
// MessageBuffer
// ============================================================================

#[test]
fn message_buffer_capacity_enforcement() {
    let mut buf = MessageBuffer::new(3);
    for i in 0..10 {
        buf.push(format!("msg {}", i));
    }
    let msgs: Vec<_> = buf.iter().collect();
    assert_eq!(msgs.len(), 3);
    assert_eq!(*msgs[0], "msg 7");
    assert_eq!(*msgs[2], "msg 9");
}

// ============================================================================
// Speedwalk
// ============================================================================

#[test]
fn speedwalk_complex_path() {
    let cmds = parse_speedwalk("/3n2e1s").unwrap();
    // 以 recall 開頭，方向用短碼
    assert_eq!(cmds, vec!["recall", "n", "n", "n", "e", "e", "s"]);
}

#[test]
fn speedwalk_single_directions() {
    let cmds = parse_speedwalk("/nesw").unwrap();
    assert_eq!(cmds, vec!["recall", "ne", "sw"]);
}

#[test]
fn speedwalk_invalid_input_returns_none() {
    assert!(parse_speedwalk("3n2e").is_none());
    assert!(parse_speedwalk("/3x").is_none());
}

// ============================================================================
// Path management
// ============================================================================

#[test]
fn path_manager_store_and_retrieve() {
    let mut pm = PathManager::new();
    pm.add(Path::new("to_shop", "/2n1e"));
    assert!(pm.get("to_shop").is_some());
    assert_eq!(pm.get("to_shop").unwrap().value, "/2n1e");
}

#[test]
fn path_recorder_record_and_pop() {
    let mut recorder = PathRecorder::default();
    recorder.start();

    recorder.record("north");
    recorder.record("east");
    recorder.record("east");
    recorder.pop_last();

    assert_eq!(recorder.recorded_commands.len(), 2);
    assert_eq!(recorder.recorded_commands[0], "north");
    assert_eq!(recorder.recorded_commands[1], "east");

    let path_str = recorder.get_path_string();
    assert_eq!(path_str, "north;east");
}

// ============================================================================
// 跨模組整合：Telnet → GMCP → 結構化資料
// ============================================================================

#[test]
fn full_gmcp_pipeline() {
    let mut stream = Vec::new();

    // GMCP: Char.Vitals
    let vitals = r#"Char.Vitals {"hp":85,"maxhp":100,"mp":50,"maxmp":75}"#;
    stream.push(IAC);
    stream.push(TelnetCommand::Sb as u8);
    stream.push(TelnetOption::Gmcp.as_byte());
    stream.extend_from_slice(vitals.as_bytes());
    stream.push(IAC);
    stream.push(TelnetCommand::Se as u8);

    stream.extend_from_slice(b"A goblin attacks you!");

    // GMCP: Room.Info
    let room = r#"Room.Info {"id":"abc123","name":"Dark Forest","exits":["north","south"]}"#;
    stream.push(IAC);
    stream.push(TelnetCommand::Sb as u8);
    stream.push(TelnetOption::Gmcp.as_byte());
    stream.extend_from_slice(room.as_bytes());
    stream.push(IAC);
    stream.push(TelnetCommand::Se as u8);

    let (text, events, _) = parse_telnet_data(&stream);
    assert_eq!(String::from_utf8_lossy(&text), "A goblin attacks you!");

    let gmcp_msgs: Vec<GmcpMessage> = events.iter()
        .filter_map(|e| {
            if let TelnetEvent::Subnegotiation(TelnetOption::Gmcp, data) = e {
                GmcpMessage::parse(data)
            } else {
                None
            }
        })
        .collect();

    assert_eq!(gmcp_msgs.len(), 2);
    assert_eq!(gmcp_msgs[0].package, "Char.Vitals");
    assert_eq!(gmcp_msgs[1].package, "Room.Info");

    // JSON 可正確解析
    let vitals_json: serde_json::Value = serde_json::from_str(gmcp_msgs[0].data.as_ref().unwrap()).unwrap();
    assert_eq!(vitals_json["hp"], 85);

    let room_json: serde_json::Value = serde_json::from_str(gmcp_msgs[1].data.as_ref().unwrap()).unwrap();
    assert_eq!(room_json["name"], "Dark Forest");
}

// ============================================================================
// Alias + Trigger 整合場景
// ============================================================================

#[test]
fn alias_trigger_workflow() {
    // 模擬完整使用流程：使用者輸入 → Alias 展開 → 伺服器回應 → Trigger 匹配

    let mut alias_mgr = AliasManager::new();
    alias_mgr.add(Alias::new("atk", "atk $1", "kill $1"));

    let mut trigger_mgr = TriggerManager::new();
    let mut t = Trigger::new("kill_echo", TriggerPattern::Regex(r"You killed (.+)!".into()));
    t.actions.push(TriggerAction::SendCommand("loot $1".into()));
    trigger_mgr.add(t);

    // 使用者輸入 → Alias 展開
    let expanded = alias_mgr.process("atk dragon");
    assert_eq!(expanded, "kill dragon");

    // 伺服器回應 → Trigger 匹配
    let matches = trigger_mgr.process("You killed dragon!");
    assert_eq!(matches.len(), 1);
    let (trigger, m) = &matches[0];
    assert_eq!(trigger.name, "kill_echo");
    assert_eq!(m.captures[0], "dragon"); // captures[0] = 第一個捕獲組

    // 展開 trigger action 的捕獲組（$1 對應 captures[0]）
    if let TriggerAction::SendCommand(cmd) = &trigger.actions[0] {
        let mut result = cmd.clone();
        for (i, cap) in m.captures.iter().enumerate() {
            result = result.replace(&format!("${}", i + 1), cap);
        }
        assert_eq!(result, "loot dragon");
    }
}

// ============================================================================
// Encoding + Telnet 整合
// ============================================================================

#[test]
fn big5_through_telnet_stream() {
    let chinese = "你好世界";
    let big5_bytes = encode_big5(chinese);

    let mut stream = Vec::new();
    stream.extend_from_slice(&big5_bytes);
    stream.extend_from_slice(&[IAC, TelnetCommand::GoAhead as u8]);
    stream.extend_from_slice(b"OK");

    let (data, _events, consumed) = parse_telnet_data(&stream);

    let mut expected = big5_bytes.clone();
    expected.extend_from_slice(b"OK");
    assert_eq!(data, expected);
    assert_eq!(consumed, stream.len());

    // Big5 部分可正確解碼
    let decoded = decode_big5(&data[..big5_bytes.len()]);
    assert_eq!(decoded, chinese);
}
