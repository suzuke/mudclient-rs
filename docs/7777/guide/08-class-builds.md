# PTT 職業配法大全

> 本文彙整自 PTT MUD 看板（MUD_7777）上的職業分析文章，主要作者為 placewind 與 motonpom。
> 原始文章保留於 `docs/7777/ptt/` 目錄。

---

## 職業系列前言

**作者:** placewind (7777的笨玩家)
**時間:** 1999-09-25

寫在前面：我為何要寫職業分類？

事實上，玩 7777 二年來，練了 20 隻 char，相信自己應該有能力寫出職業的分野了。雖然我不像一些老玩家練過上百隻 char，也不像有的人能 sol all quests 4x 次，也不像有的玩家能研究出攻擊的威力怎麼算，或者找出 bug 大加牟利。但是我自信，兩年的投入 7777 總會有一些東西能多少留給新手吧。這便算是動機 :P

這一篇先寫我對 7777 職業的形成的原則及看法，以後數篇則是專論。

我想，關於職業，先前有很多可敬的玩家已經 post 過很多了。而我所偏重的，將是在於職業之間的差別，與同種職業的變化及功用，並將探討有哪些練法及變化是可以出現卻因為各種因素而未出現的。

事實上，我認為目前 7777 的各種職業大致已經平衡。所謂的平衡，並不是用數字衡量的平衡，而是偏向哲學意味的平衡。而這也是 7777 號稱技能自由化最大的意義。舉例來說，一般認為 dragonfist 是最強 skill，尤以 kai-bxr 為最，甚至有人主張 dragon 不該破聖光。事實上，skill 的強度不該只看它最後在電腦換算後所呈現出來的數字，而是：

1. 玩家練成 skill 所花的時間精力。bxr 不像 kni 或 out-wiz，skill 沒 all 99 就到處跑，而 dragon 之難練算是知名的。
2. 雖有 dragon，但 bxr 犧牲了到處晃的能力，只能跟團。
3. 幾乎沒有法術，遇上一些小法術足以要了 bxr 的命。

以此看來，bxr 擁有 7777 唯一能破 san 的能力不合理嗎？這只是我的舉例，out-wiz 目前也趨向合理的理由我會在後面文章中補敘。

所以，7777 有沒有最強的職業或技能？我的答案是：接近沒有。PK 場上要贏，最好是專門練一隻只能 PK 用的 char。可是，不管怎麼做，7777 總有一種職業或 skill 能夠克制它。一隻老玩家眼中認為極好用的 char（職業）教給一個不懂用法的人（不一定是新手 :P）很可能被評為廢物中的廢物 :P

---

## 騎士 (Knight)

### 總覽

> 原作者：placewind (7777的笨玩家)，1999/09/25
> 來源：MUD_7777 看板

騎士的個人分法是把它視為：有多段普通攻擊，與相當數量的魔法技能。所以 kni-kai-bxr 不算 kni，而大拳頭騎士（俠客）是 kni。

#### 一、非狂暴砍系

##### 騎士第一大型

基礎屬性：`str 30, int 14, wis 14, dex 30, con 22`

事實上，kni 因為各種武器的運用，con 常常無法達到 22，但 str 30 與 dex 30 幾乎是一定要配到的。要不要學 aid 看個人，不過以 7777 目前的情況，學比較好，擊中 mob 機會會增加。

| 型號 | 技能組合 | 說明 |
|------|---------|------|
| **(1) 正常型** | two-weapon + san + blitz + aid + windslice | 會有比較多的記憶點，學些解謎用的 skill or spell。一般所謂的 mag-kni 應該就是指這型。此型與 ten-kni 都屬目前 7777 kni 中最盛行的。 |
| **(2) out-kni** | two-weapon + blitz + windslice + out | 兩年前較為風行的 kni。因為記憶點的關係，竟然沒學 san。是否砍掉 windslice 改學 aid 就要看人。不過目前 out-kni 趨近廢物，因為 -dc 只有 -14xx 附近，在 out-wiz 的 -dc -17xx 都不太好用的情況下，out-kni 的下場可想而知。此外，連 aid 都不一定學到，在面對很多現在的 mob 上可以說無力再無力——**目前建議：別練，等你太閒時再練來玩。** |
| **(3) ten-kni** | ten (已含 windslice) + two-weapon + san + aid + blitz | 擁有 ten 這個群體技結合 1000 的傷害，使 kni 在追殺 mob 上更方便。不過最重要的是——tenslice 加的 HP 很多（約 300），使 kni 在配 eq 造成 con 只有 18 或 20 的情況下 HP 仍然可能破 3000，是 ten 最大的好處。 |
| **(4) refl-kni** | refl + two-weapon + blitz + san | 需要比較高的練 baby 技術。在對抗 PK 場的 kni 擁有較高勝率，以 kni 的防禦能力上算最高的。 |
| **(5) wiz-kni** | dark + two-weapon + blitz | 因為 out-wiz 的盛行而不常見到。但其實，在一些有補助魔法的 eq 出現後，使此型能節省一些記憶點。 |
| **(6) 大拳頭** | blitz + third-att + enchant-damage + bare fist | 一般普通攻擊威力比 kai-kni-bxr 要弱。kai-kni-bxr 一拳約 6xx × 4 = 24xx；大拳頭在 dr 8x 後應該能打 4xx × 4 = 16xx。特色：能學很多奇怪 skill，例如 full heal + soulsteal + disarm + san。逛街滿好用的，不用帶補血品，不用打 no save eq。可是大 mob 打不過、會 full 的 mob 打不過、會 parry 或 block 的 mob 打不過，而且只有普通 att 常常無法秒殺中型 mob。此型因為記憶點多，可考慮改學其他 skill 克服缺點，例如不學 full 學 dark，變化極多。 |

##### 騎士第二型（智力型／隱藏魔法型）

屬性配置：
- `str` 約 22–30（以能否拿得起想要用的重型 weapon 為準）
- `int` 必 30（int 高加的魔強也較多）
- `wis` 看個人喜好（wis 雖也有加魔強，但比起 int 來不多）
- `dex` 看個人喜好（因為 int 30 能夠用 ill，所以筆者設 dex 14）
- `con` 看個人喜好

目的是利用 weapon 的隱藏攻擊魔法，配合 illusion。據舊資料，用忘憂湯匙的核爆可以打西方。技能可能可以八段攻擊（讓 weapon 的隱藏魔力出現機會高），其他以能加強魔強的 spell 為主。

筆者實驗結果：拿忘憂湯匙揮出 8xx 的核爆（對象：倒楣的珠寶商）（沒學 spellmaster）。

#### 二、狂暴砍系

針對有拿 weapon 會用盾或 weapon 化解玩家攻擊所出現的 skill。能不被 mob 的盾或 weapon 化解攻勢。但目前又快要新加入 powerslice 的 skill，建議暫時新手不要練。一般普通認為這一系很差，要看 imm 改得如何。

#### 其他 kni 技能備註

- **Hunt**：追殺 mob，可是追過去 mob 的協調度變 0，而自己的協調度不變，往往追殺 mob 的人被 mob 殺。而且 mob 逃脫 hunt 的機率好像也太高了。建議 imm 改變 hunt 的協調度，同時 hunt 到不要用機率來算，而改用熟練度越高就能 hunt 幾格（被 hunt 後是要固定連逃幾格才能逃脫追殺）。P.S. 上次被 mob hunt，flee 後協調沒降，猜測已經改了，但繼續 hunt 與否似乎仍是機率。
- **Block、Disarm**：其實是不錯的 skill，但大多數是其他種職業拿去用，kni 本系的反而少有人練。

> P.S. 7777 練騎士的極多，所以與其看筆者的文章還不如去問一些更會用 kni 的玩家。

---

### 各型騎士詳細介紹

> 以下各型作者：motonpom (超級大流氓)，2001/02/10
> 來源：中友小站 (tfshs.twbbs.org)
> MOTONPOM 所編，轉載請先告知本人。

---

### 正統型 (Pure Knight)

- **Baby 類型**：Hp
- **個人評價**：★★☆☆☆

#### 屬性配置

|          | 力量 | 智力 | 睿智 | 速度 | 體格 |
|----------|:----:|:----:|:----:|:----:|:----:|
| changebody | 22 | 14 | 14 | 22 | 18 |
| train      |  8 |  0 |  0 |  8 |  4 |
| **總合**   | 30 | 14 | 14 | 30 | 22 |

> 本人是以最常看見的屬性為主

#### 個人建議必學技能

| 技能名稱 | 英文 |
|---------|------|
| 閃電奇襲式 | Blitz |
| 十字斬 | Tenslice |
| 左手格檔 | Block |
| 卸除武器 | Disarm |
| 解救 | Rescue |
| 聖光 | Sanctuary |

#### 分析

這種騎士現在應該沒有幾個人會練了吧，雖然 Hp 超多的，但是實用性 Spell 只能挑一、二個練，所以已經慢慢的淡忘在人們的記憶之中了。這種騎士有著一個很了不起的特點，就是解救，換句話說，就是幫人家挨槍子。

不過有這種騎士的人也是可以偶爾拿來現現寶，或是跟會背刺的小偷組隊——可以一個刺人後馬上解救他，而當小偷再次刺時就 flee 再回去解救他如此重覆，也可以說是另一種賤招。PK 用這招超強的。

> 只學格檔與 disarm 的也可以說是半正統型。

---

### 凌波型 (Outdo Knight)

- **Baby 類型**：Hp
- **個人評價**：★★★☆☆

#### 屬性配置

|          | 力量 | 智力 | 睿智 | 速度 | 體格 |
|----------|:----:|:----:|:----:|:----:|:----:|
| changebody | 22 | 14 | 14 | 22 | 18 |
| train      |  8 |  0 |  0 |  8 |  4 |
| **總合**   | 30 | 14 | 14 | 30 | 22 |

#### 個人建議必學技能

| 技能名稱 | 英文 |
|---------|------|
| 閃電奇襲式 | Blitz |
| 風嘯之斬 | Windslice |
| 凌波微步 | Outdo waver |
| 聖光 | Sanctuary |
| 去除魔法術 | Dispel magic |

#### 分析

凌波騎士，一種超級難用的職業，因為對如果不是玩的相當久時間的玩家而言，agg 的切換將會是一種擾人的問題，且 Hp 跟 Spell 少到可憐。唯一的好處是能夠檔住乾坤型跟格檔型所無法抵檔的 mob 特攻，但這型的騎士說真的要完全閃過特攻也是不可能的事，因為畢竟 dc 還是太少了（看過最高的凌波騎士 dc 是 16xx），且要能夠很快速的切換 agg。如果沒時常注意或是不想每次都切換 agg 的人，建議還是挑其他的騎士來練吧，不然自己都先煩死了。

