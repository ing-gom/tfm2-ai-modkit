# 03 · SDK 지원 표면 전체 지도 (mod_api 0.4.8)

> SDK `mod_api`가 **실제로 무엇을 지원하는지** 한 장으로 본다. 출처: 0.4.8 rlib로 재생성한 rustdoc
> (`rustc 1.98.0-nightly d595fce01`, `nightly-2026-06-03`) 전수 + **컴파일 프로브로 구현 가능 여부 검증**.
> 0.4.7→0.4.8 항목 인벤토리는 **완전 동일**(구조체 58·enum 23·trait 11·매크로 1·함수 1·타입별칭 3·상수 4),
> 예제 4종·내부 모드 6종 무수정 재빌드 → 공개 시그니처 churn 없음.

mod_api는 표면적으로 **11개 trait**을 노출하지만, "trait이 존재한다 ≠ 모드가 구현할 수 있다". 어떤 trait은
**반환 타입이 비공개**라 모드가 시그니처조차 못 쓴다(=구현 불가). 이 문서는 그 경계를 **프로브로 확정**해 둔다.

## 한눈에 보기 — 지원 매트릭스

| 확장점 | 진입점(등록) | 무엇을 하나 | 상태 |
|---|---|---|---|
| `ModPlayerInputAi` | `reg.add_player_input_ai` | 인-매치 매 틱 최종 `Input` 교체 | ✅ **사용 가능**(플래그십) |
| `ModDraftScoreHook` | `reg.add_draft_score_hook` | 드래프트 픽/밴 후보 점수 가감 | ✅ **사용 가능** |
| `ModServerExtension` | `reg.set_server_extension` | 관리 계층 DB·선수 능력치(`AthleteStat`) 편집, 서버 커맨드/이벤트 | ✅ **사용 가능** |
| `ModExtension` | `reg.set_extension` | 클라 프레임 훅 + **클라 DB 조회 · 모드 영구저장(`mod_save_data`)** | ✅ DB 조회·영구저장(프로브 검증) / 🔬 게임상태 쓰기 효과·렌더 |
| `GameCtx`(조회/변형) | `init(&GameCtx)` 인자 | 월드 조회·변형 | 🔬 init만 전달·인-매치 브리지 닫힘 |
| `ModChampionInfo` | `reg.add_champion` | 새 챔피언 정의 | ⛔ **선언됐으나 구현 불가** |
| `ModItemInfo` | `reg.add_item` | 새 아이템 정의 | ⛔ **선언됐으나 구현 불가** |
| `ModAction` / `ModPassive` / `ModEffectType` / `ModEffectBuff` | (위 두 trait 경유만) | 스킬·패시브·효과·버프 빌딩블록 | ⛔ 도달 경로 비공개 |
| `ChampionInfo` | — (런타임 조회 trait) | 챔피언 런타임 스탯/패치 조회 | 🔬 미검증 |

범례: ✅ 빌드+동작 검증 · 🔬 빌드되나 인-매치 효과 미검증 · ⛔ 프로브로 **구현 불가** 확정.

---

## A. 사용 가능한 확장점 (✅ — 모드가 실제로 쓰는 길)

세부 시그니처는 [`01-modapi-ai-surface.md`](01-modapi-ai-surface.md), 행동 교정 워크플로는
[`02-behavior-correction.md`](02-behavior-correction.md).

1. **`ModPlayerInputAi`** — 인-매치 매 틱 `think(ctx, base_input) -> Pass | Replace(Input)`.
   self-상태(hp·position·champion·tick)와 엔진 캔액션(recall/run_away)으로 행동 교정. **이 키트의 1순위.**
   예제: `ai_survival`(플래그십), `ai_perf`, `match_tuner`(관찰).
2. **`ModDraftScoreHook`** — 픽/밴마다 `score_pick/score_ban(ctx, candidate, base_score) -> Pass | Add | Replace`.
   `base_score`(엔진 자체 평가)를 기준으로 가감. 예제: `draft_ai`.
3. **`ModServerExtension`** — `on_server_start` / `before·after_management_tick(&mut ServerModContext)` /
   `handle_command(&mut ServerModContext, &ModServerCommand) -> ModServerCommandResult`.
   - `ctx.database: &mut Database` — **관리 DB**. `Athlete.stat`(`AthleteStat`)을 `serde_json` 라운드트립으로 편집.
     관리 틱마다 재적용 필요. 능력치를 올리면 내장 AI가 그만큼 더 잘 둔다(입력단 개선).
   - `ctx.emit_event / emit_event_to_player / emit_event_to_team / emit_event_to_command_sender(...)` —
     서버→클라 **모드 이벤트** 송신(payload: bytes). `handle_command`로 클라가 보낸 `ModServerCommand` 수신.
     → 모드가 자체 클라이언트↔서버 메시징을 구성할 수 있다.
   - 헬퍼: `ctx.player_team_id(player_id)`, `ctx.team_player_ids(team_id)`.
