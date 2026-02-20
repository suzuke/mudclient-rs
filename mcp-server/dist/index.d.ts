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
export {};