---

### 遊俠 (Pln)

- **Baby 類型**：Hp
- **個人評價**：★★★★☆

#### 屬性配置

|          | 力量 | 智力 | 睿智 | 速度 | 體格 |
|----------|:----:|:----:|:----:|:----:|:----:|
| changebody | 22 | 14 | 14 | 22 | 18 |
| train      |  8 |  0 |  0 |  8 |  4 |
| **總合**   | 30 | 14 | 14 | 30 | 22 |

#### 個人建議必學技能

| 技能名稱 | 英文 |
|---------|------|
| 閃電奇襲式 | Blitz |
| 加強破壞 | Enhanced damage |
| 大地之斬 | Earthslice |
| 拳力 | Bare fist |
| 乾坤大挪移 | Reflexion |
| 降龍十八掌 | Dragonfist |
| 治療術 | Heal |
| 聖光 | Sanctuary |
| 去除魔法術 | Dispel magic |

> - 如無加強破壞則 att 威力會減弱
> - 降龍也可以換成天馬流星拳 (Cometfist)

#### 分析

這種騎士也可以說跟一種會 Spell 的拳法家差不多，他本身擁有騎士的初階技能——大地之斬與拳法家的乾坤大挪移和降龍十八掌，所以這種職業在四度來說叫做變種，但是他的實用性與防禦力非常的高，Hp 也不會輸給十字斬型的。

攻擊方式是採用 att 與大地之斬為主攻，降龍為輔助，所以也是很吃香。這型的騎士空手 att 一下在 mob 沒聖光時是 3xx ~ 4xx，且也能夠拿著黃金武器或是用喇嘛神劍來吸 mob 的精神力。

由於無法雙手持劍，自然 hr 會比一般的 Kni 低，這也是他的致命傷。如果今天遇到的只是普通的 mob 那也就算了，如果是遇到像 hell2 的石像鬼或是 JoJo3 的 dioo 就知道累了，att 二、三次有可能只中個幾下，所以不適合打 dc 高的 mob。

---

### 魔法絞殺型 (Hanging Knight)

- **Baby 類型**：Hp
- **個人評價**：★★★★★

#### 屬性配置

|          | 力量 | 智力 | 睿智 | 速度 | 體格 |
|----------|:----:|:----:|:----:|:----:|:----:|
| changebody | 22 | 14 | 14 | 22 | 18 |
| train      |  8 |  0 |  0 |  8 |  4 |
| **總合**   | 30 | 14 | 14 | 30 | 22 |

#### 個人建議必學技能

| 技能名稱 | 英文 |
|---------|------|
| 閃電奇襲式 | Blitz |
| 風嘯之斬 | Windslice |
| 絞殺 | Hanging |
| 治療術 | Heal |
| 聖光 | Sanctuary |
| 去除魔法術 | Dispel magic |

#### 分析

這是一種把魔法騎士所多出來的 300 點再加一些能棄則棄的技能所配出來的一種騎士，他的用法還是跟普通的騎士一樣，只是在目盲了 mob 後再加上 invis，多了一個能夠脫下武器再使用絞殺打 mob 而已。

攻擊火力夠猛，但是遇到會主動打人且會解目盲和看的到 invis 的 mob 對打時，還是乖乖的使用騎士的打法吧。

如果硬要說這一型缺點的話，就是少了一點實用的 Spell 與 Hanging 的攻擊火力不固定。個人滿喜歡這型騎士的！

---

### 智力型 (Int Knight)

- **Baby 類型**：Hp
- **個人評價**：★★★☆☆

#### 屬性配置

|          | 力量 | 智力 | 睿智 | 速度 | 體格 |
|----------|:----:|:----:|:----:|:----:|:----:|
| changebody | 18~22 | 20~22 | 14 | 20~22 | 14~18 |
| train      | 4~8   | 6~8   |  0 | 6~8   | 0~4   |
| **總合**   | 22~30 | 28~30 | 14 | 28~30 | 14~18 |

#### 個人建議必學技能

| 技能名稱 | 英文 |
|---------|------|
| 閃電奇襲式 | Blitz |
| 風嘯之斬 | Windslice |
| 治療術 | Heal |
| 聖光 | Sanctuary |
| 去除魔法術 | Dispel magic |
| 迷朧幻影 | Illusion |

> - 可以選擇性學習是否要魔力加持 (Spellmaster)，如果要學，則相對的必學闇黑結界 (Dark space)，或是考慮是否要學魔法結界 (Barrier)
> - 如果學了魔力加持，則風嘯之斬也可以不學只學大地之斬，和把聖光去掉轉學魔法結界，這樣就是 Int-Dark-Kni 了

#### 分析

此種騎士如果是 Evil 的，只要有一把繡花針在手，可以玩的非常高興，且攻擊力有時候可以一次風嘯就打到 8000 Hp 以上，是一種好玩的職業，完全是靠武器的法術來決定攻擊火力的。

缺點是 Hp 太少、如果心地是 Holy 的則還會多一個武器太少的缺點。而不管是 Holy 還是 Evil 的 Char，只要 str 沒超過 25 就不能夠拿黃金武器。

而如果是學了 Dark 的，則是 Hp 更少且實用 Spell 也會很少，但可以使用 Dark space 來讓 mob 無法使用法術補血或攻擊。但是有 Dark 的騎士整體來說還是沒辦法比凌波巫師來的好。

---

### 大拳頭 (Kai-Att)

- **Baby 類型**：Power
- **個人評價**：★★★★☆

#### 屬性配置

|          | 力量 | 智力 | 睿智 | 速度 | 體格 |
|----------|:----:|:----:|:----:|:----:|:----:|
| changebody | 22 | 14 | 14 | 22 | 18 |
| train      |  8 |  0 |  0 |  8 |  4 |
| **總合**   | 30 | 14 | 14 | 30 | 22 |

#### 個人建議必學技能

| 技能名稱 | 英文 |
|---------|------|
| 閃電奇襲式 | Blitz |
| 加強破壞 | Enhanced damage |
| 界王拳 | Kaioken |
| 降龍十八掌 | Dragonfist |

> 如無加強破壞則 att 威力會減弱

#### 分析

其實這就是一種 Bxr 的變種，只是因為他有閃電奇襲式，所以也可以把他分成是一種騎士。但這種騎士沒有 Spell，完全是以攻擊為主，攻擊是不靠武器的，可以瞬間打出爆一萬多的 Hp，威力超強，也可以打降龍，攻擊火力非常不錯。

缺點就是沒有自主能力，完全的依賴隊友的支援才有辦法生存，但還是大力推薦這種職業！

---

### 單手持武器型 (??? Knight)

- **Baby 類型**：Hp
- **個人評價**：★★☆☆☆

#### 屬性配置

|          | 力量 | 智力 | 睿智 | 速度 | 體格 |
|----------|:----:|:----:|:----:|:----:|:----:|
| changebody | 22 | 14 | 14 | 22 | 18 |
| train      |  8 |  0 |  0 |  8 |  4 |
| **總合**   | 30 | 14 | 14 | 30 | 22 |

#### 個人建議必學技能

| 技能名稱 | 英文 |
|---------|------|
| 閃電奇襲式 | Blitz |
| 加強破壞 | Enhanced damage |
| 大地之斬 | Earthslice |
| 準確攻擊 | Aid |
| 拳力 | Bare |
| 絞殺 | Hanging |
| 藏匿 | Hide |
| 治療術 | Heal |
| 聖光 | Sanctuary |
| 去除魔法術 | Dispel magic |

> - 如果學了獵殺 (Hunt) 則無法學閃電奇襲式，所以請自己選擇
> - 因為有學絞殺，所以才學藏匿來幫忙加強威力

#### 分析

其實如果不學 Hunt 的話，這種騎士跟遊俠很像；如果學了 Hunt 的話，那麼就只能夠 att 3 次了。

這種騎士應該沒什麼優點可以說，不過缺點倒是一堆：Hp 太少、hr 太低、攻擊力太弱。如果 mob 會主動解目盲且看的到 invis 與 hide 的話，那麼最強的攻擊絞殺都沒有用了。不過如果 mob 不會解又看不到你，那麼你倒是可以好好的體驗一下毀天的快感。

這種騎士說真的只比正統的好一點點，就是 Spell 比正統的多。不過整體還是不夠好，而如果往後狂暴砍順利開放的話，那麼到時候練可能會吃香很多（狂暴砍很猛）。

---

## 巫師 (Wizard)

### 淺談巫師

> 原作者：placewind (7777的笨玩家)，1999/09/26

原本，wiz 依照雕像館的分類就行了。可是除了核爆 wiz 與暴雷 wiz 之外其他的 wiz 實在沒什麼人在練在用。同時，目前 wiz 練成最大爭議是——追求最強威力還是實用卻威力不強？

筆者是站在實用的立場。因為 wiz 再強的威力都比不過 bxr 或 slr 或 thief，連 hp 都比不贏別人，所以跟團不好，所以希望大家破除只追求「哇，打起來好暴力唷」的迷思，因為實際上 wiz 在減低威力的同時會變得更強更好用 :P

但筆者介紹仍分成這兩系介紹。至於新 spells 與其他種 wiz（如雪嵐，...）因為沒練過所以也不知道如何分類。

#### 一、實用型

changebody 屬性：`int` 必 30，`wis` 最好在 26 以上（沒依據）。

一般配法：

| 屬性 | STR | INT | WIS | DEX | CON |
|------|-----|-----|-----|-----|-----|
| 數值 | 22 | 30 | 30 | 14 | 14 |

可以減少 STR 與 WIS 加到 CON 去。隨 skill 配法與 CON 不同，HP 約 1600~2200。

**(1) 核爆巫師**

- **技能：** Illusion + Nuclear + Sanctuary + Spellmaster + Dark + Dispel Magic
- 可以再考慮加學 Nuclear 之上的流星雨 Star Rain（群體技）
- 一般核爆（Nuclear）威力約 2000 左右
- **打法：** 上 Illusion 上 Sanctuary，Dispel Magic 掉 mob 的 Sanctuary，Dark mob，再用 Nuclear 打 mob
- **特色：** 最適合打會 Full Heal 或一堆怪法術的、不會主動攻擊玩家的 mob（或能 Blind 的 mob），像小少爺、阿拉雷、10 King，特別是東方不敗（enter hole 那個 area）:P（指單挑）
- **缺點：** HP 少，但因為有 Illusion，所以還好

