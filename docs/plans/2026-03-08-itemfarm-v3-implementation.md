# ItemFarm v3.0 Event-Driven Rewrite — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rewrite ItemFarm using layered state machines (`mud.state_machine()`) and event-driven architecture (`mud.on()/emit()`), replacing the manual `s.stage` + `safe_timer` + `run_id` pattern.

**Architecture:** Three-layer SM (Scheduler -> Executor -> Engage Pipeline) with centralized event parser. Composable job definitions with smart defaults. Shared modules (MudNav/MudCombat/MudLoot) unchanged.

**Tech Stack:** Lua 5.4, mud.* API (state_machine, on/once/emit/off, sm_current, sm_transition, sm_reset, timer, send, echo, collect_response)

**Known Constraint:** `mud.on()` returns a placeholder ID that may not match EventBus's real handler ID, making `mud.off()` unreliable. Use `mud.once()` for one-shot handlers, and `sm_current()` guards inside persistent handlers.

**Design Doc:** `docs/plans/2026-03-08-itemfarm-v3-design.md`

---

## Task 1: Job Definitions Data File (`ItemFarmJobs.lua`)

Extract and convert all job definitions from `scripts/itemfarm.lua` into the new format.

**Files:**
- Create: `scripts/modules/ItemFarmJobs.lua`
- Reference: `scripts/itemfarm.lua:76-250` (current job definitions)

**Step 1: Create the job definitions file**

```lua
-- scripts/modules/ItemFarmJobs.lua
-- ItemFarm v3 Job Definitions (data only)

local function extend_path(base, extra)
    local p = {}
    for i, v in ipairs(base) do p[i] = v end
    for _, v in ipairs(extra) do p[#p + 1] = v end
    return p
end

-- recall -> museum -> look painting -> Gianis City -> main square
local path_to_painting_area = {
    {cmd="recall", id="4e9c9dd2418fa5c52e762d52985dfca6fe1d77cd111c87536d3211df7cf5ca2e"},
    {cmd="w", id="478ffe9f12a30d704186f327ca56a85531b46db421602509b46fe58eb6c267c5"},
    {cmd="w", id="349965591ea7cea0ca7de23a02a1cdc517fe5d1617867d0cb8d4623c72af7dbd"},
    {cmd="w", id="314bd5656517c827bebea9e72a871802325927e2241ee17c5f4ed37b420f39a6"},
    {cmd="s", id="582cf9bb9b3444959f6ac8882c7e6f6174599a42efa746ccad882be7bfd8bfaa"},
    {cmd="s", id="b370fe0b35b66d61dd7c1b38070c1dbcea24550aabad37e266148b7244204459"},
    {cmd="s", id="5530f4e241b3cd903a4cc158d4732764c974e1bcb554ce97169140eecda97a85"},
    {cmd="e", id="97f8fe848f5492717e2b12e0538552c62937548236d20a028f7cc1ebaedb18b8"},
    {cmd="look painting", id="e913d4e99d70ba89895dab53d22aa6669d5c585ffb066eb241cb2abedace31b3"},
    {cmd="s", id="eef1fdd15511ea3c48793299ca947aa4de44be5a095cd7194a4e103740fc2e2c"},
}

local M = {}

M.defaults = {
    resources = { hp_threshold = 0, mp_threshold = 50 },
    store = { path = "recall;3n;e" },
    loot = { items = {}, sac = true, remove_nodrop = {} },
    rest_cmd = "sleep",
    poll_interval = 30,
    show_echo = true,
}

M.jobs = {
    {
        name = "商務間諜",
        disabled = true,
        search = { type = "locate", cmd = "c loc grating" },
        travel = { path = "recall;2n;2e" },
        engage = {
            mode = "summon",
            target = "spy",
            target_display = {"商務間諜"},
            attack = "c flame spy",
            summon_cmd = "c sum spy",
        },
        loot = { items = {"anesthetic", "grating"}, sac = true, remove_nodrop = {"anesthetic", "grating"} },
    },
    {
        name = "街頭混混",
        disabled = true,
        search = { type = "quest", cmd = "q 28.boy" },
        travel = { path = "recall" },
        engage = {
            mode = "summon",
            target = "boy",
            target_display = {"街頭混混"},
            attack = "c flame boy",
            summon_cmd = "c sum boy",
        },
        loot = { items = {"take"}, sac = true },
    },
    {
        name = "某校生",
        disabled = false,
        search = { type = "locate", cmd = "c loc id" },
        travel = { path = "recall" },
        engage = {
            mode = "summon",
            target = "student",
            target_display = {"某校生"},
            attack = "c nu student;c fl student",
            summon_cmd = "c sum student",
        },
        loot = { items = {"id"}, sac = true },
    },
    {
        name = "不動明王",
        disabled = true,
        search = { type = "quest", cmd = "q 6.sentinel" },
        travel = {
            path = "recall;3w;4s;ta wizard help;7w;7n;6u;7n",
            pre_cmd = "c inv",
        },
        engage = {
            mode = "direct",
            target = "sentinel",
            target_display = {"不動明王"},
            attack = "c star;c star;c star",
            dispel = {
                cmd = "c 'dispel m' sentinel",
                indicators = {"白色聖光"},
                max_retries = 15,
            },
        },
        resources = {
            hp_threshold = 100,
            hp_recover_cmd = "c heal",
            buffs = {
                { cmd = "c sa",  indicator = "聖光", fade_msg = "你四周的白色聖光消散了" },
                { cmd = "c pro", indicator = "聖佑術", fade_msg = "你感覺到失去上天的護佑." },
                { cmd = "c b",   indicator = "女神庇祐術", fade_msg = "你覺得你的好運已經結束了." },
            },
        },
        loot = { items = {"sword", "potato", "hamburg"}, sac = true },
    },
    {
        name = "闇の一族幫員",
        disabled = true,
        search = { type = "quest", cmd = "q clan_member" },
        travel = {
            path = "recall;11s;w;n;3e;2n;e;3n;e;2n;u;4n;e",
            pre_cmd = "c inv",
        },
        engage = {
            mode = "direct",
            target = "clan_member",
            target_display = {"闇の一族幫員"},
            attack = "c nu clan_member;c fl clan_member",
        },
        loot = { items = {"Xiulou"}, sac = true },
    },
    {
        name = "天堂守護者麥倫．薩爾達",
        disabled = true,
        search = { type = "quest", cmd = "q 2.paradiser" },
        travel = { path = "recall;3w;2s;5e;2s;e;op e;e" },
        engage = {
            mode = "direct",
            target = "paradiser",
            target_display = {"麥倫．薩爾達"},
            attack = "c star;c star;c star;",
            dispel = {
                cmd = "c 'dispel m' paradiser",
                indicators = {"白色聖光"},
                max_retries = 15,
            },
        },
        resources = {
            hp_threshold = 100,
            hp_recover_cmd = "c heal",
            buffs = {
                { cmd = "c sa",  indicator = "聖光", fade_msg = "你四周的白色聖光消散了" },
                { cmd = "c pro", indicator = "聖佑術", fade_msg = "你感覺到失去上天的護佑." },
                { cmd = "c b",   indicator = "女神庇祐術", fade_msg = "你覺得你的好運已經結束了." },
            },
        },
        loot = { items = {"wisdom"}, sac = true },
    },
    {
        name = "動靈帽",
        disabled = false,
        search = { type = "locate", cmd = "c loc mind" },
        travel = {
            path = extend_path(path_to_painting_area, {
                {cmd="w", id="17d58425b0bfaeb6cd94043280833333ed3aa5a503b0b094fc13e877a7fce6cb"},
                {cmd="w", id="de0bc88cac4c3db67d96141608ab2d39af185a9f6346f3f76c191b93f1bd7909"},
                {cmd="s", id="a0bdbe8419a511cf0d91f90a708e8be8aca8fa6f48d5cd1ca527e5821015edf8"},
            }),
        },
        engage = {
            mode = "charm",
            target = "student",
            target_display = {"一位魔法見習生", "魔法見習生"},
            attack = "c charm student",
            path_to_kill = "or all hi;or all recall;recall;n;n;n;e",
            kill_action = "or all drop hat;w;s;s;s;w;w;n",
        },
        loot = { items = {}, sac = false },
    },
    {
        name = "詛咒之劍",
        disabled = false,
        search = { type = "quest", cmd = "q 17.traveller" },
        travel = { path = path_to_painting_area },
        engage = {
            mode = "summon",
            target = "traveller",
            target_display = {"一位四處旅行的", "旅人"},
            attack = "c fire traveller",
            summon_cmd = "c summon traveller",
        },
        loot = { items = {"curse"}, sac = true },
    },
}

return M
```

