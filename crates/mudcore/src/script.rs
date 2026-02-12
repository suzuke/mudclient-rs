//! Lua 腳本支援模組
//!
//! 使用 mlua 整合 Lua 腳本引擎

use mlua::Lua;
use std::collections::HashMap;
use thiserror::Error;

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
    
    /// 延遲執行的 Timer (delay_ms, lua_code)
    pub timers: Vec<(u64, String)>,
    
    /// 觸發器狀態更新 (name, enabled)
    pub trigger_updates: Vec<(String, bool)>,
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
    persistent_vars: std::cell::RefCell<HashMap<String, String>>,
    /// 腳本目錄的絕對路徑，用於 dofile 查找
    scripts_dir: Option<String>,
}

impl ScriptEngine {
    /// 創建新的腳本引擎
    pub fn new() -> Self {
        let lua = Lua::new();
        Self {
            lua,
            scripts: HashMap::new(),
            persistent_vars: std::cell::RefCell::new(HashMap::new()),
            scripts_dir: None,
        }
    }

    /// 設定腳本目錄路徑（供 dofile 查找）
    pub fn set_scripts_dir(&mut self, dir: impl Into<String>) {
        self.scripts_dir = Some(dir.into());
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
        let mut result = text.to_string();
        // 簡單的替換：尋找 $ 開頭的單字
        // TODO: 使用 regex 支援更複雜的變數名或 ${var} 格式
        let vars = self.persistent_vars.borrow();
        for (key, value) in vars.iter() {
            let placeholder = format!("${}", key);
            if result.contains(&placeholder) {
                result = result.replace(&placeholder, value);
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
        self.run_code(code, message, message, captures, is_echo)
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
        self.run_code(code, message, message, captures, is_echo)
    }

    /// 運行 Lua 代碼
    fn run_code(
        &self,
        code: &str,
        message: &str,
        clean_message: &str,
        captures: &[String],
        is_echo: bool,
    ) -> Result<MudContext, ScriptError> {
        let mut context = MudContext::new();

        self.lua.scope(|scope| {
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
            
            // gag 標記
            mud.set("gag", false)?;
            
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
            mud.set("echo", echo_fn)?;
            
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

            // 覆寫 dofile：支援從 scripts_dir 查找腳本
            if let Some(dir) = &self.scripts_dir {
                self.lua.globals().set("__scripts_dir", dir.as_str())?;
                let custom_dofile = self.lua.load(r#"
                    local original_dofile = dofile
                    function dofile(path)
                        -- 如果檔案已存在，直接執行
                        local f = io.open(path, "r")
                        if f then
                            f:close()
                            return original_dofile(path)
                        end
                        -- 嘗試從 scripts_dir 查找
                        local full = __scripts_dir .. "/" .. path
                        f = io.open(full, "r")
                        if f then
                            f:close()
                            return original_dofile(full)
                        end
                        -- 嘗試只用檔名 (basename)
                        local basename = path:match("([^/\\]+)$") or path
                        if basename ~= path then
                            full = __scripts_dir .. "/" .. basename
                            f = io.open(full, "r")
                            if f then
                                f:close()
                                return original_dofile(full)
                            end
                        end
                        -- 回退到原始 dofile（會拋出錯誤）
                        return original_dofile(path)
                    end
                "#).exec();
                if let Err(e) = custom_dofile {
                    tracing::warn!("Failed to override dofile: {}", e);
                }
            }
            
            // 執行腳本
            self.lua.load(code).exec()?;
            
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
            
            Ok::<_, mlua::Error>(())
        })?;

        Ok(context)
    }

    /// 驗證腳本語法
    pub fn validate(&self, code: &str) -> Result<(), ScriptError> {
        self.lua.load(code).into_function()?;
        Ok(())
    }

    /// 呼叫全域 Lua 鉤子函數
    pub fn invoke_hook(&self, hook_name: &str, arg: &str, clean_arg: &str) -> Result<Option<MudContext>, ScriptError> {
        // 檢查函數是否存在
        if !self.lua.globals().contains_key(hook_name)? {
            return Ok(None);
        }

        // 執行呼叫
        self.lua.scope(|_scope| {
           // 我們只是檢查是否存在，實際上呼叫是透過下面的 adapter_code
           // let _func: mlua::Function = self.lua.globals().get(hook_name)?;
           Ok(())
        })?;
        
        // 為了避免 lifetime 和 borrow 問題，我們使用 execute_inline 的既有路徑
        // 構造一段呼叫代碼
        let adapter_code = format!("if _G['{0}'] then _G['{0}'](message, clean_message) end", hook_name);
        // 注意：這裡我們依賴 execute_inline 將 message 注入到全局
        
        self.run_code(&adapter_code, arg, clean_arg, &[], false).map(Some)
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
        
        // 有效語法
        assert!(engine.validate("local x = 1 + 2").is_ok());
        
        // 無效語法
        assert!(engine.validate("function broken(").is_err());
    }
}