**(2) 爆雷巫師**

- 威力略弱於核爆 wiz（少 100~200），但爆雷降臨比核爆多了不定出現的**麻痺效果**，讓人不能移動 :P
- 群體技實在太弱了，遠不及流星雨且還要看氣候
- 好處是爆雷降臨耗的 mana 比 Nuclear 少，所以爆雷 wiz 的 WIS 可以不用像核爆 wiz 那麼講究要 30
- 筆者把 WIS train 到 26，威力約 1600+。打起東方不敗卻比核爆 wiz 好打
- **技能：** Illusion + Fall Lightning + Sanctuary + Spellmaster + Dark + Dispel Magic
- 用法同核爆 wiz

> **P.S.** 新技能 Break-Dark 筆者發現 INT 30、WIS 26 下不能用。只確定 INT 30、WIS 30 下一定能用，其他 changebody 法能不能用希望有人能 post。

#### 二、暴力型巫師

筆者這方面所知不多，只知有人不學 Sanctuary、不學 Illusion，一昧以加強魔強（因為法術威力大小大部份都看魔力強度）的法術為重點——聽說威力也才 2500 附近。甚至沒學 Spellmaster（可以降低施法耗用法力），連 Dark 都......

他們的理由是跟團......但是 HP 太少（因為最強威力必在 INT 30、WIS 30 下出現）。

這樣的 wiz，筆者真的無言以對。目前 7777 跟團需要 Dark 多請 out-wiz。而筆者的想法是：wiz 在 7777 應該是能單挑那種大型卻不會主動攻擊玩家的機車型 mob。雖然 7777 的 wiz 角色似乎跟一般 MUD 裡 wiz 是跟團的重要角色不一樣，但——這只是 game 而且，誰說 wiz 就不能設成獨自修行的大魔法師 :P

---

### 凌波巫師及其變化

> 原作者：placewind (7777的笨玩家)，1999/09/25

首先先講一下 out-wiz 大致的源起。一開始應該是某個玩家想練一隻方便 Dark 的 char（因為 wiz 的 HP 實在太少），他發現如果 -dc 高（閃躲力）mob 會打不到，那如果有一隻 wiz 任何 mob 都打不到，那豈不是方便得很？於是他想出如果 DEX 30，身上穿的越少越好，-dc 就能達到最高。他的 char 據說是 `STR 14, INT 30, WIS 22, DEX 30, CON 14`，可是屬性都沒穿滿（因為 STR 14），只有 DEX 穿到 30，有 Outdo Waver + Dark + Illusion + Barrier + Sanctuary。

#### (1) INT 型凌波巫師

| 屬性 | STR | INT | WIS | DEX | CON |
|------|-----|-----|-----|-----|-----|
| 數值 | 22 | 30 | 14 | 30 | 14 |

- -dc 約破 -2000，HP 1400~1700，Mana 1700~2000+
- **主要技能：** Ear + Dark + Barrier + Soulsteal + Sanctuary + Blind + Outdo Waver + Illusion + Dispel Magic
- STR 高是為了穿更多 -dc 的 EQ，及帶更多的 Apple（補血）
- INT 30 是為了使用 Illusion（幻影朦朧）
- DEX 30 是為了讓 mob 打不到
- CON 14 是為了 -dc 高，則 mob 打不（太）到
- WIS 14 因為有 Soulsteal 可以吸 Mana
- 可以把 STR 22 改成 STR 20 或 18...，加到 CON 上以增加 HP

#### (2) CON 型凌波巫師

| 屬性 | STR | INT | WIS | DEX | CON |
|------|-----|-----|-----|-----|-----|
| 數值 | 22 | 14 | 14 | 30 | 30 |

- -dc 約 1700（破 -1650 應該是必要的）
- Mana 在 skill 全滿後應該有 1700 以上，HP 2600 以上
- **主要技能：** 不學 Illusion。Barrier 則看個人。其他點數拿來學其他對解謎有幫助的小法術

此外，有為了使用 Ear 暴力而出現的變種，放棄了 Outdo Waver，甚至 DEX 14 的。這些其實很難歸類，放在特種職業裡另外敘述。

#### 配 EQ 法則

out-wiz 最重視的有 **-dc** 和 **HP**。次要是 Mana、MV。

配 EQ 要重視：
1. **DEX 一定配滿 30**
2. **-dc 的 EQ：** 重量輕，-dc 多。-dc 的算法是 -dc 加重量
3. **HP 的增加：** 在上述 1、2 都差強人意之後，可以注重其他 EQ 的加 HP
4. **Mana 及 MV 的增加**

> **P.S.** out-wiz 算是 7777 少數不必太介意 baby 時升得好不好的職業。而 EQ 也不會太難拿或講究。之所以建議新手練 out-wiz 就是這個道理。

---

### 純火系巫師 (Pure Fire Wiz)

- **個人評價：** ★★★☆☆
- **Baby 類型：** Mana

| 屬性 | STR | INT | WIS | DEX | CON |
|------|-----|-----|-----|-----|-----|
| changebody | 18 | 22 | 22 | 14 | 14 |
| train | 2~4 | 8 | 8 | 14 | 0~2 |
| **總合** | **20~22** | **30** | **30** | **14** | **14~16** |

**建議必學技能：** 流星雨 (Star Rain)、闇黑結界 (Dark Space)、魔法結界 (Barrier)、治療術 (Heal)、迷朧幻影 (Illusion)、去除魔法術 (Dispel Magic)、破闇黑結界 (Breakdark)

> 破闇黑結界不花點數，但使用條件為 INT 及 WIS 皆 28 以上。可選擇學聖光，但攻擊火力會變低。

在四度中五種純巫師系中，火系算是其中最強的，因為他擁有群體攻擊——流星雨，在 mob 沒聖光的情形下可以打到總合 4000 上下。配合闇黑結界與迷朧幻影，用來打中、小型 mob 綽綽有餘，又有魔法結界可抵擋攻擊。缺點是 HP 太少，一但被較強攻擊打中幾乎準備歸天。

---

### 純雷系巫師 (Pure Thunder Wiz)

- **個人評價：** ★★★☆☆
- **Baby 類型：** Mana

| 屬性 | STR | INT | WIS | DEX | CON |
|------|-----|-----|-----|-----|-----|
| changebody | 18 | 22 | 18~22 | 14 | 14 |
| train | 2~4 | 8 | 4~8 | 14 | 0~4 |
| **總合** | **20~22** | **30** | **22~30** | **14** | **14~18** |

**建議必學技能：** 爆雷降臨 (Fall Lightning)、闇黑結界 (Dark Space)、魔法結界 (Barrier)、治療術 (Heal)、聖光 (Sanctuary)、迷朧幻影 (Illusion)、去除魔法術 (Dispel Magic)、破闇黑結界 (Breakdark)

雷系巫師非常適合一對一單挑中、小型 mob 或 PK。雖無群體攻擊，但防禦上加了聖光補足 HP 過少問題，被強力攻擊打中還能撿回一命的機率大增。缺點是 HP 仍然偏少。

---

### 純風系巫師 (Pure Wind Wiz)

- **個人評價：** ★★☆☆☆
- **Baby 類型：** Mana

屬性同雷系。

**建議必學技能：** 颶風術 (Hurricane)、闇黑結界、魔法結界、治療術、聖光、迷朧幻影、去除魔法術、破闇黑結界、舞空術 (Fly)

> 如果想練來解謎，最好補個大地之斬，不然包準會因為颶風打死 mob 而使解謎失敗。

風系巫師的颶風術為群體攻擊且附加**目盲**功能。獨立性夠充足，但攻擊威力不夠。最近改成敵人有 Fly 時多加約 1/3 威力，但也不過比冰系強一點點。

---

### 純土系巫師 (Pure Earth Wiz)

- **個人評價：** ★☆☆☆☆
- **Baby 類型：** Mana

| 屬性 | STR | INT | WIS | DEX | CON |
|------|-----|-----|-----|-----|-----|
| changebody | 18 | 22 | 22 | 14 | 14 |
| train | 2~4 | 8 | 8 | 14 | 0~2 |
| **總合** | **20~22** | **30** | **30** | **14** | **14~16** |

**建議必學技能：** 土噬術 (Canyon)、闇黑結界、魔法結界、治療術、聖光、迷朧幻影、去除魔法術、破闇黑結界

> 建議學大地之斬 (Earthslice)。

土系法術威力僅次於核爆跟爆雷，但 mob 個個會飛，土系遇到會飛的 mob 就沒轍。土噬術遇到 LV 55 以下 mob 直接消失，但自己也沒經驗值跟錢可拿。可以說是 wiz 中最廢的。

---

### 純冰系巫師 (Pure Ice Wiz)

- **個人評價：** ★★☆☆☆
- **Baby 類型：** Mana

屬性同雷系。

**建議必學技能：** 雪嵐 (Blizzard)、闇黑結界、魔法結界、治療術、聖光、迷朧幻影、去除魔法術、破闇黑結界

冰系巫師的雪嵐除了群體攻擊外，還可讓對手**力量減少**（讓對手拿不起武器）。但就算力量降到谷底，mob 的特攻、Spell 攻擊威力不變。現在應該只能算是為了讓 mob 拿不起武器而存在的職業。

---

### 風土並存巫師 (Canyon + Hurricane Wiz)

- **個人評價：** ★☆☆☆☆
- **Baby 類型：** Mana

屬性同火系。

**建議必學技能：** 土噬術 (Canyon)、颶風術 (Hurricane)、治療術、聖光、迷朧幻影

同時有颶風與土噬兩種最高階法術，但沒魔力加持使得兩種法術威力都不如原來，實用性法術也較少。沒有闇黑結界做後盾，用起來不輕鬆。

---

### 瘦凌波巫師 (Int Out-Wiz)

- **個人評價：** ★★★★☆
- **Baby 類型：** HP

| 屬性 | STR | INT | WIS | DEX | CON |
|------|-----|-----|-----|-----|-----|
| changebody | 16~18 | 22 | 14 | 22 | 14~16 |
| train | 2~4 | 8 | 0 | 8 | 0~2 |
| **總合** | **18~22** | **30** | **14** | **30** | **14~18** |

