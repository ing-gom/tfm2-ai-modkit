//! match_tuner — OBSERVE the built-in match AI (read-only).
//!
//! This mod never changes behaviour (always returns `Pass`). It exists to turn
//! the native AI's per-tick decisions into data: every Nth tick it logs the
//! `base_input` the engine chose plus the player context. Run a match with a
//! LOW-stat player and a HIGH-stat player and diff the logs — you'll see the
//! low-stat one emit worse targets (`Attack{Target{…}}` onto the wrong entity),
//! mis-aimed skillshots (`Skill{Dir{…}}`), and later recalls.
//!
//! Use it to study what the built-in AI actually does in a given situation, and
//! as a baseline before writing behaviour-correction logic (see ai_perf).
//!
//! Config (optional): `%TEMP%\match_tuner.cfg`
//!   sample_every = 30     # log one in every N ticks (per player). 1 = every tick
//!   only_pos     =        # blank = all; or Top/Jungle/Mid/Bottom/Support
//!   cap          = 5000   # max log lines
//!
//! Output: `%TEMP%\match_tuner.log`

use mod_api::*;
use std::sync::OnceLock;

#[path = "../../../shared/ai_common.rs"]
mod ai_common;
use ai_common::{ctx_snapshot, Logger};

const MOD_ID: &str = "match_tuner";

struct Settings {
    sample_every: usize,
    only_pos: Option<String>,
    cap: usize,
}

fn settings() -> &'static Settings {
    static S: OnceLock<Settings> = OnceLock::new();
    S.get_or_init(|| {
        let c = ai_common::Cfg::load(MOD_ID);
        Settings {
            sample_every: c.usize("sample_every", 30).max(1),
            only_pos: c.string("only_pos").filter(|s| !s.is_empty()).map(|s| s.to_string()),
            cap: c.usize("cap", 5000),
        }
    })
}

fn logger() -> &'static Logger {
    static L: OnceLock<Logger> = OnceLock::new();
    L.get_or_init(|| Logger::new(MOD_ID, settings().cap))
}

#[derive(Debug, Clone)]
struct MatchTuner;

impl ModPlayerInputAi for MatchTuner {
    fn clone_box(&self) -> Box<dyn ModPlayerInputAi> {
        Box::new(self.clone())
    }

    fn id(&self) -> &str {
        MOD_ID
    }

    // Run LAST so the `base_input` we log already reflects any other AI mods
    // loaded alongside this observer.
    fn priority(&self) -> i32 {
        10_000
    }

    fn think(&mut self, ctx: &mut PlayerAiContext, base_input: Option<Input>) -> PlayerInputDecision {
        let s = settings();

        // Position filter (optional).
        if let Some(want) = &s.only_pos {
            if !ctx.position().to_string().eq_ignore_ascii_case(want) {
                return PlayerInputDecision::Pass;
            }
        }

        // Sample by tick so we don't write 60+ lines/sec/player.
        if ctx.tick() % s.sample_every == 0 {
            logger().capped(&ctx_snapshot(ctx, &base_input));
        }

        // Read-only: never change the game's decision.
        PlayerInputDecision::Pass
    }
}

fn init(_ctx: &GameCtx) -> ModRegistration {
    let _ = settings();
    logger().line("=== match_tuner init (read-only observer) ===");
    let mut reg = ModRegistration::new(MOD_ID);
    reg.add_player_input_ai(MatchTuner);
    reg
}

declare_mod!(init);
