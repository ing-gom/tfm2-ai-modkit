# 챔피언 카탈로그 (드래프트 인덱스 순서)

`DraftScoreContext`의 `candidate` 및 `available_champions`/`ally_pick`/… 는 이 **고정 카탈로그 순서**의
인덱스(usize)다. 런타임 덤프로 확인됨(0..59). `draft_ai` 예제의 `NAMES` 배열과 동일.

| idx | id | idx | id | idx | id |
|----:|----|----:|----|----:|----|
| 0 | fighter | 20 | jiangshi | 40 | ghost |
| 1 | knight | 21 | gambler | 41 | illusionist |
| 2 | swordman | 22 | hammerer | 42 | lightning_mage |
| 3 | archer | 23 | demon | 43 | plague_doctor |
| 4 | soldier | 24 | vampire | 44 | poison_dart_hunter |
| 5 | priest | 25 | spirit_caller | 45 | shadowmancer |
| 6 | pythoness | 26 | boomerang_hunter | 46 | taoist |
| 7 | monk | 27 | inquisitor | 47 | siege_breaker |
| 8 | pyromancer | 28 | shield_bearer | 48 | android |
| 9 | ice_mage | 29 | whip_master | 49 | druid |
| 10 | ninja | 30 | werewolf | 50 | prisoner |
| 11 | magic_knight | 31 | dokkaebi | 51 | bomber |
| 12 | berserker | 32 | necromancer | 52 | voodoo_shaman |
| 13 | executioner | 33 | bard | 53 | white_mage |
| 14 | lancer | 34 | barrier_magician | 54 | wind_mage |
| 15 | ogre | 35 | chef | 55 | enchanter |
| 16 | dual_blader | 36 | clown | 56 | hitman |
| 17 | cavalry_knight | 37 | dancer | 57 | guardian_spirit |
| 18 | gunner | 38 | dark_mage | 58 | hunter |
| 19 | pole_warrior | 39 | exorcist | 59 | circus_blade |

> ⚠️ 모드(데이터 챔피언)로 챔프가 추가되면 인덱스가 밀릴 수 있다. 가능하면 인덱스를 하드코딩하기보다
> `NAMES`로 id↔idx를 매핑해 쓰라. 또한 드래프트 시점 `ChampionInfo::tags()/category()`는 비어 있으므로
> 태그 기반 로직은 별도 데이터(JSON)가 필요하다.
