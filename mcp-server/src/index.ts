#!/usr/bin/env node

/**
 * MCP Server for mudclient-rs
 *
 * 透過 HTTP API 操控 MUD client，提供以下 tools：
 * - send_command: 發送指令到 MUD
 * - read_messages: 讀取最近的訊息
 * - get_room_info: 取得當前房間資訊
 * - execute_lua: 執行 Lua 程式碼
 * - get_status: 取得連線狀態
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";

const API_BASE = process.env.MUDCLIENT_API_URL || "http://127.0.0.1:9527";

// ============================================================================
// HTTP Helper
// ============================================================================

async function apiGet(path: string): Promise<unknown> {
    const res = await fetch(`${API_BASE}${path}`);
    if (!res.ok) throw new Error(`API error: ${res.status} ${res.statusText}`);
    return res.json();
}

async function apiPost(path: string, body: unknown): Promise<unknown> {
    const res = await fetch(`${API_BASE}${path}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
    });
    if (!res.ok) throw new Error(`API error: ${res.status} ${res.statusText}`);
    return res.json();
}

// ============================================================================
// MCP Server
// ============================================================================

const server = new McpServer({
    name: "mudclient-mcp",
    version: "1.0.0",
});

// --- Tool: get_status ---
server.tool(
    "get_status",
    "取得 MUD client 的連線狀態和 Session 資訊",
    {},
    async () => {
        try {
            const data = await apiGet("/api/status") as Record<string, unknown>;
            return {
                content: [
                    {
                        type: "text" as const,
                        text: JSON.stringify(data, null, 2),
                    },
                ],
            };
        } catch (e) {
            return {
                content: [
                    {
                        type: "text" as const,
                        text: `Error: ${e instanceof Error ? e.message : String(e)}. Is mudclient-rs running?`,
                    },
                ],
                isError: true,
            };
        }
    }
);

// --- Tool: read_messages ---
server.tool(
    "read_messages",
    "讀取 MUD 伺服器最近的訊息。用於觀察遊戲狀態、NPC 對話、戰鬥結果等",
    {
        count: z
            .number()
            .min(1)
            .max(200)
            .default(20)
            .describe("要讀取的訊息行數 (1-200)"),
    },
    async ({ count }) => {
        try {
            const data = await apiGet(`/api/messages?count=${count}`) as {
                messages: string[];
                total: number;
            };
            const text = data.messages.join("\n");
            return {
                content: [
                    {
                        type: "text" as const,
                        text: `[共 ${data.total} 行，顯示最近 ${data.messages.length} 行]\n\n${text}`,
                    },
                ],
            };
        } catch (e) {
            return {
                content: [
                    {
                        type: "text" as const,
                        text: `Error: ${e instanceof Error ? e.message : String(e)}`,
                    },
                ],
                isError: true,
            };
        }
    }
);

// --- Tool: get_room_info ---
server.tool(
    "get_room_info",
    "取得當前房間的名稱、出口、Room ID 和描述",
    {},
    async () => {
        try {
            const data = await apiGet("/api/room") as Record<string, unknown>;
            return {
                content: [
                    {
                        type: "text" as const,
                        text: JSON.stringify(data, null, 2),
                    },
                ],
            };
        } catch (e) {
            return {
                content: [
                    {
                        type: "text" as const,
                        text: `Error: ${e instanceof Error ? e.message : String(e)}`,
                    },
                ],
                isError: true,
            };
        }
    }
);

// --- Tool: send_command ---
server.tool(
    "send_command",
    "發送指令到 MUD 伺服器。可以是遊戲指令（如 look, north, kill mob）或客戶端指令（如 #alias, /lua）",
    {
        command: z.string().describe("要發送的指令，例如 'look' 或 'north'"),
    },
    async ({ command }) => {
        try {
            const data = await apiPost("/api/send", { command }) as {
                ok: boolean;
                message: string;
            };
            return {
                content: [
                    {
                        type: "text" as const,
                        text: data.message,
                    },
                ],
            };
        } catch (e) {
            return {
                content: [
                    {
                        type: "text" as const,
                        text: `Error: ${e instanceof Error ? e.message : String(e)}`,
                    },
                ],
                isError: true,
            };
        }
    }
);

// --- Tool: execute_lua ---
server.tool(
    "execute_lua",
    "在 MUD client 的 Lua 環境中執行程式碼。可以使用 mud.send()、mud.echo() 等 API",
    {
        code: z.string().describe("Lua 程式碼，例如 mud.send('look') 或 mud.echo('hello')"),
    },
    async ({ code }) => {
        try {
            const data = await apiPost("/api/lua", { code }) as {
                ok: boolean;
                message: string;
            };
            return {
                content: [
                    {
                        type: "text" as const,
                        text: data.message,
                    },
                ],
            };
        } catch (e) {
            return {
                content: [
                    {
                        type: "text" as const,
                        text: `Error: ${e instanceof Error ? e.message : String(e)}`,
                    },
                ],
                isError: true,
            };
        }
    }
);

// ============================================================================
// Start Server
// ============================================================================

async function main() {
    const transport = new StdioServerTransport();
    await server.connect(transport);
    console.error("mudclient-mcp server started (stdio mode)");
}

main().catch((e) => {
    console.error("Fatal error:", e);
    process.exit(1);
});