**建議必學技能：** 闇黑結界、魔法結界、大地之斬、凌波微步 (Outdo Waver)、治療術、聖光、迷朧幻影、勾魂術 (Soulsteal)、去除魔法術

四度中非常多人用，因卓越防禦力聞名。DC 可撐到 2300 以上（不含 No Save EQ），HP 2100 以上。缺點是 HP 仍嫌少，沒強力攻擊技能，遇到很會逃跑的 mob 會很累。

---

### 肥巫師 (Fat Wiz)

- **個人評價：** ★★★★☆
- **Baby 類型：** HP、Mana

| 屬性 | STR | INT | WIS | DEX | CON |
|------|-----|-----|-----|-----|-----|
| changebody | 16~18 | 22 | 14 | 14~16 | 18~22 |
| train | 2~4 | 8 | 0 | 0~2 | 4~8 |
| **總合** | **20~22** | **30** | **14** | **14~18** | **22~30** |

**建議必學技能：** 闇黑結界、魔法結界、大地之斬、治療術、聖光、迷朧幻影、勾魂術、去除魔法術、攝心術 (Charm Person)、魔法沙漏 (Magic Clock)、召喚術 (Summon)

HP 夠多，一堆實用性 Spell，常被練來單挑較強 mob（如神龍、地藏王）或用召喚術打 Baby EQ。缺點是 DEX 低，常常打幾下就要補 Move。

---

### 肥凌波巫師 (Fat Out-Wiz)

- **個人評價：** ★★★★★
- **Baby 類型：** HP

| 屬性 | STR | INT | WIS | DEX | CON |
|------|-----|-----|-----|-----|-----|
| changebody | 18 | 14 | 14 | 22 | 22 |
| train | 4 | 0 | 0 | 8 | 8 |
| **總合** | **22** | **14** | **14** | **30** | **30** |

**建議必學技能：** 闇黑結界、魔法結界、大地之斬、凌波微步、治療術、聖光、勾魂術、去除魔法術

> 另外還有一種會學習雙手持劍 (Two Weapon) 的凌波。

改良了 INT 型 HP 過少的問題。DC 2000 以上、AC 1000 以上、HP 可達 3000。幾乎每個人都會練一隻。建議新手從這型玩起了解四度，EQ 便宜又好拿。

各式配法：注重 HP 型 3600+、DC 型 2300~2400+、AC 型 1200+（DC 基本 1800 以上較好）。

缺點同瘦凌波——沒強力攻擊技能。

---

### 肥流星雨巫師 (Fat Star Rain Wiz)

- **個人評價：** ★★★★☆
- **Baby 類型：** Mana

| 屬性 | STR | INT | WIS | DEX | CON |
|------|-----|-----|-----|-----|-----|
| changebody | 18 | 14 | 22 | 14 | 22 |
| train | 2~4 | 0 | 8 | 0~2 | 8 |
| **總合** | **20~22** | **14** | **30** | **14~16** | **30** |

**建議必學技能：** 流星雨 (Star Rain)、治療術、聖光、勾魂術、去除魔法術、魔法沙漏 (Magic Clock)

> 另有 WIS/DEX 30 型，差在 HP 而已。

流星雨不看魔法強度而看精神力多寡決定威力，所以此型巫師得以存在。缺點是沒魔力加持使流星雨每次花費精神力多，DEX 低常常 Delay 爆。

---

### 乾坤巫師 (Reflexion Wiz)

- **個人評價：** ★★★☆☆
- **Baby 類型：** Power

| 屬性 | STR | INT | WIS | DEX | CON |
|------|-----|-----|-----|-----|-----|
| changebody | 22 | 14 | 14 | 18~22 | 18~22 |
| train | 8 | 0 | 0 | 4~8 | 4~8 |
| **總合** | **30** | **14** | **14** | **22~30** | **22~30** |

**建議必學技能：** 闇黑結界、魔法結界、降龍十八掌 (Dragonfist)、天馬流星拳 (Cometfist)、乾坤大挪移 (Reflexion)、治療術、聖光、勾魂術、去除魔法術

兼具拳法，防禦有魔法結界 + 乾坤大挪移 + 聖光三者共存，HP 不會太低。CON 30 有機會破三千。缺點是 Mana 偏低，DEX 22 時每次攻擊 Delay 增加不少。

---

### 絞殺巫師 (Hanging Wiz)

- **個人評價：** ★★★☆☆
- **Baby 類型：** HP

| 屬性 | STR | INT | WIS | DEX | CON |
|------|-----|-----|-----|-----|-----|
| changebody | 22 | 22 | 14 | 14~18 | 14~18 |
| train | 8 | 8 | 0 | 0~4 | 0~4 |
| **總合** | **30** | **30** | **14** | **14~22** | **14~22** |

**建議必學技能：** 闇黑結界、魔法結界、絞殺 (Hanging)、藏匿 (Hide)、治療術、聖光、迷朧幻影、勾魂術、去除魔法術

除流星雨外攻擊力最高的巫師。實用法術多，防禦有魔法結界 + 聖光，也有闇黑結界封印用。缺點是絞殺威力不穩，遇到會主動攻擊跟解目盲的 mob 會很累。

---

## 軍人 (Soldier)

> 原作者：placewind (7777的笨玩家)，1999/09/25
> 系列文作者：motonpom (超級大流氓)，2001/02/17

軍人的 skill 只有四個：

| # | 技能 | 英文 | 記憶量 |
|---|------|------|--------|
| 1 | 射擊 | shoot | 600 |
| 2 | 鎗法 | gunner | 350 |
| 3 | 過肩摔 | buttock | 200 |
| 4 | 絞殺 | hanging | 300 |

其中 4 是 3 的進階技能，2 是 1 的加強技能。目前軍人由於改了威力，預計將會大為流行。7777 以後擁有獨立逛街能力的將會是三大系：kni、out-wiz 跟 slr（軍人）。

### 射擊基礎知識

shoot 要學了 gunner 才會變較強。在 shoot 跟 gunner 熟練 99 後，hr 100 以上，wis 20，各槍威力如下：

| 槍枝類型 | 傷害 |
|----------|------|
| 威力 68 單發槍 | 命中時 ~2200，未命中時 ~600 |
| 威力 100 的槍 | 3000 以上 |
| 警備機槍（連發） | 總計平均 ~2300 |
| 爵得槍（no save 連發） | 1400 x 4 = 5600 |

### Dex 與射擊次數對照表

| DEX | 發射鎗數 |
|-----|----------|
| 14 | 6 |
| 16 | 6 |
| 18 | 8 |
| 20 | 8 |
| 22 | 9~10 |
| 24 | 10~11 |
| 26 | 14 |
| 28 | 18 |
| 30 | 超過 24 鎗以上 |

### 屬性配置總覽

#### 配法 (1)：高 DEX 型

`str 30, int 14, wis 22, dex 30, con 14`

- 缺點：hp 太低
- 變體：`str 30, int 14, wis 21, dex 28, con 16`
- 搭配：gunner + out + san + soulsteal / Saber 型（block + disarm） / gunner + dark

#### 配法 (2)：高 CON 型

`str 30, int 14, wis 22, dex 14, con 30`

- dex 14 可在 delay 破 1000 前連發 5 槍
- 學了天馬流星拳後 hp 3200+，mana 1000+
- 搭配：gunner + refl + san + soulsteal + heal + bli / gunner + sneak / gunner + full heal

#### 配法 (3)：高 INT 型

`str 30, int 30, wis 22, dex 14, con 14`

- 目的是用 illusion
- 搭配：gunner + illusion + soulsteal + dark + san

#### 配法 (4)：絞殺型

`str 30, int 14, wis 14, dex 30, con 22`

- 不用 gunner，學 hanging + soulsteal + dark + san

---

### 格檔型 (Parry / Block - Slr)

- **個人評價：** ★★★★★
- **Baby 類型：** Hp

#### Parry 型

| 屬性 | 力量 | 智力 | 睿智 | 速度 | 體格 |
|------|------|------|------|------|------|
| 總合 | 22~30 | 14 | 20~22 | 18~30 | 18~30 |

**必學技能：** Gunner、Parry、Disarm、Heal、Sanctuary、Soulsteal、Dispel Magic、Recharge Item

- Dex 必須 26 以上，被 disarm 後就不能再格檔

#### Block 型

同屬性，但左手格檔在 600 < dc < 900 時較容易出現，Dex 不要超過 26。被 disarm 後還能繼續用左手 EQ 檔。

**必學技能：** Gunner、Block、Disarm、Heal、Sanctuary、Soulsteal、Dispel Magic、Recharge Item

此型軍人最常見，hr 最高，法術最多。Con 22 時 HP 可到 3000 上下。防禦力數一數二，建議新手先玩此型。

**共同缺點：** Disarm 失敗加很多 Delay。遇上非 att 性攻擊的 mob 只能自求多福。

---

### 乾坤大挪移型 (Refl - Slr)

- **個人評價：** ★★★★★
- **Baby 類型：** Power

| 屬性 | 力量 | 智力 | 睿智 | 速度 | 體格 |
|------|------|------|------|------|------|
| 總合 | 26~30 | 14 | 20~22 | 18~30 | 22~30 |

**必學技能：** Gunner、Reflexion、Heal、Sanctuary、Soulsteal、Dispel Magic

> 建議學 Disarm，Disarm + Parry 能多加約 50 Power。

Con 調高所以血多，但 Dex 下降使射擊鎗數較少。大力推薦。

---

### Dark 型 (Dark - Slr)

- **個人評價：** ★★★★
- **Baby 類型：** Hp

| 屬性 | 力量 | 智力 | 睿智 | 速度 | 體格 |
|------|------|------|------|------|------|
| 總合 | 22~30 | 14 | 20~26 | 18~30 | 18~26 |

**必學技能：** Gunner、Dark Space、Heal、Sanctuary、Soulsteal、Dispel Magic

> 建議學 Barrier。

封印對手法術的絕技，攻守二用非常好。可一個人解決大型 mob。缺點是 Dark 容易失敗，每次花費 Mana 與 Delay 多，HP 偏低。

---

### 凌波型 (Outdo - Slr)

- **個人評價：** ★★★
- **Baby 類型：** Hp

| 屬性 | 力量 | 智力 | 睿智 | 速度 | 體格 |
|------|------|------|------|------|------|
| 總合 | 22~30 | 14 | 20~22 | 30 | 14~22 |

**必學技能：** Gunner、Outdo Waver、Heal、Sanctuary、Soulsteal、Dispel Magic

