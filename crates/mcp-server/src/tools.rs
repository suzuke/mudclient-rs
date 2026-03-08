use rmcp::{
    ServerHandler, tool, tool_handler, tool_router,
    handler::server::{wrapper::Parameters, tool::ToolRouter},
    model::{ServerInfo, ServerCapabilities},
    schemars,
};
use serde::Deserialize;

use crate::client::MudClient;

// ============================================================================
// Request types for tool parameters
// ============================================================================

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct SessionParam {
    #[schemars(description = "Session key (optional, defaults to active session)")]
    pub session: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct ReadMessagesParam {
    #[schemars(description = "Number of messages to retrieve (default 50)")]
    pub count: Option<usize>,
    #[schemars(description = "Session key (optional)")]
    pub session: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SendCommandParam {
    #[schemars(description = "The MUD command to send")]
    pub command: String,
    #[schemars(description = "Session key (optional)")]
    pub session: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LuaParam {
    #[schemars(description = "Lua code to execute")]
    pub code: String,
    #[schemars(description = "Session key (optional)")]
    pub session: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchMapParam {
    #[schemars(description = "Room name or ID to search for")]
    pub query: String,
    #[schemars(description = "Session key (optional)")]
    pub session: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FindPathParam {
    #[schemars(description = "Source room ID or name, use \"current\" for current location")]
    pub from: String,
    #[schemars(description = "Destination room ID or name")]
    pub to: String,
    #[schemars(description = "Session key (optional)")]
    pub session: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ManageAliasParam {
    #[schemars(description = "Action: add, remove, or toggle")]
    pub action: String,
    #[schemars(description = "Alias name")]
    pub name: String,
    #[schemars(description = "Pattern to match (required for add)")]
    pub pattern: Option<String>,
    #[schemars(description = "Replacement text or Lua script (required for add)")]
    pub replacement: Option<String>,
    #[schemars(description = "Whether replacement is a Lua script")]
    pub is_script: Option<bool>,
    #[schemars(description = "Session key (optional)")]
    pub session: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ManageTriggerParam {
    #[schemars(description = "Action: add, remove, or toggle")]
    pub action: String,
    #[schemars(description = "Trigger name")]
    pub name: String,
    #[schemars(description = "Pattern to match (required for add)")]
    pub pattern: Option<String>,
    #[schemars(description = "Pattern type: contains, startswith, endswith, regex")]
    pub pattern_type: Option<String>,
    #[schemars(description = "Lua script to execute when triggered")]
    pub script: Option<String>,
    #[schemars(description = "Session key (optional)")]
    pub session: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReadWindowParam {
    #[schemars(description = "Window ID to read")]
    pub id: String,
    #[schemars(description = "Number of messages to retrieve (default 50)")]
    pub count: Option<usize>,
    #[schemars(description = "Session key (optional)")]
    pub session: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct HistoryParam {
    #[schemars(description = "Number of history entries to retrieve (default 50)")]
    pub count: Option<usize>,
    #[schemars(description = "Session key (optional)")]
    pub session: Option<String>,
}

// ============================================================================
// MCP Server with 19 tools
// ============================================================================

pub struct MudMcpServer {
    client: std::sync::Arc<MudClient>,
    tool_router: ToolRouter<Self>,
}

impl MudMcpServer {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: std::sync::Arc::new(MudClient::new(base_url)),
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl MudMcpServer {
    // --- Existing 8 tools (ported from TypeScript) ---

    #[tool(description = "List all active MUD sessions")]
    async fn list_sessions(&self, Parameters(_p): Parameters<SessionParam>) -> String {
        match self.client.get("/api/sessions", &None).await {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
            Err(e) => e,
        }
    }

    #[tool(description = "Get current session status including connection state")]
    async fn get_status(&self, Parameters(p): Parameters<SessionParam>) -> String {
        match self.client.get("/api/status", &p.session).await {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
            Err(e) => e,
        }
    }

    #[tool(description = "Read recent messages from the MUD session")]
    async fn read_messages(&self, Parameters(p): Parameters<ReadMessagesParam>) -> String {
        let count = p.count.unwrap_or(50);
        let mut params = vec![("count", count.to_string())];
        if let Some(s) = &p.session {
            params.push(("session", s.clone()));
        }
        match self.client.get_with_params("/api/messages", &params).await {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
            Err(e) => e,
        }
    }

    #[tool(description = "Clear all messages in the session buffer")]
    async fn clear_messages(&self, Parameters(p): Parameters<SessionParam>) -> String {
        match self.client.delete("/api/messages", &p.session).await {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
            Err(e) => e,
        }
    }

    #[tool(description = "Get current room information (name, exits, description)")]
    async fn get_room_info(&self, Parameters(p): Parameters<SessionParam>) -> String {
        match self.client.get("/api/room", &p.session).await {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
            Err(e) => e,
        }
    }

    #[tool(description = "Send a command to the MUD server")]
    async fn send_command(&self, Parameters(p): Parameters<SendCommandParam>) -> String {
        match self.client.post("/api/send", &serde_json::json!({ "command": p.command })).await {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
            Err(e) => e,
        }
    }

    #[tool(description = "Execute Lua code (fire-and-forget, no return value)")]
    async fn execute_lua(&self, Parameters(p): Parameters<LuaParam>) -> String {
        match self.client.post("/api/lua", &serde_json::json!({ "code": p.code })).await {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
            Err(e) => e,
        }
    }

    #[tool(description = "Evaluate Lua code and return its result")]
    async fn evaluate_lua(&self, Parameters(p): Parameters<LuaParam>) -> String {
        match self.client.post("/api/evaluate", &serde_json::json!({ "code": p.code })).await {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
            Err(e) => e,
        }
    }

    // --- New map tools (3) ---

    #[tool(description = "Get map statistics: room count, edge count, enabled state")]
    async fn get_map_stats(&self, Parameters(p): Parameters<SessionParam>) -> String {
        match self.client.get("/api/map/stats", &p.session).await {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
            Err(e) => e,
        }
    }

    #[tool(description = "Search map rooms by name or ID")]
    async fn search_map_rooms(&self, Parameters(p): Parameters<SearchMapParam>) -> String {
        let mut params = vec![("query", p.query)];
        if let Some(s) = &p.session {
            params.push(("session", s.clone()));
        }
        match self.client.get_with_params("/api/map/search", &params).await {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
            Err(e) => e,
        }
    }

    #[tool(description = "Find path between two rooms using BFS. Use \"current\" as source for current location.")]
    async fn find_map_path(&self, Parameters(p): Parameters<FindPathParam>) -> String {
        let mut body = serde_json::json!({ "from": p.from, "to": p.to });
        if let Some(s) = &p.session {
            body["session"] = serde_json::json!(s);
        }
        match self.client.post("/api/map/path", &body).await {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
            Err(e) => e,
        }
    }

    // --- Management tools (5) ---

    #[tool(description = "List all configured aliases")]
    async fn list_aliases(&self, Parameters(p): Parameters<SessionParam>) -> String {
        match self.client.get("/api/aliases", &p.session).await {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
            Err(e) => e,
        }
    }

    #[tool(description = "List all configured triggers")]
    async fn list_triggers(&self, Parameters(p): Parameters<SessionParam>) -> String {
        match self.client.get("/api/triggers", &p.session).await {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
            Err(e) => e,
        }
    }

    #[tool(description = "List all configured speedwalk paths")]
    async fn list_paths(&self, Parameters(p): Parameters<SessionParam>) -> String {
        match self.client.get("/api/paths", &p.session).await {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
            Err(e) => e,
        }
    }

    #[tool(description = "Add, remove, or toggle an alias. Action: add/remove/toggle")]
    async fn manage_alias(&self, Parameters(p): Parameters<ManageAliasParam>) -> String {
        let body = serde_json::json!({
            "action": p.action,
            "name": p.name,
            "pattern": p.pattern,
            "replacement": p.replacement,
            "is_script": p.is_script,
            "session": p.session,
        });
        match self.client.post("/api/alias", &body).await {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
            Err(e) => e,
        }
    }

    #[tool(description = "Add, remove, or toggle a trigger. Action: add/remove/toggle")]
    async fn manage_trigger(&self, Parameters(p): Parameters<ManageTriggerParam>) -> String {
        let body = serde_json::json!({
            "action": p.action,
            "name": p.name,
            "pattern": p.pattern,
            "pattern_type": p.pattern_type,
            "script": p.script,
            "session": p.session,
        });
        match self.client.post("/api/trigger", &body).await {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
            Err(e) => e,
        }
    }

    // --- Window/History tools (3) ---

    #[tool(description = "List all sub-windows")]
    async fn list_windows(&self, Parameters(p): Parameters<SessionParam>) -> String {
        match self.client.get("/api/windows", &p.session).await {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
            Err(e) => e,
        }
    }

    #[tool(description = "Read messages from a specific sub-window")]
    async fn read_window(&self, Parameters(p): Parameters<ReadWindowParam>) -> String {
        let count = p.count.unwrap_or(50);
        let path = format!("/api/window/{}", p.id);
        let mut params = vec![("count", count.to_string())];
        if let Some(s) = &p.session {
            params.push(("session", s.clone()));
        }
        match self.client.get_with_params(&path, &params).await {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
            Err(e) => e,
        }
    }

    #[tool(description = "Get command input history")]
    async fn get_command_history(&self, Parameters(p): Parameters<HistoryParam>) -> String {
        let count = p.count.unwrap_or(50);
        let mut params = vec![("count", count.to_string())];
        if let Some(s) = &p.session {
            params.push(("session", s.clone()));
        }
        match self.client.get_with_params("/api/history", &params).await {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
            Err(e) => e,
        }
    }
}

#[tool_handler]
impl ServerHandler for MudMcpServer {
    fn get_info(&self) -> ServerInfo {
        let capabilities = ServerCapabilities::builder()
            .enable_tools()
            .build();
        let mut info = ServerInfo::default();
        info.capabilities = capabilities;
        info.instructions = Some("MUD Client MCP Server — control a running MUD client via 19 tools".into());
        info
    }
}
