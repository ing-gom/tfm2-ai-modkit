# tfm2-ai-modkit

**한국어** · [English](README.md)

Teamfight Manager 2 **AI 모드 개발 키트** — *인-매치 플레이어 AI 행동 교정*(`ModPlayerInputAi`)을
만들기 위한 *모든 것*을 한 곳에 모은 베이스 레포. 드래프트 AI(`ModDraftScoreHook`)도 포함.

> **1순위 용도 — 인-매치 행동 교정.** 내장 AI가 매 틱 고른 입력을 받아 더 나은 입력으로 바꾼다(저체력
> 추격 차단·안전 리콜·포지션별 정책 …). 무엇이 가능/불가능하고 어떻게 만드는지는
> **[`docs/02-behavior-correction.md`](docs/02-behavior-correction.md)** 가이드를 먼저 읽어라.

> **필요한 것**: 게임 버전에 맞는 TFM2 Mod SDK + Rust(rustup) + MSVC 링커(VS Build Tools "Desktop C++").
> SDK 경로는 `$env:TFM2_SDK`로 지정하거나 이 레포를 `sdk\` 폴더 옆에 두면 된다 — 아래 **"SDK 준비"**.

## SDK 루트 경로 (먼저 확인)

이 레포는 **SDK를 포함하지 않는다**. 빌드하려면 게임 버전에 맞는 **TFM2 Mod SDK 루트 폴더**를 따로 둬야 한다.

**SDK 루트 = 아래 파일/폴더를 담은 폴더**(보통 이름은 `mod-sdk` 또는 `sdk`):

```
<SDK 루트>\
├─ build_mod.bat          ← 이 파일이 있는 폴더가 SDK 루트다 (빌드가 이걸로 SDK를 식별)
├─ base_version.txt       ← 현재 SDK base 버전 (예: 0.4.12)
├─ toolchain_version.txt  ← 고정할 rustc 커밋/날짜
├─ rustup_toolchain.txt   ← 핀된 nightly 툴체인 이름
├─ deps\                  ← libmod_api-*.rlib 등 prebuilt 의존성
├─ native\                ← 네이티브 라이브러리
└─ template\
```

**기본 인식 위치는 `..\sdk`** (이 레포의 부모 워크스페이스 안 `sdk\`). 즉 기본 레이아웃은:

```
<workspace>\
├─ sdk\              ← TFM2 Mod SDK 루트 (build_mod.bat가 여기 있음)
└─ tfm2-ai-modkit\   ← 이 레포
```

다른 위치에 두려면 `$env:TFM2_SDK` 환경변수나 `-Sdk <경로>` 인자로 SDK 루트를 알려준다.
**우선순위: `-Sdk <경로>` 인자 → `$env:TFM2_SDK` → `..\sdk`.**

## 호환성 (검증 버전)

| 항목 | 값 |
|---|---|
| 게임/SDK base 버전 | **0.4.12** (EA) |
| Rust 툴체인 | `nightly-2026-06-11` (rustc 1.98.0-nightly, commit `485ec3fbc` / 2026-06-10) — SDK가 자동 고정 |
| Mod API 표면 | 이 base 버전의 `mod_api` rustdoc 기준 ([`docs/01`](docs/01-modapi-ai-surface.md)), 전체 지원 지도 ([`docs/03`](docs/03-sdk-capabilities.md)) |
| 검증 | 예제 6종(ai_perf·ai_survival·match_tuner·draft_ai·db_inspector·stat_editor) 0.4.12 빌드 확인 |

> ⚠️ **SDK는 버전 강결합이다.** 다른 base 버전의 SDK에서는 `mod_api` 타입/구조가 바뀔 수 있어
> 그 버전 SDK로 다시 빌드해야 하고, 경우에 따라 예제 코드 수정이 필요할 수 있다. `build.ps1`은
> SDK의 `base_version.txt`를 읽어 현재 버전을 출력하고, 위 검증 버전과 다르면 경고한다.
> 게임이 패치되면 [`docs/01`](docs/01-modapi-ai-surface.md)의 시그니처를 그 SDK의 rustdoc으로 재확인할 것.

## 무엇이 들어있나

| 영역 | 경로 | 내용 |
|---|---|---|
| 📖 레퍼런스 | `docs/` | mod_api AI 표면(시그니처), 개요, **SDK 지원 표면 전체 지도**, 레시피, 챔피언 카탈로그 |
| 🧩 공용 코드 | `shared/ai_common.rs` | 로깅·설정·Input 헬퍼 (std-only, 의존성 0). 예제가 `#[path]`로 포함 |
| 🚀 예제 모드 | `examples/` | 바로 빌드되는 스타터 6종 (아래) |

### 예제 6종

