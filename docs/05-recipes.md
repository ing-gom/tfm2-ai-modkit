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

## 두 계층 모델 — 어디서 AI를 바꾸나

| 바꾸려는 것 | 훅 | 시점 | 예제/레시피 |
|---|---|---|---|
| **선수 능력치 자체**(AI가 잘/못 두게) | `ModServerExtension` | 관리/시즌(틱) | 아래 §I·§J |
| **매 틱 최종 결정**(이 상황 이 입력) | `ModPlayerInputAi::think` | 인-매치 | `examples/ai_perf`, §B~§F |
| **드래프트 픽/밴 점수** | `ModDraftScoreHook` | 밴픽 | `examples/draft_ai`, §G |

내부 "능력치→행동" 파이프라인 자체는 후킹할 수 없으므로(공식 mod_api 표면 밖), 모드는 **입력(능력치)**
또는 **출력(최종 Input)** 을 바꾼다.

## I. 능력치 직접 편집 (ModServerExtension)

`serde_json`으로 `AthleteStat`을 필드명 기준 라운드트립 — 필드 추가 패치에도 안 깨진다.
(`serde_json`은 코드가 참조하면 빌드 스크립트가 prebuilt rlib을 `--extern`으로 주입; Cargo.toml에 넣지 말 것.)

```rust
use serde_json::Value;

impl ModServerExtension for MyMod {
    // 관리 틱마다 재적용 — 훈련/노화가 다시 굴려도 유지된다.
    fn after_management_tick(&self, ctx: &mut ServerModContext) {
        for a in ctx.database.athletes.iter_mut() {
            // 한 팀만: if !a.with(MY_TEAM_ID) { continue; }
            let Ok(mut v) = serde_json::to_value(&a.stat) else { continue };
            if let Value::Object(m) = &mut v {
                bump(m, "judgement",     80);   // 80 미만이면 80으로 (결정 품질)
                bump(m, "concentration", 80);   // 후반 안정성
                bump(m, "control_speed", 80);   // 반응속도
            }
            if let Ok(stat) = serde_json::from_value(v) { a.stat = stat; }
        }
    }
}

// floor: 현재값이 n 미만일 때만 올린다(훈련으로 더 높으면 건드리지 않음). 0..100 클램프.
fn bump(m: &mut serde_json::Map<String, Value>, key: &str, n: u64) {
    if let Some(cur) = m.get(key).and_then(Value::as_u64) {
        m.insert(key.into(), Value::from(cur.max(n).min(100)));
    }
}
```

연산 변형: `set`=`n`, `cap`=`cur.min(n)`(너프), `scale`=`cur*p/100`. §H의 `Cfg`와 합치면
`judgement = floor:80` 같은 줄을 cfg에서 읽어 op를 고르게 만들 수 있다. 편집 가능한 능력치
키(=인게임 능력치): `judgement`,
`concentration`, `control_speed`, `skill_hit`, `skill_avoid`, `positioning`, `last_hit`, `mental`,
`order`, `roaming`, `aggressive`, `ego`, `stamina`, `condition`.

## J. 즉시 1회 적용 + 한 팀만

```rust
fn on_server_start(&self, ctx: &mut ServerModContext) {   // 세이브 로드 직후 1회
    for a in ctx.database.athletes.iter_mut() {
        if !a.with(0) { continue; }                       // team 0 선수만
        // … 위 bump() 적용 …
    }
}
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
