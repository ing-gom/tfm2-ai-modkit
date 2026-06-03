# 02 · 인-매치 행동 교정 모드 만들기 (`ModPlayerInputAi`)

> 이 키트의 1순위 용도. 내장 AI가 매 틱 고른 입력을 받아 **더 나은 입력으로 바꿔** 행동을 교정한다.
> 시작 시점에 한 번 적용되는 능력치 편집과 달리, 매 틱 상황에 반응한다.
> 정확한 시그니처는 [`01-modapi-ai-surface.md`](01-modapi-ai-surface.md), 코드 조각은 [`05-recipes.md`](05-recipes.md).

## 한 장 요약

```
내장 AI ──▶ base_input (이 틱의 결정) ──▶ think(ctx, base_input) ──▶ Pass | Replace(Input)
                                                 (당신의 모드)
```
- `think()`는 **매 틱·매 선수** 호출된다. 작고 결정적으로.
- `base_input`(내장 AI가 고른 것)을 보고, 더 낫다면 `Replace(input)`, 아니면 `Pass`.
- `Replace` 전 **반드시** `ctx.is_valid_input(&input)`. 무효 입력은 그 틱에 무시된다.

## 무엇을 보고 결정하나 — `PlayerAiContext` (self-상태)

`think()`가 주는 정보는 **자기 자신**과 엔진이 만들어 주는 *안전 입력* 몇 개뿐이다:

```rust
ctx.player_id()/athlete_id()/team()/position()/champion_name()/tick()   // 정체성
ctx.hp()/max_hp()/hp_ratio_percent()/is_hp_below_percent(t)             // 내 체력
ctx.is_valid_input(&input)                                              // 검증
ctx.get_recall_input()/get_run_away_input()/get_run_away_without_skill_input()
ctx.is_safe_to_recall()                                                 // 엔진의 안전 판단
```

➡ **이게 천장이다.** 적/아군의 위치·체력·스킬·시야 같은 *세계 상태* 를 `think()`에서 **직접 읽을 수는 없다**
(인-매치 `GameCtx` 브리지 미검증). 그래서 *직접* 판단은 **"내 상태 + base_input"** 에 한정된다.

> **★ 별표 — 캔 입력은 이미 월드 인지다.** `is_safe_to_recall()`, `get_run_away_input()`,
> `get_run_away_without_skill_input()`, `get_recall_input()` 은 **엔진이 적 위치/위협을 보고 계산**해 준다.
> "안전한가"는 주변 적을 보고 답하고, "도망 입력"은 적을 피하는 방향을 엔진이 정한다. 즉 모드는 raw
> 월드 쿼리는 못 해도 **엔진의 월드-인지 행동을 self-상태 조건으로 *트리거* 할 수 있다**:
> `내 HP 위험(self)` → `get_run_away_without_skill_input()`(엔진이 적 피해 도주 방향 계산) → `Replace`.
> survival류 교정엔 이걸로 충분한 경우가 많다.

## 자신 있게 만들 수 있는 교정 (self-상태만)

| 교정 | 신호 | 방법 |
|---|---|---|
| **더 이른/안전한 리콜** | `hp_ratio_percent` + `is_safe_to_recall` | `Replace(get_recall_input())` |
| **저체력 카이팅 / 무스킬 후퇴** | `hp` 임계 | `Replace(get_run_away_without_skill_input())` |
| **과추격 차단(disengage)** | `hp` 낮음 **AND** `base_input`이 공격적(Attack/Skill/Ult) | 공격 입력을 후퇴로 교체 |
| **후퇴 커밋(히스테리시스)** | 직전 결정 기억 | 한 번 빠지기로 하면 N틱 유지 → 내장 AI와 깜빡임 방지 |
| **포지션/챔피언별 정책** | `position()` / `champion_name()` | 캐리·서폿은 더 일찍 후퇴 등 |
| **특정 내장 결정 거부** | `base_input` 패턴 매칭 | 원치 않는 액션만 골라 교체, 나머진 `Pass` |

플래그십 예제 **[`examples/ai_survival`](../examples/ai_survival)** 가 위를 전부 구현한다(HP 티어 + 과추격
차단 + 포지션별 임계 + 후퇴 커밋). 새 행동 모드는 이걸 복사해 시작하는 걸 권한다.

## 아직 못 하는 것 (세계 상태 필요)

