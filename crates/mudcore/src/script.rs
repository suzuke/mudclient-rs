use mlua::Lua;
use std::collections::HashMap;
use thiserror::Error;
use std::cell::{Cell, RefCell};

/// 腳本執行錯誤
#[derive(Debug, Error)]
pub enum ScriptError {
    #[error("Lua 錯誤: {0}")]
    Lua(String),
    
    #[error("腳本未找到: {0}")]
    NotFound(String),
}

impl From<mlua::Error> for ScriptError {
    fn from(err: mlua::Error) -> Self {
        ScriptError::Lua(err.to_string())
    }
}

/// 日誌控制指令
#[derive(Debug, Clone, PartialEq)]
pub enum LogControl {
    Start(String),
    Stop,
}

/// MUD 腳本上下文（腳本執行後的結果）
#[derive(Debug, Clone, Default)]
pub struct MudContext {
    /// 待發送的命令隊列
    pub commands: Vec<String>,
    
    /// 變數存儲
    pub variables: HashMap<String, String>,
    
    /// 是否應該抑制當前訊息
    pub gag: bool,
    
    /// 本地顯示的訊息（mud.echo）
    pub echos: Vec<String>,
    
    /// 輸出到子視窗的訊息 (window_id, message)
    pub window_outputs: Vec<(String, String)>,
    
    /// 寫入日誌的訊息
    pub log_messages: Vec<String>,
    
    /// 日誌控制指令
    pub log_control: Option<LogControl>,
    
    /// 延遲執行的 Timer (delay_ms, lua_code)
    pub timers: Vec<(u64, String)>,
    
    /// 觸發器狀態更新 (name, enabled)
    pub trigger_updates: Vec<(String, bool)>,

    /// 指令回應收集請求 (command, callback_code)
    pub response_collectors: Vec<(String, String)>,

    /// LLM 請求佇列 (prompt, callback_lua_code, model)
    pub llm_requests: Vec<LlmRequest>,

    /// Event handler registrations: (event_name, lua_code, priority, once)
    pub event_registrations: Vec<(String, String, i32, bool)>,
    /// Event handler removals: handler_id
    pub event_removals: Vec<u64>,
    /// Events to emit: (event_name, data_json)
    pub event_emissions: Vec<(String, Option<String>)>,

    /// Trigger group updates: (group_name, enabled)
    pub group_updates: Vec<(String, bool)>,

    /// State machine definitions: (name, initial, states_json, transitions_json)
    pub state_machine_defs: Vec<(String, String, String, String)>,
    /// State machine manual transitions: (machine_name, event_name)
    pub state_machine_transitions: Vec<(String, String)>,
    /// State machine resets: machine_name
    pub state_machine_resets: Vec<String>,

    /// Key binding updates: (key_combo, Some(lua_code) = bind, None = unbind)
    pub key_binding_updates: Vec<(String, Option<String>)>,
}

/// LLM 非同步請求
#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub prompt: String,
    pub callback_code: String,
    pub model: Option<String>,
}

impl MudContext {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Lua 腳本引擎
pub struct ScriptEngine {
    /// Lua 解釋器實例
    lua: Lua,
    /// 已載入的腳本
    scripts: HashMap<String, String>,
    /// 持久化變數（跨觸發器共享）
    persistent_vars: RefCell<HashMap<String, String>>,
    /// 腳本目錄的絕對路徑，用於 dofile 查找
    scripts_dir: Option<String>,
    /// 當前房間 (Thread-local storage concept within engine)
    current_room: RefCell<Option<crate::map::Room>>,
    /// dofile/package.path 是否已設定（避免每次 run_code 重複執行）
    dofile_initialized: Cell<bool>,
}

impl ScriptEngine {
    /// 創建新的腳本引擎
    pub fn new() -> Self {
        let lua = Lua::new();
        Self {
            lua,
            scripts: HashMap::new(),
            persistent_vars: RefCell::new(HashMap::new()),
            scripts_dir: None,
            current_room: RefCell::new(None),
            dofile_initialized: Cell::new(false),
        }
    }

    /// 設定腳本目錄路徑（供 dofile 查找）
    pub fn set_scripts_dir(&mut self, dir: impl Into<String>) {
        self.scripts_dir = Some(dir.into());
    }

    /// 設定當前房間
    pub fn set_current_room(&self, room: Option<crate::map::Room>) {
        *self.current_room.borrow_mut() = room;
    }

    /// 載入腳本
    pub fn load_script(&mut self, name: impl Into<String>, code: impl Into<String>) {
        self.scripts.insert(name.into(), code.into());
    }

    /// 移除腳本
    pub fn remove_script(&mut self, name: &str) -> bool {
        self.scripts.remove(name).is_some()
    }

    /// 展開變數 (將 $var 替換為變數值)
    pub fn expand_variables(&self, text: &str) -> String {
        if !text.contains('$') {
            return text.to_string();
        }
        let vars = self.persistent_vars.borrow();
        if vars.is_empty() {
            return text.to_string();
        }
        let mut result = text.to_string();
        // 按 key 長度降序排列，避免 $hp 先於 $hp_max 被替換
        let mut sorted_vars: Vec<_> = vars.iter().collect();
        sorted_vars.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        for (key, value) in sorted_vars {
            let placeholder = format!("${}", key);
            // replace 會在無匹配時直接返回原字串的 clone，故先 contains 檢查避免無謂分配
            if result.contains(placeholder.as_str()) {
                result = result.replace(placeholder.as_str(), value);
            }
        }
        result
    }