**Step 2: Verify file loads**

Run in MUD client: `lua require("scripts.modules.ItemFarmJobs")` — should not error.

**Step 3: Commit**

```bash
git add scripts/modules/ItemFarmJobs.lua
git commit -m "feat(itemfarm-v3): extract job definitions to ItemFarmJobs.lua"
```

---

## Task 2: Event Parser Module (`ItemFarmParser.lua`)

Centralized MUD text -> semantic event converter.

**Files:**
- Create: `scripts/modules/ItemFarmParser.lua`

**Step 1: Create the parser module**

```lua
-- scripts/modules/ItemFarmParser.lua
-- Centralized MUD text -> semantic event parser for ItemFarm v3

local string = string
local tonumber = tonumber
local ipairs = ipairs

local M = {}

-- Static parse rules: { pattern, event, [parse_fn] }
-- parse_fn(clean_line) -> data table or nil
local RULES = {
    -- Combat events
    { p = "魂歸西天了",             ev = "ifarm:mob_killed" },
    { p = "逃了",                  ev = "ifarm:mob_fled" },
    { p = "離開了",                ev = "ifarm:mob_fled" },
    { p = "目標不在",              ev = "ifarm:target_missing" },
    { p = "施法的目標不在",         ev = "ifarm:target_missing" },
    { p = "沒有這個生物",           ev = "ifarm:target_missing" },

    -- Search events
    { p = "他正在這個世界中",        ev = "ifarm:search_found", data = {type = "quest"} },
    { p = "攜帶著",                ev = "ifarm:search_found", data = {type = "locate"} },

    -- Charm events
    { p = "開始跟隨你了",           ev = "ifarm:charm_success" },
    { p = "不受你的言語所迷惑",      ev = "ifarm:charm_resist" },

    -- Combat detection (emergency)
    { p = "伺機而動",              ev = "ifarm:unexpected_combat" },
    { p = "蓄勢待發",              ev = "ifarm:unexpected_combat" },
    { p = "身陷戰鬥中",            ev = "ifarm:unexpected_combat" },

    -- Flee
    { p = "你為了保命而不顧面子從戰鬥中逃了", ev = "ifarm:flee_success" },
    { p = "你逃跑失敗了",          ev = "ifarm:flee_failed" },

    -- Loot
    { p = "丟下了",                ev = "ifarm:mob_dropped" },

    -- Spell list
    { p = "目前對你產生影響的法術或技巧有", ev = "ifarm:spell_list_start" },
}

-- Dynamic rules added at runtime (buff fade messages)
local dynamic_rules = {}

function M.add_fade_rule(fade_msg, indicator)
    dynamic_rules[#dynamic_rules + 1] = {
        p = fade_msg,
        ev = "ifarm:buff_faded",
        data = { indicator = indicator },
    }
end

function M.clear_dynamic_rules()
    dynamic_rules = {}
end

-- Parse "你報告自己的狀況: HP/Max 生命力 MA/Max 精神力 V/Max 移動力 P/Max 內力"
local function parse_report(line)
    local hp, hp_max = string.match(line, "(%d+)/(%d+)%s+生命力")
    local mp, mp_max = string.match(line, "(%d+)/(%d+)%s+精神力")
    if hp and mp then
        return {
            hp = tonumber(hp), hp_max = tonumber(hp_max),
            mp = tonumber(mp), mp_max = tonumber(mp_max),
        }
    end
    return nil
end

-- Parse "法術: 'XXX' ... 達 N 小時"
local function parse_spell(line)
    local name, hours = string.match(line, "法術:%s+'(.-)'.*達%s+(-?%d+)%s+小時")
    if name then
        return { name = name, hours = tonumber(hours) }
    end
    return nil
end

-- Main parse function: called from on_server_message hook
-- Returns nothing; emits events via mud.emit()
function M.parse(line, clean_line, is_echo)
    if is_echo then return end

    local cl = clean_line:gsub("\r", "")
    local len = #cl
    if len < 2 then return end

    -- Skip chat/emote lines
    if string.find(cl, "^【") then return end
    if string.find(cl, "^%s*「.*」") then return end

    -- Status report (special: has parse function)
    if string.find(cl, "你報告自己的狀況", 1, true) then
        local data = parse_report(cl)
        if data then mud.emit("ifarm:status_report", data) end
        return
    end

    -- Spell detection
    local spell_data = parse_spell(cl)
    if spell_data then
        mud.emit("ifarm:spell_detected", spell_data)
        return
    end

    -- Score-based HP/MP parsing (fallback for score command output)
    local h_cur, h_max = string.match(cl, "生命力:?%s+(%d+)/%s+(%d+)")
    if h_cur then
        local m_cur, m_max = string.match(cl, "精神力:?%s+(%d+)/%s+(%d+)")
        mud.emit("ifarm:score_hp_mp", {
            hp = tonumber(h_cur), hp_max = tonumber(h_max),
            mp = m_cur and tonumber(m_cur) or nil,
            mp_max = m_max and tonumber(m_max) or nil,
        })
        return
    end

    -- Static rules
    for _, rule in ipairs(RULES) do
        if string.find(cl, rule.p, 1, true) then
            local data = rule.data or {}
            data.line = cl
            mud.emit(rule.ev, data)
            return  -- first match wins
        end
    end

    -- Dynamic rules (buff fade)
    for _, rule in ipairs(dynamic_rules) do
        if string.find(cl, rule.p, 1, true) then
            mud.emit(rule.ev, rule.data)
            return
        end
    end
end

return M
```

**Step 2: Commit**

```bash
git add scripts/modules/ItemFarmParser.lua
git commit -m "feat(itemfarm-v3): add centralized event parser module"
```

---

## Task 3: Engage Pipeline Module (`ItemFarmEngage.lua`)

The composable engage sub-SM: verify_mob -> dispel -> check_status -> buffs -> core_action -> kill_confirmed.

**Files:**
- Create: `scripts/modules/ItemFarmEngage.lua`

**Step 1: Create the engage module**

This is the most complex module. It builds the engage SM dynamically from job config.

