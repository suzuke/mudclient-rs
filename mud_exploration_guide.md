# MUD 探索指南 (void7777.ddns.net:7777)

## 角色資訊
- **名稱**: Clauden (冒險者職業)
- **Session**: 3
- **等級**: 5 (已存檔)
- **屬性**: STR 22, INT 14, WIS 14, DEX 22, CON 18

## API 操作
- 發送指令: `POST /api/send` body: `{"command":"...","session":"3"}`
- 讀取訊息: `GET /api/messages?count=N&session=3`
- 清除訊息: `DELETE /api/messages?session=3`
- 管理 trigger: `POST /api/trigger` body: `{"action":"add/remove","name":"...","pattern":"...","pattern_type":"contains","script":"..."}`
- **重要**: 需要定期發送空指令 `""` 來觸發 MUD game tick（HP/MP/MV 恢復）

## 城市導航
```
市中心 (recall 回到這裡)
  ↓ s
科學之路
  ↓ s
和平大道
  ↓ w
學校外
  ↓ u
學校入口 ← 核心路口
  ↑ n: 便利商店 (買食物/飲料)
  ↑ u: 雕像館 (學技能/法術)
  ↓ s: 訓練場入口 (H-300 buff 機器人)
     ↓ d: 體能訓練場 (練級區)
```

## 雕像館完整地圖
```
大堂
  ↑ n: 冒險者の館 (scan, create water/food, create spring, continual light, fly, locate object, identify, recharge item, ventriloquate)
  ↓ s: 軍人の館 (shoot, gunner, buttock, hanging)
  ← w: 通道
        ↑ n: 騎士の館 (enhanced damage, second attack, dodge, kick, bash, parry, rescue, disarm)
        ↓ s: 小偷の館
        ← w: 通道
              ↑ n: 牧師の館 (cure light, cure serious, heal, refresh, bless, protection, sanctuary, cure blindness)
              ↓ s: 巫師の館 (spellmaster, dark space, curse, magic clock, blindness, sleep, charm person, illusion, poison, weaken, confuse)
              ← w: 魔劍士の館 (swordmaster, shadowslash, fusislash, shanyanslash, thunderslash, lifeslash, soulslash, fireslash, flameslash, nuclearslash, lightningslash)
  → e: 通道
        ↑ n: 拳法家の館 (bare fist, ejinjing, saulinfist, lohanfist, tigerfist, bannofist, handsfist, kingonfinger, wushangfinger)
        ↓ s: 盜帥の館
        → e: 通道
              ↑ n: 劍士の館
              ↓ s: 道士の館 (mountainslash, changjunslash, locate object, identify, enchant weapon, remove nodrop, time power)
```

## 已學技能 (Level 5 時)
| 技能 | 熟練度 | 來源 |
|------|--------|------|
| saulinfist (少林長拳) | 71% | 拳法家 |
| bare fist (拳力) | 57%+ | 拳法家 |
| ejinjing (易筋經) | 75% | 拳法家 |
| enhanced damage (加強破壞) | 69% | 騎士 |
| second attack (二段突刺) | 44% | 騎士 |
| dodge (閃躲) | 51% | 騎士 |
| scan (搜查) | 71% | 冒險者 |
| create food (製造食物術) | 81% | 冒險者 |
| create water (造水術) | 71% | 冒險者 |

## 戰鬥技巧

### 拳法施放
- **`f sau`** — 少林長拳，主動技能，消耗內力(PW)，傷害極高(600+)
- **`f loh`** — 羅漢拳（需學習，記憶量較大）
- **`kill <target>`** — 開始普通攻擊
- **`flee`** — 逃跑

### 攻擊企圖心
- **`agg 100`** — 設定最大攻擊企圖心，大幅提升攻擊頻率和傷害
- 設定為 100 後普通攻擊每回合約 43-45 傷害

