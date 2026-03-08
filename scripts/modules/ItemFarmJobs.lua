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
    resources = { hp_threshold = 80, mp_threshold = 50 },
    store = { path = "recall;3n;e" },
    loot = { items = {}, sac = true, remove_nodrop = {} },
    rest_cmd = "sleep",
    poll_interval = 30,
    show_echo = true,
}

M.jobs = {
    {
        name = "商務間諜",
        disabled = false,
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
        disabled = false,
        search = { type = "quest", cmd = "q 28.boy" },
        travel = { path = "recall" },
        engage = {
            mode = "summon_charm",
            target = "boy",
            target_display = {"街頭混混"},
            attack = "c charm boy",
            summon_cmd = "c sum boy",
            pre_lead_cmds = "or all hi;or all recall",
            path_to_kill = "recall;n;n;n;e",
            pre_kill_cmds = "or all rem take;or all drop take",
            purge_path = "w;s;s;s;w;w;n",
        },
        loot = { items = {}, sac = false },
    },
    {
        name = "某校生",
        disabled = true,
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
            pre_lead_cmds = "or all hi;or all recall",
            path_to_kill = "recall;n;n;n;e",
            pre_kill_cmds = "or all drop hat",
            purge_path = "w;s;s;s;w;w;n",
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