```lua
-- scripts/modules/ItemFarmEngage.lua
-- Composable Engage Pipeline for ItemFarm v3

local string = string
local table = table
local ipairs = ipairs
local tonumber = tonumber
local math = math

local function require_module(name)
    local paths = { "scripts.modules." .. name, "modules." .. name, name }
    for _, p in ipairs(paths) do
        local ok, res = pcall(require, p)
        if ok then return res end
    end
    error("ItemFarmEngage: cannot load " .. name)
end

local MudCombat = require_module("MudCombat")
local MudNav = require_module("MudNav")

local M = {}

-- Helper: parse "cmd1;cmd2;3w" into individual send() calls
local function send_cmds(str)
    for cmd in string.gmatch(str, "[^;]+") do
        cmd = cmd:match("^%s*(.-)%s*$")
        if cmd ~= "" then
            local count, actual = cmd:match("^(%d+)(%a.*)$")
            if count then
                for _ = 1, tonumber(count) do mud.send(actual) end
            else
                mud.send(cmd)
            end
        end
    end
end

-- Helper: check if target_display matches a line
local function match_target(line, target_display)
    if not target_display then return false end
    if type(target_display) == "string" then
        return string.find(line, target_display, 1, true) ~= nil
    end
    for _, kw in ipairs(target_display) do
        if string.find(line, kw, 1, true) then return true end
    end
    return false
end

-- State definitions for each possible phase
-- Each returns: { states = {name -> def}, transitions = {list} }
-- enter/exit code references _G.ItemFarm.engage namespace

local function phase_verify_mob(job)
    return {
        states = {
            verify_mob = {
                enter = string.format([[
                    _G.ItemFarm.engage.echo("Confirm target present...")
                    mud.send("l")
                    mud.on("ifarm:mob_killed", [[
                        local d = data or {}
                        if _G.ItemFarm.engage.match_target(d.line) and mud.sm_current("itemfarm_engage") == "verify_mob" then
                            mud.sm_transition("itemfarm_engage", "done")
                        end
                    ]], 0)
                ]]),
                timeout_secs = 3.0,
                timeout_goto = "verify_loc",
            },
            verify_loc = {
                enter = string.format(
                    [[_G.ItemFarm.engage.echo("Target not in room, checking status...")
                    mud.send(%q)]],
                    job.search.cmd
                ),
                timeout_secs = 3.0,
                timeout_goto = "failed",
            },
        },
        order = {"verify_mob", "verify_loc"},
    }
end

-- Build SM definition from job config
-- Returns the SM name ("itemfarm_engage") and starts the SM
function M.start(job, merged_resources, on_done, on_failed)
    -- Store context for callbacks
    _G.ItemFarm.engage = _G.ItemFarm.engage or {}
    _G.ItemFarm.engage.job = job
    _G.ItemFarm.engage.resources = merged_resources
    _G.ItemFarm.engage.on_done = on_done
    _G.ItemFarm.engage.on_failed = on_failed
    _G.ItemFarm.engage.active_spells = _G.ItemFarm.engage.active_spells or {}
    _G.ItemFarm.engage.dispel_retries = 0

    _G.ItemFarm.engage.echo = function(msg)
        if _G.ItemFarm and _G.ItemFarm.echo then
            _G.ItemFarm.echo("[Engage] " .. msg)
        end
    end

    _G.ItemFarm.engage.match_target = function(line)
        if not line then return false end
        return match_target(line, job.engage.target_display)
    end

    -- Build ordered phase list
    local states = {}
    local transitions = {}
    local order = {}

    -- Phase: verify_mob (skip for summon)
    if job.engage.mode ~= "summon" then
        order[#order + 1] = "verify_mob"
        states.verify_mob = {
            enter = string.format([[
                _G.ItemFarm.engage.echo("Confirming target in room...")
                mud.send("l")
            ]]),
            timeout_secs = 3.0,
            timeout_goto = "verify_loc",
        }
        -- verify_loc is a fallback state for when mob not in room
        states.verify_loc = {
            enter = string.format([[
                _G.ItemFarm.engage.echo("Target not in room, sending search_cmd...")
                mud.send(%q)
            ]], job.search.cmd),
            timeout_secs = 3.0,
            timeout_goto = "failed",
        }
    end

    -- Phase: dispelling
    if job.engage.dispel then
        order[#order + 1] = "dispelling"
        states.dispelling = {
            enter = string.format([[
                _G.ItemFarm.engage.echo("Dispelling...")
                _G.ItemFarm.engage.dispel_retries = 0
                mud.send(%q)
                mud.timer(1.5, [==[mud.send("l")]==])
            ]], job.engage.dispel.cmd),
            timeout_secs = 5.0,
            timeout_goto = "dispel_check",
        }
        states.dispel_check = {
            enter = string.format([[
                local e = _G.ItemFarm.engage
                e.dispel_retries = e.dispel_retries + 1
                local max = %d
                if e.dispel_retries >= max then
                    e.echo("Dispel failed " .. max .. " times, aborting")
                    mud.sm_transition("itemfarm_engage", "fail")
                else
                    e.echo("Dispel retry " .. e.dispel_retries .. "/" .. max)
                    mud.send(%q)
                    mud.timer(1.5, [==[mud.send("l")]==])
                end
            ]], job.engage.dispel.max_retries or 10, job.engage.dispel.cmd),
            timeout_secs = 5.0,
            timeout_goto = "dispel_check",  -- loop until max
        }
    end

    -- Phase: check_status
    if merged_resources.hp_threshold and merged_resources.hp_threshold > 0
       or merged_resources.mp_threshold and merged_resources.mp_threshold > 0 then
        order[#order + 1] = "check_status"
        states.check_status = {
            enter = [[
                _G.ItemFarm.engage.echo("Checking HP/MP status...")
                mud.send("rep")
                mud.send("score aff")
            ]],
            timeout_secs = 5.0,
            timeout_goto = "check_status",  -- retry on timeout
        }
    end

    -- Phase: buffing
    if merged_resources.buffs and #merged_resources.buffs > 0 then
        order[#order + 1] = "buffing"
        states.buffing = {
            enter = [[
                _G.ItemFarm.engage.apply_next_buff()
            ]],
            timeout_secs = 30.0,
            timeout_goto = "buffing",  -- retry
        }
    end

    -- Core action phases (mode-dependent)
    local mode = job.engage.mode or "direct"
    if mode == "summon" then
        order[#order + 1] = "summoning"
        states.summoning = {
            enter = string.format([[
                _G.ItemFarm.engage.echo("Summoning target...")
                _G.ItemFarm.engage.start_summon()
            ]]),
        }
        order[#order + 1] = "fighting"
        states.fighting = {
            enter = string.format([[
                _G.ItemFarm.engage.echo("Fighting!")
                _G.ItemFarm.engage.send_attack()
            ]]),
            timeout_secs = 60.0,
            timeout_goto = "failed",
        }
    elseif mode == "direct" then
        order[#order + 1] = "fighting"
        states.fighting = {
            enter = string.format([[
                _G.ItemFarm.engage.echo("Fighting!")
                _G.ItemFarm.engage.send_attack()
            ]]),
            timeout_secs = 60.0,
            timeout_goto = "failed",
        }
    elseif mode == "charm" then
        order[#order + 1] = "charming"
        states.charming = {
            enter = [[
                _G.ItemFarm.engage.echo("Charming target...")
                _G.ItemFarm.engage.charm_retries = 0
                _G.ItemFarm.engage.send_attack()
            ]],
            timeout_secs = 3.0,
            timeout_goto = "charm_retry",
        }
        states.charm_retry = {
            enter = [[
                local e = _G.ItemFarm.engage
                e.charm_retries = (e.charm_retries or 0) + 1
                if e.charm_retries > 3 then
                    e.echo("Charm failed 3 times, aborting")
                    mud.sm_transition("itemfarm_engage", "fail")
                else
                    e.echo("Charm retry " .. e.charm_retries .. "/3")
                    e.send_attack()
                end
            ]],
            timeout_secs = 3.0,
            timeout_goto = "charm_retry",
        }

        order[#order + 1] = "leading"
        states.leading = {
            enter = [[
                _G.ItemFarm.engage.echo("Leading charmed target to kill zone...")
                _G.ItemFarm.engage.lead_to_kill()
            ]],
        }
        order[#order + 1] = "waiting_kill"
        states.waiting_kill = {
            enter = string.format([[
                _G.ItemFarm.engage.echo("Waiting for target to be killed...")
                mud.send("wa")
                mud.send("c ref")
                %s
            ]], job.engage.kill_action and ('_G.ItemFarm.engage.send_cmds("' .. job.engage.kill_action .. '")') or ""),
            timeout_secs = 30.0,
            timeout_goto = "kill_confirmed",  -- force proceed on timeout
        }
    end

    -- Terminal states
    order[#order + 1] = "kill_confirmed"
    states.kill_confirmed = {
        enter = [[
            _G.ItemFarm.engage.echo("Kill confirmed!")
            mud.emit("ifarm:engage_done")
        ]],
    }
    states.failed = {
        enter = [[
            _G.ItemFarm.engage.echo("Engage failed")
            mud.emit("ifarm:engage_failed")
        ]],
    }

    -- Wire transitions: order[i] --"done"--> order[i+1]
    for i = 1, #order - 1 do
        transitions[#transitions + 1] = { from = order[i], event = "done", to = order[i + 1] }
    end
    -- All non-terminal -> failed on "fail"
    for _, phase in ipairs(order) do
        if phase ~= "kill_confirmed" and phase ~= "failed" then
            transitions[#transitions + 1] = { from = phase, event = "fail", to = "failed" }
        end
    end
    -- Special: verify_mob needs mob_sighted event
    if states.verify_mob then
        transitions[#transitions + 1] = { from = "verify_mob", event = "mob_sighted", to = order[2] or "kill_confirmed" }
        transitions[#transitions + 1] = { from = "verify_loc", event = "mob_alive_elsewhere", to = "failed" }
    end
    -- Special: dispel_check loops or proceeds
    if states.dispel_check then
        -- Find next phase after dispelling
        local next_after_dispel = nil
        for i, p in ipairs(order) do
            if p == "dispelling" and order[i + 1] then
                next_after_dispel = order[i + 1]
                break
            end
        end
        if next_after_dispel then
            transitions[#transitions + 1] = { from = "dispelling", event = "dispel_success", to = next_after_dispel }
            transitions[#transitions + 1] = { from = "dispel_check", event = "dispel_success", to = next_after_dispel }
            transitions[#transitions + 1] = { from = "dispel_check", event = "fail", to = "failed" }
        end
    end
    -- Special: charm success -> leading
    if states.charming then
        transitions[#transitions + 1] = { from = "charming", event = "charm_ok", to = "leading" }
        transitions[#transitions + 1] = { from = "charm_retry", event = "charm_ok", to = "leading" }
    end
    -- Special: summoning -> fighting
    if states.summoning then
        transitions[#transitions + 1] = { from = "summoning", event = "summon_ok", to = "fighting" }
    end
    -- fighting -> kill_confirmed
    if states.fighting then
        transitions[#transitions + 1] = { from = "fighting", event = "killed", to = "kill_confirmed" }
    end
    -- waiting_kill -> kill_confirmed
    if states.waiting_kill then
        transitions[#transitions + 1] = { from = "waiting_kill", event = "killed", to = "kill_confirmed" }
    end

    -- Register event handlers for this engage session
    M.register_event_handlers(job)

    -- Create the SM
    local initial = order[1]
    mud.state_machine("itemfarm_engage", {
        initial = initial,
        states = states,
        transitions = transitions,
    })
end

-- Register mud.on() handlers that bridge ifarm:* events to SM transitions
function M.register_event_handlers(job)
    local td = job.engage.target_display

    -- mob_killed -> check target match -> transition "killed"
    mud.on("ifarm:mob_killed", string.format([[
        local d = data or {}
        local line = d.line or ""
        local cur = mud.sm_current("itemfarm_engage")
        if not cur then return end
        local td = %s
        local matched = false
        if type(td) == "table" then
            for _, kw in ipairs(td) do
                if string.find(line, kw, 1, true) then matched = true; break end
            end
        elseif type(td) == "string" then
            matched = string.find(line, td, 1, true) ~= nil
        end
        if matched then
            if cur == "fighting" or cur == "waiting_kill" then
                mud.sm_transition("itemfarm_engage", "killed")
            end
        end
    ]], M.serialize_target(td)), 0)

    -- mob_fled -> fail (for fighting phase)
    mud.on("ifarm:mob_fled", [[
        local cur = mud.sm_current("itemfarm_engage")
        if cur == "fighting" then
            mud.sm_transition("itemfarm_engage", "fail")
        end
    ]], 0)

    -- target_missing -> fail
    mud.on("ifarm:target_missing", [[
        local cur = mud.sm_current("itemfarm_engage")
        if cur == "fighting" or cur == "charming" or cur == "charm_retry" then
            mud.sm_transition("itemfarm_engage", "fail")
        end
    ]], 0)

    -- For verify_mob: look output matching
    if job.engage.mode ~= "summon" then
        mud.on("on_server_message", string.format([[
            local cur = mud.sm_current("itemfarm_engage")
            if cur ~= "verify_mob" and cur ~= "verify_loc" then return end
            local cl = clean_line:gsub("\r", "")
            local td = %s
            local matched = false
            if type(td) == "table" then
                for _, kw in ipairs(td) do
                    if string.find(cl, kw, 1, true) then matched = true; break end
                end
            elseif type(td) == "string" then
                matched = string.find(cl, td, 1, true) ~= nil
            end
            if matched and not string.find(cl, "屍體", 1, true) and not string.find(cl, "corpse", 1, true) then
                if cur == "verify_mob" then
                    mud.sm_transition("itemfarm_engage", "mob_sighted")
                elseif cur == "verify_loc" then
                    mud.sm_transition("itemfarm_engage", "mob_alive_elsewhere")
                end
            end
        ]], M.serialize_target(td)), 0)
    end

    -- Dispel: check indicators in look output
    if job.engage.dispel then
        local indicators = job.engage.dispel.indicators or {}
        mud.on("on_server_message", string.format([[
            local cur = mud.sm_current("itemfarm_engage")
            if cur ~= "dispelling" and cur ~= "dispel_check" then return end
            local cl = clean_line:gsub("\r", "")
            local td = %s
            local indicators = %s
            -- Check if mob line with target
            local mob_matched = false
            if type(td) == "table" then
                for _, kw in ipairs(td) do
                    if string.find(cl, kw, 1, true) then mob_matched = true; break end
                end
            elseif type(td) == "string" then
                mob_matched = string.find(cl, td, 1, true) ~= nil
            end
            if mob_matched and not string.find(cl, "屍體", 1, true) then
                local has_indicator = false
                for _, ind in ipairs(indicators) do
                    if string.find(cl, ind, 1, true) then has_indicator = true; break end
                end
                if has_indicator then
                    -- Still has protection, need more dispel
                    -- timeout_goto will handle retry
                else
                    mud.sm_transition("itemfarm_engage", "dispel_success")
                end
            end
        ]], M.serialize_target(td), M.serialize_list(indicators)), 0)
    end

    -- Status report handler
    mud.on("ifarm:status_report", [[
        local cur = mud.sm_current("itemfarm_engage")
        if cur ~= "check_status" then return end
        local d = data or {}
        _G.ItemFarm.engage.last_hp = d.hp
        _G.ItemFarm.engage.last_hp_max = d.hp_max
        _G.ItemFarm.engage.last_mp = d.mp
        _G.ItemFarm.engage.last_mp_max = d.mp_max
        _G.ItemFarm.engage.evaluate_status()
    ]], 0)

    -- Charm success
    mud.on("ifarm:charm_success", [[
        local cur = mud.sm_current("itemfarm_engage")
        if cur == "charming" or cur == "charm_retry" then
            mud.sm_transition("itemfarm_engage", "charm_ok")
        end
    ]], 0)

    -- Spell detection (for buff tracking)
    mud.on("ifarm:spell_detected", [[
        local d = data or {}
        if d.name and d.hours then
            _G.ItemFarm.engage.active_spells[d.name] = d.hours
        end
    ]], 0)

    mud.on("ifarm:spell_list_start", [[
        _G.ItemFarm.engage.active_spells = {}
    ]], 0)

    -- Buff faded
    mud.on("ifarm:buff_faded", [[
        local d = data or {}
        if d.indicator then
            _G.ItemFarm.engage.active_spells[d.indicator] = nil
        end
        local cur = mud.sm_current("itemfarm_engage")
        if cur == "buffing" then
            _G.ItemFarm.engage.apply_next_buff()
        end
    ]], 0)

    -- Engage done/failed -> call callbacks
    mud.once("ifarm:engage_done", [[
        local fn = _G.ItemFarm.engage.on_done
        if fn then fn() end
    ]], 0)

    mud.once("ifarm:engage_failed", [[
        local fn = _G.ItemFarm.engage.on_failed
        if fn then fn() end
    ]], 0)

    -- Mob dropped (charm mode)
    mud.on("ifarm:mob_dropped", [[
        local cur = mud.sm_current("itemfarm_engage")
        if cur == "waiting_kill" then
            mud.sm_transition("itemfarm_engage", "killed")
        end
    ]], 0)
end

-- Helper functions stored on _G.ItemFarm.engage for use in SM callbacks

function M.setup_helpers(job, merged_resources)
    local e = _G.ItemFarm.engage

    e.send_attack = function()
        send_cmds(job.engage.attack)
    end

    e.send_cmds = function(str)
        send_cmds(str)
    end

    e.start_summon = function()
        MudCombat.safe_summon(job.engage.target_display, job.engage.summon_cmd, {
            max_retries = 3, retry_delay = 2.0, verify_delay = 1.0,
        }, function()
            mud.sm_transition("itemfarm_engage", "summon_ok")
        end, function()
            mud.sm_transition("itemfarm_engage", "fail")
        end)
    end

    e.lead_to_kill = function()
        if job.engage.path_to_kill then
            MudNav.walk(job.engage.path_to_kill, function()
                -- Walk done, SM should already be in leading state
                -- Transition to waiting_kill
                mud.sm_transition("itemfarm_engage", "done")
            end)
        else
            mud.sm_transition("itemfarm_engage", "done")
        end
    end

    e.evaluate_status = function()
        local hp_pct = (e.last_hp_max and e.last_hp_max > 0) and (e.last_hp / e.last_hp_max * 100) or 100
        local mp_pct = (e.last_mp_max and e.last_mp_max > 0) and (e.last_mp / e.last_mp_max * 100) or 100
        local hp_ok = (merged_resources.hp_threshold or 0) == 0 or hp_pct >= merged_resources.hp_threshold
        local mp_ok = (merged_resources.mp_threshold or 0) == 0 or mp_pct >= merged_resources.mp_threshold
        if hp_ok and mp_ok then
            mud.sm_transition("itemfarm_engage", "done")
        else
            e.echo("HP/MP insufficient, failing engage")
            mud.sm_transition("itemfarm_engage", "fail")
        end
    end

    e.apply_next_buff = function()
        local buffs = merged_resources.buffs or {}
        for _, b in ipairs(buffs) do
            local hours = e.active_spells[b.indicator]
            if not hours or hours == 0 then
                e.echo("Applying buff: " .. b.indicator)
                mud.send(b.cmd)
                -- Wait for spell to take effect, timer will re-check
                mud.timer(2.0, [=[
                    if mud.sm_current("itemfarm_engage") == "buffing" then
                        _G.ItemFarm.engage.apply_next_buff()
                    end
                ]=])
                return
            end
        end
        -- All buffs present
        e.echo("All buffs active")
        mud.sm_transition("itemfarm_engage", "done")
    end
end

-- Serialize target_display for embedding in string code
function M.serialize_target(td)
    if type(td) == "string" then
        return string.format("%q", td)
    elseif type(td) == "table" then
        local parts = {}
        for _, v in ipairs(td) do
            parts[#parts + 1] = string.format("%q", v)
        end
        return "{" .. table.concat(parts, ",") .. "}"
    end
    return "nil"
end

function M.serialize_list(list)
    local parts = {}
    for _, v in ipairs(list) do
        parts[#parts + 1] = string.format("%q", v)
    end
    return "{" .. table.concat(parts, ",") .. "}"
end

-- Cleanup: remove the engage SM
function M.cleanup()
    -- Note: mud.sm_reset or removing SM not yet supported in API
    -- For now, just clear the context
    if _G.ItemFarm then
        _G.ItemFarm.engage = nil
    end
end

return M
```

