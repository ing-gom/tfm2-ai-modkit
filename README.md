# tfm2-ai-modkit

Teamfight Manager 2 **AI 모드 개발 키트** — 인-매치 플레이어 AI(`ModPlayerInputAi`)와
드래프트 AI(`ModDraftScoreHook`)를 만들기 위한 *모든 것*을 한 곳에 모은 베이스 레포.

> 이 키트는 상위 워크스페이스 `tfm2-mod-dev`의 SDK(`../sdk`)를 그대로 사용해 빌드한다.
> 별도 SDK 설치 불필요 — 부모 폴더에 `sdk/`가 있으면 된다.

## 무엇이 들어있나

| 영역 | 경로 | 내용 |
|---|---|---|
| 📖 레퍼런스 | `docs/` | mod_api AI 표면(시그니처), 개요, 레시피, 챔피언 카탈로그 |
| 🧩 공용 코드 | `shared/ai_common.rs` | 로깅·설정·Input 헬퍼 (std-only, 의존성 0). 예제가 `#[path]`로 포함 |
| 🚀 예제 모드 | `examples/` | 바로 빌드되는 스타터 3종 (아래) |

### 예제 3종

- **`ai_perf`** — `ModPlayerInputAi` 기반 **판단/성능 향상** 스타터. 안전한 리콜·저체력 카이팅 + "여기에 당신의 행동을" TODO 슬롯.
- **`match_tuner`** — **매치엔진 관찰** 도구. 내장 AI가 매 틱 내놓는 `base_input`을 샘플링·로깅해 *낮은 능력치 선수가 실제로 어떻게 헛짓하는지*를 데이터로 본다. 읽기 전용(항상 `Pass`).
- **`draft_ai`** — `ModDraftScoreHook` 기반 **드래프트 AI** 스타터. 픽/밴 점수 관찰 + 선호 챔프 가중 예시.

## 빠른 시작

```powershell
# 단일 예제 빌드 (기본: ai_perf)
.\build.ps1                 # → examples\ai_perf\ai_perf.dll
.\build.ps1 match_tuner
.\build.ps1 draft_ai

# 전부 빌드
.\build-all.ps1
```

빌드된 `<mod>.dll`이 들어있는 `examples\<mod>\` 폴더를 게임의 `mods\` 로 복사하면 로드된다.
(상위 `../deploy.ps1` 참고. 빌드는 부모 `../build.ps1`와 동일한 툴체인 핀·`mod_api` 주입 메커니즘을 재사용한다.)

## 먼저 읽을 것

1. [`docs/00-overview.md`](docs/00-overview.md) — 능력치 2층 구조와 모드 관점 결정 흐름
2. [`docs/01-modapi-ai-surface.md`](docs/01-modapi-ai-surface.md) — 후킹 가능한 API 정확한 시그니처
3. [`docs/05-recipes.md`](docs/05-recipes.md) — 자주 쓰는 패턴 모음
4. [`docs/champion-catalog.md`](docs/champion-catalog.md) — 드래프트 인덱스 0..59

## 라이선스 / 작성

author: `inggom`. EA 기간 동안 SDK 버전 강결합 — 게임 패치 시 부모 `../update-sdk.ps1`로 SDK 갱신 후 재빌드.
