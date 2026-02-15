-- Simple Lua Test Runner
-- local lfs = require("lfs") -- Disabled to avoid dependency

-- Global assertions
function _G.assert_equal(expected, actual, msg)
    if expected ~= actual then
        error((msg or "") .. " Expected '" .. tostring(expected) .. "', got '" .. tostring(actual) .. "'")
    end
end

function _G.assert_deep_equal(expected, actual, msg)
    if type(expected) ~= "table" or type(actual) ~= "table" then
        if expected ~= actual then
             error((msg or "") .. " Expected '" .. tostring(expected) .. "', got '" .. tostring(actual) .. "'")
        end
        return
    end
    -- Simple shallow check for now, can expand later
    for k, v in pairs(expected) do
        if actual[k] ~= v then
             error((msg or "") .. " Key '"..k.."' mismatch. Expected " .. tostring(v) .. ", got " .. tostring(actual[k]))
        end
    end
end

-- Test structure
function _G.describe(name, fn)
    print("📦 " .. name)
    fn()
end

function _G.it(name, fn)
    local status, err = pcall(fn)
    if status then
        print("  ✅ " .. name)
    else
        print("  ❌ " .. name)
        print("     Error: " .. tostring(err))
    end
end

-- Runner logic
local test_files = {
    "scripts/tests/test_utils.lua",
    "scripts/tests/test_nav.lua",
    "scripts/tests/test_explorer.lua",
    "scripts/tests/test_combat.lua",
    "scripts/tests/test_mob_finder.lua",
    "scripts/tests/test_ikkoku_quest.lua",
    "scripts/tests/test_itemfarm.lua",
    "scripts/tests/test_ikkoku_integration_real.lua",
   -- "scripts/smurf_quest.lua" -- Syntax check only
}

print("Running Tests...")
for _, file in ipairs(test_files) do
    local chunk, err = loadfile(file)
    if chunk then
        chunk()
    else
        print("Failed to load " .. file .. ": " .. tostring(err))
    end
end