**Step 2: Commit**

```bash
git add scripts/modules/ItemFarmEngage.lua
git commit -m "feat(itemfarm-v3): add composable engage pipeline module"
```

---

## Task 4: Job Executor Module (`ItemFarmExecutor.lua`)

The inner SM: search -> travel -> engage -> loot -> store -> done.

**Files:**
- Create: `scripts/modules/ItemFarmExecutor.lua`

**Step 1: Create the executor module**

```lua
-- scripts/modules/ItemFarmExecutor.lua
-- Job Executor SM for ItemFarm v3
-- Manages single job execution: search -> travel -> engage -> loot -> store -> done

local string = string
local ipairs = ipairs

local function require_module(name)
    local paths = { "scripts.modules." .. name, "modules." .. name, name }
    for _, p in ipairs(paths) do
        local ok, res = pcall(require, p)
        if ok then return res end
    end
    error("ItemFarmExecutor: cannot load " .. name)
end

local MudNav = require_module("MudNav")
local MudLoot = require_module("MudLoot")
local ItemFarmEngage = require_module("ItemFarmEngage")

local M = {}

-- Helper: parse and send commands
local function send_cmds(str)
    for cmd in string.gmatch(str, "[^;]+") do
        cmd = cmd:match("^%s*(.-)%s*$")
        if cmd ~= "" then
            local count, actual = cmd:match("^(%d+)(%a.*)$")
            if count then
                for _ = 1, tonumber(count) do mud.send(actual) end
            else
                mud.send(cmd)
            end
        end
    end
end

-- Deep merge: b overrides a, field by field
local function deep_merge(a, b)
    local result = {}
    for k, v in pairs(a) do
        if type(v) == "table" and type(b[k]) == "table" then
            result[k] = deep_merge(v, b[k])
        else
            result[k] = v
        end
    end
    for k, v in pairs(b) do
        if result[k] == nil then
            result[k] = v
        elseif type(result[k]) ~= "table" or type(v) ~= "table" then
            result[k] = v
        end
    end
    return result
end

-- Start executing a job
-- on_done(reason): called when job finishes ("done" or "not_found" or "failed")
function M.start(job, defaults, on_done)
    -- Merge resources with defaults
    local merged_resources = deep_merge(defaults.resources or {}, job.resources or {})
    local merged_loot = deep_merge(defaults.loot or {}, job.loot or {})
    local merged_store = deep_merge(defaults.store or {}, job.store or {})

    -- Store context
    _G.ItemFarm.executor = {
        job = job,
        defaults = defaults,
        merged_resources = merged_resources,
        merged_loot = merged_loot,
        merged_store = merged_store,
        on_done = on_done,
    }

    local echo = function(msg)
        if _G.ItemFarm and _G.ItemFarm.echo then
            _G.ItemFarm.echo("[" .. job.name .. "] " .. msg)
        end
    end
    _G.ItemFarm.executor.echo = echo

    -- Build SM states
    local states = {
        searching = {
            enter = string.format([[
                _G.ItemFarm.executor.echo("Searching for target...")
                if %q ~= "quest" then mud.send("wa") end
                mud.send(%q)
            ]], job.search.type, job.search.cmd),
            timeout_secs = 3.0,
            timeout_goto = "not_found",
        },
        traveling = {
            enter = [[
                _G.ItemFarm.executor.echo("Traveling to target...")
                _G.ItemFarm.executor.do_travel()
            ]],
        },
        engaging = {
            enter = [[
                _G.ItemFarm.executor.echo("Engaging target...")
                _G.ItemFarm.executor.do_engage()
            ]],
        },
        looting = {
            enter = [[
                _G.ItemFarm.executor.echo("Looting...")
                _G.ItemFarm.executor.do_loot()
            ]],
            timeout_secs = 10.0,
            timeout_goto = "storing",
        },
        storing = {
            enter = [[
                _G.ItemFarm.executor.echo("Going to storage...")
                _G.ItemFarm.executor.do_store()
            ]],
        },
        done = {
            enter = [[
                _G.ItemFarm.executor.echo("Job complete!")
                mud.emit("ifarm:job_done", {name = _G.ItemFarm.executor.job.name})
            ]],
        },
        not_found = {
            enter = [[
                _G.ItemFarm.executor.echo("Target not found")
                mud.emit("ifarm:job_failed", {name = _G.ItemFarm.executor.job.name, reason = "not_found"})
            ]],
        },
        failed = {
            enter = [[
                _G.ItemFarm.executor.echo("Job failed")
                mud.emit("ifarm:job_failed", {name = _G.ItemFarm.executor.job.name, reason = "error"})
            ]],
        },
    }

    local transitions = {
        { from = "searching",  event = "found",         to = "traveling" },
        { from = "traveling",  event = "arrived",       to = "engaging" },
        { from = "engaging",   event = "engage_done",   to = "looting" },
        { from = "engaging",   event = "engage_failed", to = "failed" },
        { from = "looting",    event = "loot_done",     to = "storing" },
        { from = "storing",    event = "store_done",    to = "done" },
        -- Failures
        { from = "searching",  event = "fail",          to = "failed" },
        { from = "traveling",  event = "fail",          to = "failed" },
    }

    -- Register event handlers
    M.register_event_handlers(job)

    -- Create SM
    mud.state_machine("itemfarm_job", {
        initial = "searching",
        states = states,
        transitions = transitions,
    })
end

function M.register_event_handlers(job)
    -- Search found
    local search_event = (job.search.type == "quest") and "ifarm:search_found" or "ifarm:search_found"
    mud.on(search_event, string.format([[
        local cur = mud.sm_current("itemfarm_job")
        if cur ~= "searching" then return end
        local d = data or {}
        -- For "locate" type, also need to match target
        if %q == "locate" then
            local line = d.line or ""
            local td = %s
            local matched = false
            if type(td) == "table" then
                for _, kw in ipairs(td) do
                    if string.find(line, kw, 1, true) then matched = true; break end
                end
            else
                matched = string.find(line, tostring(td), 1, true) ~= nil
            end
            if not matched then return end
        end
        mud.sm_transition("itemfarm_job", "found")
    ]], job.search.type, ItemFarmEngage.serialize_target(job.engage.target_display)), 0)

    -- Engage done/failed -> job SM transitions
    mud.on("ifarm:engage_done", [[
        local cur = mud.sm_current("itemfarm_job")
        if cur == "engaging" then
            mud.sm_transition("itemfarm_job", "engage_done")
        end
    ]], 0)

    mud.on("ifarm:engage_failed", [[
        local cur = mud.sm_current("itemfarm_job")
        if cur == "engaging" then
            mud.sm_transition("itemfarm_job", "engage_failed")
        end
    ]], 0)

    -- Job done/failed -> notify scheduler
    mud.on("ifarm:job_done", [[
        local fn = _G.ItemFarm.executor.on_done
        if fn then fn("done") end
    ]], 0)

    mud.on("ifarm:job_failed", [[
        local d = data or {}
        local fn = _G.ItemFarm.executor.on_done
        if fn then fn(d.reason or "error") end
    ]], 0)
end

-- Action implementations stored on _G.ItemFarm.executor

function M.setup_actions(job, defaults, merged_resources, merged_loot, merged_store)
    local ex = _G.ItemFarm.executor

    ex.do_travel = function()
        -- Pre-travel command
        if job.travel and job.travel.pre_cmd then
            send_cmds(job.travel.pre_cmd)
        end
        local path = job.travel and job.travel.path or "recall"
        MudNav.walk(path, function()
            mud.sm_transition("itemfarm_job", "arrived")
        end)
    end

    ex.do_engage = function()
        ItemFarmEngage.setup_helpers(job, merged_resources)
        ItemFarmEngage.start(job, merged_resources,
            function() -- on_done
                -- engage_done event is emitted by SM enter callback
            end,
            function() -- on_failed
                -- engage_failed event is emitted by SM enter callback
            end
        )
    end

    ex.do_loot = function()
        local items = merged_loot.items or {}
        local sac = merged_loot.sac
        if #items == 0 and not sac then
            mud.sm_transition("itemfarm_job", "loot_done")
            return
        end
        MudLoot.process_loot({
            items = items,
            sac = sac,
            loot_ground = true,
            fallback_blind = true,
        }, function()
            mud.sm_transition("itemfarm_job", "loot_done")
        end)
    end

    ex.do_store = function()
        local path = merged_store.path or "recall;3n;e"
        MudNav.walk(path, function()
            -- Remove nodrop + drop items
            local remove = merged_loot.remove_nodrop or {}
            for _, item in ipairs(remove) do
                mud.send("c 'remove n' " .. item)
            end
            local items = merged_loot.items or {}
            -- Delay drop if we had remove_nodrop
            local delay = #remove > 0 and 1.5 or 0
            mud.timer(delay, string.format([[
                if mud.sm_current("itemfarm_job") == "storing" then
                    local items = %s
                    for _, item in ipairs(items) do
                        mud.send("dro " .. item)
                    end
                    mud.sm_transition("itemfarm_job", "store_done")
                end
            ]], ItemFarmEngage.serialize_list(items)))
        end)
    end
end

function M.cleanup()
    ItemFarmEngage.cleanup()
    if _G.ItemFarm then
        _G.ItemFarm.executor = nil
    end
end

return M
```