    /// 執行腳本
    pub fn execute(
        &self,
        script_name: &str,
        message: &str,
        captures: &[String],
        is_echo: bool,
    ) -> Result<MudContext, ScriptError> {
        let code = self
            .scripts
            .get(script_name)
            .ok_or_else(|| ScriptError::NotFound(script_name.to_string()))?;

        // 執行腳本時也預設 clean_message = message
        self.run_code(code, None, message, message, captures, is_echo).map(|(ctx, _)| ctx)
    }

    /// 執行內聯代碼
    pub fn execute_inline(
        &self,
        code: &str,
        message: &str,
        captures: &[String],
        is_echo: bool,
    ) -> Result<MudContext, ScriptError> {
        // inline 執行通常只有 message，沒有特定的 clean_message 來源，預設與 message 相同或空
        // 這裡為了兼容現有調用，將 clean_message 設為與 message 相同
        self.run_code(code, None, message, message, captures, is_echo).map(|(ctx, _)| ctx)
    }

    /// 執行內聯代碼並返回 JSON 字串結果 (用於 evaluate_lua)
    pub fn execute_inline_with_result(
        &self,
        code: &str,
        message: &str,
        captures: &[String],
    ) -> Result<String, ScriptError> {
        let (_, result) = self.run_code("", Some(code), message, message, captures, false)?;
        Ok(result.unwrap_or_else(|| "null".to_string()))
    }

    fn lua_value_to_json(val: &mlua::Value) -> serde_json::Value {
        match val {
            mlua::Value::Nil => serde_json::Value::Null,
            mlua::Value::Boolean(b) => serde_json::Value::Bool(*b),
            mlua::Value::Integer(i) => serde_json::Value::Number((*i).into()),
            mlua::Value::Number(n) => {
                if let Some(num) = serde_json::Number::from_f64(*n) {
                    serde_json::Value::Number(num)
                } else {
                    serde_json::Value::Null
                }
            }
            mlua::Value::String(s) => {
                serde_json::Value::String(s.to_string_lossy())
            }
            mlua::Value::Table(t) => {
                // Determine if it's an array or object
                let len = t.len().unwrap_or(0);
                if len > 0 {
                    let mut arr = Vec::new();
                    for i in 1..=len {
                        if let Ok(v) = t.get(i) {
                            arr.push(Self::lua_value_to_json(&v));
                        } else {
                            arr.push(serde_json::Value::Null);
                        }
                    }
                    serde_json::Value::Array(arr)
                } else {
                    let mut obj = serde_json::Map::new();
                    for pair in t.pairs::<mlua::Value, mlua::Value>() {
                        if let Ok((k, v)) = pair {
                            let key_str = match k {
                                mlua::Value::String(s) => s.to_string_lossy(),
                                mlua::Value::Integer(i) => i.to_string(),
                                _ => continue, // Cannot use non-string/int keys in JSON object
                            };
                            obj.insert(key_str, Self::lua_value_to_json(&v));
                        }
                    }
                    serde_json::Value::Object(obj)
                }
            }
            _ => serde_json::Value::String(format!("{:?}", val)),
        }
    }

    /// 運行 Lua 代碼
    fn run_code(
        &self,
        code: &str,
        eval_code: Option<&str>,
        message: &str,
        clean_message: &str,
        captures: &[String],
        is_echo: bool,
    ) -> Result<(MudContext, Option<String>), ScriptError> {
        let mut context = MudContext::new();

        let eval_result_str = self.lua.scope(|scope| {
            // 創建 mud 表用於存放 API
            let mud = self.lua.create_table()?;
            
            // 創建命令列表
            let commands = self.lua.create_table()?;
            mud.set("commands", commands)?;
            
            // 創建變數表並載入已儲存的持久化變數
            let variables = self.lua.create_table()?;
            for (key, value) in self.persistent_vars.borrow().iter() {
                variables.set(key.as_str(), value.as_str())?;
            }
            mud.set("variables", variables)?;
            
            // 創建 echos 表（本地顯示）
            let echos = self.lua.create_table()?;
            mud.set("echos", echos)?;
            
            // 創建 window_outputs 表（子視窗輸出）
            let window_outputs = self.lua.create_table()?;
            mud.set("window_outputs", window_outputs)?;
            
            // 創建 log_messages 表
            let log_messages = self.lua.create_table()?;
            mud.set("log_messages", log_messages)?;
            
            // 創建 timers 表
            let timers = self.lua.create_table()?;
            mud.set("timers", timers)?;
            
            // 創建 trigger_updates 表
            let trigger_updates = self.lua.create_table()?;
            mud.set("trigger_updates", trigger_updates)?;

            // 創建 response_collectors 表
            let response_collectors = self.lua.create_table()?;
            mud.set("response_collectors", response_collectors)?;

            // 創建 llm_requests 表
            let llm_requests = self.lua.create_table()?;
            mud.set("llm_requests", llm_requests)?;

            // Event system tables
            let event_registrations = self.lua.create_table()?;
            mud.set("_event_registrations", event_registrations)?;

            let event_removals = self.lua.create_table()?;
            mud.set("_event_removals", event_removals)?;

            let event_emissions = self.lua.create_table()?;
            mud.set("_event_emissions", event_emissions)?;

            // Group updates table
            let group_updates = self.lua.create_table()?;
            mud.set("_group_updates", group_updates)?;

            // State machine tables
            let sm_defs = self.lua.create_table()?;
            mud.set("_sm_defs", sm_defs)?;
            let sm_transitions = self.lua.create_table()?;
            mud.set("_sm_transitions", sm_transitions)?;
            let sm_resets = self.lua.create_table()?;
            mud.set("_sm_resets", sm_resets)?;
            let sm_states = self.lua.create_table()?;
            mud.set("_sm_states", sm_states)?;

            // Key binding table
            let key_binding_updates = self.lua.create_table()?;
            mud.set("_key_binding_updates", key_binding_updates)?;

            // gag 標記
            mud.set("gag", false)?;

            // Log Control
            let log_control = self.lua.create_table()?;
            mud.set("_log_control", log_control)?; // Internal use
            
            // 是否為回顯
            mud.set("is_echo", is_echo)?;
            
            // mud.send(command) 函數
            let send_fn = scope.create_function_mut(|lua, cmd: String| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                let cmds: mlua::Table = mud.get("commands")?;
                let len = cmds.len()? + 1;
                cmds.set(len, cmd)?;
                Ok(())
            })?;
            mud.set("send", send_fn)?;
            
            // mud.gag_message() 函數
            let gag_fn = scope.create_function_mut(|lua, ()| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                mud.set("gag", true)?;
                Ok(())
            })?;
            mud.set("gag_message", gag_fn)?;
            
            // mud.echo(text) 函數 - 本地顯示訊息
            let echo_fn = scope.create_function(|lua, text: String| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                let echos: mlua::Table = mud.get("echos")?;
                let len = echos.len()? + 1;
                echos.set(len, text)?;
                Ok(())
            })?;
            mud.set("echo", echo_fn.clone())?;
            
