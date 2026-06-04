# 00 · TFM2 AI 개요 (모드 관점)

> AI 모드를 만들기 전에, mod_api가 노출하는 범위에서 "능력치와 행동이 어디서 만나는지"의 큰 그림.

## 능력치는 두 개의 분리된 층이다

| 층 | 타입 | 항목 | 역할 |
|---|---|---|---|
| **전투 수치** (챔피언+아이템) | `EntityStat` `#[repr(C)]` | attack, magic_power, hp, defence, magic_resistance, move_speed, hp_regen, stack, crit_chance | **데미지·생존 계산** |
| **선수 능력치** (트레이닝 대상) | `Athlete.stat` (`AthleteStat`) 등 | (mod_api에 필드 비공개) | 선수의 **판단·숙련도**를 좌우 |

`EntityStat`은 mod_api에 공개돼 있고 데미지/생존만 담당한다. 선수 능력치(`AthleteStat`)는 `Athlete`
구조체에 존재하지만 개별 필드 accessor는 mod_api에 노출돼 있지 않다 — 게임이 매치에서 이를 AI의 판단
품질로 변환한다(변환 로직 자체는 비공개). **모드는 그 변환 결과로 나온 최종 입력(`Input`)을 다룬다.**

다만 `AthleteStat`은 `Serialize`/`Deserialize`라, `ModServerExtension`(관리 계층)에서 `serde_json`으로
**필드명 기준 읽기/쓰기**가 가능하다 — 능력치를 올리면 그만큼 내장 AI가 더 잘 둔다(입력단 개선). 변환
파이프라인은 못 건드리므로, 모드가 AI를 바꾸는 길은 결국 **입력(능력치)** 또는 **출력(최종 `Input`)** 둘이다.
상세 표면은 [`01-modapi-ai-surface.md`](01-modapi-ai-surface.md) §ModServerExtension.

## 모드 관점 결정 흐름 (매 틱)

```
내장 AI  ──▶  base_input (Input)  ──▶  ModPlayerInputAi::think  ──▶  Pass | Replace(Input)
                                              (당신의 모드)
```

행동의 전체 어휘(공개):

```
Input        = Move{x,y} | Return | Attack{t} | Skill{t} | Skill2{t} | Ult{t}
InputTarget  = Target{id} | Dir{dx,dy} | Pos{x,y} | None
```

`PlayerAiContext` 헬퍼로 hp/position/champion/tick과 recall·run-away 입력, `is_valid_input` 등에 접근한다.
정확한 시그니처는 [`01-modapi-ai-surface.md`](01-modapi-ai-surface.md).

행동 교정 예제: [`examples/ai_perf`](../examples/ai_perf)(스타터: 안전 리콜+카이팅+TODO 슬롯),
[`examples/ai_survival`](../examples/ai_survival)(상태 기반 생존 거버너 — 포지션별 임계·과추격 차단·후퇴
커밋, **self-상태만** 사용). 패턴 모음은 [`05-recipes.md`](05-recipes.md). ⚠️ 적/아군 위치 같은 *세계 상태* 는
`think()`에 안정적으로 노출돼 있지 않다(인-매치 `GameCtx` 브리지 미검증) — 그래서 위 두 예제는 자기 HP·
포지션만으로 동작한다. 타깃·합류·포지셔닝 같은 세계 인식 교정은 그 브리지 검증이 선행 과제.

## 모드가 끼어들 수 있는 지점

| 지점 | 트레이트 | 무엇을 바꾸나 |
|---|---|---|
| 인-매치, 매 틱 | **`ModPlayerInputAi`** | 최종 `Input` 교체 (성능향상·행동수정의 핵심) |
| 드래프트, 픽/밴마다 | **`ModDraftScoreHook`** | 후보 챔피언 점수 가감 |
| 서버/관리 로직 | `ModServerExtension` | **선수 능력치(`AthleteStat`) 직접 편집** · 운영 · DB (매치 밖) |
| 클라이언트 프레임 | `ModExtension` | scene/UI/asset 접근, 렌더 + **클라 DB 조회·모드 영구저장**(`mod_save_data`). [`03`](03-sdk-capabilities.md) §A-4 |
| 콘텐츠 | `ModChampionInfo`/`ModItemInfo`/`ModAction`/`ModEffectType`/`ModPassive` | 새 챔프/아이템/스킬 효과 — ⛔ **0.4.8 구현 불가**(반환 타입 비공개, 프로브 확정. [`03`](03-sdk-capabilities.md) §B) |

## 핵심 원칙

- **`think()`는 시뮬레이션 안에서 돈다.** 작고 결정적이게. 매 틱·매 선수 호출됨.
- **유효하지 않은 Input은 그 프레임에 무시된다.** `ctx.is_valid_input(&input)`로 항상 검증.
- **hook priority: 낮을수록 먼저, 높을수록 나중**(앞 결과 덮어씀). 관찰 모드는 높게(마지막), 교정 모드는 중간.
- **핸들은 콜백 스코프 한정.** 콜백 사이에 저장 금지. 누적 상태는 `OnceLock<Mutex<…>>`에 값(id·좌표)만.

## 내장 AI가 *무엇을* 하는지 관찰하기

내장 AI가 매 틱 내놓는 `base_input`을 직접 로깅해 데이터로 보려면 [`examples/match_tuner`](../examples/match_tuner)
를 쓴다(읽기 전용). 저능력/고능력 선수를 각각 돌려 로그를 비교하면, 약한 선수가 더 나쁜 타깃/입력을
내는 것을 확인할 수 있다 — 행동 교정 로직을 설계하기 전 출발점.