- **타깃 선정**(엉뚱한 적 치는 것 교정) — 적 위치/체력이 필요.
- **한타 합류·교전 타이밍** — 아군/적 위치가 필요.
- **포지셔닝/스킬 회피** — 주변 적·투사체 위치가 필요.
- **오브젝트 컨트롤·시야** — 맵 상태가 필요.

이들은 전부 `think()`에 안 들어오는 세계 상태에 의존한다. 지금 구조로는 불가. (열쇠는 인-매치
`GameCtx↔think` 브리지 검증 — 그게 풀리면 이 표가 위로 옮겨진다. 미해결 과제.)

> 왜 self-상태만으로도 의미가 있나: 내장 AI의 실수 상당수는 **실력이 낮을수록 결정이 흔들리는** 데서
> 온다(약체 선수가 무리하게 들어가 죽는 등). "내 체력이 위험한데 내장 AI가 공격을 고름" 같은 self-상태
> 신호만으로도 그 죽음의 상당수를 막을 수 있다 — ai_survival이 노리는 지점.

## 개발 워크플로

1. **관찰** — [`examples/match_tuner`](../examples/match_tuner)로 한 경기의 `base_input`을 로깅한다.
   어떤 상황에서 내장 AI가 어떤 `Input`을 내는지(특히 죽기 직전 패턴) 데이터로 본다.
2. **진단** — 고치고 싶은 한 가지 행동을 정한다(예: "저체력인데 계속 추격하다 죽음").
   그게 self-상태로 감지 가능한지 확인(위 표). 아니면 지금은 보류.
3. **작성** — `ai_survival`을 복사해 `think()`에 규칙을 추가. 항상 `is_valid_input`로 게이트.
   콜백 사이 상태는 값(id·틱·모드)만, 핸들은 절대 저장 금지(→ [`05-recipes.md`](05-recipes.md) §F).
4. **검증** — 게임에 설치(`<game>\mods\<id>\`)하고 한 경기. `%TEMP%\<id>.log`로 결정을 로깅해
   교체가 실제로 행동을 바꾸는지, 더 잘 사는지 대조. **인게임 검증은 필수**(시뮬 결과는 정적으로 못 봄).

## 지켜야 할 규칙

- **`think()`는 시뮬 안에서 매 틱·매 선수 돈다.** 무거운 할당/루프 금지. 작고 결정적으로.
- **`is_valid_input()` 없이 `Replace` 금지** — 그 틱 무시되고 디버깅만 어려워진다.
- **핸들은 콜백 스코프 한정.** 누적 상태는 `OnceLock<Mutex<…>>`에 *값* 만(좌표·id·틱·모드).
- **`matches()`로 대상 한정** — 내 팀만/특정 포지션만 등. 불필요한 선수는 일찍 거른다.
- **`priority()`** — 낮을수록 먼저, 높을수록 나중(앞 결과 덮음). 관찰 모드는 높게.
- **여러 모드가 같은 입력을 다투지 않게** priority로 순서를 명시.

## 세계 상태에 대해 (조사 결론)

타깃·합류·포지셔닝 같은 교정엔 **인-매치 세계 상태**(적/아군 위치)가 필요한데, 지원 API로는 닿지 않는다.
조사 결과 요약:
- **지원 경로**: 없음. `think()`의 `PlayerAiContext`는 self-상태 + (월드-인지) 캔 액션만 노출.
- **`GameCtx` 캡처**: 불가. 모드가 받는 유일한 `GameCtx`(init)는 매치 시작 전이라 내부 상태가 비어 있어,
  나중에 쓰면 크래시한다(인게임 확인).
- **세계가 *기술적으로는* think()에 존재**: 엔진 내부적으론 라이브 핸들이 있어, 깊은 비공식 메모리
  리버싱으로 *원리상* 읽을 수는 있다. 그러나 게임 패치마다 깨지고 크래시가 잦은 unsafe 작업이라
  **배포용 모드엔 부적합**(개인 실험 영역).

➡ **그래서 안정적이고 지원되는 천장은 "self-상태 + 엔진의 월드-인지 캔 액션"이다**(이 가이드 전체가
그 위에서 동작). 엔진의 `get_run_away_*`/`is_safe_to_recall`이 이미 적 위치를 고려해 주므로, survival·
disengage·리콜 류 교정은 견고하게 가능하다. 세계 인지가 *직접* 필요한 정밀 교정은 현 SDK 범위 밖이다.