**Step 2: Commit**

```bash
git add scripts/modules/ItemFarmExecutor.lua
git commit -m "feat(itemfarm-v3): add job executor SM module"
```

---

## Task 5: Main Entry Point (`itemfarm_v3.lua`)

Scheduler SM + external API + hook wiring.

**Files:**
- Create: `scripts/itemfarm_v3.lua`

**Step 1: Create the main entry point**

```lua
-- scripts/itemfarm_v3.lua
-- ItemFarm v3.0 - Event-Driven Auto Farm System
-- Architecture: Scheduler SM -> Executor SM -> Engage Pipeline SM

_G.ItemFarm = _G.ItemFarm or {}

local function require_module(name)
    local paths = { "scripts.modules." .. name, "modules." .. name, name }
    for _, p in ipairs(paths) do
        local ok, res = pcall(require, p)
        if ok then return res end
    end
    error("ItemFarm v3: cannot load " .. name)
end

local MudUtils = require_module("MudUtils")
local ItemFarmJobs = require_module("ItemFarmJobs")
local ItemFarmParser = require_module("ItemFarmParser")
local ItemFarmExecutor = require_module("ItemFarmExecutor")

local string = string
local ipairs = ipairs
local math = math

-- ===== State =====
_G.ItemFarm.state = {
    running = false,
    current_job = 1,
    jobs_checked = 0,
    loot_count = 0,
    show_echo = ItemFarmJobs.defaults.show_echo,
}

-- ===== Jobs reference =====
_G.ItemFarm.jobs = ItemFarmJobs.jobs
_G.ItemFarm.defaults = ItemFarmJobs.defaults

-- ===== Echo helpers =====
function _G.ItemFarm.echo(msg)
    if _G.ItemFarm.state.show_echo then
        mud.echo("[ItemFarm] " .. msg)
    end
end

function _G.ItemFarm.echo_force(msg)
    mud.echo("[ItemFarm] " .. msg)
end

-- ===== Job helpers =====
function _G.ItemFarm.job()
    return _G.ItemFarm.jobs[_G.ItemFarm.state.current_job]
end

local function count_active_jobs()
    local n = 0
    for _, j in ipairs(_G.ItemFarm.jobs) do
        if not j.disabled then n = n + 1 end
    end
    return n
end

local function find_next_active_job(from)
    local total = #_G.ItemFarm.jobs
    for i = 1, total do
        local idx = ((from - 1 + i) % total) + 1
        if not _G.ItemFarm.jobs[idx].disabled then
            return idx
        end
    end
    return nil
end

-- ===== Scheduler SM =====
local function create_scheduler_sm()
    local defaults = _G.ItemFarm.defaults

    mud.state_machine("itemfarm_scheduler", {
        initial = "idle",
        states = {
            idle = {
                enter = [[_G.ItemFarm._scheduler_idle()]],
            },
            executing = {
                enter = [[_G.ItemFarm._scheduler_executing()]],
            },
            rotating = {
                enter = [[_G.ItemFarm._scheduler_rotating()]],
            },
            waiting_respawn = {
                enter = string.format([[
                    _G.ItemFarm.echo("All targets not respawned, resting " .. %d .. "s...")
                    mud.send(%q)
                ]], defaults.poll_interval, defaults.rest_cmd or "sleep"),
                timeout_secs = defaults.poll_interval,
                timeout_goto = "idle",
            },
            stopped = {
                enter = [[_G.ItemFarm._scheduler_stopped()]],
            },
        },
        transitions = {
            { from = "idle",              event = "job_found",      to = "executing" },
            { from = "idle",              event = "no_active_jobs", to = "stopped" },
            { from = "executing",         event = "job_done",       to = "rotating" },
            { from = "executing",         event = "job_failed",     to = "rotating" },
            { from = "rotating",          event = "next_ready",     to = "idle" },
            { from = "rotating",          event = "all_checked",    to = "waiting_respawn" },
            { from = "waiting_respawn",   event = "wake",           to = "idle" },
            -- Emergency stop from any state
            { from = "idle",              event = "stop",           to = "stopped" },
            { from = "executing",         event = "stop",           to = "stopped" },
            { from = "rotating",          event = "stop",           to = "stopped" },
            { from = "waiting_respawn",   event = "stop",           to = "stopped" },
        },
    })
end

-- Scheduler state callbacks
function _G.ItemFarm._scheduler_idle()
    local s = _G.ItemFarm.state
    s.jobs_checked = 0

    -- Find first active job starting from current
    local idx = find_next_active_job(s.current_job - 1)
    if not idx then
        _G.ItemFarm.echo_force("No active jobs")
        mud.sm_transition("itemfarm_scheduler", "no_active_jobs")
        return
    end

    s.current_job = idx
    local j = _G.ItemFarm.job()
    _G.ItemFarm.echo("Starting job [" .. idx .. "] " .. j.name)
    mud.sm_transition("itemfarm_scheduler", "job_found")
end

function _G.ItemFarm._scheduler_executing()
    local s = _G.ItemFarm.state
    local j = _G.ItemFarm.job()
    local defaults = _G.ItemFarm.defaults

    ItemFarmExecutor.start(j, defaults, function(reason)
        if reason == "done" then
            s.loot_count = s.loot_count + 1
            _G.ItemFarm.echo("Loot count: " .. s.loot_count)
        end
        ItemFarmExecutor.cleanup()
        mud.sm_transition("itemfarm_scheduler", reason == "done" and "job_done" or "job_failed")
    end)

    -- Setup executor actions after SM is created
    local merged_res = _G.ItemFarm.executor and _G.ItemFarm.executor.merged_resources or {}
    local merged_loot = _G.ItemFarm.executor and _G.ItemFarm.executor.merged_loot or {}
    local merged_store = _G.ItemFarm.executor and _G.ItemFarm.executor.merged_store or {}
    ItemFarmExecutor.setup_actions(j, defaults, merged_res, merged_loot, merged_store)
end

function _G.ItemFarm._scheduler_rotating()
    local s = _G.ItemFarm.state
    local active = count_active_jobs()

    -- Mark current job as checked (only if not disabled)
    local j = _G.ItemFarm.job()
    if not j.disabled then
        s.jobs_checked = s.jobs_checked + 1
    end

    if s.jobs_checked >= active then
        _G.ItemFarm.echo("All jobs checked this round")
        mud.sm_transition("itemfarm_scheduler", "all_checked")
        return
    end

    -- Move to next active job
    local idx = find_next_active_job(s.current_job)
    if not idx then
        mud.sm_transition("itemfarm_scheduler", "all_checked")
        return
    end

    s.current_job = idx
    _G.ItemFarm.echo("Rotating to [" .. idx .. "] " .. _G.ItemFarm.jobs[idx].name)
    mud.sm_transition("itemfarm_scheduler", "next_ready")
end

function _G.ItemFarm._scheduler_stopped()
    local s = _G.ItemFarm.state
    s.running = false
    _G.ItemFarm.echo_force("Stopped. Total loot: " .. s.loot_count)
    MudUtils.stop_log()
end

-- ===== Register Parser Hook =====
MudUtils.register_hook("ItemFarm", function(line, clean_line, is_echo)
    if not _G.ItemFarm.state.running then return end
    ItemFarmParser.parse(line, clean_line, is_echo)
end)

-- ===== Register Global Emergency Handler =====
local function register_global_handlers()
    mud.on("ifarm:unexpected_combat", [[
        local cur = mud.sm_current("itemfarm_engage")
        -- Only trigger emergency if not already fighting
        if cur and cur ~= "fighting" and cur ~= "waiting_kill" then
            _G.ItemFarm.echo_force("EMERGENCY: unexpected combat! Fleeing...")
            mud.send("fl")
            mud.send("recall")
            local j = _G.ItemFarm.job()
            if j then j.disabled = true end
            mud.sm_transition("itemfarm_engage", "fail")
        end
    ]], -100)  -- High priority (low number = runs first)
end

-- ===== External API =====
function _G.ItemFarm.start()
    if _G.ItemFarm.state.running then
        _G.ItemFarm.echo_force("Already running")
        return
    end

    local s = _G.ItemFarm.state
    s.running = true
    s.loot_count = 0
    s.current_job = 1
    s.jobs_checked = 0

    _G.ItemFarm.echo_force("Starting ItemFarm v3.0 (" .. #_G.ItemFarm.jobs .. " jobs)")
    MudUtils.start_log("itemfarm")
    MudUtils.register_quest("ItemFarm", _G.ItemFarm.stop)

    -- Register buff fade rules from all jobs
    ItemFarmParser.clear_dynamic_rules()
    for _, j in ipairs(_G.ItemFarm.jobs) do
        if j.resources and j.resources.buffs then
            for _, b in ipairs(j.resources.buffs) do
                if b.fade_msg then
                    ItemFarmParser.add_fade_rule(b.fade_msg, b.indicator)
                end
            end
        end
    end

    register_global_handlers()

    -- Inventory check then start scheduler
    MudUtils.start_inventory_check(function()
        _G.ItemFarm.echo("Inventory check passed, starting scheduler...")
        create_scheduler_sm()
    end)
end

function _G.ItemFarm.stop()
    if not _G.ItemFarm.state.running then return end
    _G.ItemFarm.state.running = false

    -- Transition scheduler to stopped (will trigger cleanup)
    local cur = mud.sm_current("itemfarm_scheduler")
    if cur and cur ~= "stopped" then
        mud.sm_transition("itemfarm_scheduler", "stop")
    else
        _G.ItemFarm._scheduler_stopped()
    end
end

function _G.ItemFarm.status()
    local s = _G.ItemFarm.state
    _G.ItemFarm.echo_force("=== ItemFarm v3.0 Status ===")
    _G.ItemFarm.echo_force("Running: " .. (s.running and "yes" or "no"))
    _G.ItemFarm.echo_force("Loot count: " .. s.loot_count)

    if s.running then
        _G.ItemFarm.echo_force("Scheduler: " .. (mud.sm_current("itemfarm_scheduler") or "?"))
        _G.ItemFarm.echo_force("Executor:  " .. (mud.sm_current("itemfarm_job") or "-"))
        _G.ItemFarm.echo_force("Engage:    " .. (mud.sm_current("itemfarm_engage") or "-"))
        local j = _G.ItemFarm.job()
        if j then
            _G.ItemFarm.echo_force("Current job: [" .. s.current_job .. "] " .. j.name)
        end
    end

    _G.ItemFarm.echo_force("Jobs:")
    for i, j in ipairs(_G.ItemFarm.jobs) do
        local marker = (i == s.current_job and s.running) and " <" or ""
        local disabled = j.disabled and " [OFF]" or ""
        _G.ItemFarm.echo_force("  [" .. i .. "] " .. j.name .. " (" .. (j.engage and j.engage.mode or "?") .. ")" .. disabled .. marker)
    end
end

function _G.ItemFarm.toggle_job(index)
    local j = _G.ItemFarm.jobs[tonumber(index)]
    if not j then
        _G.ItemFarm.echo_force("Job not found: " .. tostring(index))
        return
    end
    j.disabled = not j.disabled
    _G.ItemFarm.echo_force("[" .. index .. "] " .. j.name .. ": " .. (j.disabled and "DISABLED" or "ENABLED"))
end

function _G.ItemFarm.toggle_echo()
    _G.ItemFarm.state.show_echo = not _G.ItemFarm.state.show_echo
    _G.ItemFarm.echo_force("Echo: " .. (_G.ItemFarm.state.show_echo and "ON" or "OFF"))
end

function _G.ItemFarm.reload()
    package.loaded["scripts.itemfarm_v3"] = nil
    package.loaded["scripts.modules.ItemFarmJobs"] = nil
    package.loaded["scripts.modules.ItemFarmParser"] = nil
    package.loaded["scripts.modules.ItemFarmExecutor"] = nil
    package.loaded["scripts.modules.ItemFarmEngage"] = nil
    require("scripts.itemfarm_v3")
end

-- ===== Init =====
_G.ItemFarm.echo_force("=== ItemFarm v3.0 (Event-Driven) ===")
_G.ItemFarm.echo_force("Commands: ItemFarm.start() / .stop() / .status() / .toggle_job(n) / .toggle_echo()")
_G.ItemFarm.echo_force("Jobs: " .. #_G.ItemFarm.jobs)
for i, j in ipairs(_G.ItemFarm.jobs) do
    local disabled = j.disabled and " [OFF]" or ""
    _G.ItemFarm.echo_force("  [" .. i .. "] " .. j.name .. " (" .. (j.engage and j.engage.mode or "?") .. ")" .. disabled)
end

return _G.ItemFarm
```

