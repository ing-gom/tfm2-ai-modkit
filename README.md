# tfm2-ai-modkit

**English** · [한국어](README.ko.md)

A base repo gathering *everything* you need to build a Teamfight Manager 2 **AI mod** —
primarily *in-match player AI behaviour correction* (`ModPlayerInputAi`). Draft AI
(`ModDraftScoreHook`) is included too.

> **Primary use — in-match behaviour correction.** Take the input the built-in AI picks each
> tick and replace it with a better one (block low-HP chases, safe recalls, per-position policy …).
> For what is and isn't possible and how to build it, read the
> **[`docs/02-behavior-correction.md`](docs/02-behavior-correction.md)** guide first.

> **What you need**: a TFM2 Mod SDK matching the game version + Rust (rustup) + the MSVC linker
> (VS Build Tools "Desktop C++"). Point the build at the SDK via `$env:TFM2_SDK`, or place this
> repo next to an `sdk\` folder — see **"SDK setup"** below.

## SDK root path (read this first)

This repo **does not contain the SDK**. To build, you must supply the **TFM2 Mod SDK root folder**
for your game version separately.

**The SDK root = the folder that contains these files/dirs** (usually named `mod-sdk` or `sdk`):

```
<SDK root>\
├─ build_mod.bat          ← the folder containing this file IS the SDK root (the build detects it by this)
├─ base_version.txt       ← current SDK base version (e.g. 0.4.12)
├─ toolchain_version.txt  ← the rustc commit/date to pin
├─ rustup_toolchain.txt   ← the pinned nightly toolchain name
├─ deps\                  ← prebuilt deps such as libmod_api-*.rlib
├─ native\                ← native libraries
└─ template\
```

**The default lookup location is `..\sdk`** (an `sdk\` folder inside this repo's parent workspace).
So the default layout is:

```
<workspace>\
├─ sdk\              ← TFM2 Mod SDK root (build_mod.bat lives here)
└─ tfm2-ai-modkit\   ← this repo
```

To keep the SDK elsewhere, tell the build where the SDK root is via the `$env:TFM2_SDK`
environment variable or the `-Sdk <path>` argument.
**Priority: `-Sdk <path>` argument → `$env:TFM2_SDK` → `..\sdk`.**

## Compatibility (verified version)

| Item | Value |
|---|---|
| Game / SDK base version | **0.4.12** (EA) |
| Rust toolchain | `nightly-2026-06-11` (rustc 1.98.0-nightly, commit `485ec3fbc` / 2026-06-10) — pinned automatically by the SDK |
| Mod API surface | Based on this base version's `mod_api` rustdoc ([`docs/01`](docs/01-modapi-ai-surface.md)); full support map ([`docs/03`](docs/03-sdk-capabilities.md)) |
| Verification | All 6 examples (ai_perf · ai_survival · match_tuner · draft_ai · db_inspector · stat_editor) build against 0.4.12 |

> ⚠️ **The SDK is version-bound.** With an SDK of a different base version, `mod_api` types/structure
> may change, so you must rebuild with that version's SDK and occasionally tweak example code.
> `build.ps1` reads the SDK's `base_version.txt`, prints the current version, and warns if it differs
> from the verified version above. If the game is patched, re-check the signatures in
> [`docs/01`](docs/01-modapi-ai-surface.md) against that SDK's rustdoc.

## What's inside

| Area | Path | Contents |
|---|---|---|
| 📖 Reference | `docs/` | mod_api AI surface (signatures), overview, **full SDK support map**, recipes, champion catalog |
| 🧩 Shared code | `shared/ai_common.rs` | logging / config / Input helpers (std-only, zero deps). Included by examples via `#[path]` |
| 🚀 Example mods | `examples/` | 6 ready-to-build starters (below) |

### The 6 examples