4. **`ModExtension`** — `on_init/pre_update/post_update/pre_render/post_render/on_end`. 각 훅이 받는
   **`&mut Scene`** 가 인-매치일 때 `Scene::InGame { data: ClientData }` 로, 여기서 **클라이언트 DB 전체에 도달**한다.
   이게 "DB 접근을 모드에 열어준" 실제 경로다(콘텐츠 저작과 달리 **프로브로 도달 확인** — `_apiprobe/probe_db.rs`).

   ```rust
   fn pre_update(&self, scene: &mut Scene, _ui: &mut UI<(),UIOutEvent>, _assets: &mut Assets, _dt: f32) {
       if let Scene::InGame { data } = scene {
           let db = data.db();                 // Ref<ClientDatabase>  — 조회
           for (_id, a) in &db.athletes { /* a.stat 읽기 … */ }
           let _teams = &db.teams;             // teams/staffs/leagues/matches/knowledge_bases …
           drop(db);
           let mut db = data.db_mut();          // RefMut — 쓰기
           db.mod_save_data.set_string("my_mod", "key", "value");   // ← 영구저장(세이브에 포함)
       }
   }
   ```

   - **조회**(✅ 검증): `ClientData::db()/db_mut()` → `ClientDatabase` 공개 컬렉션 `teams/athletes/staffs/leagues/
     knowledge_bases/matches/match_replays/league_competitions/...` 직접 읽기. 매니지먼트 전체 상태가 보인다.
   - **모드 영구저장**(✅ 검증·의도된 용도): `db.mod_save_data: ModSaveData` 는 **모드 전용 네임스페이스 키-값
     스토어**(세이브 파일에 저장됨) — `set_string/get_string`, `set_bytes/get_bytes`, `set_version/save_version`,
     `keys/contains_key/remove_key/clear_namespace`. 모드 설정·진행도를 게임 세이브와 함께 보존하는 정식 경로.
   - **게임상태 쓰기**(🔬): `db_mut()`로 `athletes` 등도 변경 자체는 컴파일되나, 그 변경이 게임 진행에 반영되는지(이
     `ClientDatabase`가 권위 상태인지 클라 뷰인지)는 인게임 검증 필요. **능력치 영구 편집은 `ModServerExtension`
     경로(§3, 검증됨)를 쓰라.** `ModExtension`은 조회·영구저장·오버레이가 검증된 용도.
   - 렌더: `RenderState` 접근하나 커스텀 캔버스는 인자 타입(`RenderCommand`)이 비공개라 막힘(프로브 확인).
   - 예제: **`examples/db_inspector`** — 매치 진입마다 클라 DB(선수/팀 수) 조회 + `mod_save_data`에 진입
     횟수를 세이브에 영구 저장(세션 간 유지). 위 패턴의 빌드 검증된 최소 스타터.
5. **`GameCtx`**(🔬) — `init(&GameCtx)`에만 전달되는 월드 핸들. `deal_damage/heal/add_buff/apply_cc/
   get_entity/distance_sq/is_visible` 등 조회·변형 메서드를 갖지만 **매 틱 GameCtx를 주는 공식 훅이 없고**,
   init 시점엔 sim이 미가동(state=null)이라 호출 시 크래시 → **인-매치 월드 쿼리 경로는 닫혀 있음**. 설계 참고용.

---

## B. 선언됐으나 모드에서 구현 불가 (⛔ — 프로브로 확정)

mod_api에 **콘텐츠 저작 trait과 등록 메서드가 존재**한다(`reg.add_champion` / `reg.add_item`). 그러나 그
trait들의 **필수 메서드가 비공개 타입을 반환**해서, 모드는 impl 블록의 시그니처조차 작성할 수 없다.

