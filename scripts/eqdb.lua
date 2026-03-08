-- ============================================================
-- EqDB - Equipment Database v1.0
-- ============================================================
-- Auto-record equipment from identify/lore, search, compare,
-- and weight-based recommendation.
--
-- Usage:
--   /lua dofile("eqdb.lua")
--   Then just use "c id <item>" or "c lore <item>" as usual.
--
-- Commands:
--   EqDB.search("belt")          -- keyword search
--   EqDB.slot("腰部")            -- list items by slot
--   EqDB.list()                  -- list all items
--   EqDB.info(物品編號)           -- single item detail
--   EqDB.scan_eq()               -- parse eq output to fill slots
--   EqDB.recommend({智力=10, 精神力=5})  -- weighted recommendation
--   EqDB.save()                  -- manual save
--   EqDB.reload()                -- reload script
-- ============================================================

local function require_module(name)
    local paths = { "scripts.modules." .. name, "modules." .. name, name }
    for _, p in ipairs(paths) do
        local ok, res = pcall(require, p)
        if ok then return res end
    end
    error("EqDB cannot load dependency: " .. name)
end

local MudUtils = require_module("MudUtils")

_G.EqDB = _G.EqDB or {}

local DB_PATH = "data/equipment_db.json"

-- ===== JSON Encode/Decode =====

local function json_escape(s)
    return s:gsub('\\', '\\\\'):gsub('"', '\\"'):gsub('\n', '\\n'):gsub('\r', '\\r'):gsub('\t', '\\t')
end

