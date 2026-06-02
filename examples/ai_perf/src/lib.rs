//! ai_perf — STARTER: improve the in-match player AI via `ModPlayerInputAi`.
//!
//! The hook fires every tick for each player. The engine hands you the input it
//! *would* execute (`base_input`); you return either `Pass` (keep it) or
//! `Replace(input)` (override it). Invalid inputs are ignored for that tick, so
//! always gate a Replace behind `ctx.is_valid_input(&input)`.
//!
//! Ships two safe, self-contained behaviours and a TODO slot:
//!   1. recall  — when low HP AND the engine says it's safe, recall instead of
//!                lingering and dying.
//!   2. kite    — when critically low and recall isn't safe, run away without
//!                burning a skill, instead of trading into death.
//!   3. <yours> — the marked slot in `think()`.
//!
//! Config (optional): `%TEMP%\ai_perf.cfg` or `<game>\mods\ai_perf\ai_perf.cfg`
//!   enabled   = on
//!   recall    = on
//!   recall_hp = 35      # recall when hp% <= this and safe
//!   kite_hp   = 20      # run away when hp% <= this
//!
//! Everything in `think()` runs INSIDE the match simulation — keep it small and
//! deterministic.

use mod_api::*;
use std::sync::OnceLock;

#[path = "../../../shared/ai_common.rs"]
mod ai_common;
use ai_common::Logger;

const MOD_ID: &str = "ai_perf";

struct Settings {
    enabled: bool,
    recall: bool,
    recall_hp: usize,
    kite_hp: usize,
}

fn settings() -> &'static Settings {
    static S: OnceLock<Settings> = OnceLock::new();
    S.get_or_init(|| {
        let c = ai_common::Cfg::load(MOD_ID);
        let s = Settings {
            enabled: c.bool("enabled", true),
            recall: c.bool("recall", true),
            recall_hp: c.usize("recall_hp", 35).min(100),
            kite_hp: c.usize("kite_hp", 20).min(100),
        };
        logger().line(&format!(
            "config: enabled={} recall={} recall_hp={} kite_hp={}",
            s.enabled, s.recall, s.recall_hp, s.kite_hp
        ));
        s
    })
}

fn logger() -> &'static Logger {
    static L: OnceLock<Logger> = OnceLock::new();
    L.get_or_init(|| Logger::new(MOD_ID, 2000))
}

#[derive(Debug, Clone)]
struct AiPerf;

impl ModPlayerInputAi for AiPerf {
    fn clone_box(&self) -> Box<dyn ModPlayerInputAi> {
        Box::new(self.clone())
    }

    fn id(&self) -> &str {
        MOD_ID
    }

    /// Lower runs first; higher overrides earlier hooks. Default is 0.
    fn priority(&self) -> i32 {
        100
    }

    /// Restrict which players this hook applies to. Return false to skip a
    /// player entirely (e.g. only your own team, only a position/champion).
    fn matches(&self, _ctx: &PlayerAiInitContext) -> bool {
        settings().enabled
    }

    fn think(&mut self, ctx: &mut PlayerAiContext, base_input: Option<Input>) -> PlayerInputDecision {
        let s = settings();

        // 1) Safer recall: low HP and the engine deems it safe.
        if s.recall {
            if let Some(hp) = ctx.hp_ratio_percent() {
                if hp <= s.recall_hp && ctx.is_safe_to_recall() {
                    if let Some(recall) = ctx.get_recall_input() {
                        if ctx.is_valid_input(&recall) {
                            return PlayerInputDecision::Replace(recall);
                        }
                    }
                }
            }
        }

        // 2) Kite out when critically low (recall wasn't safe/available above).
        if let Some(hp) = ctx.hp_ratio_percent() {
            if hp <= s.kite_hp {
                if let Some(run) = ctx.get_run_away_without_skill_input() {
                    if ctx.is_valid_input(&run) {
                        return PlayerInputDecision::Replace(run);
                    }
                }
            }
        }

        // 3) ── YOUR BEHAVIOUR HERE ────────────────────────────────────────
        // `base_input` is what the native AI chose this tick (None if idle).
        // Build an Input with ai_common::input::{attack, skill_at, move_to, …},
        // validate it, and return Replace(...). Examples of what you can read:
        //   ctx.position(), ctx.champion_name(), ctx.hp_ratio_percent()
        // See docs/05-recipes.md for patterns (focus-fire, skillshot lead, …).
        let _ = base_input;

        PlayerInputDecision::Pass
    }
}

fn init(_ctx: &GameCtx) -> ModRegistration {
    logger().line("=== ai_perf init ===");
    let _ = settings(); // load + log config at startup
    let mut reg = ModRegistration::new(MOD_ID);
    reg.add_player_input_ai(AiPerf);
    reg
}

declare_mod!(init);