```rust
// _apiprobe/probe_content.rs — `use mod_api::*` 후 타입 이름이 resolve 되는지 검사
fn champion_base(_: ChampionBaseInfo) {}   // ❌ E0425 cannot find type
fn growth(_: GrowthInfo) {}                // ❌ E0425
fn attack(_: CharacterAttackInfo) {}       // ❌ E0425
fn action(_: ActionInfo) {}                // ❌ E0425
fn passive(_: PassiveInfo) {}              // ❌ E0425
fn item_stat(_: ItemStat) {}               // ❌ E0425  (game-core\src\setting.rs 내부)
// 대조군 — 이건 공개라 resolve 됨:
fn entity_stat(_: EntityStat) {}           // ✅
fn mod_effect(_: ModEffect) {}             // ✅
fn buff_state(_: BuffState) {}             // ✅
```

따라서:

- **`ModChampionInfo`** — 필수 `stat()->ChampionBaseInfo`, `growth()->GrowthInfo`, `attack()->CharacterAttackInfo`,
  `skill()/skill2()->ActionInfo` 가 전부 비공개 타입 → impl 불가 → `add_champion` **사실상 죽은 경로**.
- **`ModItemInfo`** — 필수 `stat()->ItemStat` 비공개 → impl 불가 → `add_item` 죽은 경로.
- **`ModAction`/`ModPassive`/`ModEffectType`/`ModEffectBuff`** — 타입상 일부는 구현 가능해 보이나, 이들을 게임에
  연결하는 유일 통로가 위 두 trait의 비공개 반환 타입(`ActionInfo`/`PassiveInfo`)이라 **도달 경로가 없음**.

> 이는 AI 내부 플래닝 타입(`SmallActionPlay` 등)이 의도적으로 숨겨진 것과 **동일한 패턴**이다. EA 단계라
> 콘텐츠 저작 API는 형태만 노출되고 생성자 타입은 엔진 내부에 갇혀 있다. **SDK 버전이 오를 때마다 위
> 프로브를 다시 돌려** 이 타입들이 공개로 풀렸는지 재확인할 것(§D).

---

## C. 공개 데이터 타입 카탈로그 (빌딩블록)

콘텐츠 trait은 막혀 있어도, 아래 enum/struct는 **공개**라 (지금은 주로 조회/설계용) 이름을 쓰고 매칭할 수 있다.

**행동·입력**: `Input`, `InputTarget`, `Position`, `PlayerInputDecision`, `DraftScoreDecision` — [`01`](01-modapi-ai-surface.md).

**전투 수치**: `EntityStat`#[repr(C)](attack/magic_power/hp/defence/magic_resistance/move_speed/hp_regen/stack/crit_chance, 전부 usize).

**효과/버프(공개·구성 가능)**:
- `ModEffect{ range, growth_range, start_timing, casting: CastingType, target: CastingTarget, attack_type: AttackType, effect_type: Box<dyn ModEffectType> }`
- `BuffState`#[repr(C)] — 37개 필드 스탯 곱/증감(attack_mult·move_speed_mult·cc_immune·undying·ignore_wall …), `duration: BuffType`.
- `CCInfo{ cc_type: u32, tick: u64 }` — 엔티티의 활성 CC 조회용.

