//! draft_ai — STARTER: adjust the draft pick/ban AI via `ModDraftScoreHook`.
//!
//! The hook fires once per (candidate champion) after the built-in draft AI has
//! produced its own `base_score`. You return:
//!   - `Pass`            keep the engine score (read-only / observe)
//!   - `Add(delta)`      nudge it by ±delta
//!   - `Replace(score)`  set it outright
//!
//! `DraftScoreContext` gives the live board: `phase`, `available_champions`,
//! `ally_pick/enemy_pick`, `ally_ban/enemy_ban`, `is_explore`, `difficulty`.
//! Champion candidates are indices into the fixed catalog order (see NAMES /
//! docs/champion-catalog.md).
//!
//! NOTE (verified): `ChampionInfo::category()/tags()` are NOT populated at draft
//! time — a tag-based synergy engine has no data to work with. The reliable
//! signal is the engine's own `base_score`. This starter logs it and shows an
//! opt-in flat nudge for a configured favourite.
//!
//! Config (optional): `%TEMP%\draft_ai.cfg`
//!   log_scores = on        # log per-board candidate scores to %TEMP%\draft_ai.log
//!   favour     =           # champion id to nudge, e.g. `fighter` (blank = none)
//!   favour_add = 50        # score delta applied to the favoured champion

use mod_api::*;
use std::sync::OnceLock;

#[path = "../../../shared/ai_common.rs"]
mod ai_common;
use ai_common::Logger;

const MOD_ID: &str = "draft_ai";

/// index -> champion id, fixed catalog order (0..59). See docs/champion-catalog.md.
const NAMES: [&str; 60] = [
    "fighter", "knight", "swordman", "archer", "soldier", "priest", "pythoness",
    "monk", "pyromancer", "ice_mage", "ninja", "magic_knight", "berserker",
    "executioner", "lancer", "ogre", "dual_blader", "cavalry_knight", "gunner",
    "pole_warrior", "jiangshi", "gambler", "hammerer", "demon", "vampire",
    "spirit_caller", "boomerang_hunter", "inquisitor", "shield_bearer",
    "whip_master", "werewolf", "dokkaebi", "necromancer", "bard",
    "barrier_magician", "chef", "clown", "dancer", "dark_mage", "exorcist",
    "ghost", "illusionist", "lightning_mage", "plague_doctor",
    "poison_dart_hunter", "shadowmancer", "taoist", "siege_breaker", "android",
    "druid", "prisoner", "bomber", "voodoo_shaman", "white_mage", "wind_mage",
    "enchanter", "hitman", "guardian_spirit", "hunter", "circus_blade",
];

fn name(i: usize) -> &'static str {
    NAMES.get(i).copied().unwrap_or("?")
}

fn names(ids: &[usize]) -> String {
    ids.iter().map(|&i| name(i)).collect::<Vec<_>>().join(",")
}

struct Settings {
    log_scores: bool,
    favour_idx: Option<usize>,
    favour_add: f32,
}

fn settings() -> &'static Settings {
    static S: OnceLock<Settings> = OnceLock::new();
    S.get_or_init(|| {
        let c = ai_common::Cfg::load(MOD_ID);
        let favour_idx = c
            .string("favour")
            .filter(|s| !s.is_empty())
            .and_then(|want| NAMES.iter().position(|&n| n == want));
        Settings {
            log_scores: c.bool("log_scores", true),
            favour_idx,
            favour_add: c.f32("favour_add", 50.0),
        }
    })
}

fn logger() -> &'static Logger {
    static L: OnceLock<Logger> = OnceLock::new();
    L.get_or_init(|| Logger::new(MOD_ID, 5000))
}

/// Shared scoring decision for both pick and ban.
fn decide(ctx: &DraftScoreContext, candidate: usize, base_score: f32, kind: &str) -> DraftScoreDecision {
    let s = settings();

    if s.log_scores {
        logger().capped(&format!(
            "{kind} [{:?}] cand={:<18} base={:.3} ally=[{}] enemy=[{}]",
            ctx.phase,
            name(candidate),
            base_score,
            names(ctx.ally_pick),
            names(ctx.enemy_pick),
        ));
    }

    // Opt-in nudge for a favoured champion. Default config favours nobody → Pass.
    if let Some(fav) = s.favour_idx {
        if candidate == fav {
            return DraftScoreDecision::Add(s.favour_add);
        }
    }

    DraftScoreDecision::Pass
}

#[derive(Debug)]
struct DraftAi;

impl ModDraftScoreHook for DraftAi {
    fn id(&self) -> &str {
        MOD_ID
    }

    fn priority(&self) -> i32 {
        100
    }

    fn score_pick(&self, ctx: &DraftScoreContext, candidate: usize, base_score: f32) -> DraftScoreDecision {
        decide(ctx, candidate, base_score, "PICK")
    }

    fn score_ban(&self, ctx: &DraftScoreContext, candidate: usize, base_score: f32) -> DraftScoreDecision {
        decide(ctx, candidate, base_score, "BAN ")
    }
}

fn init(_ctx: &GameCtx) -> ModRegistration {
    let _ = settings();
    logger().line("=== draft_ai init ===");
    let mut reg = ModRegistration::new(MOD_ID);
    reg.add_draft_score_hook(DraftAi);
    reg
}

declare_mod!(init);