- **`ai_survival`** ⭐ — **in-match behaviour-correction flagship** (`ModPlayerInputAi`). A state-based
  survival governor: HP-tier response + over-chase blocking + per-position thresholds + retreat
  commit (hysteresis). Uses self-state only. **Copy this to start a new behaviour mod.**
  → [`docs/02-behavior-correction.md`](docs/02-behavior-correction.md).
- **`ai_perf`** — entry-level `ModPlayerInputAi` starter. Safe recall · low-HP kiting + a
  "put your behaviour here" TODO slot.
- **`match_tuner`** — a **match-engine observation** tool. Samples and logs the `base_input` the
  built-in AI emits each tick so you can see, as data, *how a low-stat player actually misplays*.
  Read-only (always `Pass`). The starting point for designing a behaviour mod.
- **`draft_ai`** — a **draft AI** starter built on `ModDraftScoreHook`. Pick/ban score observation +
  a preferred-champ weighting example.
- **`db_inspector`** — a **client DB access** starter (`ModExtension`). On entering a match it reads
  the client database (player/team counts) via `Scene::InGame`→`ClientData`, and **persists a
  "match-entry count" into the save** via `mod_save_data` (survives across sessions). Game state is
  read-only. → [`docs/03-sdk-capabilities.md`](docs/03-sdk-capabilities.md) §A-4.
- **`stat_editor`** — a **stat-editing** starter (`ModServerExtension`). The canonical path to
  "changing the AI's numbers": at the management layer, directly edit each athlete's `AthleteStat`
  via `serde_json` (set/floor/cap/scale, team filter) and **re-apply every management tick** so
  training/aging can't undo it. Raise the stats and the built-in AI plays better.
  → [`docs/01`](docs/01-modapi-ai-surface.md) §ModServerExtension, [`docs/05-recipes.md`](docs/05-recipes.md).

## SDK setup

Building needs a **TFM2 Mod SDK root** matching your game version (see "SDK root path" above).
Point the build at it one of two ways:

```powershell
# (A) Specify the SDK root via env var — works wherever this repo lives
$env:TFM2_SDK = "C:\path\to\mod-sdk"

# (B) Or place this repo beside an sdk\ folder (default lookup: ..\sdk)
#   <workspace>\
#   ├─ sdk\              ← TFM2 Mod SDK root
#   └─ tfm2-ai-modkit\   ← this repo
```

Priority: `-Sdk <path>` argument → `$env:TFM2_SDK` → `..\sdk`. (The build script auto-pins the exact
nightly toolchain from the SDK's `toolchain_version.txt` and injects `mod_api`.)

## Build & install

```powershell
# Build a single example (default: ai_perf)
.\build.ps1                              # → examples\ai_perf\ai_perf.dll
.\build.ps1 match_tuner
.\build.ps1 draft_ai -Sdk "C:\path\to\mod-sdk"

# Build everything
.\build-all.ps1
```

Copy the built `examples\<mod>\` folder (`<mod>.dll` + `mod.mod_info`) into the game's `mods\<mod>\`
to load it. When a game patch changes the SDK, swap in that version's SDK and rebuild.

## Read first

1. [`docs/00-overview.md`](docs/00-overview.md) — the two-layer stat model and the mod-perspective decision flow
2. [`docs/02-behavior-correction.md`](docs/02-behavior-correction.md) ⭐ — **building in-match behaviour-correction mods** (the heart of this kit): what you can/can't do · workflow · the self-state ceiling
3. [`docs/01-modapi-ai-surface.md`](docs/01-modapi-ai-surface.md) — exact signatures of the hookable API
4. [`docs/03-sdk-capabilities.md`](docs/03-sdk-capabilities.md) — **the full map of what the SDK actually supports** (what works / doesn't, probe-verified)
5. [`docs/05-recipes.md`](docs/05-recipes.md) — a collection of common patterns
6. [`docs/champion-catalog.md`](docs/champion-catalog.md) — draft indices 0..59

## License / authorship

author: `inggom`. Version-bound to the SDK during EA — each game patch requires updating to that
version's SDK and rebuilding.