local function json_encode_value(val, indent, depth)
    indent = indent or "  "
    depth = depth or 0
    local pad = string.rep(indent, depth)
    local pad1 = string.rep(indent, depth + 1)

    if val == nil then return "null" end
    local t = type(val)
    if t == "boolean" then return tostring(val) end
    if t == "number" then
        if val == math.floor(val) then return string.format("%d", val) end
        return tostring(val)
    end
    if t == "string" then return '"' .. json_escape(val) .. '"' end
    if t == "table" then
        -- check if array
        local is_array = (#val > 0)
        if is_array then
            local parts = {}
            for _, v in ipairs(val) do
                table.insert(parts, json_encode_value(v, indent, depth + 1))
            end
            return "[" .. table.concat(parts, ", ") .. "]"
        end
        -- object
        local parts = {}
        local keys = {}
        for k in pairs(val) do table.insert(keys, k) end
        table.sort(keys, function(a, b) return tostring(a) < tostring(b) end)
        for _, k in ipairs(keys) do
            local v = val[k]
            table.insert(parts, pad1 .. '"' .. json_escape(tostring(k)) .. '": ' .. json_encode_value(v, indent, depth + 1))
        end
        if #parts == 0 then return "{}" end
        return "{\n" .. table.concat(parts, ",\n") .. "\n" .. pad .. "}"
    end
    return "null"
end

local function json_encode(tbl)
    -- top-level: object keyed by item_id
    local parts = {}
    local keys = {}
    for k in pairs(tbl) do table.insert(keys, k) end
    table.sort(keys, function(a, b) return tostring(a) < tostring(b) end)
    for _, k in ipairs(keys) do
        local v = tbl[k]
        table.insert(parts, '  "' .. json_escape(tostring(k)) .. '": ' .. json_encode_value(v, "  ", 1))
    end
    if #parts == 0 then return "{}" end
    return "{\n" .. table.concat(parts, ",\n") .. "\n}"
end

-- Minimal JSON decoder
local function json_decode(str)
    if not str or str == "" then return {} end
    -- Use Lua pattern-based parsing for our known structure
    local fn, err = load("return " .. str:gsub('%[', '{'):gsub('%]', '}'):gsub('"([^"]-)":', '["%1"]='):gsub(':null', ':nil'):gsub(':true', ':true'):gsub(':false', ':false'))
    if fn then
        local ok, result = pcall(fn)
        if ok and type(result) == "table" then return result end
    end
    -- Fallback: try with a safer approach
    -- Replace JSON syntax with Lua table syntax
    local lua_str = str
    lua_str = lua_str:gsub('"([^"]-)":', function(k) return '["' .. k .. '"]=' end)
    lua_str = lua_str:gsub('%[%s*(%d)', '{ %1')
    lua_str = lua_str:gsub('(%d)%s*%]', '%1 }')
    lua_str = lua_str:gsub(':null', '=nil')
    lua_str = lua_str:gsub(':true', '=true')
    lua_str = lua_str:gsub(':false', '=false')
    lua_str = lua_str:gsub(':', '=')
    lua_str = lua_str:gsub('%[', '{')
    lua_str = lua_str:gsub('%]', '}')
    local fn2 = load("return " .. lua_str)
    if fn2 then
        local ok2, result2 = pcall(fn2)
        if ok2 and type(result2) == "table" then return result2 end
    end
    mud.echo("[EqDB] JSON decode failed")
    return {}
end

-- ===== DB Load/Save =====

function EqDB._load_db()
    local file = io.open(DB_PATH, "r")
    if not file then
        EqDB.db = {}
        return
    end
    local content = file:read("*all")
    file:close()
    EqDB.db = json_decode(content)
end

function EqDB.save()
    local file = io.open(DB_PATH, "w")
    if not file then
        mud.echo("[EqDB] Cannot write to " .. DB_PATH)
        return
    end
    file:write(json_encode(EqDB.db))
    file:close()
    local count = 0
    for _ in pairs(EqDB.db) do count = count + 1 end
    mud.echo("[EqDB] Saved " .. count .. " items")
end

-- ===== Identify Output Parser =====

local SLOT_MAP = {
    ["照明器具"] = "照明器具",
    ["左手手指"] = "左手手指",
    ["右手手指"] = "右手手指",
    ["脖  子"]   = "脖子",
    ["身  體"]   = "身體",
    ["頭  部"]   = "頭部",
    ["腿  部"]   = "腿部",
    ["腳  部"]   = "腳部",
    ["手  部"]   = "手部",
    ["手  臂"]   = "手臂",
    ["左手拿著"] = "左手拿著",
    ["背  部"]   = "背部",
    ["腰  部"]   = "腰部",
    ["左手腕"]   = "左手腕",
    ["右手腕"]   = "右手腕",
    ["右手拿著"] = "右手拿著",
    ["握在手上"] = "握在手上",
    ["戴在胸前"] = "戴在胸前",
    ["坐  騎"]   = "坐騎",
}

-- Map wear confirmation messages to slots
local WEAR_SLOT_MAP = {
    ["穿在你的身上"]   = "身體",
    ["戴在你的頭上"]   = "頭部",
    ["穿在你的腿上"]   = "腿部",
    ["穿在你的腳上"]   = "腳部",
    ["戴在你的手上"]   = "手部",
    ["穿在你的手臂上"] = "手臂",
    ["繫在你的腰上"]   = "腰部",
    ["繫在你的腰間"]   = "腰部",
    ["披在你的背上"]   = "背部",
    ["戴在你的左手指上"] = "左手手指",
    ["戴在你的右手指上"] = "右手手指",
    ["戴在你的脖子上"] = "脖子",
    ["戴在你的左手腕上"] = "左手腕",
    ["戴在你的右手腕上"] = "右手腕",
    ["佩戴在你的胸前"] = "戴在胸前",
    ["握在你的右手當武器"] = "右手拿著",
    ["握在你的左手當武器"] = "左手拿著",
    ["拿在你的手上"]   = "握在手上",
}

-- Map type to default slot (for unique-slot types)
local TYPE_SLOT_MAP = {
    ["武器"]     = "右手拿著",
    ["胸章"]     = "戴在胸前",
    ["照明器具"] = "照明器具",
}

function EqDB._parse_identify(lines)
    if not lines or #lines == 0 then return nil end

    local item = {}

    for _, raw_line in ipairs(lines) do
        local line = raw_line:match("^%s*(.-)%s*$") or raw_line -- trim

        -- Line 1: 物品'keyword' 類別:TYPE, 特別旗標:FLAGS.
        local kw, tp, flags = line:match("^物品'(.-)' 類別:(.-),%s*特別旗標:(.-)%.$")
        if kw then
            item.keyword = kw
            item.type = tp
            item.flags = flags:match("^%s*(.-)%s*$") or flags
            item.no_keep = (item.flags:find("無法保留的") ~= nil)
            -- Alignment restrictions from flags
            item.align = { good = true, neutral = true, evil = true }
            if item.flags:find("反邪惡物") then item.align.evil = false end
            if item.flags:find("反神聖物") then item.align.good = false end
            if item.flags:find("反中立物") then item.align.neutral = false end
            -- Gender restrictions from flags
            item.gender = { male = true, neutral = true, female = true }
            if item.flags:find("反男性") then item.gender.male = false end
            if item.flags:find("反女性") then item.gender.female = false end
            if item.flags:find("反中性") then item.gender.neutral = false end
            -- default slot from type
            if TYPE_SLOT_MAP[tp] then
                item.slot = TYPE_SLOT_MAP[tp]
            end
        end

        -- Line 2: 重量 N, 價值 N, 最低使用等級 N, 物品編號: N.
        local w = line:match("^重量%s+(%d+),")
        if w then
            item.weight = tonumber(w)
        end

        -- Weapon damage: 可造成 N 至 N 不等的傷害力(平均傷害力 N).
        local dmin, dmax, davg = line:match("^可造成%s+(%d+)%s+至%s+(%d+)%s+不等的傷害力%(平均傷害力%s+(%d+)%)%.$")
        if dmin then
            item.damage = { min = tonumber(dmin), max = tonumber(dmax), avg = tonumber(davg) }
        end

        -- Hidden magic
        if line:find("附有隱藏魔力") then
            item.hidden_magic = true
        end

        -- Affects: 影響 ATTR N 點.
        local attr, val = line:match("^影響%s+(.-)%s+([%-]?%d+)%s+點%.$")
        if attr then
            item.affects = item.affects or {}
            item.affects[attr] = tonumber(val)
        end
    end

    if not item.keyword then return nil end

    item.updated_at = os.date("%Y-%m-%d")
    return item
end

-- ===== Hook: Auto-capture identify/lore output =====

-- Track the display name from "你口誦古老的法術咒語" or wear messages
EqDB._last_identify_name = nil

function EqDB._setup_hooks()
    MudUtils.unregister_hook("EqDB")

    MudUtils.register_hook("EqDB", function(line, clean_line)
        -- Use clean_line (ANSI stripped, \r already trimmed by Rust)
        line = clean_line or line

        -- Detect identify output start
        if line:match("^物品'.-' 類別:") then
            EqDB._collecting = true
            EqDB._collect_buf = { line }
            EqDB._collect_blank_count = 0
            return
        end

        -- Continue collecting identify output (skip blank lines, stop on prompt)
        if EqDB._collecting then
            if line:match("^%(hp") or line:match("^%(%d") then
                -- Prompt = end of identify output
                EqDB._collecting = false
                EqDB._process_collected(EqDB._collect_buf)
                EqDB._collect_buf = nil
                return
            elseif line == "" then
                -- Blank lines are normal between identify lines, skip them
                EqDB._collect_blank_count = (EqDB._collect_blank_count or 0) + 1
                if EqDB._collect_blank_count > 5 then
                    -- Too many blanks, abort
                    EqDB._collecting = false
                    EqDB._process_collected(EqDB._collect_buf)
                    EqDB._collect_buf = nil
                end
                return
            else
                EqDB._collect_blank_count = 0
                table.insert(EqDB._collect_buf, line)
            end
            return
        end

        -- Detect wear/wield messages for slot mapping
        -- Format: 你把 NAME 穿在你的身上.
        local worn_name, wear_action = line:match("^你把%s+(.-)%s+(穿在.+%.)$")
        if not worn_name then
            worn_name, wear_action = line:match("^你把%s+(.-)%s+(戴在.+%.)$")
        end
        if not worn_name then
            worn_name, wear_action = line:match("^你把%s+(.-)%s+(繫在.+%.)$")
        end
        if not worn_name then
            worn_name, wear_action = line:match("^你把%s+(.-)%s+(披在.+%.)$")
        end
        if not worn_name then
            worn_name, wear_action = line:match("^你把%s+(.-)%s+(握在.+%.)$")
        end
        if not worn_name then
            worn_name, wear_action = line:match("^你把%s+(.-)%s+(拿在.+%.)$")
        end
        if not worn_name then
            worn_name, wear_action = line:match("^你把%s+(.-)%s+(佩戴在.+%.)$")
        end

        if worn_name and wear_action then
            -- Remove trailing dot from action
            local action = wear_action:gsub("%.$", "")
            local slot = WEAR_SLOT_MAP[action]
            if slot then
                EqDB._update_slot_by_name(worn_name, slot)
            end
        end
    end)
end

function EqDB._process_collected(lines)
    local item = EqDB._parse_identify(lines)
    if not item then return end

    local id = item.keyword
    local existing = EqDB.db[id]

    -- Preserve slot from existing entry if we don't have a new one
    if existing and existing.slot and not item.slot then
        item.slot = existing.slot
    end
    -- Preserve name from existing
    if existing and existing.name and not item.name then
        item.name = existing.name
    end

    -- Check pending slot from wear message (matched by display name)
    if EqDB._pending_slot and not item.slot then
        item.slot = EqDB._pending_slot.slot
        item.name = item.name or EqDB._pending_slot.name
        EqDB._pending_slot = nil
    end

    EqDB.db[id] = item
    EqDB.save()

    -- Store display_name→keyword mapping for wear detection
    EqDB._name_to_id = EqDB._name_to_id or {}
    if item.name then
        EqDB._name_to_id[item.name] = id
    end

    -- Display
    mud.echo("[EqDB] Recorded: " .. (item.name or item.keyword) .. " (" .. (item.type or "") .. ")")
    if item.slot then
        mud.echo("  Slot: " .. item.slot)
    end
    if item.affects then
        local parts = {}
        for k, v in pairs(item.affects) do
            local sign = v >= 0 and "+" or ""
            table.insert(parts, k .. " " .. sign .. v)
        end
        mud.echo("  Affects: " .. table.concat(parts, ", "))
    end
    if item.no_keep then
        mud.echo("  [!] Cannot keep (無法保留)")
    end

    -- Auto-compare with same-slot items
    if item.slot then
        EqDB._auto_compare(item)
    end
end

function EqDB._update_slot_by_name(display_name, slot)
    -- Try name→id mapping first
    local mapped_id = (EqDB._name_to_id or {})[display_name]
    if mapped_id and EqDB.db[mapped_id] then
        local item = EqDB.db[mapped_id]
        local changed = false
        if item.slot ~= slot then item.slot = slot; changed = true end
        if not item.name then item.name = display_name; changed = true end
        if changed then
            EqDB.save()
            mud.echo("[EqDB] Slot updated: " .. display_name .. " -> " .. slot)
        end
        return
    end
    -- Fallback: search DB by name or keyword
    for id, item in pairs(EqDB.db) do
        if item.name == display_name or item.keyword == display_name then
            local changed = false
            if item.slot ~= slot then item.slot = slot; changed = true end
            if not item.name and display_name ~= item.keyword then item.name = display_name; changed = true end
            if changed then
                EqDB.save()
                mud.echo("[EqDB] Slot updated: " .. display_name .. " -> " .. slot)
            end
            return
        end
    end
    -- Store pending slot for future identify
    EqDB._pending_slot = { name = display_name, slot = slot }
end

-- ===== Auto-Compare =====

function EqDB._auto_compare(new_item)
    local slot = new_item.slot
    local rivals = {}
    for id, item in pairs(EqDB.db) do
        if item.slot == slot and id ~= new_item.keyword then
            table.insert(rivals, item)
        end
    end
    if #rivals == 0 then return end

    mud.echo("[EqDB] --- Compare with " .. #rivals .. " item(s) in [" .. slot .. "] ---")
    for _, rival in ipairs(rivals) do
        local diffs = {}
        -- Collect all attribute keys
        local all_keys = {}
        local new_aff = new_item.affects or {}
        local riv_aff = rival.affects or {}
        for k in pairs(new_aff) do all_keys[k] = true end
        for k in pairs(riv_aff) do all_keys[k] = true end

        -- Affects comparison
        local sorted_keys = {}
        for k in pairs(all_keys) do table.insert(sorted_keys, k) end
        table.sort(sorted_keys)
        for _, k in ipairs(sorted_keys) do
            local diff = (new_aff[k] or 0) - (riv_aff[k] or 0)
            if diff ~= 0 then
                local sign = diff > 0 and "+" or ""
                table.insert(diffs, k .. " " .. sign .. diff)
            end
        end

        local rival_label = rival.name or rival.keyword
        if #diffs > 0 then
            mud.echo("  vs " .. rival_label .. ": " .. table.concat(diffs, ", "))
        else
            mud.echo("  vs " .. rival_label .. ": identical stats")
        end
    end
end

-- ===== Scan EQ (batch fill slots) =====

function EqDB.scan_eq()
    mud.echo("[EqDB] Scanning equipment...")
    mud.collect_response("eq", function(lines)
        _G.EqDB._on_eq_collected(lines)
    end)
end

function EqDB._on_eq_collected(lines)
    lines = lines or {}
    local updated = 0

    for _, line in ipairs(lines) do
        -- Format: §SLOT§   (flags) DisplayName/keyword
        local raw_slot, rest = line:match("^§(.-)§%s+(.+)$")
        if raw_slot then
            local slot = SLOT_MAP[raw_slot]
            if slot then
                -- Extract keyword after the last /
                local keyword = rest:match("/([%w%s_]+)$")
                -- Extract display name: after flags, before /
                local display_name = rest:match("%)%s+(.-)%s*/") or rest:match("^(.-)%s*/")

                if keyword then
                    keyword = keyword:match("^%s*(.-)%s*$") -- trim
                end
                if display_name then
                    display_name = display_name:match("^%s*(.-)%s*$") -- trim
                end

                -- Find in DB and update slot + name
                if keyword then
                    for id, item in pairs(EqDB.db) do
                        if item.keyword == keyword then
                            local changed = false
                            if item.slot ~= slot then
                                item.slot = slot
                                changed = true
                            end
                            if display_name and item.name ~= display_name then
                                item.name = display_name
                                changed = true
                            end
                            if changed then updated = updated + 1 end
                            break
                        end
                    end
                end
            end
        end
    end

    if updated > 0 then
        EqDB.save()
    end
    mud.echo("[EqDB] Scan complete, updated " .. updated .. " item(s)")
end

-- ===== Search / Query =====

function EqDB.search(query)
    query = query:lower()
    local results = {}
    for id, item in pairs(EqDB.db) do
        local match = false
        if item.keyword and item.keyword:lower():find(query, 1, true) then match = true end
        if item.name and item.name:lower():find(query, 1, true) then match = true end
        if item.type and item.type:lower():find(query, 1, true) then match = true end
        if item.flags and item.flags:lower():find(query, 1, true) then match = true end
        if match then table.insert(results, { id = id, item = item }) end
    end
    EqDB._display_results(results, "Search: " .. query)
end

function EqDB.slot(slot_name)
    local results = {}
    for id, item in pairs(EqDB.db) do
        if item.slot == slot_name then
            table.insert(results, { id = id, item = item })
        end
    end
    EqDB._display_results(results, "Slot: " .. slot_name)
end

function EqDB.list()
    local results = {}
    for id, item in pairs(EqDB.db) do
        table.insert(results, { id = id, item = item })
    end
    table.sort(results, function(a, b)
        local sa = a.item.slot or "zzz"
        local sb = b.item.slot or "zzz"
        if sa ~= sb then return sa < sb end
        return (a.item.keyword or "") < (b.item.keyword or "")
    end)
    EqDB._display_results(results, "All Items")
end

function EqDB.info(keyword)
    local item = EqDB.db[keyword]
    if not item then
        mud.echo("[EqDB] Item not found: " .. keyword)
        return
    end
    mud.echo("========================================")
    mud.echo("  " .. (item.name or item.keyword))
    mud.echo("  Keyword: " .. (item.keyword or "?"))
    mud.echo("  Type: " .. (item.type or "?") .. "  Slot: " .. (item.slot or "unknown"))
    mud.echo("  Flags: " .. (item.flags or "none"))
    mud.echo("  Weight: " .. (item.weight or "?"))
    mud.echo("  Keep: " .. (item.no_keep and "NO" or "YES"))
    if item.damage then
        mud.echo("  Damage: " .. item.damage.min .. "-" .. item.damage.max .. " (avg " .. item.damage.avg .. ")")
    end
    if item.hidden_magic then
        mud.echo("  [Hidden Magic]")
    end
    if item.affects then
        for k, v in pairs(item.affects) do
            local sign = v >= 0 and "+" or ""
            mud.echo("  " .. k .. ": " .. sign .. v)
        end
    end
    mud.echo("========================================")
end

function EqDB.delete(keyword)
    local item = EqDB.db[keyword]
    if not item then
        mud.echo("[EqDB] Item not found: " .. keyword)
        return
    end
    local label = item.name or item.keyword
    EqDB.db[keyword] = nil
    -- Clean name mapping
    if EqDB._name_to_id then
        for k, v in pairs(EqDB._name_to_id) do
            if v == keyword then EqDB._name_to_id[k] = nil end
        end
    end
    EqDB.save()
    mud.echo("[EqDB] Deleted: " .. label)
end

function EqDB._display_results(results, title)
    if #results == 0 then
        mud.echo("[EqDB] " .. title .. ": no results")
        return
    end
    mud.echo("[EqDB] " .. title .. " (" .. #results .. " items)")
    mud.echo("--------------------------------------------------")
    for _, r in ipairs(results) do
        local item = r.item
        local parts = {}
        table.insert(parts, (item.name or item.keyword))
        table.insert(parts, "(" .. r.id .. ")")
        if item.slot then table.insert(parts, "{" .. item.slot .. "}") end
        if item.type then table.insert(parts, item.type) end
        if item.no_keep then table.insert(parts, "[!NoKeep]") end

        -- Brief affects
        if item.affects then
            local aff_parts = {}
            for k, v in pairs(item.affects) do
                local sign = v >= 0 and "+" or ""
                table.insert(aff_parts, k .. sign .. v)
            end
            if #aff_parts > 0 then
                table.insert(parts, table.concat(aff_parts, " "))
            end
        end
        mud.echo("  " .. table.concat(parts, " | "))
    end
    mud.echo("--------------------------------------------------")
end

-- ===== Recommend =====

function EqDB.recommend(weights)
    if not weights or type(weights) ~= "table" then
        mud.echo("[EqDB] Usage: EqDB.recommend({智力=10, 精神力=5, 護甲強度=3})")
        return
    end

    -- Collect all known slots
    local slot_items = {}
    for id, item in pairs(EqDB.db) do
        if item.slot then
            slot_items[item.slot] = slot_items[item.slot] or {}
            table.insert(slot_items[item.slot], { id = id, item = item })
        end
    end

    local function score_item(item)
        local s = 0
        local aff = item.affects or {}
        for attr, w in pairs(weights) do
            s = s + (aff[attr] or 0) * w
        end
        return s
    end

    mud.echo("========================================")
    mud.echo("[EqDB] Equipment Recommendation")
    mud.echo("  Weights: ")
    local w_parts = {}
    for k, v in pairs(weights) do
        table.insert(w_parts, k .. "=" .. v)
    end
    mud.echo("  " .. table.concat(w_parts, ", "))
    mud.echo("--------------------------------------------------")

    local total = 0
    local sorted_slots = {}
    for slot in pairs(slot_items) do table.insert(sorted_slots, slot) end
    table.sort(sorted_slots)

    for _, slot in ipairs(sorted_slots) do
        local items = slot_items[slot]
        -- Score and sort
        local scored = {}
        for _, entry in ipairs(items) do
            local s = score_item(entry.item)
            table.insert(scored, { id = entry.id, item = entry.item, score = s })
        end
        table.sort(scored, function(a, b) return a.score > b.score end)

        local best = scored[1]
        if best then
            total = total + best.score
            local label = best.item.name or best.item.keyword
            local detail = {}
            local aff = best.item.affects or {}
            for attr in pairs(weights) do
                if attr ~= "AC" and attr ~= "護甲強度" and aff[attr] then
                    local sign = aff[attr] >= 0 and "+" or ""
                    table.insert(detail, attr .. sign .. aff[attr])
                end
            end

            local runner_up = ""
            if #scored > 1 then
                runner_up = " (2nd: " .. (scored[2].item.name or scored[2].item.keyword) .. " score:" .. scored[2].score .. ")"
            end

            mud.echo(string.format("  [%s] %s  score:%.0f  %s%s",
                slot, label, best.score, table.concat(detail, " "), runner_up))
        end
    end

    mud.echo("--------------------------------------------------")
    mud.echo("  TOTAL SCORE: " .. total)
    mud.echo("========================================")
end

-- ===== Init =====

function EqDB.reload()
    package.loaded["eqdb"] = nil
    package.loaded["scripts.eqdb"] = nil
    dofile("scripts/eqdb.lua")
end

function EqDB.init()
    EqDB._load_db()
    EqDB._setup_hooks()

    local count = 0
    for _ in pairs(EqDB.db) do count = count + 1 end

    mud.echo("========================================")
    mud.echo("[EqDB] Equipment Database v1.0")
    mud.echo("  DB: " .. count .. " items loaded")
    mud.echo("  Auto-recording identify/lore output")
    mud.echo("  Commands: search/slot/list/info/scan_eq/recommend/save")
    mud.echo("========================================")

    _G.Help = _G.Help or {}
    _G.Help.registry = _G.Help.registry or {}
    _G.Help.registry["EqDB"] = {
        desc = "Equipment Database",
        usage = [[
  EqDB.search("belt")     -- keyword search
  EqDB.slot("腰部")       -- list by slot
  EqDB.list()             -- list all
  EqDB.info(ID)           -- item detail
  EqDB.scan_eq()          -- fill slots from eq
  EqDB.recommend({智力=10, 精神力=5})  -- recommend
  EqDB.save()             -- manual save
  EqDB.reload()           -- reload script]]
    }
end

EqDB.init()
