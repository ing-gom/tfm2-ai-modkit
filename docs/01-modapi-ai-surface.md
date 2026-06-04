# 01 · mod_api AI 표면 (정확한 시그니처)

> 출처: SDK `mod_api` rustdoc. **기준 버전: SDK base 0.4.8** (EA). API 버전 강결합 —
> 다른 SDK 버전에서는 시그니처가 바뀔 수 있으니 그 버전 rustdoc으로 재확인할 것.

## ModPlayerInputAi — 인-매치 매 틱 결정 교체

```rust
pub trait ModPlayerInputAi: Send + Sync + Debug {
    fn clone_box(&self) -> Box<dyn ModPlayerInputAi>;   // 필수
    fn id(&self) -> &str;                                // 필수
    fn think(&mut self,
             ctx: &mut PlayerAiContext,
             base_input: Option<Input>) -> PlayerInputDecision;  // 필수
    fn priority(&self) -> i32 { 0 }                      // 제공(override 가능)
    fn matches(&self, _ctx: &PlayerAiInitContext) -> bool { true }  // 제공
}
```

- `base_input` = 내장 AI가 이번 틱에 고른 입력(없으면 `None`).
- 반환 `PlayerInputDecision::{Pass, Replace(Input)}`.
- `matches()`로 적용 대상 한정(team/position/champion). `false`면 그 선수는 스킵.

> 내부 플래닝 타입(`SmallActionPlay`, `Operation`, `Strategy`)은 **의도적으로 숨겨져** 있고
> `PlayerAiContext` 헬퍼 뒤로만 노출된다(공식 rustdoc 문구). 모드는 최종 `Input`만 다룬다.

### PlayerAiContext (헬퍼 메서드만 노출)

```rust
ctx.player_id() -> usize
ctx.athlete_id() -> usize
ctx.team() -> usize
ctx.position() -> Position            // Top/Jungle/Mid/Bottom/Support, Display 구현
ctx.champion_name() -> &str
ctx.tick() -> usize
ctx.hp() -> Option<usize>
ctx.max_hp() -> Option<usize>
ctx.hp_ratio_percent() -> Option<usize>
ctx.is_hp_below_percent(threshold: usize) -> bool
ctx.is_valid_input(&Input) -> bool                    // Replace 전 반드시 검증
ctx.get_recall_input() -> Option<Input>
ctx.get_run_away_input() -> Option<Input>
ctx.get_run_away_without_skill_input() -> Option<Input>
ctx.is_safe_to_recall() -> bool
```

### PlayerAiInitContext (matches에 전달)

```rust
pub struct PlayerAiInitContext {
    pub player_id: usize, pub athlete_id: usize,
    pub team: usize, pub position: Position, pub champion_name: String,
}
```

### Input / InputTarget (행동의 전체 어휘)

```rust
pub enum Input {
    Move { x: u64, y: u64 },
    Return,                                 // 귀환(리콜)
    Attack { target: InputTarget },
    Skill  { target: InputTarget },
    Skill2 { target: InputTarget },
    Ult    { target: InputTarget },
}
pub enum InputTarget {
    Target { target_id: usize },            // 특정 엔티티
    Dir    { dir_x: i64, dir_y: i64 },      // 스킬샷 방향
    Pos    { x: u64, y: u64 },              // 지점
    None,
}
// InputTarget::adjust(from_x, from_y, range) -> InputTarget   // 사거리 보정
// Input::is_act() -> bool
```

`shared/ai_common.rs::input` 에 생성 헬퍼(`attack(id)`, `skill_at(x,y)`, `skill_dir(dx,dy)`, `recall()` …) 있음.

## ModDraftScoreHook — 드래프트 픽/밴 점수 조정

```rust
pub trait ModDraftScoreHook: Send + Sync + Debug {
    fn id(&self) -> &str;                                // 필수
    fn priority(&self) -> i32 { 0 }
    fn score_pick(&self, ctx: &DraftScoreContext, candidate: usize, base_score: f32)
        -> DraftScoreDecision { DraftScoreDecision::Pass }
    fn score_ban(&self,  ctx: &DraftScoreContext, candidate: usize, base_score: f32)
        -> DraftScoreDecision { DraftScoreDecision::Pass }
}

pub enum DraftScoreDecision { Pass, Add(f32), Replace(f32) }

pub struct DraftScoreContext<'a> {
    pub phase: DraftScorePhase,
    pub available_champions: &'a [usize],
    pub ally_ban: &'a [usize],  pub enemy_ban: &'a [usize],
    pub ally_pick: &'a [usize], pub enemy_pick: &'a [usize],
    pub is_explore: bool,
    pub difficulty: Difficulty,
}
```

- `candidate` = 카탈로그 순서 인덱스(0..59) → [`champion-catalog.md`](champion-catalog.md).
- `base_score` = **엔진 자체 드래프트 평가값** (가장 신뢰할 신호).
- ⚠️ 확인됨: 드래프트 시점에 `ChampionInfo::category()/tags()`는 **채워져 있지 않다**(전부 Melee/[]).
  → 태그 기반 시너지 엔진은 데이터가 없음. `base_score`를 기준으로 가감하라.

## ModServerExtension — 관리/DB 계층에서 능력치 직접 수정

인-매치 AI 행동은 게임이 각 선수의 **raw 능력치(`AthleteStat`)에서 파생**한다. 그 파이프라인 자체는
모드가 못 건드리지만(게임 바이너리에 있음), **소스 능력치는 관리 계층에서 직접 읽고 쓸 수 있다** —
이게 "AI의 숫자를 바꾸는" 공식 경로다. 능력치를 올리면 그만큼 AI가 더 잘 둔다.

