# ItemFarm v3.0 Design — Event-Driven Rewrite

Date: 2026-03-08

## 1. Goals

- Full rewrite of ItemFarm using the new event-driven APIs (`mud.state_machine()`, `mud.on()/emit()`)
- Replace manual `s.stage` string + `safe_timer` + `run_id` with layered state machines
- Replace monolithic `on_server_message` (340 lines) with centralized parser + distributed event handlers
- Composable job definition format with smart defaults
- Maximum extensibility for adding new jobs and modes

## 2. Architecture

```
+-------------------------------------+
|         ItemFarm Scheduler          |  Outer SM: job rotation
|  (idle -> executing -> rotating)    |
+-----------------+-------------------+
|         Job Executor                |  Inner SM: single job execution
|  (search -> travel -> engage ->     |
|   loot -> store -> done)            |
+-----------------+-------------------+
|       Engage Pipeline               |  Sub SM: composable phases
| [verify_mob]->[dispel]->[status]->  |
| [buffs]->[core_action]->confirmed   |
+---------+---------+-----------------+
| MudNav  |MudCombat|  MudLoot        |  Shared modules
+---------+---------+-----------------+
|     Event Parser (centralized)      |  MUD text -> semantic events
+---------+---------+-----------------+
```

## 3. State Machines

### 3.1 Scheduler SM (`itemfarm_scheduler`)

| State | enter | timeout | timeout_goto |
|-------|-------|---------|-------------|
| `idle` | Scan jobs, find next enabled job | - | - |
| `executing` | Create Job Executor SM | - | - |
| `rotating` | `jobs_checked++`, check if all rotated | - | - |
| `waiting_respawn` | rest + wait | `poll_interval` sec | `idle` |
| `stopped` | cleanup, destroy all SMs | - | - |

Transitions:
- `idle` -> `executing` on `job_found`
- `idle` -> `stopped` on `no_active_jobs`
- `executing` -> `rotating` on `job_done` / `job_failed`
- `rotating` -> `idle` on `next_job_ready`
- `rotating` -> `waiting_respawn` on `all_checked`
- `waiting_respawn` -> `idle` on timeout (automatic)
- `*` -> `stopped` on `user_stop`

### 3.2 Job Executor SM (`itemfarm_job`)

| State | enter | timeout | timeout_goto |
|-------|-------|---------|-------------|
| `searching` | Send search_cmd | 3s | `not_found` |
| `traveling` | MudNav.walk to mob | - | - |
| `engaging` | Build + start engage pipeline SM | - | - |
| `looting` | MudLoot.process_loot | 10s | `storing` |
| `storing` | walk to storage, remove_nodrop, drop items | - | - |
| `done` | emit `job_done` | - | - |
| `not_found` | emit `job_failed` (reason: not_found) | - | - |
| `failed` | emit `job_failed` (reason: error) | - | - |

### 3.3 Engage Pipeline SM (`itemfarm_engage`)

Dynamically composed based on job config. Phases are included only when relevant config exists.

| Phase | Include condition | enter action | timeout | timeout_goto |
|-------|------------------|-------------|---------|-------------|
| `verify_mob` | mode != "summon" | send `l` to confirm mob present | 3s | `verify_loc` |
| `verify_loc` | follows verify_mob timeout | send search_cmd to check alive/dead | 3s | `failed` |
| `dispelling` | `engage.dispel` exists | cast dispel + `l` to check | 3s per attempt, max N | `failed` |
| `check_status` | `resources.hp_threshold` or `mp_threshold` | send `rep` + `score aff` | 5s | retry self |
| `buffing` | `resources.buffs` exists | apply missing buffs one by one | 30s | retry self |
| `core_action` | always | depends on mode (see below) | - | - |
| `kill_confirmed` | always | emit `engage_done` | - | - |
| `failed` | any phase failure | emit `engage_failed` | - | - |

**Pipeline construction logic:**

```lua
function build_engage_pipeline(job)
    local order = {}

    if job.engage.mode ~= "summon" then
        append(order, "verify_mob")
    end
    if job.engage.dispel then
        append(order, "dispelling")
    end
    if has_resource_thresholds(job) then
        append(order, "check_status")
    end
    if has_buffs(job) then
        append(order, "buffing")
    end
    append_core_phases(order, job.engage.mode)
    append(order, "kill_confirmed")

    -- Wire transitions: order[i] --"done"--> order[i+1]
    -- All phases --"fail"--> "failed"
    return build_sm("itemfarm_engage", order)
end
```

**core_action by mode:**

- **summon**: `summoning` -> `fighting` (MudCombat.safe_summon then attack)
- **direct**: `fighting` (send attack_cmd directly)
- **charm**: `charming` -> `leading` -> `waiting_kill`

## 4. Event System

### 4.1 Centralized Parser

A single `on_server_message` hook that converts MUD text into semantic events via `mud.emit()`:

```lua
local PARSE_RULES = {
    -- Combat events
    { pattern = "魂歸西天了",         event = "mob_killed" },
    { pattern = "逃了",              event = "mob_fled" },
    { pattern = "離開了",            event = "mob_fled" },
    { pattern = "目標不在",          event = "target_missing" },
    { pattern = "施法的目標不在",     event = "target_missing" },
    { pattern = "沒有這個生物",       event = "target_missing" },

    -- Search events
    { pattern = "他正在這個世界中",    event = "search_found_quest" },
    { pattern = "攜帶著",            event = "search_found_locate" },

    -- Charm events
    { pattern = "開始跟隨你了",       event = "charm_success" },
    { pattern = "不受你的言語所迷惑",  event = "charm_resist" },

    -- Summon events (delegated to MudCombat, but parser can also emit)

    -- Status events
    { pattern = "你報告自己的狀況",    event = "status_report",  parse = parse_report },
    { pattern = "法術:%s+'(.-)'",    event = "spell_detected",  parse = parse_spell },
    { pattern = "目前對你產生影響的法術或技巧有", event = "spell_list_start" },

    -- Combat detection (global)
    { pattern = "伺機而動",          event = "unexpected_combat" },
    { pattern = "蓄勢待發",          event = "unexpected_combat" },
    { pattern = "身陷戰鬥中",        event = "unexpected_combat" },

    -- Flee events
    { pattern = "從戰鬥中逃了",       event = "flee_success" },
    { pattern = "逃跑失敗",          event = "flee_failed" },

    -- Loot events
    { pattern = "丟下了",            event = "mob_dropped_item" },
}
```

Additionally, buff fade messages are dynamically registered from job configs at startup.

### 4.2 Event Handler Pattern

Each SM phase registers handlers on enter and removes them on exit:

```lua
-- In verify_mob phase enter callback:
local h1 = mud.on("mob_sighted", [[ sm_transition("itemfarm_engage", "done") ]])
local h2 = mud.on("target_missing", [[ sm_transition("itemfarm_engage", "fail") ]])
-- Store handler IDs for cleanup in exit callback
```

### 4.3 Global Event Handlers (always active while running)

- `unexpected_combat` -> emergency escape (flee + recall + disable job)
- `user_stop` -> destroy all SMs, cleanup

## 5. Job Definition Format

```lua
{
    name = "不動明王",
    disabled = false,

    search = {
        type = "quest",           -- "quest" | "locate"
        cmd = "q 6.sentinel",
    },

    travel = {
        path = "recall;3w;4s;ta wizard help;7w;7n;6u;7n",
        pre_cmd = "c inv",        -- optional: run before navigation
    },

    engage = {
        mode = "direct",          -- "summon" | "direct" | "charm"
        target = "sentinel",      -- keyword for game commands
        target_display = {"不動明王"},  -- display names for text matching
        attack = "c star;c star;c star",

        -- Optional, mode-specific:
        summon_cmd = nil,         -- summon mode only
        dispel = {                -- optional, any mode
            cmd = "c 'dispel m' sentinel",
            indicators = {"白色聖光"},
            max_retries = 15,
        },
        path_to_kill = nil,       -- charm mode only
        kill_action = nil,        -- charm mode only
    },

    resources = {                 -- optional, inherits from global config
        hp_threshold = 100,
        mp_threshold = 50,
        hp_recover_cmd = "c heal",
        buffs = {
            { cmd = "c sa",  indicator = "聖光", fade_msg = "白色聖光消散了" },
            { cmd = "c pro", indicator = "聖佑術", fade_msg = "失去上天的護佑" },
            { cmd = "c b",   indicator = "女神庇祐術", fade_msg = "你覺得你的好運已經結束了" },
        },
    },

    loot = {                      -- optional
        items = {"sword", "potato", "hamburg"},
        sac = true,
        remove_nodrop = {},
    },

    store = {                     -- optional, inherits from global config
        path = "recall;3n;e",
    },
}
```

**Default inheritance:** Omitted sections fall back to global config:

```lua
ItemFarm.defaults = {
    resources = { hp_threshold = 0, mp_threshold = 50 },
    store = { path = "recall;3n;e" },
    loot = { items = {}, sac = true, remove_nodrop = {} },
}
```

Deep merge: job-level config overrides defaults field by field.

## 6. Stale Timer Prevention

1. **SM timeout** replaces most `safe_timer` uses (search timeout, dispel timeout, charm timeout, respawn wait)
2. **Remaining timers** (e.g., 0.5s delay before attack) check `mud.sm_current()`:
   ```lua
   mud.timer(0.5, [[
       if mud.sm_current("itemfarm_engage") == "fighting" then
           send_cmds(attack)
       end
   ]])
   ```
3. **`user_stop`** event destroys all SMs. SM destruction runs exit callbacks which call `mud.off()` on all registered handlers. No run_id needed.

## 7. External API (unchanged)

```lua
ItemFarm.start()          -- Create Scheduler SM, begin
ItemFarm.stop()           -- emit "user_stop", destroy SMs
ItemFarm.status()         -- Read SM states, display
ItemFarm.toggle_job(n)    -- Toggle job disabled flag
ItemFarm.toggle_echo()    -- Toggle verbose logging
ItemFarm.reload()         -- Re-require script
```

## 8. File Structure

```
scripts/
    itemfarm_v3.lua               -- Entry point + Scheduler + external API
    modules/
        ItemFarmParser.lua        -- Centralized text -> event parser
        ItemFarmExecutor.lua      -- Job Executor SM logic
        ItemFarmEngage.lua        -- Engage pipeline SM (summon/direct/charm)
        ItemFarmJobs.lua          -- Job definitions (data only)
```

## 9. Migration Notes

- `itemfarm_v3.lua` is a new file; existing `itemfarm.lua` remains untouched
- Job definitions are extracted to `ItemFarmJobs.lua` for easy editing
- `_G.ItemFarm` namespace is reused; loading v3 replaces v2
- MudCombat/MudNav/MudLoot interfaces remain unchanged