### 戰鬥策略
1. 用 `scan` 偵查周圍房間，找**只有小型光球**的方向
2. `kill ball` 開始戰鬥
3. 在戰鬥中使用 `f sau` 施放少林長拳（等計時器到 [0] 時輸入）
4. 少林長拳一擊 600+ 可以直接擊殺小型/大型光球
5. 如果誤打大型光球，用 `flee` 逃跑
6. **注意**: `kill ball` 會打到房間裡第一個 "ball"，可能是大型光球

### 目標選擇
- **小型光球**: ~95-200 EXP, 101 金幣，安全目標
- **大型光球**: ~300+ EXP, 303 金幣，用 f sau 可一擊殺
- **A-11/A-12 機器人**: 不建議打（用途不明）
- **H-300 機器人**: 友方 NPC，施 buff

## Buff 系統

### H-300 機器人 Buff
- 位置: 訓練場入口
- 獲取方式: 在訓練場入口 `sleep`，等 H-300 施法
- Buff 種類:
  - 四度護甲術 (armor) — 防禦加成
  - 女神庇祐術 (bless) — 命中加成
  - 神靈呼喚術 — 再生效果
  - heal — 治療
- **注意**: 需要等待較長時間（每個 buff 之間間隔不定）
- 需要定期發送空指令 `""` 觸發 tick

## 補給

### 便利商店 (學校入口北方)
- 太空食品 (food): 11 金幣
- 可口可樂 (cola): 10 金幣
- 建議每次出發帶 3-5 份食物和飲料

### 法術造食
- `cast 'create food'` — 造乾糧 (20 MP)
- `cast 'create water'` — 造的物品不能喝，只能吃 (`eat magic`)

### 飢餓/口渴影響
- 飢餓口渴會阻止 MP 恢復
- 嚴重時 HP 恢復也受影響
- 定期檢查並補充

## 裝備
| 部位 | 裝備 |
|------|------|
| 頭部 | 螺絲帽 (cap/ring) |
| 脖子 | 平安符 (amulet) |
| 身體 | 流行T恤 (shirt) |
| 腿部 | 護膝 (leggings) |
| 腳部 | 球鞋 (sport boots) |
| 手臂 | 護臂 (sleeves) |
| 背部 | 披風 (cape) |
| 腰部 | 皮帶 (belt) |
| 左手腕 | 佛珠 (rosary) |
| 持有 | 螢光棒 (lighter) — **照明必需** |

- **太空罩 (space helmet)**: 不要戴！會造成「渾身不對勁」負面效果

## 重要指令
- `recall` — 回到市中心
- `score` — 查看角色狀態
- `i` — 查看背包
- `eq` — 查看裝備
- `scan` — 偵查周圍房間
- `look` — 查看當前房間
- `sleep` / `wake` — 睡覺/起床（恢復 HP/MP/MV）
- `spells` — 查看可學法術
- `skills` — 查看可學技能
- `prac <skill>` — 練習技能（需要在雕像旁，消耗金幣）
- `learn statue <skill>` — 學習新技能（需要在對應雕像旁，消耗記憶點數）

## 訓練場地圖特性
- 訓練場是一個 grid 型多房間區域
- 光球和機器人會在房間間移動
- 某些房間是暗的，需要持有螢光棒照明
- 有些小型光球是 aggressive（進入房間自動攻擊）
- 大型光球也會移動但不一定 aggressive

## 已知問題和注意事項
1. `kill ball` 打到的是房間裡**第一個** ball，可能是大型光球
2. 沒有 buff 時傷害很低（大部分攻擊 miss）
3. `create water` 製造的物品無法 `drink`，要用 `eat magic` 吃掉
4. 死亡後裝備會掉落在死亡地點，需要回去撿
5. MV（移動力）走路會消耗，回復需要休息
6. PW（內力）使用拳法會消耗，自然恢復或休息恢復

## 下一步建議
- 學習 lohanfist（羅漢拳）— 需要更多記憶點數（目前 110）
- 練習 saulinfist 到更高熟練度
- 探索訓練場下方（有氣流描述暗示有下層）
- 嘗試其他練級區域
- 學習 armor 法術（自己施加護甲）
- 練習 second attack 和 dodge 到更高熟練度
