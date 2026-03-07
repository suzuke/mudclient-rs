# EqDB - Equipment Database Design

## Overview
Auto-record equipment attributes from identify/lore output, provide search, comparison, and weight-based recommendation.

## Features

### 1. Auto-Record (identify/lore interception)
- Use `mud.collect_response("c id <target>")` or hook `on_server_message` matching `物品'xxx' 類別:`
- Parse full identify output into structured data
- Save to `data/equipment_db.json`, keyed by `物品編號` (unique)
- Duplicate id overwrites/updates existing entry

### 2. Slot Auto-Detection
- Intercept wear messages: `穿在你的身上` → body, `戴在你的手上` → hands, etc.
- Associate with most recently identified/worn item keyword, write back to DB
- `EqDB.scan_eq()` — parse `eq` output to batch-fill slot info for all worn items

Slot mapping from wear/eq messages:
| eq output | slot key |
|-----------|----------|
| 照明器具 | light |
| 左手手指 | finger_l |
| 右手手指 | finger_r |
| 脖子 | neck (x2) |
| 身體 | body |
| 頭部 | head |
| 腿部 | legs |
| 腳部 | feet |
| 手部 | hands |
| 手臂 | arms |
| 左手拿著 | shield |
| 背部 | about |
| 腰部 | waist |
| 左手腕 | wrist_l |
| 右手腕 | wrist_r |
| 右手拿著 | wield |
| 握在手上 | hold |
| 戴在胸前 | badge |
| 坐騎 | mount |

### 3. Search / Query
- `EqDB.search("belt")` — keyword fuzzy search
- `EqDB.slot("腰部")` — list all items for a slot
- `EqDB.list()` — list all
- `EqDB.info(item_id)` — single item detail

### 4. Auto-Compare
- After identify completes, if item has slot info, auto-compare with same-slot items in DB
- Show diff in +/- format: `護甲強度 +2, 智力 -1, 躲避能力 +100`

### 5. Equipment Recommendation
- `EqDB.recommend({智力=10, 精神力=5, 護甲強度=3})`
- For each slot, score all items by weighted sum, pick highest
- Output full recommended loadout + total score

## Data Structure

```json
{
  "265174442": {
    "keyword": "et",
    "name": "倚天劍",
    "type": "武器",
    "slot": "右手拿著",
    "flags": "閃爍著 嗡嗡作響著 附著魔法的",
    "no_keep": false,
    "weight": 25,
    "armor_class": null,
    "damage": {"min": 70, "max": 80, "avg": 75},
    "hidden_magic": true,
    "affects": {"攻擊傷害力": 15, "攻擊命中率": 10, "體格": -2},
    "updated_at": "2026-03-07"
  }
}
```

`no_keep`: parsed from flags containing `無法保留的`

## Identify Output Format

### Armor:
```
物品'red belt' 類別:護甲, 特別旗標:閃爍著 附著魔法的 反邪惡物.
重量 7, 價值 37740, 最低使用等級 60, 物品編號: 264457350.
護甲強度 18.
影響 躲避能力 200 點.
影響 體格 4 點.
```

### Weapon:
```
物品'sword' 類別:武器, 特別旗標:閃爍著 附著魔法的 反邪惡物 無法保留的.
重量 70, 價值 43483, 最低使用等級 59, 物品編號: 772909365.
可造成 17 至 50 不等的傷害力(平均傷害力 33).
而且看起來似乎附有隱藏魔力!!
影響 攻擊傷害力 6 點.
影響 攻擊命中率 8 點.
```

### Other (badge, light, etc.):
```
物品'Alien Medal' 類別:胸章, 特別旗標:嗡嗡作響著.
重量 1, 價值 57200, 最低使用等級 60, 物品編號: 241075145.
影響 生命力 10 點.
影響 力量 2 點.
```

## Files
- `scripts/eqdb.lua` — main script (~400 lines)
- `data/equipment_db.json` — persistent storage

## Dependencies
- `MudUtils` (Hook Registry, safe_timer)
- `mud.collect_response` (collect identify output)
- `mud.save_json` / `mud.load_json` (persistence)