實用性比凌波騎士好，dc 更低，且軍人一般 agg 0 射擊不需切換。Dex 必 30 導致 Str 與 Con 取捨困難。

---

### 智力型 (Int - Slr)

- **個人評價：** ★★~★★★

| 屬性 | 力量 | 智力 | 睿智 | 速度 | 體格 |
|------|------|------|------|------|------|
| 總合 | 20~30 | 30 | 20~30 | 14~30 | 14~30 |

**必學技能：** Gunner、Illusion

任何一種軍人的翻本，條件為 Int 必 30。最適合配 Dark 型與牧師型。練好玩的軍人。

---

### 絞殺型 (Hanging - Slr)

完全追求 dr。

#### 界王拳型（★★★★，Power Baby）

| 屬性 | 力量 | 智力 | 睿智 | 速度 | 體格 |
|------|------|------|------|------|------|
| 總合 | 30 | 14 | 14 | 30 | 22 |

**必學技能：** Hanging、Kaiokan、Dragonfist、Sadfist

#### 普通絞殺型（★★★，Hp Baby）

同屬性。**必學技能：** Hanging、Heal、Sanctuary、Soulsteal、Dispel Magic、Hide

#### 純智力絞殺型（★★★，Hp Baby）

Str 30, Int 30, 其餘分配。**必學技能：** Hanging、Heal、Sanctuary、Dispel Magic、Illusion、Hide

---

### 騎士型 (Slr - Knight)

- **個人評價：** ★★★★
- **Baby 類型：** Hp

| 屬性 | 力量 | 智力 | 睿智 | 速度 | 體格 |
|------|------|------|------|------|------|
| 總合 | 22~28 | 14 | 20~22 | 28~30 | 14~22 |

**必學技能：** Blitz、Two Weapon、Gunner、Heal、Sanctuary、Dispel Magic

會 att 的軍人。hr 可破 120 甚至 130。缺點是 hp 太少或力量太低。

---

### 牧師型 (Cle - Slr)

- **個人評價：** ★★
- **Baby 類型：** Hp、Mana

**必學技能：** Gunner、Gfull Heal、Holyaura

能攻擊的牧師，但受限於點數分配，無法習得多數實用法術。

---

### 兩用型 (Shoot + Hanging - Slr)

- **個人評價：** ★★★★
- **Baby 類型：** Hp

| 屬性 | 力量 | 智力 | 睿智 | 速度 | 體格 |
|------|------|------|------|------|------|
| 總合 | 30 | 14 | 20~22 | 14~30 | 14~30 |

**必學技能：** Gunner、Hanging、Heal、Sanctuary、Dispel Magic

射擊 + 絞殺都能玩，視狀況選擇。缺點是需帶兩套 EQ（hr 和 dr 各一套），必須學會快速鍵。

---

## 小偷 (Thief)

> 原作者：placewind (7777的笨玩家)，1999/09/26

目前 Thief 大致分成迷香型與背刺型，也有混合型。

#### 基本技能解說

**一、迷香 (Benumb)**
- 藥劑在 SOGO 的藥劑師有賣，很重且貴
- INT 28 以上施用成功率才高
- 對 LV 70 以下且 INT 比自己低的 mob 才能迷倒
- STR 最好 25 以上（載重力 999）

**二、背刺 (Backstab)**
- 學加強背刺 (BS Power) 後威力大增
- 最大威力與 DR 相關，但起伏可差 2000+
- 失敗時自己 delay 約 700

**三、遁逃 (Abscond)**
- DEX 24 以上才能使用，指定方向逃逸

**四、刺瞎 (Piercy)**
- 效果同目盲但更強力，很多 mob 不怕 bli 而怕刺瞎

---

### 純迷香型 (Ben-Thief)

- **個人評價：** ★★★★~★★★★★
- **Baby 類型：** HP

| 屬性 | 力量 | 智力 | 睿智 | 速度 | 體格 |
|------|------|------|------|------|------|
| 總合 | 20~22 | 30 | 14 | 14~30 | 14~30 |

**必學技能：** Benumb、Pretend、Piercy、Spy、Abscond（需 DEX 24+）、Heal、Remove Nodrop

> 建議學大地之斬和勾魂術。

純輔助用，專門迷昏不必要殺的 mob。HP 偏低，不學聖光很容易死。

**placewind 配法範例：** STR 25, INT 30, WIS 14, DEX 24, CON 17。技能 BS + ABS + ILL + PIE + Soulsteal + SAN + BEN + Magic Clock + Confuse。戰法：上 SAN、ILL → PIE mob → ABS 逃 → Confuse mob → BEN 迷倒。

---

### 背刺型 (BS-Thief)

- **Baby 類型：** HP

| 屬性 | 力量 | 智力 | 睿智 | 速度 | 體格 |
|------|------|------|------|------|------|
| 總合 | 30 | 14 | 14 | 30 | 22 |

**必學技能：** BS Power、Piercy、Abscond

純破壞力型。DR 裝後最大攻擊可達 7777。背刺造成暫時麻痺。

---

### 兩用型 (BS + Ben Thief)

- **個人評價：** ★★★★~★★★★★
- **Baby 類型：** HP

| 屬性 | 力量 | 智力 | 睿智 | 速度 | 體格 |
|------|------|------|------|------|------|
| 總合 | 26~30 | 30 | 14 | 18~24 | 18~24 |

> DEX 建議 24 以使用遁逃術。

**必學技能：** BS Power、Benumb、Piercy、Abscond

同時有迷香和背刺。STR 與 INT 花太多點導致 HP 太少，記憶點所剩不多，EQ 難同時滿足 STR、INT、DR。

---

### 肥小偷 (Fat-Thief)

- **個人評價：** ★★★★~★★★★★
- **Baby 類型：** HP

| 屬性 | 力量 | 智力 | 睿智 | 速度 | 體格 |
|------|------|------|------|------|------|
| 總合 | 22 | 14 | 14 | 30 | 30 |

**必學技能：** Piercy、Abscond、Sanctuary

> 建議學大地之斬。打爵德建議學 Frostdust 跟 Disarm。

血多不怕被打，專門刺瞎敵人。被刺瞎就算 mob 會解目盲也沒用，可用區域佔七、八成。另類肉盾。

---

### 忍者 (Ninja)

- **個人評價：** ★★~★★★★★
- **Baby 類型：** HP

| 屬性 | 力量 | 智力 | 睿智 | 速度 | 體格 |
|------|------|------|------|------|------|
| 總合 | 30 | 14 | 14 | 30 | 22 |

**必學技能：** BS Power、Third Attack、Enhanced Damage、Piercy、Abscond

背刺後用 ATT 攻擊再 ABS 逃跑。沒實用 Spell，怕打但 PK 滿讓人頭痛。耍寶型職業。

---

## 拳法家 (Boxer)

> 原作者：placewind (7777的笨玩家)，1999/09/25

拳法家依照技能分較好。新手最好不要練 BXR，因為太注重 baby 升級屬性的培養。

隨各人對 DR、HR 的追求不同，會有中立、邪惡、神聖之分。神聖略多 HR，邪惡略多 DR。邪惡打心地神聖的 mob 威力少 1/10（有聖佑術）。中立不受 PRO 影響又有邪惡的 DR，但基本 EQ skying 反中立。

**BXR 主要特色:** Dragonfist 是目前全 7777 唯一能不被聖光擋一半傷害的技能。mob 有聖光 dragonfist 威力約減 1/5。

### 1. 純種 Kai-BXR

- STR 必 30，DEX/CON 隨 EQ 認識決定（DEX 30/CON 22 或 CON 30/DEX 22 或 DEX 26/CON 26）
- KAI 喊一次加 DEX 2
- DEX 30 可在 delay 1000 內連打 5 發 dragon，最宜 PK
- CON 30 跟團 HP 多
- 內力 skill all 99 約 2600~2800，極限 2999
- **主要 skill:** KAI + Dragon + Cure BLI，或 KAI + Dragon + REFL

### 2. Kni-Kai-BXR

- 最高能喊 5 倍界王
- KAI 後四段普通攻擊 6xx x 4 = 2400
- **技能:** KAI, Dragon, EJI, Blitz, Second Attack, Enhanced Damage
- Baby 好的話可加學 SAN 甚至 Heal

### 3. Stun-BXR

- 必學 Taigi, Stun, Dragon
- 威力算 BXR 中最弱，但 HP 最多（CON 30 約 4500，一定破 4000）

### 4. Taigi-BXR

- 不學 Stun，可能配一點劍法或學 REFL

### 5. Swd-BXR

結合 SWD 的 skill，主攻擊仍是 Dragonfist。劍法加高內力用。Evil 型內力可破 3000，dragon 威力僅次於 evil-kai-bxr。

**5a. 練 EJI 型** / **5b. 練 OKRA 型**

練 OKRA 者必 evil，可用 dragon 或 evilslash 兩種攻擊。Evilslash 要 OKRA 99、男性、極邪惡、高 DR，可打 2000+（超過 2500），只耗內力 100，delay 比 dragon 少。DEX 30 下連續炮火。不能破 SAN 但 mob 沒 SAN 時很猛。

---

## 劍士 (Swordsman)

> 原作者：placewind (7777的笨玩家)，1999/09/25

理論上有太極劍劍士、獨孤劍士、辟邪劍士。辟邪劍士跟 dragonfist 結合放入 BXR。純太極劍士沒破 SAN 能力，很少見。

### 獨孤劍士 (Sol-SWD)

- 威力隨 MV 減少而減弱，打一次 sol 扣 300 MV
- MV 越高打越強，MV 破 5000 可超過 2500
- 可增加敵人 delay
- 一般 STR 30, INT 22, WIS 14, DEX 30, CON 14（INT 22 增加 mob delay 機會大幅提高）
- HP 有 2700+，最大 MV 可達 5500 甚至破 6000
- **配 EQ 法則:** 屬性滿後 → MV → HR

---

## 牧師 (Cleric)

> 原作者：placewind (7777的笨玩家)，1999/09/25

### 1. 最原型牧師

- **屬性:** STR 22, INT 30, WIS 30, DEX 14, CON 14
- 有 Spellmaster, Gfull, Holy, Polar, Transfer 等
- HP 約 2300，Mana 5500~6500
- 變化：加 EAR + ILL / 聖光箭型 (Holy Arrow，記憶量 450) / 放棄 Transfer 加學 Soulsteal
- **缺點:** HP 太少

### 2. DEX 30 型

