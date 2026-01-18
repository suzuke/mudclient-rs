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
}

impl ScriptEngine {
    /// 創建新的腳本引擎
    pub fn new() -> Self {
        let lua = Lua::new();
        Self {
            lua,
            scripts: HashMap::new(),
        }
    }

    /// 載入腳本
    pub fn load_script(&mut self, name: impl Into<String>, code: impl Into<String>) {
        self.scripts.insert(name.into(), code.into());
    }

    /// 移除腳本
    pub fn remove_script(&mut self, name: &str) -> bool {
        self.scripts.remove(name).is_some()
    }

    /// 執行腳本
    pub fn execute(
        &self,
        script_name: &str,
        message: &str,
        captures: &[String],
    ) -> Result<MudContext, ScriptError> {
        let code = self
            .scripts
            .get(script_name)
            .ok_or_else(|| ScriptError::NotFound(script_name.to_string()))?;

        self.run_code(code, message, captures)
    }

    /// 執行內聯代碼
    pub fn execute_inline(
        &self,
        code: &str,
        message: &str,
        captures: &[String],
    ) -> Result<MudContext, ScriptError> {
        self.run_code(code, message, captures)
    }

    /// 運行 Lua 代碼
    fn run_code(
        &self,
        code: &str,
        message: &str,
        captures: &[String],
    ) -> Result<MudContext, ScriptError> {
        let mut context = MudContext::new();

        self.lua.scope(|scope| {
            // 創建 mud 表用於存放 API
            let mud = self.lua.create_table()?;
            
            // 創建命令列表
            let commands = self.lua.create_table()?;
            mud.set("commands", commands)?;
            
            // 創建變數表
            let variables = self.lua.create_table()?;
            mud.set("variables", variables)?;
            
            // gag 標記
            mud.set("gag", false)?;
            
            // mud.send(command) 函數
            let send_fn = scope.create_function_mut(|_, _cmd: String| {
                // 這裡會在腳本結束後從 mud.commands 收集
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
            
            // mud.log(message) 函數
            let log_fn = scope.create_function(|_, msg: String| {
                tracing::info!("[Script] {}", msg);
                Ok(())
            })?;
            mud.set("log", log_fn)?;
            
            // 設置全局變數
            self.lua.globals().set("mud", mud)?;
            self.lua.globals().set("message", message)?;
            
            // 設置 captures 表
            let captures_table = self.lua.create_table()?;
            for (i, cap) in captures.iter().enumerate() {
                captures_table.set(i + 1, cap.as_str())?;
            }
            self.lua.globals().set("captures", captures_table)?;
            
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
            
            // 收集 variables
            if let Ok(vars) = mud.get::<mlua::Table>("variables") {
                for pair in vars.pairs::<String, String>() {
                    if let Ok((k, v)) = pair {
                        context.variables.insert(k, v);
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
