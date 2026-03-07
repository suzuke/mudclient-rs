# QuestEngine Design

## Problem

Quest scripts like `poker_quest.lua` are 600+ lines of imperative code mixing navigation, combat, loot, and state management. Every new quest requires duplicating this complexity. The goal: enable a workflow where the user manually solves a quest once, provides the log to Claude, and gets a working quest script auto-generated.

## Solution

A pure Lua declarative `QuestEngine` module (`scripts/modules/QuestEngine.lua`, ~400 lines) that orchestrates existing modules (MudNav, MudExplorer, MudCombat, MudLoot, MudUtils).

## Step Types

| Type | Purpose | Key Fields |
|------|---------|------------|
| `navigate` | Move along a fixed path | `path`, `on_fail` |
| `hunt` | Explore area to find and kill target | `target`, `attack_cmd`, `loot` |
| `give` | Give item to NPC | `item`, `npc` |
| `say` | Say something to trigger quest progression | `text`, `expect` |
| `interact` | Custom command with expected response | `cmd`, `expect`, `retry` |

## Path Format (Mixed)

Paths support three element types in a single array:

```lua
path = {
    "s", "s", "w",                                          -- plain direction
    {cmd="u", id="abc123..."},                               -- ID-verified step
    {action="push stone", expect="石頭移開了", cond="石頭擋住了去路"},  -- conditional/maze
}
```

- **String**: send as direction command
- **Table with `id`**: send `cmd`, verify arrival via `mud.get_current_room_id()`
- **Table with `action`**: execute `action`, optionally check `cond` first, wait for `expect` response

## Example: PokerQuest as Declarative Script

```lua
local QuestEngine = require("scripts.modules.QuestEngine")

QuestEngine.define("poker_quest", {
    recall_cmd = "recall",
    steps = {
        {type="navigate", name="go_to_mountain",
         path={"s","s","s","s","s","s","e","e","u","u","u",
               {cmd="u", id="20ce628e..."}}},

        {type="hunt", name="find_spade",
         target="Spade", attack_cmd="f sau",
         loot={items={"stone"}, sac=true}},

        {type="navigate", name="deliver_stone",
         path={"n","n","w","w","n","n",
               {cmd="w", id="6faf86d9..."}}},

        {type="give", name="give_stone",
         item="stone", npc="diamond"},

        {type="navigate", name="go_to_queen", path={...}},
        {type="say", name="talk_queen", text="say quest"},
        {type="navigate", name="go_to_palace", path={...}},
        {type="interact", name="finish",
         cmd="give heart queen", expect="完成"},
    }
})

QuestEngine.run("poker_quest")
```

~20-30 lines vs 600+ lines.

## Architecture

```
QuestEngine (orchestrator, pure Lua)
    |
    +-- Step Handlers (navigate/hunt/give/say/interact)
    |       |
    |       +-- MudNav.deliver_path()    -- fixed path walking
    |       +-- MudExplorer.explore()    -- DFS area search
    |       +-- MudCombat.*              -- combat automation
    |       +-- MudLoot.process_loot()   -- corpse looting
    |       +-- mud.send() / mud.collect_response()
    |
    +-- State Machine
    |       current_step index, retries, phase within step
    |
    +-- Error Recovery
            per-step on_fail (recall, retry, skip)
            global recall fallback
```

### Key Design Decisions

1. **Pure Lua** — no Rust changes needed. QuestEngine calls existing modules directly.
2. **Existing modules unchanged** — MudExplorer, MudCombat, MudLoot, MudNav continue to work independently for non-quest use cases (farming, training, etc.).
3. **Sequential step execution** — steps run one at a time with callback chaining. Each step handler calls `QuestEngine.advance()` on completion.
4. **Log-to-script workflow** — the declarative format is designed so Claude can parse a manual play log and generate the `steps` table automatically.

## Event Flow (hunt step example)

1. QuestEngine sets `MudExplorer.config.target = step.target`
2. Calls `MudExplorer.explore(callback)`
3. On target found → `MudCombat.start(step.attack_cmd)`
4. On combat done → `MudLoot.process_loot(step.loot, callback)`
5. On loot done → `MudExplorer.get_path_to_start()` → walk back
6. On return → `QuestEngine.advance()` → next step

## Non-Quest Scripts

Scripts like `autocast.lua`, `practice.lua`, `ticker.lua`, `itemfarm.lua` are not quest scripts — they handle farming, training, and utility tasks. These continue using MudCombat/MudExplorer/MudLoot directly. QuestEngine is an additional orchestration layer, not a replacement.

## Scope

- **Phase 1**: QuestEngine core + navigate/hunt/give/say/interact handlers
- **Phase 2**: Port `poker_quest.lua` to declarative format, validate end-to-end
- **Phase 3**: Port `ikkoku_quest.lua` and `smurf_quest.lua`
- **Future**: Claude log parser tool for auto-generating quest definitions