**Step 2: Commit**

```bash
git add scripts/itemfarm_v3.lua
git commit -m "feat(itemfarm-v3): add main entry point with scheduler SM and external API"
```

---

## Task 6: Integration Testing (Manual)

Since this is a MUD client script, testing is done live in the MUD client.

**Step 1: Load the script**

In MUD client, run: `lua require("scripts.itemfarm_v3")`

Expected: See init banner with job list, no errors.

**Step 2: Test status command**

Run: `lua ItemFarm.status()`

Expected: Shows "Running: no", all jobs listed with modes.

**Step 3: Test toggle_job**

Run: `lua ItemFarm.toggle_job(1)`

Expected: Job 1 toggles disabled state.

**Step 4: Test start/stop cycle**

Run: `lua ItemFarm.start()`

Expected: See "Starting ItemFarm v3.0", inventory check, scheduler begins searching.

Run: `lua ItemFarm.stop()`

Expected: See "Stopped. Total loot: 0", clean shutdown.

**Step 5: Test with summon mode job (某校生)**

Ensure job 3 (某校生) is enabled, start ItemFarm. Observe:
1. Scheduler enters `idle` -> finds job -> `executing`
2. Executor: `searching` -> sends `c loc id` -> detects "攜帶著" -> `traveling`
3. Travel to recall -> `engaging`
4. Engage: `summoning` (MudCombat.safe_summon) -> `fighting` -> `kill_confirmed`
5. `looting` -> `storing` -> `done`
6. Scheduler: `rotating` -> back to `idle`