**분류 enum(전부 #[repr(C)])**:
- `ChampionCategory{ Melee, Range, Magician, Util, Assassin }`
- `ChampionSubCategory{ Rush, Sub, CC, Tank, Single, Range, Poking, Assassin }`
- `ChampionTag{ AD, AP, Heal, Shield, Dot, CC, Range, Melee, Tank, Magic }`
- `ItemCategory{ AD, AttackSpeed, Defense, MagicResistance, Magic, Hp }`
- `ItemTag{ AD, AP, AS, Defense, MagicResistance, HP, DefensePenetration, Vamp, HealReduce, ShieldBreak, MoveSpeed, AttackRange, Shield, HpPercentDamage, ASDebuff, ReflectDamage, Toughness, MRDebuff, RangeDamage, MRPenetration, CooltimeReduce, DotDamage, HPRegen, MyHpPercentDamage, ShareDamage, Range }`
- `CastingType{ Targeting, Position, Direction, None }`
- `CastingTarget{ Ally, AllyChampion, AllyChampionInCC, AllyNotSelf, AllyOnlySelf, Enemy, EnemyWithoutTower, EnemyChampion, EnemyChampionInCC, EnemyChampionRecentlyAttacked, Both, BothWithoutTower, BothChampion, None }`
- `BuffType{ Permanent, Time{tick}, WithShield }`
- `DamageType{ AD, AP, Fixed }`
- `AttackType{ BaseAttack, Skill, Dot, DotIgnoreShield, Item, Well }`

**DB 레코드 타입**: `Athlete`, `Team`, `Staff`, `League`, `Tournament`, `MatchInfo`, `MatchReplayData`,
`TeamTrainingPlan`, `TeamResearchData`, `SoloRankMatch`, `RecruitDoneAthlete`, `KnowledgeBase`,
`LeagueCompetition`, `TournamentCompetition` 등 — 필드는 rustdoc 참조. `AthleteStat`만 serde 라운드트립으로
편집(능력치 모드의 핵심), 나머지는 운영 데이터. **접근 2경로**:
- 관리(권위·쓰기): `ModServerExtension` → `ctx.database: &mut Database` (§A-3, 능력치 편집 검증됨).
- 클라(조회·영구저장): `ModExtension` → `Scene::InGame{data: ClientData}` → `data.db()/db_mut()` →
  `ClientDatabase`(같은 컬렉션들 + `mod_save_data`). 조회·영구저장 검증, 게임상태 쓰기 효과는 미검증 (§A-4).

**모드 영구저장 — `ModSaveData`(✅ 의도된 용도)**: `ClientDatabase.mod_save_data`. 모드별 네임스페이스 키-값 스토어,
**게임 세이브에 함께 저장**된다. ID·키 최대 128자.
```rust
set_string(mod_id,key,impl Into<String>)->bool   get_string(mod_id,key)->Option<String>
set_bytes(mod_id,key,Vec<u8>)->bool              get_bytes(mod_id,key)->Option<Vec<u8>>
set_version(mod_id,usize)->bool                  save_version(mod_id)->usize     // 세이브 포맷 버저닝
keys(mod_id)->Vec<String>  contains_key(mod_id,key)->bool  remove_key(mod_id,key)->bool  clear_namespace(mod_id)->bool
namespace(mod_id)->Option<&ModSaveNamespace>     has_namespace/namespace_ids/namespace_count
```

**모드 메시징**: `ModServerCommand`(클라→서버 커맨드) + `ModServerExtension::handle_command -> ModServerCommandResult`,
서버→클라 `ctx.emit_event*` + `ModClientEvent`(`ClientDatabase.mod_events`). 모드 자체 통신 채널 구성용.

> ⚠️ 챔피언 0..59 카탈로그(드래프트 `candidate` 인덱스)는 [`champion-catalog.md`](champion-catalog.md).
> 위 `ChampionInfoSheet`에 60종 내장 챔프(`fighter`·`knight`·…)와 `mod_champions`가 들어 있으나 필드 타입은 비공개.

---

## D. 버전 / 호환 & 직접 재확인하는 법

**상수·매크로**:
- `API_VERSION: (u32, u32)` — 게임이 로드 시 모드 호환성 체크. `decode_api_version()`로 해석.
- `declare_mod!(init)` — DLL 엔트리포인트 선언 매크로. `init(&GameCtx) -> ModRegistration` 한 함수만 쓰면 됨.
- `ModRegistration::new(mod_id)` 후 `add_*` / `set_*` 로 확장점 등록. `mod_id`는 폴더명과 일치.

**SDK가 바뀌면(패치마다) 이 문서를 재검증하는 절차** — 모든 판정은 재현 가능하다:

```powershell
# 1) 0.4.x rlib로 rustdoc 재생성 → 항목 인벤토리(all.html) 비교
$sdk = "..\sdk"; $env:RUSTUP_TOOLCHAIN = (gc "$sdk\rustup_toolchain.txt" -Raw).Trim()
$rlib = (gci "$sdk\deps\libmod_api-*.rlib")[0].FullName
rustdoc --edition 2021 -L "dependency=$sdk\deps" --extern "mod_api=$rlib" `
        _apiprobe\reexport.rs --crate-name modapi_doc -o _apiprobe\apidoc

# 2) 콘텐츠 저작 타입이 공개로 풀렸는지 프로브 (위 §B 코드)
rustc --edition 2021 --crate-type lib -L "dependency=$sdk\deps" --extern "mod_api=$rlib" `
      _apiprobe\probe_content.rs
#   → E0425 사라지면: 콘텐츠 저작이 열린 것. 이 문서 B→A로 승격하고 예제 추가.
```

`reexport.rs`(`pub use mod_api::*`)와 `probe_content.rs`는 private 워크스페이스의 `_apiprobe/`에 있다(공개
키트 비포함). 이 절차로 **"trait이 보인다"가 아니라 "모드가 실제로 구현 가능한가"** 를 매 버전 못 박는다.
