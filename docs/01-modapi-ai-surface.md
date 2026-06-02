# 01 · mod_api AI 표면 (정확한 시그니처)

> 출처: SDK `mod_api` rustdoc(`_apiprobe/apidoc`). API 버전 결합 — SDK 갱신 시 재확인.

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

## 등록 & 우선순위

```rust
let mut reg = ModRegistration::new("my_mod");      // 폴더명과 일치해야 함
reg.add_player_input_ai(MyAi);
reg.add_draft_score_hook(MyDraftHook);
reg   // init에서 반환
```

`priority()` **낮을수록 먼저** 실행, 높은 게 나중에 앞 결과를 덮어쓴다.

## ModExtension (클라이언트 프레임 훅 — 참고)

```rust
on_init / pre_update / post_update / pre_render / post_render / on_end
// scene / UI / assets / RenderState 접근. 매치 시뮬 결정엔 관여 안 함.
```