**Step 6: Test with charm mode job (動靈帽)**

Enable only the charm job, observe:
1. Search -> travel (long path) -> engage
2. `verify_mob` -> `charming` -> `leading` -> `waiting_kill` -> `kill_confirmed`
3. Loot -> store -> done

**Step 7: Commit final state**

```bash
git add -A
git commit -m "feat(itemfarm-v3): complete v3.0 event-driven rewrite"
```

---

## Task 7: Fix `mud.on()` / `mud.off()` Handler ID Mismatch (Rust-side)

**Important:** This is a prerequisite fix. Currently `mud.on()` returns a Lua-local counter that may not match the EventBus's real handler ID. This makes `mud.off()` unreliable.

**Files:**
- Modify: `crates/mudcore/src/script.rs:584-620` (on/once/off functions)

**Step 1: Fix `mud.on()` to return the real handler ID**

The fix: instead of returning a local counter, store the registration and have session.rs return the real ID back to Lua. However, since the current architecture is deferred (Lua collects registrations, Rust processes them later), we cannot return the real ID synchronously.

**Alternative approach:** Make the EventBus ID assignment deterministic by using a shared counter. Add a field to MudContext:

In `script.rs`, replace the placeholder counter with the EventBus's `next_id`:
- Add `event_next_id: u64` to `ScriptEngine` state
- Before each execution, sync `event_next_id` from session's `event_bus.next_id`
- `mud.on()` returns `event_next_id + offset` (matching what EventBus will assign)

This is a Rust change that should be done before Tasks 1-5 for reliable handler cleanup. If not done, all event handlers in Tasks 2-5 use `sm_current()` guards (which is the fallback pattern already in the design).

**Step 2: (If fix is deferred) Verify sm_current() guard pattern is sufficient**

All event handlers in the codebase already check `mud.sm_current()` before acting, so the handler ID mismatch does not cause incorrect behavior — just potential handler leaks (handlers that never get removed). This is acceptable for v3.0 initial release.

**Step 3: Commit**

If Rust fix is made:
```bash
git add crates/mudcore/src/script.rs
git commit -m "fix: sync event handler IDs between Lua and EventBus"
```

---

## Summary: Execution Order

| Task | Description | Dependencies |
|------|-------------|-------------|
| 7 | Fix handler ID mismatch (optional, Rust) | None |
| 1 | Job definitions data file | None |
| 2 | Event parser module | None |
| 3 | Engage pipeline module | None |
| 4 | Job executor module | Task 3 |
| 5 | Main entry point | Tasks 1-4 |
| 6 | Integration testing | Task 5 |

Tasks 1, 2, 3 can run in parallel. Task 7 is optional but recommended.