- (1) STR 22, INT 14, WIS 14, DEX 30, CON 30
- (2) STR 22, INT 14, WIS 30, DEX 30, CON 14
- 減少被普通攻擊損傷，必學 Soulsteal
- 共同缺點：技能熟練度難練

### 3. WIS 30 + CON 30 型

- 最可怕的 CLE，HP 及 Mana 都可破 5000
- 預計將會是未來 CLE 流行主力

### 4. EAR 暴力型

- STR 22, INT 30, WIS 14, DEX 14, CON 30
- EAR 打極暴力，可共達 2000 傷害
- 缺點：Mana 過少

> **配 EQ 法:** 屬性滿後加 Mana、HP。
>
> **P.S.** God Bless 只要記憶點許可每種 CLE 都能學。Cross 對付不死系超好用，INT 30 WIS 30 的 CLE 應該會抬頭。

---

## 道士 (Taoist)

### 全真型 (Changjun-Tat)

- **個人評價：** ★★★★★
- **Baby 類型：** Mana

**必學技能：** 全真劍法、驅屍術 (Corpse Control)、治療術、聖光、勾魂術、迷朧幻影、魔力時鐘 (Magic Clock)、武器開光 (Weapon Bloom)、去除魔法術、穿門術 (Pass Door)、五鬼運財術 (Barter)、煉符咒 (Makescroll)、召喚術 (Summon)

「製符機」，專長製作各種符咒。法力儲量豐沛，優秀的支援型角色。缺點是 HP 偏低，面對高等級敵人時較脆弱。

---

### 天罡型 (Kai-Tat)

- **個人評價：** ★★
- **Baby 類型：** Power

| 屬性 | 力量 | 智力 | 睿智 | 速度 | 體格 |
|------|------|------|------|------|------|
| 總合 | 30 | 14 | 14 | 30 | 22 |

**必學技能：** 天罡北斗陣 (Plowslash)、界王拳 (Kaioken)、烈火陣 (Case_fire)

必須 7 隻都有天罡且組團才能使用。號稱出招必定全中，總合威力四度最強。但要找 7 隻道士很麻煩，一人狀況不對全隊無法出招。

---

### 流星雨型 (Star Rain-Tat)

- **個人評價：** ★★★
- **Baby 類型：** Mana

| 屬性 | 力量 | 智力 | 睿智 | 速度 | 體格 |
|------|------|------|------|------|------|
| 總合 | 20~22 | 30 | 30 | 14 | 14~16 |

**必學技能：** 流星雨 (Star Rain)、煉符咒 (Makescroll)

> 建議學五鬼運財術與召喚術。

可做核爆、流星雨符給其他職業使用，威力與自己施法一樣。每到子午大法時間可完全補充精神力（中立型一天 2 次）。缺點是沒魔力加持使花費精神力多，HP 跟純巫師差不多。

---

## 特種職業 (Special Classes)

> 原作者：placewind (7777的笨玩家)，1999/09/25

完全找不到分類依據的職業。

**基本屬性:** STR 22, INT 30, WIS 14, DEX 14, CON 30

完全為用 EAR 而生。EAR 每下 300+，300 x 5 = 1700，全 7777 最暴力。

可結合：
1. **TAT（道士）：** 練天罡北斗陣或 Full Heal 或 WIZ 法術造符。主要 skill：Time Power + EAR + Make Paper + ILL
2. **CLE（牧師）：** 能攻擊的牧師變種
3. **Thief（小偷）：** 只能用迷香。EAR + BEN。無法學遁逃（DEX 14）
4. **WIZ（巫師）：** 學 Dark + EAR
5. **BXR（拳法家）：** REFL + Dark + EAR。據說為打 sango2 而出現

> 此型極為注重升級 baby 屬性的培養。

---

## 升級與 Baby 補充

### 升級技能

> 作者：motonpom (超級大流氓)，2001/02/09

Power、HP、DEX 型 baby 先學 skill/spell 再 changebody；Mana 型可直接 changebody。Train 請在 changebody 與學 skill 前先完成。

#### LV 10 以下必學技能

| 技能 | 英文名 | 記憶量 |
|------|--------|--------|
| 少林長拳 | saulinfist | 100 |
| 羅漢拳 | lohanfist | 150 |
| 輕度治療 | cure light | 5 |
| 強力治療 | cure serious | 5 |
| 治療術 | heal | 65 |
| 茅山劍法 | mountainslash | 25 |
| 虛影劍法 | shadowslash | 50 |
| 補充體力術 | refresh | 20 |
| 體力復原術 | full refresh | 80 |
| 神靈呼喚術 | armor | 20 |
| 氣盾 | shield | 30 |
| 巨人之力 | giant strength | 90 |
| 呼風喚雨術 | control weather | 10 |
| 舞空術 | fly | 30 |
| 腹語術 | ventriloquate | 5 |
| 造水術 | create water | 5 |
| 製造食物術 | create food | 5 |
| 造泉術 | create spring | 5 |
| 光球術 | continual light | 5 |
| 搜查 | scan | 10 |
| 定位術 | locate object | 10 |
| 鑑定術 | identify | 10 |

#### LV 11 以上

學到大地之斬後 cancel 少林長拳與羅漢拳，改學其他技能。

| 技能 | 英文名 | 記憶量 |
|------|--------|--------|
| 大地之斬 | earthslice | 250 |
| 復明術 | cure blindness | 20 |
| 伏羲劍法 | fusislash | 150 |
| 女神庇祐術 | bless | 60 |
| 去除詛咒術 | remove curse | 25 |
| 聖佑術 | protection | 30 |
| 聖光 | sanctuary | 210 |
| 雙重治療術 | double heal | 230 |
| 完全治療術 | full heal | 400 |

再學補 HP 上限的 spell：

| 技能 | 英文名 | 記憶量 |
|------|--------|--------|
| 魔法彈 | magic missile | 5 |
| 火臂術 | burning hands | 25 |
| 能源球 | powerball | 60 |
| 寒霜之觸 | chill touch | 25 |
| 霜塵術 | frostdust | 60 |
| 爆岩擊 | rockblast | 40 |
| 氣旋術 | cyclone | 60 |
| 雷之招喚 | call lightning | 60 |

> **PS 1.** pra 熟練度看 INT 高低決定多寡，盡量在 INT 高時去 pra 可得到較多 HP、Mana、MV 與 Power。
>
> **PS 2.** 不學 cure poison 因為升級不需到有毒 mob 區域，但 remove curse 保險學一下。
>
> **PS 3.** 拳法升級時 `agg 100`；學到大地之斬後一律 `agg 0` 攻擊，受傷較少。

---

### Move Baby EQ 穿法

> 作者：motonpom，2001/02/11

**屬性:** STR 22, INT 14, WIS 14, DEX 30, CON 30

（以下假設永遠上 giant power，一開始 STR = 3）

| 等級 | 裝備變化 | 累計 Str/Int/Wis/Dex/Con |
|------|---------|--------------------------|
| LV 1 | 慈悲心、格鬥脛甲、巨人之力、粉紅冰戒指 x2、太空傳送裝置 | 5/0/0/2/0 |
| LV 3 | 亞麻布袍子、粉紅冰戒指→一刻館之戒 x2 | 5/0/1/2/0 |
| LV 5 | 矮人手套、黑曜石短劍、小馬 | 6/0/1/5/0 |
| LV 8 | 蘑菇護身符 x2 | 6/0/0/5/4 |
| LV 10 | 一刻館之戒→粉紅冰戒指 x2、精靈護臂、生命之盾 | 6/0/0/7/7 |
| LV 13 | 青椒手鐲、洋蔥手鐲 | 6/0/0/8/8 (滿) |

> LV 15 後換穿僧侶的臭襪 (+Move)。LV 18 前穿滿。lvup_move 330 以上算合格。

---

### Power Baby EQ 穿法

> 作者：motonpom，2001/02/11

**屬性:** STR 30, INT 14, WIS 14, DEX 30, CON 22

| 等級 | 裝備變化 | 累計 Str/Int/Wis/Dex/Con |
|------|---------|--------------------------|
| LV 1 | 慈悲心、格鬥脛甲、巨人之力、粉紅冰戒指 x2、太空傳送裝置 | 5/0/0/2/0 |
| LV 3 | 亞麻布袍子、粉紅冰戒指→一刻館之戒 x2 | 5/0/1/2/0 |
| LV 5 | 勇士之劍、矮人手套、太空傳送→五代土產、小馬 | 8/0/1/2/1 |
| LV 8 | 蘑菇護身符 x2 | 8/0/1/2/5 |
| LV 10 | 一刻館之戒→粉紅冰戒指 x2、精靈護臂、生命之盾 | 8/0/0/4/8 |
| LV 13 | 荔枝領帶 x2→蘑菇護身符 x2 | 8/0/0/8/4 (滿) |

> Power Baby 非常難養，lvup_power 325 以上偷笑，合格標準 320 以上。

## 職業配裝範例

> 來源：PTT MUD 看板精華區，配裝指南系列文章

### Block Wiz（善）

> 來源：https://www.ptt.cc/man/mud/DBA7/D546/D489/M.1590154379.A.3AA.html

Block Wiz 善良陣營職業。源自 Lilina 撰寫的阿拉丁系列區域後開始流行，因為「兼具防禦、實用與攻擊性」的特點備受歡迎。

**建議技能配置：**

| 技能名稱 | 英文 | 記憶點 |
|----------|------|--------|
| 左手格擋 | block | 250 |
| 卸除武器 | disarm | 200 |
| 解救 | rescue | 100 |
| 闇黑結界 | dark space | 20 |
| 大地之斬 | earthslice | 250 |

**範例角色數值：** 等級 60，智力 30、體格 24、力量 22。生命力 2852/3187，精神力 1671/1671。

**核心裝備清單：**
- 火之盾（左手）
- 冶鍊銀闊劍（右手）
- 水瓶座聖衣（身體）
- 雷火罩袍（背部）
- 極光腰帶（腰部）
- 飄浮之靴（腳部）
- 黑雷破天臂（手臂）
- 其他多件閃爍或特效裝備

**評價：** 防禦能力強、實用性高。缺點是失去核心武器後攻擊力下降明顯，新手難以取得相關裝備。建議先學會單挑幽靈船後再嘗試。

---

### Star Wiz（善）

> 來源：https://www.ptt.cc/man/mud/DBA7/D546/D489/M.1590154379.A.443.html

此職業具備強大的群體技能，再加上基本配裝容易取得，十分推薦新手與老手使用。對於某些免疫物理攻擊的怪物，魔法職業特別有效。