```rust
pub trait ModServerExtension: Send + Sync + Debug {
    fn on_server_start(&self, _ctx: &mut ServerModContext) {}       // 세이브 로드 시 1회
    fn before_management_tick(&self, _ctx: &mut ServerModContext) {} // 관리 틱 직전
    fn after_management_tick(&self, _ctx: &mut ServerModContext) {}  // 관리 틱 직후(훈련/노화 반영 뒤)
}

pub struct ServerModContext<'a> {
    pub mod_id: &'a str,
    pub database: &'a mut Database,        // .athletes 등 전체 DB
    pub server_state: &'a mut ServerState,
}
impl ServerModContext<'_> {
    pub fn player_team_id(&self, player_id: PlayerId) -> Option<usize>;
    pub fn team_player_ids(&self, team_id: usize) -> Vec<PlayerId>;
}
```

`ctx.database.athletes.iter_mut()` → `&mut Athlete`:

```rust
pub struct Athlete {
    pub id: usize, pub name: String, pub age: usize,
    pub stat: AthleteStat,             // ← 인-매치 행동을 만드는 능력치 (편집 대상)
    pub hidden: AthleteHiddenStat,
    pub initial_stat: AthleteStat, pub prev_week_stat: AthleteStat, /* …25 fields */
}
impl Athlete { pub fn with(&self, team_id: usize) -> bool; pub fn main_position(&self) -> Position; /* … */ }
```

- `AthleteStat`은 `Serialize`/`Deserialize` → `serde_json`으로 **필드명 기준** 라운드트립 편집이 가능
  (필드 추가 패치에도 안 깨짐). serde 필드명(=인게임 능력치): `judgement`(판단·결정 품질),
  `concentration`(집중·후반 안정성), `control_speed`(컨트롤·반응속도), `skill_hit`(스킬 적중),
  `skill_avoid`(스킬 회피), `positioning`(포지셔닝), `last_hit`(CS), `mental`, `order`/`roaming`/
  `aggressive`/`ego`(플레이 성향), `stamina`/`condition`.
- 관리 틱마다 훈련·노화가 능력치를 다시 굴리므로, **변경을 유지하려면 `after_management_tick`에서 재적용**.
- 복사해 쓸 수 있는 완성형 코드(set/floor/cap/scale, 한 팀만, 즉시 1회 적용): [`05-recipes.md`](05-recipes.md) §I·§J.
  `serde_json`을 쓰므로 빌드 스크립트가 prebuilt rlib을 `--extern`으로 자동 주입한다(소스가 참조하면).

## GameCtx — 인-매치 월드 조회 / 변형 (참고)

`init(&GameCtx)`에 전달되는 월드 핸들. 조회 + 월드 변형 메서드를 갖지만, **매 틱 GameCtx를 넘겨주는
공식 훅은 아직 없다**(think엔 `PlayerAiContext`만, 프레임 훅엔 Scene/UI/Assets만 전달). 따라서 인-매치
변형(`deal_damage` 등) 호출 경로는 미검증 — 조회/설계 참고용으로만.

```rust
// 조회
ctx.tick()->usize  ctx.seed()->u64  ctx.is_end()->bool  ctx.score_diff(team)->i32
ctx.get_entity(id)->Option<EntityRef>  ctx.entity_count()/entity_at(i)
ctx.get_player(id)->Option<PlayerRef>  ctx.player_count()/player_at(i)
ctx.champion_count()/champion_id_at(i)  ctx.tower_count()/tower_id_at(i)
ctx.projectile_count()/projectile_at(i)  ctx.kill_log_count()/kill_log_at(i)->KillLogEntry
ctx.distance_sq(id1,id2)->u64  ctx.is_visible(team,id)->bool
// 변형 (&mut self — 전달 경로 미검증)
ctx.deal_damage(attacker,target,ad,ap,attack_type)  ctx.heal(caster,target,amount)
ctx.add_buff(target,BuffState)  ctx.apply_cc(target,CCState)
ctx.debug_draw_line(...)/debug_draw_circle(...)
// 모드 간 서비스: ctx.register_service(...) / ctx.query_service(...)

EntityRef: handle/id/team/level/pos/hp/shield/radius/stat()->EntityStat/
           is_alive/is_champion/is_tower/is_minion/is_targetable/buff_count/buff_at/cc_count/cc_at
PlayerRef: handle/champion()->Option<EntityRef>/team/position/level/gold/cs/kills/deaths/assists/
           is_alive/respawn_time
#[repr(C)] EntityStat { attack, magic_power, hp, defence, magic_resistance,
                        move_speed, hp_regen, stack, crit_chance }   // 전부 usize, 데미지/생존 전용
```

## 등록 & 우선순위

```rust
let mut reg = ModRegistration::new("my_mod");      // 폴더명과 일치해야 함
reg.add_player_input_ai(MyAi);                     // 인-매치 입력 교체
reg.add_draft_score_hook(MyDraftHook);             // 드래프트 점수
reg.set_server_extension(MyServerExt);             // 관리/DB 계층 (능력치 편집)
reg.set_extension(MyClientExt);                    // 클라 프레임 훅
// reg.add_champion(...) / reg.add_item(...)        // 콘텐츠 추가
reg   // init에서 반환
```

`priority()` **낮을수록 먼저** 실행, 높은 게 나중에 앞 결과를 덮어쓴다.

## ModExtension (클라이언트 프레임 훅 — 참고)

```rust
on_init / pre_update / post_update / pre_render / post_render / on_end
// scene / UI / assets / RenderState 접근. 매치 시뮬 결정엔 관여 안 함.
```
