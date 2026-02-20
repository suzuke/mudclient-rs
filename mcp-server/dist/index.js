#!/usr/bin/env node
/**
 * MCP Server for mudclient-rs (v2 - Multi-Session)
 *
 * 透過 HTTP API 操控 MUD client，提供以下 tools：
 * - list_sessions: 列出所有 Session
 * - send_command: 發送指令到 MUD
 * - read_messages: 讀取最近的訊息
 * - clear_messages: 清空訊息緩衝區
 * - get_room_info: 取得當前房間資訊
 * - execute_lua: 執行 Lua 程式碼
 * - evaluate_lua: 執行 Lua 並回傳結果
 * - get_status: 取得連線狀態
 *
 * 所有工具（除 list_sessions）支援可選 session 參數，
 * 不指定時預設使用活動中的 Session。
 */
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
const API_BASE = process.env.MUDCLIENT_API_URL || "http://127.0.0.1:9527";
// ============================================================================
// HTTP Helper (session-aware)
// ============================================================================
function sessionQuery(session) {
    return session ? `?session=${encodeURIComponent(session)}` : "";
}
async function apiGet(path, session) {
    const url = `${API_BASE}${path}${path.includes("?") ? "&" : "?"}${session ? `session=${encodeURIComponent(session)}` : ""}`;
    const res = await fetch(url);
    if (!res.ok)
        throw new Error(`API error: ${res.status} ${res.statusText}`);
    return res.json();
}
async function apiPost(path, body, session) {
    const res = await fetch(`${API_BASE}${path}${sessionQuery(session)}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
    });
    if (!res.ok)
        throw new Error(`API error: ${res.status} ${res.statusText}`);
    return res.json();
}
async function apiDelete(path, session) {
    const res = await fetch(`${API_BASE}${path}${sessionQuery(session)}`, {
        method: "DELETE",
    });
    if (!res.ok)
        throw new Error(`API error: ${res.status} ${res.statusText}`);
    return res.json();
}
// Common error handler
function errorResult(e) {
    return {
        content: [{
                type: "text",
                text: `Error: ${e instanceof Error ? e.message : String(e)}`,
            }],
        isError: true,
    };
}
// Optional session schema (reusable)
const sessionParam = z.string().optional().describe("Session key（可選）。不指定時使用活動中的 Session。可透過 list_sessions 查詢所有 Session key。");
// ============================================================================
// MCP Server
// ============================================================================
const server = new McpServer({
    name: "mudclient-mcp",
    version: "2.0.0",
});
// --- Tool: list_sessions ---
server.tool("list_sessions", "列出所有可用的 Session，包含 session_key、顯示名稱、連線狀態、是否為活動 Session", {}, async () => {
    try {
        const data = await apiGet("/api/sessions");
        return {
            content: [{
                    type: "text",
                    text: JSON.stringify(data, null, 2),
                }],
        };
    }
    catch (e) {
        return errorResult(e);
    }
});
// --- Tool: get_status ---
server.tool("get_status", "取得 MUD client 的連線狀態和 Session 資訊", { session: sessionParam }, async ({ session }) => {
    try {
        const data = await apiGet("/api/status", session);
        return {
            content: [{
                    type: "text",
                    text: JSON.stringify(data, null, 2),
                }],
        };
    }
    catch (e) {
        return errorResult(e);
    }
});
// --- Tool: read_messages ---
server.tool("read_messages", "讀取 MUD 伺服器最近的訊息。用於觀察遊戲狀態、NPC 對話、戰鬥結果等", {
    count: z.number().min(1).max(200).default(20).describe("要讀取的訊息行數 (1-200)"),
    session: sessionParam,
}, async ({ count, session }) => {
    try {
        const qs = `count=${count}${session ? `&session=${encodeURIComponent(session)}` : ""}`;
        const data = await apiGet(`/api/messages?${qs}`);
        const text = data.messages.join("\n");
        return {
            content: [{
                    type: "text",
                    text: `[Session: ${data.session_key}, 共 ${data.total} 行，顯示最近 ${data.messages.length} 行]\n\n${text}`,
                }],
        };
    }
    catch (e) {
        return errorResult(e);
    }
});
// --- Tool: clear_messages ---
server.tool("clear_messages", "清空指定 Session 的訊息緩衝區。用於清除舊訊息，確保之後讀取到的都是最新狀態。", { session: sessionParam }, async ({ session }) => {
    try {
        const data = await apiDelete(`/api/messages`, session);
        return {
            content: [{ type: "text", text: data.message }],
        };
    }
    catch (e) {
        return errorResult(e);
    }
});
// --- Tool: get_room_info ---
server.tool("get_room_info", "取得當前房間的名稱、出口、Room ID 和描述", { session: sessionParam }, async ({ session }) => {
    try {
        const data = await apiGet("/api/room", session);
        return {
            content: [{
                    type: "text",
                    text: JSON.stringify(data, null, 2),
                }],
        };
    }
    catch (e) {
        return errorResult(e);
    }
});
// --- Tool: send_command ---
server.tool("send_command", "發送指令到 MUD 伺服器。可以是遊戲指令（如 look, north, kill mob）或客戶端指令（如 #alias, /lua）", {
    command: z.string().describe("要發送的指令，例如 'look' 或 'north'"),
    session: sessionParam,
}, async ({ command, session }) => {
    try {
        const data = await apiPost("/api/send", { command }, session);
        return {
            content: [{ type: "text", text: data.message }],
        };
    }
    catch (e) {
        return errorResult(e);
    }
});
// --- Tool: execute_lua ---
server.tool("execute_lua", "在 MUD client 的 Lua 環境中非同步執行程式碼（無回傳值）。可以使用 mud.send()、mud.echo() 等 API。如果需要回傳值，請改用 evaluate_lua。", {
    code: z.string().describe("Lua 程式碼，例如 mud.send('look') 或 mud.echo('hello')"),
    session: sessionParam,
}, async ({ code, session }) => {
    try {
        const data = await apiPost("/api/lua", { code }, session);
        return {
            content: [{ type: "text", text: data.message }],
        };
    }
    catch (e) {
        return errorResult(e);
    }
});
// --- Tool: evaluate_lua ---
server.tool("evaluate_lua", "在 MUD client 的 Lua 環境中執行程式碼並等待回傳結果。用於讀取遊戲狀態（例如：return mud.get_current_room_id()）。", {
    code: z.string().describe("要求回傳值的 Lua 程式碼，例如 'return mud.get_current_room_id()'"),
    session: sessionParam,
}, async ({ code, session }) => {
    try {
        const data = await apiPost("/api/evaluate", { code }, session);
        return {
            content: [{ type: "text", text: data.message }],
        };
    }
    catch (e) {
        return errorResult(e);
    }
});
// ============================================================================
// Start Server
// ============================================================================
async function main() {
    const transport = new StdioServerTransport();
    await server.connect(transport);
    console.error("mudclient-mcp server v2.0 started (multi-session, stdio mode)");
}
main().catch((e) => {
    console.error("Fatal error:", e);
    process.exit(1);
});