入門配置通常設定智力與睿智各 30 點，使技能快速增長。配合迷朦幻影等技巧，幾乎可應對所有怪物。

**建議技能配置：**

| 技能名稱 | 英文 | 記憶點 |
|----------|------|--------|
| 迷朦幻影 | illusion | 50 |
| 流星雨 | star rain | 150 |
| 魔法沙漏 | magic clock | 50 |

**範例角色數值：** 等級 60，智力 30、睿智 30，精神力 6137。

**主要裝備清單：**
- 照明：七星燈
- 手指：綠寶石戒指 x2
- 頸部：白霧徽章 x2
- 身體：牡羊座聖衣
- 頭部：紫薇頭巾
- 腿部：刺針護脛
- 腳部：七彩蓮花座
- 主手：薇薇安手杖（鵝毛羽扇可備選）
- 背部：風暴罩袍
- 腰部：極光腰帶

**評價：** 角色掌握超過 50 項技能，熟練度多達 99/100 以上，包含治療、增益、控制與攻擊法術。

---

### Star Wiz（中立）

> 來源：https://www.ptt.cc/man/mud/DBA7/D546/D489/M.1590154379.A.FA7.html（2013 年 5 月）

**基本屬性配置：** 力量 22、智力 30、睿智 30、速度 14、體格 14。
**數值：** 1869/1869 生命力、6151/6151 精神力。

**完整裝備列表：**

| 部位 | 裝備 |
|------|------|
| 頭部 | 英王皇冠 |
| 身體 | 護國袈裟 |
| 背部 | 風暴罩袍 |
| 脖子 x2 | 雷斯徽章 |
| 手指 x2 | 綠寶石戒指 |
| 手腕 x2 | 雷斯手環 |
| 手部 | 般若波羅葉 |
| 腿部 | 刺針護脛 |
| 腳部 | 七彩蓮花座 |
| 握在手上 | 聖魔法杖 |
| 左手 | 太平要術 |
| 右手 | 灰鳥族之劍 |
| 腰部 | 破魂黑束腰 |
| 照明器具 | 智慧之光 |

**評價：** 此配置「最大的好處在於取得方便」，無需完成困難任務。體格較低會影響生命值，但整體而言適合新手玩家使用。

---

## SKILL 組合（角色配置範例）

以下為各種角色配置的技能組合範例，供建角時參考。

### 17. kai-kni

```
[   enhanced damage]熟練度:  99/   2  [     second attack]熟練度:  99/   2
[      third attack]熟練度:  99/   3  [         bare fist]熟練度:  99/   3
[             blitz]熟練度:  99/   2  [          ejinjing]熟練度:  99/   1
[        dragonfist]熟練度:  99/   6  [         lohanfist]熟練度:  99/  29
[           sadfist]熟練度:  99/   9  [        saulinfist]熟練度:  99/   5
[        kamekameha]熟練度:  99/  22  [         gankitama]熟練度:  61/ 316
[           kaioken]熟練度:  99/   6  [         drunkfist]熟練度:  99/  32
```

`<2837hp 110m 583mv 2319ip>`

### 18. star-wiz

```
[             armor]熟練度:  99/   2  [             bless]熟練度:  99/  52
[         blindness]熟練度:  99/  68  [     burning hands]熟練度:  99/  68
[    call lightning]熟練度:  99/  86  [      charm person]熟練度:  99/  62
[       chill touch]熟練度:  99/  52  [   continual light]熟練度:  99/  55
[   control weather]熟練度:  99/  39  [       create food]熟練度:  99/ 105
[     create spring]熟練度:  99/  84  [      create water]熟練度:  99/  20
[    cure blindness]熟練度:  99/  22  [        cure light]熟練度:  99/  39
[       cure poison]熟練度:  99/  70  [      cure serious]熟練度:  99/  71
[             curse]熟練度:  99/ 100  [       detect evil]熟練度:  99/  39
[      detect invis]熟練度:  99/  24  [      detect magic]熟練度:  99/  55
[     detect poison]熟練度:  99/  71  [       faerie fire]熟練度:  99/  36
[        faerie fog]熟練度:  99/  68  [          fireball]熟練度:  99/  20
[       flamestrike]熟練度:  99/  20  [               fly]熟練度:  99/  84
[              heal]熟練度:  99/  48  [       infravision]熟練度:  99/  39
[             invis]熟練度:  99/  52  [    know alignment]熟練度:  99/  23
[     magic missile]熟練度:  99/   4  [        mass invis]熟練度:  99/  63
[          illusion]熟練度:  99/  47  [            poison]熟練度:  99/  38
[        protection]熟練度:  99/  36  [           refresh]熟練度:  99/  52
[      remove curse]熟練度:  99/ 103  [         sanctuary]熟練度:  99/  52
[             sleep]熟練度:  99/  62  [            summon]熟練度:  99/ 107
[          teleport]熟練度:  99/  84  [     ventriloquate]熟練度:  99/  71
[            weaken]熟練度:  99/  47  [              lore]熟練度:  99/  20
[         lifesteal]熟練度:  99/  36  [         soulsteal]熟練度:  99/  84
[      nuclearblast]熟練度:  99/   4  [    cure paralysis]熟練度:  99/  23
[         star rain]熟練度:  99/ 100  [         powerball]熟練度:  99/  36
[         rockblast]熟練度:  99/   4  [           cyclone]熟練度:  99/  84
```

`<2144hp 5314m 686mv>`

### 19. mag-kni-bxr

```
[             bless]熟練度:  99/  12  [       create food]熟練度:  99/   9
[     create spring]熟練度:  99/  28  [      create water]熟練度:  99/  35
[    cure blindness]熟練度:  99/  28  [        cure light]熟練度:  99/  30
[      cure serious]熟練度:  99/  18  [              heal]熟練度:  99/  15
[     magic missile]熟練度:  99/   7  [        protection]熟練度:  99/  14
[           refresh]熟練度:  99/  11  [         sanctuary]熟練度:  99/  18
[         rockblast]熟練度:  99/  33  [   enhanced damage]熟練度:  99/   4
[     second attack]熟練度:  99/   5  [      third attack]熟練度:  99/   2
[         bare fist]熟練度:  99/   4  [             blitz]熟練度:  99/   2
[         reflexion]熟練度:  99/  14  [          ejinjing]熟練度:  99/   4
[         cometfist]熟練度:  99/   2  [        dragonfist]熟練度:  99/  14
[         lohanfist]熟練度:  99/  42  [           sadfist]熟練度:  99/  24
[        saulinfist]熟練度:  99/  14  [         drunkfist]熟練度:  99/  42
[         fusislash]熟練度:  99/  22  [     mountainslash]熟練度:  99/  22
[       shadowslash]熟練度:  99/  22
```

`<3492hp 491m 1797mv>`

### 20. tai-swd-bxr

```
[         bare fist]熟練度:  72/ 260  [       swordmaster]熟練度:  61/ 553
[          ejinjing]熟練度:  94/ 205  [        dragonfist]熟練度:  99/  14
[         drunkfist]熟練度:  99/  14  [        bloomslash]熟練度:  99/  42
[      chunshislash]熟練度:  99/  22  [         fusislash]熟練度:  99/  22
[       shadowslash]熟練度:  99/  10  [      shanyanslash]熟練度:  99/  22
[        taigislash]熟練度:  99/  36  [        twoyislash]熟練度:  99/   8
[         girlslash]熟練度:  71/   0  [       flowerslash]熟練度:  99/   1
```

`<生命3251 精神133 移動2317>`

### 21. out-wiz (Lace)

```
[             armor]熟練度:  99/  14  [             bless]熟練度:  99/  35
[         blindness]熟練度:  97/ 825  [   control weather]熟練度:  99/  23
[       create food]熟練度:  99/  26  [      create water]熟練度:  99/  35
[    cure blindness]熟練度:  99/  15  [        cure light]熟練度:  99/   2
[       cure poison]熟練度:  99/  29  [      cure serious]熟練度:  99/  26
[             curse]熟練度:  71/   0  [       detect evil]熟練度:  99/  16
[     detect hidden]熟練度:  99/  28  [      detect invis]熟練度:  99/  45
[      detect magic]熟練度:  99/  37  [     detect poison]熟練度:  99/  37
[      dispel magic]熟練度:  99/  37  [       faerie fire]熟練度:  71/ 161
[        faerie fog]熟練度:  99/  35  [               fly]熟練度:  99/  35
[              heal]熟練度:  99/  49  [          identify]熟練度:  86/  66
[       infravision]熟練度:  99/  23  [    know alignment]熟練度:  99/  16
[     locate object]熟練度:  86/ 846  [     magic missile]熟練度:  80/ 186
[        protection]熟練度:  99/  14  [           refresh]熟練度:  99/  24
[      remove curse]熟練度:  91/ 922  [         sanctuary]熟練度:  99/   7
[            shield]熟練度:  99/   7  [        stone skin]熟練度:  99/  42
[              lore]熟練度:  99/  22  [         lifesteal]熟練度:  75/  71
[         soulsteal]熟練度:  99/  14  [     remove nodrop]熟練度:  85/ 419
[           barrier]熟練度:  99/   2  [        dark space]熟練度:  85/ 572
[             dodge]熟練度:  99/  14  [        earthslice]熟練度:  99/  10
[              scan]熟練度:  99/   1  [           agility]熟練度:  99/   5
[       outdo waver]熟練度:  79/ 912  [       spellmaster]熟練度:  99/   3
```

`2821/2821 1911/1911 2034/2034 413/401`

### 22. mag-swd

魔劍配置，技能較多，僅供參考（還有很多沒練完的技能）。

