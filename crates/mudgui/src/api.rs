//! HTTP API 模組
//!
//! 提供 REST API 讓外部程式（如 MCP Server）操控 MUD client。
//! 監聽 `127.0.0.1:9527`，僅限本機存取。
//!
//! 支援多 Session：所有端點接受 `?session=<key>` 參數指定 Session，
//! 不指定則使用當前活動的 Session。

use std::collections::{HashMap, VecDeque};
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
// Per-Session 共享狀態
// ============================================================================

/// 單一 Session 的 API 狀態
#[derive(Debug)]
pub struct ApiState {
    /// Session 的唯一 key（由 SessionId 產生）
    pub session_key: String,
    /// Session 的顯示名稱
    pub display_name: String,
    /// 最近的訊息（純文字，已去除 ANSI）
    pub recent_messages: VecDeque<String>,
    /// 最大訊息數量
    pub max_messages: usize,
    /// 當前房間資訊
    pub current_room: Option<RoomInfo>,
    /// 連線狀態描述
    pub connection_status: String,
    /// 待發送的指令佇列（由 HTTP handler 寫入，GUI 讀取執行）
    pub pending_commands: VecDeque<String>,
    /// 待執行的 Lua 程式碼佇列
    pub pending_lua: VecDeque<String>,
    /// 待評估的 Lua 程式碼佇列 (附帶 oneshot channel 用於回傳結果)
    pub pending_eval_lua: VecDeque<(String, tokio::sync::oneshot::Sender<String>)>,
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
    pub fn new(session_key: String, display_name: String) -> Self {
        Self {
            session_key,
            display_name,
            recent_messages: VecDeque::with_capacity(200),
            max_messages: 200,
            current_room: None,
            connection_status: "disconnected".to_string(),
            pending_commands: VecDeque::new(),
            pending_lua: VecDeque::new(),
            pending_eval_lua: VecDeque::new(),
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

    /// 取出所有待評估的 Lua（GUI 呼叫）
    pub fn drain_eval_lua(&mut self) -> Vec<(String, tokio::sync::oneshot::Sender<String>)> {
        self.pending_eval_lua.drain(..).collect()
    }
}

pub type SharedApiState = Arc<Mutex<ApiState>>;

// ============================================================================
// ApiStateManager — 管理所有 Session 的 API 狀態
// ============================================================================

/// 管理多個 Session 的 API 狀態
#[derive(Debug, Clone)]
pub struct ApiStateManager {
    inner: Arc<Mutex<ApiStateManagerInner>>,
}

#[derive(Debug)]
struct ApiStateManagerInner {
    /// session_key → SharedApiState
    states: HashMap<String, SharedApiState>,
    /// 目前活動的 session key
    active_key: Option<String>,
    /// session_key 的插入順序（用於列舉）
    order: Vec<String>,
}

impl ApiStateManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ApiStateManagerInner {
                states: HashMap::new(),
                active_key: None,
                order: Vec::new(),
            })),
        }
    }

    /// 註冊一個新的 Session，回傳其專屬的 SharedApiState
    pub fn register_session(&self, session_key: &str, display_name: &str) -> SharedApiState {
        let mut inner = self.inner.lock().expect("API state manager lock poisoned");
        if let Some(existing) = inner.states.get(session_key) {
            return existing.clone();
        }
        let state = Arc::new(Mutex::new(ApiState::new(
            session_key.to_string(),
            display_name.to_string(),
        )));
        inner.states.insert(session_key.to_string(), state.clone());
        inner.order.push(session_key.to_string());
        // 第一個註冊的自動成為 active
        if inner.active_key.is_none() {
            inner.active_key = Some(session_key.to_string());
        }
        state
    }


    /// 設定當前活動的 Session
    pub fn set_active(&self, session_key: &str) {
        let mut inner = self.inner.lock().expect("API state manager lock poisoned");
        if inner.states.contains_key(session_key) {
            inner.active_key = Some(session_key.to_string());
        }
    }

    /// 取得指定 Session 的 ApiState（透過 key）
    pub fn get(&self, session_key: &str) -> Option<SharedApiState> {
        let inner = self.inner.lock().expect("API state manager lock poisoned");
        inner.states.get(session_key).cloned()
    }

    /// 取得當前活動 Session 的 ApiState
    /// 若 active_key 指向的 Session 已不存在，自動 fallback 到第一個可用的 Session
    pub fn get_active(&self) -> Option<SharedApiState> {
        let mut inner = self.inner.lock().expect("API state manager lock poisoned");
        // 先嘗試用 active_key
        if let Some(key) = &inner.active_key {
            if let Some(state) = inner.states.get(key) {
                return Some(state.clone());
            }
        }
        // active_key 失效 → fallback 到 order 中第一個仍存在的 session
        let fallback = inner.order.iter()
            .find_map(|k| inner.states.get(k).map(|s| (k.clone(), s.clone())));
        if let Some((key, state)) = fallback {
            tracing::warn!("API active_key 失效，自動切換至 session '{}'", key);
            inner.active_key = Some(key);
            Some(state)
        } else {
            None
        }
    }

    /// 列出所有 Session 資訊
    pub fn list_sessions(&self) -> Vec<SessionInfo> {
        let inner = self.inner.lock().expect("API state manager lock poisoned");
        inner.order.iter().filter_map(|key| {
            let state = inner.states.get(key)?;
            let s = state.lock().ok()?;
            Some(SessionInfo {
                session_key: key.clone(),
                display_name: s.display_name.clone(),
                status: s.connection_status.clone(),
                is_active: inner.active_key.as_deref() == Some(key.as_str()),
            })
        }).collect()
    }

    /// 取得所有 session key → SharedApiState 的映射（用於遍歷 drain）
    pub fn all_states(&self) -> Vec<(String, SharedApiState)> {
        let inner = self.inner.lock().expect("API state manager lock poisoned");
        inner.order.iter().filter_map(|key| {
            inner.states.get(key).map(|s| (key.clone(), s.clone()))
        }).collect()
    }

    /// 解析 session 參數：若指定了 ?session=key 則用該 session，否則用 active
    /// 不帶 session 時，若 active_key 失效會自動 fallback（由 get_active 處理）
    /// 明確指定不存在的 key 則回傳 None → 404
    fn resolve(&self, session_param: Option<&str>) -> Option<SharedApiState> {
        match session_param {
            Some(key) => self.get(key),
            None => self.get_active(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub session_key: String,
    pub display_name: String,
    pub status: String,
    pub is_active: bool,
}

// ============================================================================
// API 請求/回應結構
// ============================================================================

#[derive(Deserialize)]
pub struct MessagesQuery {
    count: Option<usize>,
    session: Option<String>,
}

#[derive(Deserialize)]
pub struct SessionQuery {
    session: Option<String>,
}

#[derive(Serialize)]
struct StatusResponse {
    status: String,
    session_name: String,
    session_key: String,
    connected: bool,
    api_version: String,
}

#[derive(Serialize)]
struct MessagesResponse {
    messages: Vec<String>,
    total: usize,
    session_key: String,
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

#[derive(Serialize)]
struct SessionsResponse {
    sessions: Vec<SessionInfo>,
}

// ============================================================================
// HTTP Handlers
// ============================================================================

async fn health() -> &'static str {
    "mudclient-rs API v2 (multi-session)"
}

async fn get_sessions(
    State(mgr): State<ApiStateManager>,
) -> Json<SessionsResponse> {
    Json(SessionsResponse {
        sessions: mgr.list_sessions(),
    })
}

async fn get_status(
    State(mgr): State<ApiStateManager>,
    Query(params): Query<SessionQuery>,
) -> Result<Json<StatusResponse>, StatusCode> {
    let state = mgr.resolve(params.session.as_deref())
        .ok_or(StatusCode::NOT_FOUND)?;
    let s = state.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let connected = s.connection_status.contains("connected")
        || s.connection_status.contains("Connected");
    Ok(Json(StatusResponse {
        status: s.connection_status.clone(),
        session_name: s.display_name.clone(),
        session_key: s.session_key.clone(),
        connected,
        api_version: "2.0".to_string(),
    }))
}

async fn get_messages(
    State(mgr): State<ApiStateManager>,
    Query(params): Query<MessagesQuery>,
) -> Result<Json<MessagesResponse>, StatusCode> {
    let state = mgr.resolve(params.session.as_deref())
        .ok_or(StatusCode::NOT_FOUND)?;
    let s = state.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let count = params.count.unwrap_or(50).min(s.recent_messages.len());
    let start = s.recent_messages.len().saturating_sub(count);
    let messages: Vec<String> = s.recent_messages.iter().skip(start).cloned().collect();
    let session_key = s.session_key.clone();
    Ok(Json(MessagesResponse {
        total: s.recent_messages.len(),
        messages,
        session_key,
    }))
}

async fn delete_messages(
    State(mgr): State<ApiStateManager>,
    Query(params): Query<SessionQuery>,
) -> Result<(StatusCode, Json<OkResponse>), StatusCode> {
    let state = mgr.resolve(params.session.as_deref())
        .ok_or(StatusCode::NOT_FOUND)?;
    let mut s = state.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let num_cleared = s.recent_messages.len();
    s.recent_messages.clear();
    Ok((
        StatusCode::OK,
        Json(OkResponse {
            ok: true,
            message: format!("Cleared {} messages from session {}", num_cleared, s.session_key),
        }),
    ))
}

async fn get_room(
    State(mgr): State<ApiStateManager>,
    Query(params): Query<SessionQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let state = mgr.resolve(params.session.as_deref())
        .ok_or(StatusCode::NOT_FOUND)?;
    let s = state.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match &s.current_room {
        Some(room) => Ok(Json(serde_json::json!({
            "found": true,
            "name": room.name,
            "exits": room.exits,
            "room_id": room.room_id,
            "description": room.description,
            "session_key": s.session_key,
        }))),
        None => Ok(Json(serde_json::json!({
            "found": false,
            "session_key": s.session_key,
        }))),
    }
}

async fn send_command(
    State(mgr): State<ApiStateManager>,
    Query(params): Query<SessionQuery>,
    Json(payload): Json<SendRequest>,
) -> Result<(StatusCode, Json<OkResponse>), StatusCode> {
    let state = mgr.resolve(params.session.as_deref())
        .ok_or(StatusCode::NOT_FOUND)?;
    let mut s = state.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    s.pending_commands.push_back(payload.command.clone());
    Ok((
        StatusCode::OK,
        Json(OkResponse {
            ok: true,
            message: format!("Queued command: {}", payload.command),
        }),
    ))
}

async fn execute_lua(
    State(mgr): State<ApiStateManager>,
    Query(params): Query<SessionQuery>,
    Json(payload): Json<LuaRequest>,
) -> Result<(StatusCode, Json<OkResponse>), StatusCode> {
    let state = mgr.resolve(params.session.as_deref())
        .ok_or(StatusCode::NOT_FOUND)?;
    let mut s = state.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    s.pending_lua.push_back(payload.code.clone());
    Ok((
        StatusCode::OK,
        Json(OkResponse {
            ok: true,
            message: "Queued Lua code for execution".to_string(),
        }),
    ))
}

async fn evaluate_lua(
    State(mgr): State<ApiStateManager>,
    Query(params): Query<SessionQuery>,
    Json(payload): Json<LuaRequest>,
) -> Result<(StatusCode, Json<OkResponse>), StatusCode> {
    let state = mgr.resolve(params.session.as_deref())
        .ok_or(StatusCode::NOT_FOUND)?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    {
        let mut s = state.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        s.pending_eval_lua.push_back((payload.code.clone(), tx));
    }

    // Wait for the result with a timeout of 5 seconds
    match tokio::time::timeout(std::time::Duration::from_secs(5), rx).await {
        Ok(Ok(result)) => Ok((
            StatusCode::OK,
            Json(OkResponse {
                ok: true,
                message: result,
            }),
        )),
        Ok(Err(_)) => Ok((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(OkResponse {
                ok: false,
                message: "Failed to wait for Lua execution result".to_string(),
            }),
        )),
        Err(_) => Ok((
            StatusCode::REQUEST_TIMEOUT,
            Json(OkResponse {
                ok: false,
                message: "Timeout waiting for Lua execution".to_string(),
            }),
        )),
    }
}

// ============================================================================
// 啟動 API Server
// ============================================================================

/// 建立 API Router（使用 ApiStateManager）
pub fn create_api_router(mgr: ApiStateManager) -> Router {
    Router::new()
        .route("/", get(health))
        .route("/api/sessions", get(get_sessions))
        .route("/api/status", get(get_status))
        .route("/api/messages", get(get_messages))
        .route("/api/messages", axum::routing::delete(delete_messages))
        .route("/api/room", get(get_room))
        .route("/api/send", post(send_command))
        .route("/api/lua", post(execute_lua))
        .route("/api/evaluate", post(evaluate_lua))
        .layer(CorsLayer::permissive())
        .with_state(mgr)
}

/// 在背景啟動 API server
pub fn start_api_server(runtime: &tokio::runtime::Runtime, mgr: ApiStateManager) {
    let router = create_api_router(mgr);

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