- **`ai_survival`** ⭐ — **인-매치 행동 교정 플래그십**(`ModPlayerInputAi`). 상태 기반 생존 거버너:
  HP 티어 대응 + 과추격 차단 + 포지션별 임계 + 후퇴 커밋(히스테리시스). self-상태만 사용. **새 행동
  모드는 이걸 복사해 시작**하라. → [`docs/02-behavior-correction.md`](docs/02-behavior-correction.md).
- **`ai_perf`** — `ModPlayerInputAi` 입문 스타터. 안전한 리콜·저체력 카이팅 + "여기에 당신의 행동을" TODO 슬롯.
- **`match_tuner`** — **매치엔진 관찰** 도구. 내장 AI가 매 틱 내놓는 `base_input`을 샘플링·로깅해 *낮은 능력치 선수가 실제로 어떻게 헛짓하는지*를 데이터로 본다. 읽기 전용(항상 `Pass`). 행동 모드 설계의 출발점.
- **`draft_ai`** — `ModDraftScoreHook` 기반 **드래프트 AI** 스타터. 픽/밴 점수 관찰 + 선호 챔프 가중 예시.
- **`db_inspector`** — **클라 DB 접근 스타터**(`ModExtension`). 매치 진입 시 `Scene::InGame`→`ClientData`로 클라 데이터베이스(선수/팀 수)를 조회하고, `mod_save_data`로 "매치 진입 횟수"를 **세이브에 영구 저장**한다(세션 간 유지). 게임 상태는 읽기 전용. → [`docs/03-sdk-capabilities.md`](docs/03-sdk-capabilities.md) §A-4.
- **`stat_editor`** — **능력치 편집 스타터**(`ModServerExtension`). "AI의 숫자를 바꾸는" 정식 경로: 관리 계층에서 각 선수의 `AthleteStat`을 `serde_json`으로 직접 편집(set/floor/cap/scale, 팀 필터)하고 **관리 틱마다 재적용**해 훈련·노화가 지우지 못하게 한다. 능력치를 올리면 내장 AI가 더 잘 둔다. → [`docs/01`](docs/01-modapi-ai-surface.md) §ModServerExtension, [`docs/05-recipes.md`](docs/05-recipes.md).

## SDK 준비

빌드에는 게임 버전에 맞는 **TFM2 Mod SDK 루트**(위 "SDK 루트 경로" 참고)가 필요하다. 둘 중 하나로 위치를 알려준다:

```powershell
# (A) 환경변수로 SDK 루트 지정 — 레포를 어디에 두든 OK
$env:TFM2_SDK = "C:\path\to\mod-sdk"

# (B) 또는 이 레포를 sdk\ 옆에 배치 (기본 인식: ..\sdk)
#   <workspace>\
#   ├─ sdk\              ← TFM2 Mod SDK 루트
#   └─ tfm2-ai-modkit\   ← 이 레포
```

우선순위: `-Sdk <경로>` 인자 → `$env:TFM2_SDK` → `..\sdk`. (빌드 스크립트가 SDK의 `toolchain_version.txt`로 정확한 nightly 툴체인을 자동 고정하고 `mod_api`를 주입한다.)

## 빌드 & 설치

```powershell
# 단일 예제 빌드 (기본: ai_perf)
.\build.ps1                              # → examples\ai_perf\ai_perf.dll
.\build.ps1 match_tuner
.\build.ps1 draft_ai -Sdk "C:\path\to\mod-sdk"

# 전부 빌드
.\build-all.ps1
```

빌드된 `examples\<mod>\` 폴더(`<mod>.dll` + `mod.mod_info`)를 게임의 `mods\<mod>\`로 복사하면 로드된다. 게임 패치로 SDK가 바뀌면 해당 버전 SDK로 갈아끼우고 재빌드.

## 먼저 읽을 것

1. [`docs/00-overview.md`](docs/00-overview.md) — 능력치 2층 구조와 모드 관점 결정 흐름
2. [`docs/02-behavior-correction.md`](docs/02-behavior-correction.md) ⭐ — **인-매치 행동 교정 모드 만들기**(이 키트의 핵심): 할 수 있는 것/없는 것·워크플로·self-상태 천장
3. [`docs/01-modapi-ai-surface.md`](docs/01-modapi-ai-surface.md) — 후킹 가능한 API 정확한 시그니처
4. [`docs/03-sdk-capabilities.md`](docs/03-sdk-capabilities.md) — **SDK가 실제로 지원하는 전체 표면 지도**(무엇이 되고/안 되는지, 프로브 검증)
5. [`docs/05-recipes.md`](docs/05-recipes.md) — 자주 쓰는 패턴 모음
6. [`docs/champion-catalog.md`](docs/champion-catalog.md) — 드래프트 인덱스 0..59

## 라이선스 / 작성

author: `inggom`. EA 기간 동안 SDK 버전 강결합 — 게임 패치마다 해당 버전 SDK로 갱신 후 재빌드 필요.