```
[             bless]熟練度:  99/  51  [         blindness]熟練度:  99/  36
[     burning hands]熟練度:  99/ 106  [    call lightning]熟練度:  99/  86
[       chill touch]熟練度:  99/   4  [   continual light]熟練度:  99/  55
[   control weather]熟練度:  99/ 103  [       create food]熟練度:  99/   7
[     create spring]熟練度:  99/  84  [      create water]熟練度:  99/  36
[    cure blindness]熟練度:  99/  54  [        cure light]熟練度:  99/  40
[       cure poison]熟練度:  83/ 927  [      cure serious]熟練度:  99/   1
[             curse]熟練度:  94/1007  [       detect evil]熟練度:  99/  71
[      detect invis]熟練度:  99/  56  [      detect magic]熟練度:  99/  85
[     detect poison]熟練度:  85/   0  [      dispel magic]熟練度:  99/  55
[       faerie fire]熟練度:  94/ 264  [        faerie fog]熟練度:  99/  74
[          fireball]熟練度:  99/  42  [       flamestrike]熟練度:  99/  47
[               fly]熟練度:  99/  84  [              heal]熟練度:  99/  94
[          identify]熟練度:  99/  42  [       infravision]熟練度:  99/  71
[             invis]熟練度:  99/ 100  [    know alignment]熟練度:  99/ 103
[     locate object]熟練度:  99/  88  [     magic missile]熟練度:  99/ 106
[        mass invis]熟練度:  99/  36  [          illusion]熟練度:  99/  77
[            poison]熟練度:  81/ 561  [        protection]熟練度:  99/  20
[           refresh]熟練度:  99/  52  [      remove curse]熟練度:  93/   4
[         sanctuary]熟練度:  99/  20  [     ventriloquate]熟練度:  99/ 103
[              lore]熟練度:  99/ 106  [         lifesteal]熟練度:  75/ 560
[         soulsteal]熟練度:  99/  20  [     recharge item]熟練度:  85/ 497
[         powerball]熟練度:  99/ 106  [           cyclone]熟練度:  95/  14
[              scan]熟練度:  81/ 337  [       swordmaster]熟練度:  99/  24
[         fireslash]熟練度:  99/ 130  [        flameslash]熟練度:  99/  95
[         fusislash]熟練度:  98/ 715  [         lifeslash]熟練度:  99/  84
[       shadowslash]熟練度:  99/   2  [      shanyanslash]熟練度:  96/ 725
[         soulslash]熟練度:  99/  84
```

### 23. bare-kni

角色：弱拳騎士 Eastbxr（等級 60，力量 30 智力 12 睿智 14 速度 28 體格 22）

- 生命力: 3160 / 精神力: 908 / 移動力: 1642 / 內力: 1997
- 加強命中率: 41 / 加強傷害率: 84
- 內力強度 1638，魔法強度 136

```
[             bless]熟練度:  99/  35  [         blindness]熟練度:  99/  28
[   continual light]熟練度:  99/   3  [   control weather]熟練度:  99/  26
[       create food]熟練度:  99/   7  [     create spring]熟練度:  99/  26
[      create water]熟練度:  99/  26  [    cure blindness]熟練度:  99/  16
[        cure light]熟練度:  99/  18  [       cure poison]熟練度:  99/  31
[      cure serious]熟練度:  99/  22  [             curse]熟練度:  99/  28
[      detect invis]熟練度:  99/  10  [      detect magic]熟練度:  99/  11
[     detect poison]熟練度:  99/  30  [      dispel magic]熟練度:  99/  18
[       faerie fire]熟練度:  99/  20  [               fly]熟練度:  99/  16
[              heal]熟練度:  99/   6  [          identify]熟練度:  99/  24
[       infravision]熟練度:  99/  24  [     locate object]熟練度:  99/   8
[     magic missile]熟練度:  77/ 854  [        protection]熟練度:  99/  30
[           refresh]熟練度:  99/   2  [      remove curse]熟練度:  99/  27
[         sanctuary]熟練度:  99/   7  [     remove nodrop]熟練度:  99/  35
[             dodge]熟練度:  99/   1  [   enhanced damage]熟練度:  99/   3
[     second attack]熟練度:  99/   2  [      third attack]熟練度:  99/   2
[         bare fist]熟練度:  99/   4  [             blitz]熟練度:  99/   2
[         reflexion]熟練度:  99/  11  [              scan]熟練度:  99/   2
[          ejinjing]熟練度:  99/   3  [         cometfist]熟練度:  99/   2
[        dragonfist]熟練度:  99/   6  [         lohanfist]熟練度:  99/  25
[           sadfist]熟練度:  99/  25  [        saulinfist]熟練度:  99/  47
```

### 24. mag-kni

角色：時空冒險者 Linix（等級 60，力量 30 智力 12 睿智 14 速度 30 體格 22）

- 生命力: 3002 / 精神力: 962/1097 / 移動力: 2469 / 內力: 505

```
[             bless]熟練度:  99/  16  [         blindness]熟練度:  99/   9
[   control weather]熟練度:  99/  15  [    cure blindness]熟練度:  99/  30
[        cure light]熟練度:  99/  25  [       cure poison]熟練度:  99/  16
[      cure serious]熟練度:  99/  30  [             curse]熟練度:  99/   6
[       detect evil]熟練度:  99/  25  [     detect hidden]熟練度:  93/ 682
[      detect invis]熟練度:  99/  23  [      detect magic]熟練度:  99/  26
[     detect poison]熟練度:  99/   5  [      dispel magic]熟練度:  99/  23
[       faerie fire]熟練度:  99/  12  [        faerie fog]熟練度:  90/ 160
[               fly]熟練度:  99/  29  [              heal]熟練度:  99/   5
[          identify]熟練度:  99/  27  [       infravision]熟練度:  97/ 460
[             invis]熟練度:  99/  32  [    know alignment]熟練度:  97/ 204
[     locate object]熟練度:  99/  34  [     magic missile]熟練度:  99/   5
[        protection]熟練度:  99/  18  [           refresh]熟練度:  99/   4
[      remove curse]熟練度:  99/  11  [         sanctuary]熟練度:  99/   9
[              lore]熟練度:  97/ 363  [     remove nodrop]熟練度:  99/  18
[      full refresh]熟練度:  90/ 815  [           cyclone]熟練度:  89/  56
[             dodge]熟練度:  99/   7  [   enhanced damage]熟練度:  99/   2
[     second attack]熟練度:  99/   5  [      third attack]熟練度:  99/   4
[               aid]熟練度:  99/   3  [             blitz]熟練度:  99/   3
[        earthslice]熟練度:  99/  10  [              scan]熟練度:  99/   2
[         windslice]熟練度:  99/   9  [        two weapon]熟練度:  99/   1
```

### 25. sole-swd (Protoss)

角色：弱劍士 Protoss（等級 60，力量 30 智力 22 睿智 14 速度 30 體格 14）

- 生命力: 2760 / 精神力: 249 / 移動力: 5384 / 內力: 2543
- 加強命中率: 61 / 加強傷害率: 29

```
[   control weather]熟練度:  99/  22  [        cure light]熟練度:  99/  18
[      cure serious]熟練度:  99/  53  [               fly]熟練度:  99/  15
[           refresh]熟練度:  99/  39  [      full refresh]熟練度:  99/  57
[       swordmaster]熟練度:  65/ 721  [        bloomslash]熟練度:  99/  42
[      chunshislash]熟練度:  99/  43  [         fusislash]熟練度:  99/  43
[       shadowslash]熟練度:  99/  40  [      shanyanslash]熟練度:  99/  43
[        taigislash]熟練度:  99/  45  [        twoyislash]熟練度:  99/  22
[         girlslash]熟練度:  99/   8  [       flowerslash]熟練度:  99/  21
[         soleslash]熟練度:  99/  37
```

### 26. sole-swd（屬性配置版）

屬性配置：

- Change Body: 力量 22, 智力 18, 睿智 14, 速度 22, 體格 14
- Train: 力量 08, 智力 04, 睿智 00, 速度 08, 體格 00
- 最終: 力量 30, 智力 22, 睿智 14, 速度 30, 體格 14

> **註解**：INT 為什麼要 22 呢？其實是為了讓 MOB 的 Delay 增加。根據研究發現 INT 22 的 soleslash 每一劍可造成對方 250 的 Delay，所以建議練 INT 22 的劍客。

```
[             armor]熟練度:  71/   0  [   control weather]熟練度:  81/   0
[    cure blindness]熟練度:  76/  74  [        cure light]熟練度:  81/   0
[      cure serious]熟練度:  90/  64  [               fly]熟練度:  71/ 148
[           refresh]熟練度:  73/ 598  [      full refresh]熟練度:  85/ 743
[             dodge]熟練度:  99/   6  [              scan]熟練度:  74/ 340
[       swordmaster]熟練度:  74/ 863  [      chunshislash]熟練度:  99/  45
[         fusislash]熟練度:  99/  39  [       shadowslash]熟練度:  99/  14
[      shanyanslash]熟練度:  99/  14  [        taigislash]熟練度:  99/   7
[        twoyislash]熟練度:  99/   1  [         girlslash]熟練度:  99/   9
[       flowerslash]熟練度:  99/   8  [         soleslash]熟練度:  99/   4
```

### 27. fat-out-wiz

```
[             armor]熟練度:  99/  21  [             bless]熟練度:  99/   7
[         blindness]熟練度:  78/ 300  [   control weather]熟練度:  99/  44
[       create food]熟練度:  99/  37  [     create spring]熟練度:  99/  42
[      create water]熟練度:  99/   7  [    cure blindness]熟練度:  99/  36
[        cure light]熟練度:  99/  44  [       cure poison]熟練度:  99/  15
[      cure serious]熟練度:  99/  37  [             curse]熟練度:  71/  80
[       detect evil]熟練度:  99/  16  [     detect hidden]熟練度:  99/   6
[      detect invis]熟練度:  77/ 730  [      detect magic]熟練度:  81/  35
[     detect poison]熟練度:  81/   0  [      dispel magic]熟練度:  94/ 827
[       faerie fire]熟練度:  71/   0  [        faerie fog]熟練度:  71/   0
[               fly]熟練度:  75/ 848  [              heal]熟練度:  76/ 880
[          identify]熟練度:  74/ 853  [       infravision]熟練度:  81/   0
[    know alignment]熟練度:  81/   0  [     locate object]熟練度:  79/ 178
[        protection]熟練度:  74/ 369  [           refresh]熟練度:  99/  30
[      remove curse]熟練度:  81/ 616  [         sanctuary]熟練度:  79/ 576
[            shield]熟練度:  73/ 639  [        stone skin]熟練度:  73/ 759
[              lore]熟練度:  75/  77  [         lifesteal]熟練度:  74/ 248
[         soulsteal]熟練度:  94/ 501  [     remove nodrop]熟練度:  71/ 129
[           barrier]熟練度:  84/ 141  [        dark space]熟練度:  77/ 198
[             dodge]熟練度:  99/   2  [        earthslice]熟練度:  99/  20
[              scan]熟練度:  73/ 161  [           agility]熟練度:  99/   2
[       outdo waver]熟練度:  99/   7  [       spellmaster]熟練度:  99/  10
```