            // 覆寫 print 為 mud.echo
            self.lua.globals().set("print", echo_fn)?;
            
            // mud.window(name, text) 函數 - 輸出到子視窗
            let window_fn = scope.create_function(|lua, (name, text): (String, String)| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                let outputs: mlua::Table = mud.get("window_outputs")?;
                let len = outputs.len()? + 1;
                let pair = lua.create_table()?;
                pair.set(1, name)?;
                pair.set(2, text)?;
                outputs.set(len, pair)?;
                Ok(())
            })?;
            mud.set("window", window_fn)?;
            
            // mud.log(message) 函數 - 寫入日誌
            let log_fn = scope.create_function(|lua, msg: String| {
                tracing::info!("[Script] {}", msg);
                let mud: mlua::Table = lua.globals().get("mud")?;
                let logs: mlua::Table = mud.get("log_messages")?;
                let len = logs.len()? + 1;
                logs.set(len, msg)?;
                Ok(())
            })?;
            mud.set("log", log_fn)?;

            // mud.start_log(path)
            let start_log_fn = scope.create_function(|lua, path: String| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                let ctrl: mlua::Table = mud.get("_log_control")?;
                ctrl.set("action", "start")?;
                ctrl.set("path", path)?;
                Ok(())
            })?;
            mud.set("start_log", start_log_fn)?;

            // mud.stop_log()
            let stop_log_fn = scope.create_function(|lua, ()| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                let ctrl: mlua::Table = mud.get("_log_control")?;
                ctrl.set("action", "stop")?;
                Ok(())
            })?;
            mud.set("stop_log", stop_log_fn)?;
            
            // mud.timer(seconds, code) 函數 - 延遲執行
            let timer_fn = scope.create_function(|lua, (seconds, lua_code): (f64, String)| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                let timers: mlua::Table = mud.get("timers")?;
                let len = timers.len()? + 1;
                let pair = lua.create_table()?;
                pair.set(1, (seconds * 1000.0) as u64)?; // 轉換為毫秒
                pair.set(2, lua_code)?;
                timers.set(len, pair)?;
                Ok(())
            })?;
            mud.set("timer", timer_fn)?;
            
            // mud.enable_trigger(name, enabled) 函數 - 啟用/禁用觸發器
            let enable_trigger_fn = scope.create_function(|lua, (name, enabled): (String, bool)| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                let updates: mlua::Table = mud.get("trigger_updates")?;
                let len = updates.len()? + 1;
                let pair = lua.create_table()?;
                pair.set(1, name)?;
                pair.set(2, enabled)?;
                updates.set(len, pair)?;
                Ok(())
            })?;
            mud.set("enable_trigger", enable_trigger_fn)?;

            // mud.enable_group(group_name, enabled) 函數 - 啟用/禁用觸發器群組
            let enable_group_fn = scope.create_function_mut(|lua, (group, enabled): (String, bool)| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                let updates: mlua::Table = mud.get("_group_updates")?;
                let len = updates.len()? + 1;
                let entry = lua.create_table()?;
                entry.set(1, group)?;
                entry.set(2, enabled)?;
                updates.set(len, entry)?;
                Ok(())
            })?;
            mud.set("enable_group", enable_group_fn)?;

            // mud.collect_response(cmd, callback_code) 函數
            // 發送指令並收集所有回應行直到 prompt（網路層指令佇列化）
            let collect_response_fn = scope.create_function(|lua, (cmd, callback_code): (String, String)| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                let collectors: mlua::Table = mud.get("response_collectors")?;
                let len = collectors.len()? + 1;
                let entry = lua.create_table()?;
                entry.set(1, cmd)?;
                entry.set(2, callback_code)?;
                collectors.set(len, entry)?;
                Ok(())
            })?;
            mud.set("collect_response", collect_response_fn)?;

            // mud.send_chain(cmds, [callback_code]) 函數
            // 依序發送多個指令，每個指令等待 server 回應後才發下一個
            // cmds: table of strings, callback_code: 最後執行的 Lua code（可選）
            // 內部展開為多個 collect_response，利用網路層的佇列串連
            let send_chain_fn = scope.create_function(|lua, (cmds, callback_code): (mlua::Table, Option<String>)| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                let collectors: mlua::Table = mud.get("response_collectors")?;
                let cmd_count = cmds.len()?;

                for i in 1..=cmd_count {
                    let cmd: String = cmds.get(i)?;
                    let base = collectors.len()? + 1;
                    let entry = lua.create_table()?;
                    entry.set(1, cmd)?;
                    // 只有最後一個指令帶 callback，其餘用空字串（不執行 callback）
                    if i == cmd_count {
                        entry.set(2, callback_code.as_deref().unwrap_or(""))?;
                    } else {
                        entry.set(2, "")?;
                    }
                    collectors.set(base, entry)?;
                }
                Ok(())
            })?;
            mud.set("send_chain", send_chain_fn)?;

            // mud.ask_llm(prompt, callback_lua_code, [model])
            // 非同步呼叫 LLM，回覆後執行 callback（callback 中 $RESULT 被替換為回覆）
            let ask_llm_fn = scope.create_function(|lua, (prompt, callback_code, model): (String, String, Option<String>)| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                let requests: mlua::Table = mud.get("llm_requests")?;
                let len = requests.len()? + 1;
                let entry = lua.create_table()?;
                entry.set(1, prompt)?;
                entry.set(2, callback_code)?;
                entry.set(3, model.unwrap_or_default())?;
                requests.set(len, entry)?;
                Ok(())
            })?;
            mud.set("ask_llm", ask_llm_fn)?;

            // mud.get_room_id(name, desc, exits, [strict]) -> string
            let get_room_id_fn = scope.create_function(|_lua, (name, desc, exits, strict): (String, String, Vec<String>, Option<bool>)| {
                let room = crate::map::Room::new(&name, &desc, exits);
                Ok(room.hash(strict.unwrap_or(true)))
            })?;
            mud.set("get_room_id", get_room_id_fn)?;

            // mud.get_current_room_id([strict]) -> string | nil
            let current_room = self.current_room.borrow().clone();
            let current_room_for_id = current_room.clone();
            
            let get_current_room_id_fn = scope.create_function(move |_lua, strict: Option<bool>| {
                if let Some(room) = &current_room_for_id {
                    Ok(Some(room.hash(strict.unwrap_or(true))))
                } else {
                    Ok(None)
                }
            })?;
            mud.set("get_current_room_id", get_current_room_id_fn)?;

            // mud.get_current_room() -> {name, description, exits} | nil
            let get_current_room_fn = scope.create_function(move |lua, ()| {
                if let Some(room) = &current_room {
                    let tbl = lua.create_table()?;
                    tbl.set("name", room.name.clone())?;
                    tbl.set("description", room.description.clone())?;
                    tbl.set("exits", room.exits.clone())?;
                    Ok(Some(tbl))
                } else {
                    Ok(None)
                }
            })?;
            mud.set("get_current_room", get_current_room_fn)?;

            // mud.on(event_name, lua_code, [priority]) -> handler_id (placeholder)
            let on_fn = scope.create_function_mut(|lua, (event_name, code, priority): (String, String, Option<i32>)| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                let regs: mlua::Table = mud.get("_event_registrations")?;
                let len = regs.len()? + 1;
                let entry = lua.create_table()?;
                entry.set(1, event_name)?;
                entry.set(2, code)?;
                entry.set(3, priority.unwrap_or(0))?;
                entry.set(4, false)?; // once = false
                regs.set(len, entry)?;
                Ok(len) // placeholder ID
            })?;
            mud.set("on", on_fn)?;

            // mud.once(event_name, lua_code, [priority]) -> handler_id (placeholder)
            let once_fn = scope.create_function_mut(|lua, (event_name, code, priority): (String, String, Option<i32>)| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                let regs: mlua::Table = mud.get("_event_registrations")?;
                let len = regs.len()? + 1;
                let entry = lua.create_table()?;
                entry.set(1, event_name)?;
                entry.set(2, code)?;
                entry.set(3, priority.unwrap_or(0))?;
                entry.set(4, true)?; // once = true
                regs.set(len, entry)?;
                Ok(len)
            })?;
            mud.set("once", once_fn)?;

            // mud.off(handler_id)
            let off_fn = scope.create_function_mut(|lua, handler_id: u64| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                let removals: mlua::Table = mud.get("_event_removals")?;
                let len = removals.len()? + 1;
                removals.set(len, handler_id)?;
                Ok(())
            })?;
            mud.set("off", off_fn)?;

            // mud.emit(event_name, [data])
            let emit_fn = scope.create_function_mut(|lua, (event_name, data): (String, Option<mlua::Value>)| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                let emissions: mlua::Table = mud.get("_event_emissions")?;
                let len = emissions.len()? + 1;
                let entry = lua.create_table()?;
                entry.set(1, event_name)?;
                let json_str: Option<String> = match data {
                    Some(ref v @ mlua::Value::Table(_)) => {
                        let json_val = ScriptEngine::lua_value_to_json(v);
                        Some(serde_json::to_string(&json_val).unwrap_or_default())
                    }
                    Some(mlua::Value::String(s)) => Some(s.to_str()?.to_string()),
                    _ => None,
                };
                entry.set(2, json_str)?;
                emissions.set(len, entry)?;
                Ok(())
            })?;
            mud.set("emit", emit_fn)?;

            // mud.state_machine(name, definition)
            // definition = { initial = "state", states = { state = { enter = "code", exit = "code", timeout = {seconds=N, goto="state"} } }, transitions = { {from=, event=, to=} } }
            let sm_fn = scope.create_function_mut(|lua, (name, def): (String, mlua::Table)| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                let defs: mlua::Table = mud.get("_sm_defs")?;

                let initial: String = def.get("initial")?;
                let states_table: mlua::Table = def.get("states")?;

                // Serialize states to JSON
                let mut states_map = serde_json::Map::new();
                for pair in states_table.pairs::<String, mlua::Table>() {
                    let (state_name, state_def) = pair?;
                    let mut obj = serde_json::Map::new();
                    if let Ok(enter) = state_def.get::<String>("enter") { obj.insert("enter".into(), serde_json::Value::String(enter)); }
                    if let Ok(exit) = state_def.get::<String>("exit") { obj.insert("exit".into(), serde_json::Value::String(exit)); }
                    if let Ok(timeout) = state_def.get::<mlua::Table>("timeout") {
                        if let (Ok(secs), Ok(goto)) = (timeout.get::<f64>("seconds"), timeout.get::<String>("goto")) {
                            obj.insert("timeout_secs".into(), serde_json::json!(secs));
                            obj.insert("timeout_goto".into(), serde_json::Value::String(goto));
                        }
                    }
                    states_map.insert(state_name, serde_json::Value::Object(obj));
                }

                // Serialize transitions
                let trans_table: mlua::Table = def.get("transitions")?;
                let mut trans_arr = Vec::new();
                for i in 1..=trans_table.len()? {
                    let t: mlua::Table = trans_table.get(i)?;
                    trans_arr.push(serde_json::json!({
                        "from": t.get::<String>("from")?,
                        "event": t.get::<String>("event")?,
                        "to": t.get::<String>("to")?,
                    }));
                }

                let len = defs.len()? + 1;
                let entry = lua.create_table()?;
                entry.set(1, name.clone())?;
                entry.set(2, initial)?;
                entry.set(3, serde_json::Value::Object(states_map).to_string())?;
                entry.set(4, serde_json::Value::Array(trans_arr).to_string())?;
                defs.set(len, entry)?;
                Ok(name)
            })?;
            mud.set("state_machine", sm_fn)?;

            // mud.sm_current(name) -> state string or nil
            let sm_current_fn = scope.create_function(|lua, name: String| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                if let Ok(states) = mud.get::<mlua::Table>("_sm_states") {
                    let state: Option<String> = states.get(name).ok();
                    Ok(state)
                } else {
                    Ok(None)
                }
            })?;
            mud.set("sm_current", sm_current_fn)?;

            // mud.sm_transition(name, event)
            let sm_trans_fn = scope.create_function_mut(|lua, (name, event): (String, String)| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                let transitions: mlua::Table = mud.get("_sm_transitions")?;
                let len = transitions.len()? + 1;
                let entry = lua.create_table()?;
                entry.set(1, name)?;
                entry.set(2, event)?;
                transitions.set(len, entry)?;
                Ok(())
            })?;
            mud.set("sm_transition", sm_trans_fn)?;

            // mud.sm_reset(name)
            let sm_reset_fn = scope.create_function_mut(|lua, name: String| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                let resets: mlua::Table = mud.get("_sm_resets")?;
                let len = resets.len()? + 1;
                resets.set(len, name)?;
                Ok(())
            })?;
            mud.set("sm_reset", sm_reset_fn)?;

            // mud.bind_key(key_combo, lua_code)
            let bind_key_fn = scope.create_function_mut(|lua, (key, code): (String, String)| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                let updates: mlua::Table = mud.get("_key_binding_updates")?;
                let len = updates.len()? + 1;
                let entry = lua.create_table()?;
                entry.set(1, key.to_lowercase())?;
                entry.set(2, code)?;
                updates.set(len, entry)?;
                Ok(())
            })?;
            mud.set("bind_key", bind_key_fn)?;

            // mud.unbind_key(key_combo)
            let unbind_key_fn = scope.create_function_mut(|lua, key: String| {
                let mud: mlua::Table = lua.globals().get("mud")?;
                let updates: mlua::Table = mud.get("_key_binding_updates")?;
                let len = updates.len()? + 1;
                let entry = lua.create_table()?;
                entry.set(1, key.to_lowercase())?;
                // No second value = unbind
                updates.set(len, entry)?;
                Ok(())
            })?;
            mud.set("unbind_key", unbind_key_fn)?;

            // 設置全局變數
            self.lua.globals().set("mud", mud)?;
            self.lua.globals().set("message", message)?;
            self.lua.globals().set("clean_message", clean_message)?;
            
            // 設置 captures 表
            let captures_table = self.lua.create_table()?;
            for (i, cap) in captures.iter().enumerate() {
                captures_table.set(i + 1, cap.as_str())?;
            }
            self.lua.globals().set("captures", captures_table)?;

            // 覆寫 dofile：支援從 scripts_dir 查找腳本（僅首次設定）
            if !self.dofile_initialized.get() {
                if let Some(dir) = &self.scripts_dir {
                    self.lua.globals().set("__scripts_dir", dir.as_str())?;

                    let update_path = self.lua.load(r#"
                        local path_sep = package.config:sub(1,1)
                        local scripts_path = __scripts_dir .. path_sep .. "?.lua"
                        local scripts_init = __scripts_dir .. path_sep .. "?" .. path_sep .. "init.lua"

                        if not string.find(package.path, scripts_path, 1, true) then
                            package.path = package.path .. ";" .. scripts_path .. ";" .. scripts_init
                        end
                    "#).exec();
                    if let Err(e) = update_path {
                        tracing::warn!("Failed to update package.path: {}", e);
                    }

                    let custom_dofile = self.lua.load(r#"
                        local original_dofile = dofile
                        function dofile(path)
                            local f = io.open(path, "r")
                            if f then
                                f:close()
                                return original_dofile(path)
                            end
                            local full = __scripts_dir .. "/" .. path
                            f = io.open(full, "r")
                            if f then
                                f:close()
                                return original_dofile(full)
                            end
                            local basename = path:match("([^/\\]+)$") or path
                            if basename ~= path then
                                full = __scripts_dir .. "/" .. basename
                                f = io.open(full, "r")
                                if f then
                                    f:close()
                                    return original_dofile(full)
                                end
                            end
                            return original_dofile(path)
                        end
                    "#).exec();
                    if let Err(e) = custom_dofile {
                        tracing::warn!("Failed to override dofile: {}", e);
                    }
                }
                self.dofile_initialized.set(true);
            }
            
            // 執行腳本
            if !code.is_empty() {
                self.lua.load(code).exec()?;
            }
            
            // 如果有 eval_code，執行並獲取結果
            let mut eval_res_str = None;
            if let Some(ec) = eval_code {
                let lua_result: mlua::Value = self.lua.load(ec).eval()?;
                let json_value = Self::lua_value_to_json(&lua_result);
                eval_res_str = Some(serde_json::to_string(&json_value).unwrap_or_else(|_| "null".to_string()));
            }
            
            // 收集結果
            let mud: mlua::Table = self.lua.globals().get("mud")?;
            
            // 收集 gag 狀態
            context.gag = mud.get::<bool>("gag").unwrap_or(false);
            
            // 收集 commands
            if let Ok(cmds) = mud.get::<mlua::Table>("commands") {
                for pair in cmds.pairs::<i64, String>() {
                    if let Ok((_, cmd)) = pair {
                        context.commands.push(cmd);
                    }
                }
            }
            
            // 收集 variables 並持久化儲存
            if let Ok(vars) = mud.get::<mlua::Table>("variables") {
                let mut persistent = self.persistent_vars.borrow_mut();
                for pair in vars.pairs::<String, String>() {
                    if let Ok((k, v)) = pair {
                        persistent.insert(k.clone(), v.clone());
                        context.variables.insert(k, v);
                    }
                }
            }
            
            // 收集 echos
            if let Ok(echos) = mud.get::<mlua::Table>("echos") {
                for pair in echos.pairs::<i64, String>() {
                    if let Ok((_, text)) = pair {
                        context.echos.push(text);
                    }
                }
            }
            
            // 收集 window_outputs
            if let Ok(outputs) = mud.get::<mlua::Table>("window_outputs") {
                for pair in outputs.pairs::<i64, mlua::Table>() {
                    if let Ok((_, tbl)) = pair {
                        if let (Ok(name), Ok(text)) = (tbl.get::<String>(1), tbl.get::<String>(2)) {
                            context.window_outputs.push((name, text));
                        }
                    }
                }
            }
            
            // 收集 log_messages
            if let Ok(logs) = mud.get::<mlua::Table>("log_messages") {
                for pair in logs.pairs::<i64, String>() {
                    if let Ok((_, msg)) = pair {
                        context.log_messages.push(msg);
                    }
                }
            }

            // 收集 log_control
            if let Ok(ctrl) = mud.get::<mlua::Table>("_log_control") {
                if let Ok(action) = ctrl.get::<String>("action") {
                    if action == "start" {
                        if let Ok(path) = ctrl.get::<String>("path") {
                            context.log_control = Some(LogControl::Start(path));
                        }
                    } else if action == "stop" {
                         context.log_control = Some(LogControl::Stop);
                    }
                }
            }
            
            // 收集 timers
            if let Ok(timers) = mud.get::<mlua::Table>("timers") {
                for pair in timers.pairs::<i64, mlua::Table>() {
                    if let Ok((_, tbl)) = pair {
                        if let (Ok(delay_ms), Ok(code)) = (tbl.get::<u64>(1), tbl.get::<String>(2)) {
                            // 計時器觸發永遠不被視為回顯
                            context.timers.push((delay_ms, code));
                        }
                    }
                }
            }
            
            // 收集 trigger_updates
            if let Ok(updates) = mud.get::<mlua::Table>("trigger_updates") {
                for pair in updates.pairs::<i64, mlua::Table>() {
                    if let Ok((_, tbl)) = pair {
                        if let (Ok(name), Ok(enabled)) = (tbl.get::<String>(1), tbl.get::<bool>(2)) {
                            context.trigger_updates.push((name, enabled));
                        }
                    }
                }
            }

            // 收集 group_updates
            if let Ok(updates) = mud.get::<mlua::Table>("_group_updates") {
                for pair in updates.pairs::<i64, mlua::Table>() {
                    if let Ok((_, entry)) = pair {
                        if let (Ok(group), Ok(enabled)) = (entry.get::<String>(1), entry.get::<bool>(2)) {
                            context.group_updates.push((group, enabled));
                        }
                    }
                }
            }

            // 收集 response_collectors
            if let Ok(collectors) = mud.get::<mlua::Table>("response_collectors") {
                for pair in collectors.pairs::<i64, mlua::Table>() {
                    if let Ok((_, tbl)) = pair {
                        if let (Ok(cmd), Ok(callback)) = (tbl.get::<String>(1), tbl.get::<String>(2)) {
                            context.response_collectors.push((cmd, callback));
                        }
                    }
                }
            }

            // 收集 llm_requests
            if let Ok(requests) = mud.get::<mlua::Table>("llm_requests") {
                for pair in requests.pairs::<i64, mlua::Table>() {
                    if let Ok((_, tbl)) = pair {
                        if let (Ok(prompt), Ok(callback)) = (tbl.get::<String>(1), tbl.get::<String>(2)) {
                            let model = tbl.get::<String>(3).ok().filter(|s| !s.is_empty());
                            context.llm_requests.push(LlmRequest {
                                prompt,
                                callback_code: callback,
                                model,
                            });
                        }
                    }
                }
            }

            // 收集 event_registrations
            if let Ok(regs) = mud.get::<mlua::Table>("_event_registrations") {
                for pair in regs.pairs::<i64, mlua::Table>() {
                    if let Ok((_, entry)) = pair {
                        if let (Ok(name), Ok(code), Ok(priority), Ok(once)) = (
                            entry.get::<String>(1),
                            entry.get::<String>(2),
                            entry.get::<i32>(3),
                            entry.get::<bool>(4),
                        ) {
                            context.event_registrations.push((name, code, priority, once));
                        }
                    }
                }
            }

            // 收集 event_removals
            if let Ok(removals) = mud.get::<mlua::Table>("_event_removals") {
                for pair in removals.pairs::<i64, u64>() {
                    if let Ok((_, id)) = pair {
                        context.event_removals.push(id);
                    }
                }
            }

            // 收集 event_emissions
            if let Ok(emissions) = mud.get::<mlua::Table>("_event_emissions") {
                for pair in emissions.pairs::<i64, mlua::Table>() {
                    if let Ok((_, entry)) = pair {
                        if let Ok(name) = entry.get::<String>(1) {
                            let data = entry.get::<Option<String>>(2).unwrap_or(None);
                            context.event_emissions.push((name, data));
                        }
                    }
                }
            }

            // 收集 state machine definitions
            if let Ok(defs) = mud.get::<mlua::Table>("_sm_defs") {
                for pair in defs.pairs::<i64, mlua::Table>() {
                    if let Ok((_, entry)) = pair {
                        if let (Ok(name), Ok(initial), Ok(states_json), Ok(trans_json)) = (
                            entry.get::<String>(1), entry.get::<String>(2),
                            entry.get::<String>(3), entry.get::<String>(4),
                        ) {
                            context.state_machine_defs.push((name, initial, states_json, trans_json));
                        }
                    }
                }
            }

            // 收集 state machine transitions
            if let Ok(transitions) = mud.get::<mlua::Table>("_sm_transitions") {
                for pair in transitions.pairs::<i64, mlua::Table>() {
                    if let Ok((_, entry)) = pair {
                        if let (Ok(name), Ok(event)) = (entry.get::<String>(1), entry.get::<String>(2)) {
                            context.state_machine_transitions.push((name, event));
                        }
                    }
                }
            }

            // 收集 state machine resets
            if let Ok(resets) = mud.get::<mlua::Table>("_sm_resets") {
                for pair in resets.pairs::<i64, String>() {
                    if let Ok((_, name)) = pair {
                        context.state_machine_resets.push(name);
                    }
                }
            }

            // 收集 key_binding_updates
            if let Ok(updates) = mud.get::<mlua::Table>("_key_binding_updates") {
                for pair in updates.pairs::<i64, mlua::Table>() {
                    if let Ok((_, entry)) = pair {
                        if let Ok(key) = entry.get::<String>(1) {
                            let code: Option<String> = entry.get::<String>(2).ok();
                            context.key_binding_updates.push((key, code));
                        }
                    }
                }
            }

            Ok::<_, mlua::Error>(eval_res_str)
        })?;

        Ok((context, eval_result_str))
    }

    /// 驗證腳本語法
    pub fn validate(&self, code: &str) -> Result<(), ScriptError> {
        self.lua.load(code).into_function()?;
        Ok(())
    }

    /// 呼叫全域 Lua 鉤子函數
    pub fn invoke_hook(&self, hook_name: &str, arg: &str, clean_arg: &str, is_echo: bool) -> Result<Option<MudContext>, ScriptError> {
        // 驗證 hook_name 是合法的 Lua 識別符，防止程式碼注入
        if !hook_name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || hook_name.is_empty()
            || hook_name.as_bytes()[0].is_ascii_digit()
        {
            return Err(ScriptError::Lua(format!("invalid hook name: {}", hook_name)));
        }

        // 檢查函數是否存在
        if !self.lua.globals().contains_key(hook_name)? {
            return Ok(None);
        }

        // 執行呼叫
        self.lua.scope(|_scope| {
           Ok(())
        })?;

        let adapter_code = format!("if _G['{0}'] then _G['{0}'](message, clean_message, is_echo) end", hook_name);

        self.run_code(&adapter_code, None, arg, clean_arg, &[], is_echo).map(|(ctx, _)| Some(ctx))
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_engine_creation() {
        let engine = ScriptEngine::new();
        assert!(engine.validate("local x = 1").is_ok());
    }

    #[test]
    fn test_script_gag() {
        let engine = ScriptEngine::new();
        let result = engine
            .execute_inline(
                r#"
if string.find(message, "廣告") then
    mud.gag_message()
end
"#,
                "這是一則廣告",
                &[],
                false,
            )
            .unwrap();

        assert!(result.gag);
    }

    #[test]
    fn test_script_validation() {
        let engine = ScriptEngine::new();

        assert!(engine.validate("local x = 1 + 2").is_ok());
        assert!(engine.validate("function broken(").is_err());
    }

    #[test]
    fn test_persistent_vars_across_calls() {
        let engine = ScriptEngine::new();

        // First call: set a variable
        let ctx1 = engine.execute_inline(
            r#"mud.variables.hp = "100""#,
            "", &[], false,
        ).unwrap();
        assert_eq!(ctx1.variables.get("hp").unwrap(), "100");

        // Second call: variable should persist
        let ctx2 = engine.execute_inline(
            r#"mud.send(mud.variables.hp)"#,
            "", &[], false,
        ).unwrap();
        assert_eq!(ctx2.commands, vec!["100"]);
    }

    #[test]
    fn test_expand_variables() {
        let engine = ScriptEngine::new();

        // Set variables
        engine.execute_inline(
            r#"mud.variables.target = "dragon"; mud.variables.target_id = "mob123""#,
            "", &[], false,
        ).unwrap();

        // Longer key should be replaced first ($target_id before $target)
        let result = engine.expand_variables("kill $target_id then $target");
        assert_eq!(result, "kill mob123 then dragon");
    }

    #[test]
    fn test_dofile_setup_cached() {
        let mut engine = ScriptEngine::new();
        engine.set_scripts_dir("/tmp/test_scripts");

        // Execute twice — should not fail even with missing scripts_dir
        let r1 = engine.execute_inline("local x = 1", "", &[], false);
        assert!(r1.is_ok());
        let r2 = engine.execute_inline("local y = 2", "", &[], false);
        assert!(r2.is_ok());
    }

    #[test]
    fn test_multiple_rapid_executions() {
        let engine = ScriptEngine::new();

        // Simulate rapid trigger firing
        for i in 0..100 {
            let code = format!(r#"mud.send("cmd_{}")"#, i);
            let ctx = engine.execute_inline(&code, "test message", &[], false).unwrap();
            assert_eq!(ctx.commands.len(), 1);
            assert_eq!(ctx.commands[0], format!("cmd_{}", i));
        }
    }

    #[test]
    fn test_invoke_hook() {
        let engine = ScriptEngine::new();

        // Define a hook
        engine.execute_inline(
            r#"function on_server_message(msg, clean, is_echo)
                if string.find(clean, "gold") then
                    mud.send("count gold")
                end
            end"#,
            "", &[], false,
        ).unwrap();

        // Invoke hook
        let result = engine.invoke_hook("on_server_message", "You got gold!", "You got gold!", false).unwrap();
        assert!(result.is_some());
        let ctx = result.unwrap();
        assert_eq!(ctx.commands, vec!["count gold"]);

        // Non-matching hook
        let result2 = engine.invoke_hook("on_server_message", "hello", "hello", false).unwrap();
        assert!(result2.is_some());
        let ctx2 = result2.unwrap();
        assert!(ctx2.commands.is_empty());

        // Non-existent hook
        let result3 = engine.invoke_hook("on_nonexistent", "", "", false).unwrap();
        assert!(result3.is_none());
    }

    #[test]
    fn test_event_api() {
        let engine = ScriptEngine::new();
        let result = engine.execute_inline(
            r#"
            mud.on("combat_end", "mud.send('loot')")
            mud.once("connected", "mud.send('look')", 10)
            mud.emit("custom_event", {key = "value"})
            mud.off(1)
            "#,
            "",
            &[],
            false,
        ).unwrap();

        assert_eq!(result.event_registrations.len(), 2);
        assert_eq!(result.event_registrations[0].0, "combat_end");
        assert!(!result.event_registrations[0].3); // not once
        assert_eq!(result.event_registrations[1].0, "connected");
        assert!(result.event_registrations[1].3); // once
        assert_eq!(result.event_registrations[1].2, 10); // priority
        assert_eq!(result.event_removals.len(), 1);
        assert_eq!(result.event_emissions.len(), 1);
        assert_eq!(result.event_emissions[0].0, "custom_event");
    }
}
