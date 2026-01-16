//! Python 腳本支援模組
//!
//! 使用 PyO3 整合 Python 腳本引擎

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;
use thiserror::Error;

/// 腳本執行錯誤
#[derive(Debug, Error)]
pub enum ScriptError {
    #[error("Python 錯誤: {0}")]
    Python(String),
    
    #[error("腳本未找到: {0}")]
    NotFound(String),
}

impl From<PyErr> for ScriptError {
    fn from(err: PyErr) -> Self {
        ScriptError::Python(err.to_string())
    }
}

/// MUD 腳本上下文（提供給 Python 腳本的 API）
#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct MudContext {
    /// 待發送的命令隊列
    #[pyo3(get)]
    pub commands: Vec<String>,
    
    /// 變數存儲
    #[pyo3(get)]
    pub variables: HashMap<String, String>,
    
    /// 是否應該抑制當前訊息
    #[pyo3(get, set)]
    pub gag: bool,
}

#[pymethods]
impl MudContext {
    #[new]
    fn new() -> Self {
        Self::default()
    }

    /// 發送命令到 MUD
    fn send(&mut self, command: &str) {
        self.commands.push(command.to_string());
    }

    /// 發送多個命令
    fn send_all(&mut self, commands: Vec<String>) {
        self.commands.extend(commands);
    }

    /// 設置變數
    fn set_var(&mut self, key: &str, value: &str) {
        self.variables.insert(key.to_string(), value.to_string());
    }

    /// 獲取變數
    fn get_var(&self, key: &str) -> Option<String> {
        self.variables.get(key).cloned()
    }

    /// 抑制當前訊息顯示
    fn gag_message(&mut self) {
        self.gag = true;
    }

    /// 輸出到日誌
    fn log(&self, message: &str) {
        tracing::info!("[Script] {}", message);
    }
}

/// Python 腳本引擎
pub struct ScriptEngine {
    /// 已載入的腳本
    scripts: HashMap<String, String>,
}

impl ScriptEngine {
    /// 創建新的腳本引擎
    pub fn new() -> Self {
        Self {
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

    /// 運行 Python 代碼
    fn run_code(
        &self,
        code: &str,
        message: &str,
        captures: &[String],
    ) -> Result<MudContext, ScriptError> {
        Python::with_gil(|py| {
            // 創建上下文
            let context = MudContext::new();
            let context_obj = Py::new(py, context)?;

            // 準備全局變數
            let globals = PyDict::new(py);
            globals.set_item("mud", context_obj.clone_ref(py))?;
            globals.set_item("message", message)?;
            
            // 將 captures 轉換為 Python list
            let captures_list = PyList::new(py, captures)?;
            globals.set_item("captures", &captures_list)?;

            // 添加 builtins
            let builtins = py.import("builtins")?;
            globals.set_item("__builtins__", &builtins)?;

            // 使用 eval 調用內建的 exec 函數
            let exec_func = builtins.getattr("exec")?;
            let compile_func = builtins.getattr("compile")?;
            
            // 編譯並執行代碼
            let code_obj = compile_func.call1((code, "<script>", "exec"))?;
            exec_func.call1((code_obj, &globals))?;

            // 提取結果
            let result: MudContext = context_obj.extract(py)?;
            Ok(result)
        })
    }

    /// 驗證腳本語法
    pub fn validate(&self, code: &str) -> Result<(), ScriptError> {
        Python::with_gil(|py| {
            let builtins = py.import("builtins")?;
            let compile_func = builtins.getattr("compile")?;
            compile_func.call1((code, "<script>", "exec"))?;
            Ok(())
        })
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
    fn test_script_send_command() {
        let engine = ScriptEngine::new();
        let result = engine
            .execute_inline(
                r#"
mud.send("kill kobold")
mud.send("loot")
"#,
                "test message",
                &[],
            )
            .unwrap();

        assert_eq!(result.commands, vec!["kill kobold", "loot"]);
    }

    #[test]
    fn test_script_with_captures() {
        let engine = ScriptEngine::new();
        let result = engine
            .execute_inline(
                r#"
if captures:
    mud.send(f"echo Got {captures[0]} gold")
"#,
                "你獲得了 100 金幣",
                &["100".to_string()],
            )
            .unwrap();

        assert_eq!(result.commands, vec!["echo Got 100 gold"]);
    }

    #[test]
    fn test_script_variables() {
        let engine = ScriptEngine::new();
        let result = engine
            .execute_inline(
                r#"
mud.set_var("hp", "100")
mud.set_var("mp", "50")
"#,
                "",
                &[],
            )
            .unwrap();

        assert_eq!(result.variables.get("hp"), Some(&"100".to_string()));
        assert_eq!(result.variables.get("mp"), Some(&"50".to_string()));
    }

    #[test]
    fn test_script_gag() {
        let engine = ScriptEngine::new();
        let result = engine
            .execute_inline(
                r#"
if "廣告" in message:
    mud.gag_message()
"#,
                "這是一則廣告",
                &[],
            )
            .unwrap();

        assert!(result.gag);
    }

    #[test]
    fn test_named_script() {
        let mut engine = ScriptEngine::new();
        engine.load_script("auto_loot", r#"mud.send("get all")"#);

        let result = engine.execute("auto_loot", "", &[]).unwrap();
        assert_eq!(result.commands, vec!["get all"]);
    }

    #[test]
    fn test_script_validation() {
        let engine = ScriptEngine::new();
        
        // 有效語法
        assert!(engine.validate("x = 1 + 2").is_ok());
        
        // 無效語法
        assert!(engine.validate("def broken(").is_err());
    }
}
