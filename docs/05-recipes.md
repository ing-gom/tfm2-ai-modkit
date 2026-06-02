# 05 · 레시피 (자주 쓰는 패턴)

> `shared/ai_common.rs`의 헬퍼(`Logger`, `Cfg`, `input::*`, `ctx_snapshot`)를 전제로 함.

## A. 특정 대상/포지션만 처리

```rust
fn matches(&self, ctx: &PlayerAiInitContext) -> bool {
    ctx.team == 0 && ctx.position == Position::Bottom   // 내 팀 봇만
}
```

## B. 저체력 안전 리콜 (검증 포함)

```rust
if let Some(hp) = ctx.hp_ratio_percent() {
    if hp <= 35 && ctx.is_safe_to_recall() {
        if let Some(r) = ctx.get_recall_input() {
            if ctx.is_valid_input(&r) { return PlayerInputDecision::Replace(r); }
        }
    }
}
```

## C. 위험할 때 스킬 안 쓰고 빠지기

```rust
if ctx.is_hp_below_percent(20) {
    if let Some(run) = ctx.get_run_away_without_skill_input() {
        if ctx.is_valid_input(&run) { return PlayerInputDecision::Replace(run); }
    }
}
```

## D. 내장 AI 결정 관찰(튜닝용)

```rust
if ctx.tick() % 30 == 0 {
    logger().capped(&ai_common::ctx_snapshot(ctx, &base_input));
}
return PlayerInputDecision::Pass;   // 읽기 전용
```
→ 저능력/고능력 선수를 각각 돌려 `base=Attack{Target{…}}` / `Skill{Dir{…}}` 패턴을 비교.
이게 `examples/match_tuner`가 하는 일.

## E. base_input을 조건부로만 덮기

```rust
match base_input {
    // 내장 AI가 공격을 고른 경우에만 타깃을 바꾸고 싶다면
    Some(Input::Attack { .. }) => {
        let better = ai_common::input::attack(my_target_id);
        if ctx.is_valid_input(&better) {
            return PlayerInputDecision::Replace(better);
        }
        PlayerInputDecision::Pass
    }
    _ => PlayerInputDecision::Pass,   // 나머진 그대로
}
```

## F. 콜백 간 상태 누적 (핸들 저장 금지 → id/값만)

```rust
use std::sync::{Mutex, OnceLock};
fn state() -> &'static Mutex<MyState> {
    static S: OnceLock<Mutex<MyState>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(MyState::default()))
}
// think 안에서: state().lock().unwrap() 으로 접근. 핸들/참조는 절대 저장하지 말 것.
```

## G. 드래프트 점수 가감

```rust
fn score_pick(&self, ctx: &DraftScoreContext, cand: usize, base: f32) -> DraftScoreDecision {
    // 적이 AD 위주면 특정 챔프 가중 (base_score 기준으로 nudge)
    if cand == MY_FAVOURED && ctx.enemy_pick.len() >= 2 {
        return DraftScoreDecision::Add(40.0);
    }
    DraftScoreDecision::Pass
}
```

## H. 의존성 없는 설정 토글

```rust
let c = ai_common::Cfg::load("my_mod");      // %TEMP%\my_mod.cfg or mods\my_mod\my_mod.cfg
let aggressive = c.bool("aggressive", false);
let hp_gate    = c.usize("hp_gate", 30);
```

---

## 하지 말 것

- `think()`에서 무거운 연산/할당 루프 (매 틱·매 선수 호출).
- `is_valid_input()` 없이 `Replace` (그 프레임 무시되고 디버깅만 어려워짐).
- 콜백 사이 핸들/참조 저장 (샌드박스 위반 → UB/크래시 위험). id·좌표 같은 값만 보관.
- 한 키를 여러 모드가 동시에 덮어쓰는 구조 (priority로 순서 명시).

## 더 깊이

`examples/match_tuner`로 내장 AI의 `base_input`을 경기 중에 로깅하면, 어떤 상황에서 어떤
`Input`/`InputTarget`이 나오는지 데이터로 관찰할 수 있다 — 행동 교정 로직을 만들기 전 출발점.
