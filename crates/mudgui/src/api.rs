//! HTTP API 模組
//!
//! 提供 REST API 讓外部程式（如 MCP Server）操控 MUD client。
//! 監聽 `127.0.0.1:9527`，僅限本機存取。

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

// ============================================================================
// 共享狀態
// ============================================================================

/// API 共享狀態，透過 Arc<Mutex<>> 在 HTTP server 和 GUI 之間共享
#[derive(Debug)]
pub struct ApiState {
    /// 最近的訊息（純文字，已去除 ANSI）
    pub recent_messages: VecDeque<String>,
    /// 最大訊息數量
    pub max_messages: usize,
    /// 當前房間資訊
    pub current_room: Option<RoomInfo>,
    /// 連線狀態描述
    pub connection_status: String,
    /// 當前 Session 名稱
    pub session_name: String,
    /// 待發送的指令佇列（由 HTTP handler 寫入，GUI 讀取執行）
    pub pending_commands: VecDeque<String>,
    /// 待執行的 Lua 程式碼佇列
    pub pending_lua: VecDeque<String>,
}

/// 房間資訊
#[derive(Debug, Clone, Serialize)]
pub struct RoomInfo {
    pub name: String,
    pub exits: Vec<String>,
    pub room_id: Option<String>,
    pub description: String,
}

impl ApiState {
    pub fn new() -> Self {
        Self {
            recent_messages: VecDeque::with_capacity(200),
            max_messages: 200,
            current_room: None,
            connection_status: "disconnected".to_string(),
            session_name: String::new(),
            pending_commands: VecDeque::new(),
            pending_lua: VecDeque::new(),
        }
    }

    /// 追加一行訊息（GUI 呼叫）
    pub fn push_message(&mut self, msg: String) {
        if self.recent_messages.len() >= self.max_messages {
            self.recent_messages.pop_front();
        }
        self.recent_messages.push_back(msg);
    }

    /// 取出所有待發送的指令（GUI 呼叫）
    pub fn drain_commands(&mut self) -> Vec<String> {
        self.pending_commands.drain(..).collect()
    }

    /// 取出所有待執行的 Lua（GUI 呼叫）
    pub fn drain_lua(&mut self) -> Vec<String> {
        self.pending_lua.drain(..).collect()
    }
}

pub type SharedApiState = Arc<Mutex<ApiState>>;

// ============================================================================
// API 請求/回應結構
// ============================================================================

#[derive(Deserialize)]
pub struct MessagesQuery {
    count: Option<usize>,
}

#[derive(Serialize)]
struct StatusResponse {
    status: String,
    session_name: String,
    connected: bool,
    api_version: String,
}

#[derive(Serialize)]
struct MessagesResponse {
    messages: Vec<String>,
    total: usize,
}

#[derive(Deserialize)]
struct SendRequest {
    command: String,
}

#[derive(Deserialize)]
struct LuaRequest {
    code: String,
}

#[derive(Serialize)]
struct OkResponse {
    ok: bool,
    message: String,
}

// ============================================================================
// HTTP Handlers
// ============================================================================

async fn health() -> &'static str {
    "mudclient-rs API v1"
}

async fn get_status(State(state): State<SharedApiState>) -> Json<StatusResponse> {
    let s = state.lock().unwrap();
    let connected = s.connection_status.contains("connected")
        || s.connection_status.contains("Connected");
    Json(StatusResponse {
        status: s.connection_status.clone(),
        session_name: s.session_name.clone(),
        connected,
        api_version: "1.0".to_string(),
    })
}

async fn get_messages(
    State(state): State<SharedApiState>,
    Query(params): Query<MessagesQuery>,
) -> Json<MessagesResponse> {
    let s = state.lock().unwrap();
    let count = params.count.unwrap_or(50).min(s.recent_messages.len());
    let start = s.recent_messages.len().saturating_sub(count);
    let messages: Vec<String> = s.recent_messages.iter().skip(start).cloned().collect();
    Json(MessagesResponse {
        total: s.recent_messages.len(),
        messages,
    })
}

async fn get_room(State(state): State<SharedApiState>) -> Json<serde_json::Value> {
    let s = state.lock().unwrap();
    match &s.current_room {
        Some(room) => Json(serde_json::json!({
            "found": true,
            "name": room.name,
            "exits": room.exits,
            "room_id": room.room_id,
            "description": room.description,
        })),
        None => Json(serde_json::json!({
            "found": false,
        })),
    }
}

async fn send_command(
    State(state): State<SharedApiState>,
    Json(payload): Json<SendRequest>,
) -> (StatusCode, Json<OkResponse>) {
    let mut s = state.lock().unwrap();
    s.pending_commands.push_back(payload.command.clone());
    (
        StatusCode::OK,
        Json(OkResponse {
            ok: true,
            message: format!("Queued command: {}", payload.command),
        }),
    )
}

async fn execute_lua(
    State(state): State<SharedApiState>,
    Json(payload): Json<LuaRequest>,
) -> (StatusCode, Json<OkResponse>) {
    let mut s = state.lock().unwrap();
    s.pending_lua.push_back(payload.code.clone());
    (
        StatusCode::OK,
        Json(OkResponse {
            ok: true,
            message: "Queued Lua code for execution".to_string(),
        }),
    )
}

// ============================================================================
// 啟動 API Server
// ============================================================================

/// 啟動 HTTP API Server（應在 tokio runtime 上 spawn）
pub fn create_api_router(state: SharedApiState) -> Router {
    Router::new()
        .route("/", get(health))
        .route("/api/status", get(get_status))
        .route("/api/messages", get(get_messages))
        .route("/api/room", get(get_room))
        .route("/api/send", post(send_command))
        .route("/api/lua", post(execute_lua))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// 在背景啟動 API server
pub fn start_api_server(runtime: &tokio::runtime::Runtime, state: SharedApiState) {
    let router = create_api_router(state);

    runtime.spawn(async move {
        let listener = match tokio::net::TcpListener::bind("127.0.0.1:9527").await {
            Ok(l) => {
                tracing::info!("🌐 API Server 啟動於 http://127.0.0.1:9527");
                l
            }
            Err(e) => {
                tracing::error!("❌ API Server 啟動失敗: {}", e);
                return;
            }
        };

        if let Err(e) = axum::serve(listener, router).await {
            tracing::error!("API Server 錯誤: {}", e);
        }
    });
}
