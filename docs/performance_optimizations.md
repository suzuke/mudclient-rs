# Performance Optimizations (2026-03)

## Completed Fixes

### High Priority
| # | Issue | Fix | File |
|---|-------|-----|------|
| 1 | ScriptEngine dofile re-init every call | `dofile_initialized` guard, skip repeated setup | `script.rs` |
| 1b | `expand_variables` no early exit | Added `$` check + empty vars check | `script.rs` |
| 2/9 | Redundant `strip_ansi` in trigger pipeline | Added `process_pre_stripped()`, `should_gag_pre_stripped()`, `collect_commands_pre_stripped()` | `trigger.rs`, `session.rs` |
| 3 | Per-char `ch.to_string()` in render loop (11 sites) | Reusable `char_buf: String` buffer | `app/mod.rs` |
| 21 | BFS pathfinding clones entire path Vec per node | Parent-pointer backtracking, O(V×P) → O(V) | `database.rs` |

### Medium Priority
| # | Issue | Fix | File |
|---|-------|-----|------|
| 11 | TCP 8KB buffer allocated per read call | Persistent `read_buffer` field on TelnetClient | `telnet/client.rs` |
| 12 | LLM dispatch spawns new Tokio runtime per call | `Handle::try_current()` reuse with thread fallback | `session.rs` |
| 15 | API session lookup via String comparison | Parse to u64 first, compare numerically | `app/mod.rs` |
| 19 | `input_history: Vec<String>` with `remove(0)` | Changed to `VecDeque<String>` with `pop_front()` | `session.rs`, `app/mod.rs` |
| 28 | Reconnect polling `request_repaint()` every frame | `request_repaint_after(Duration::from_secs(1))` | `app/mod.rs` |

## Skipped (Not Worth Fixing)
| # | Reason |
|---|--------|
| 4 | `main_job.clone()` — both consumers need ownership, can't eliminate |
| 5 | ANSI parsing per frame — already limited to visible_lines |
| 6/7/8 | glyph_cache/section maps/FontId clone — egui API requires owned values |
| 10 | `expand_variables` O(n×text_len) — vars count is tiny (<20), early exit sufficient |
| 13 | Mutex lock duration — short hold, standard pattern |
| 14 | `process_api_commands` iterates all sessions — has early continue, only 1-3 sessions |
| 16 | `screen_words` growth — already has 1000 cap + 5min cutoff |
| 17 | Recursive multi-line processing — only 1 level deep (split by \n) |

## Test Coverage Added
- `trigger.rs`: 6 tests for pre_stripped methods
- `script.rs`: 5 tests for dofile caching + expand_variables
- `database.rs`: 2 tests for BFS (longer path, shortest path verification)
- All 40+ workspace tests pass
