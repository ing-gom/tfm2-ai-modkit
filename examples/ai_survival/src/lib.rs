//! ai_survival — improve in-match AI *behaviour* (not stats) via `ModPlayerInputAi`.
//!
//! You cannot edit how the engine turns stats into decisions (that pipeline is
//! in the game binary). What you CAN do is read the input the native AI chose
//! this tick (`base_input`) and `Replace` it with a better one. This example is
//! a stateful "survival governor" built entirely from SELF-STATE — the only
//! world knowledge `PlayerAiContext` reliably exposes today (your own HP /
//! position + the engine's own safe-recall / run-away helpers). It does NOT need
//! enemy/ally positions, so it works without the (still unverified) in-match
//! GameCtx world-query bridge.
//!
//! What it does, in priority order, each tick:
//!   1. honour an active retreat/recall *commitment* — once we decide to back
//!      off we hold it for a few ticks so we actually disengage instead of
//!      flip-flopping with the native AI every frame (the #1 reason naive
//!      per-tick overrides look worse than vanilla).
//!   2. danger HP  → run away WITHOUT burning a skill (don't trade into death).
//!   3. recall band + engine says safe → recall.
//!   4. caution band AND the native AI is being aggressive → veto the chase
//!      (anti-overchase). If it's already playing safe, leave it alone.
//!   5. otherwise Pass — trust the native AI while healthy.
//!
//! Thresholds are POSITION-AWARE: carries (Mid/Bottom) and Support retreat
//! earlier than front-liners (Top/Jungle), via `carry_bias` percentage points.
//!
//! Config (optional): `<game>\mods\ai_survival\ai_survival.cfg` or `%TEMP%\ai_survival.cfg`
//!   enabled      = on
//!   recall_hp    = 35     # recall at/below this hp% when safe
//!   caution_hp   = 50     # below this, veto aggressive native inputs
//!   danger_hp    = 20     # below this, flee (no skill)
//!   carry_bias   = 10     # carries/supports use thresholds this many pts higher
//!   commit_ticks = 30     # hold a retreat/recall decision this many ticks
//!
//! Everything in `think()` runs INSIDE the match simulation — keep it small and
//! deterministic. State is kept per-player as plain values (never store handles).

use mod_api::*;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[path = "../../../shared/ai_common.rs"]
mod ai_common;
use ai_common::Logger;

const MOD_ID: &str = "ai_survival";

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

struct Settings {
    enabled: bool,
    recall_hp: usize,
    caution_hp: usize,
    danger_hp: usize,
    carry_bias: usize,
    commit_ticks: usize,
}

fn settings() -> &'static Settings {
    static S: OnceLock<Settings> = OnceLock::new();
    S.get_or_init(|| {
        let c = ai_common::Cfg::load(MOD_ID);
        let s = Settings {
            enabled: c.bool("enabled", true),
            recall_hp: c.usize("recall_hp", 35).min(100),
            caution_hp: c.usize("caution_hp", 50).min(100),
            danger_hp: c.usize("danger_hp", 20).min(100),
            carry_bias: c.usize("carry_bias", 10).min(100),
            commit_ticks: c.usize("commit_ticks", 30),
        };
        logger().line(&format!(
            "config: enabled={} recall_hp={} caution_hp={} danger_hp={} carry_bias={} commit_ticks={}",
            s.enabled, s.recall_hp, s.caution_hp, s.danger_hp, s.carry_bias, s.commit_ticks
        ));
        s
    })
}

fn logger() -> &'static Logger {
    static L: OnceLock<Logger> = OnceLock::new();
    L.get_or_init(|| Logger::new(MOD_ID, 3000))
}

// ---------------------------------------------------------------------------
// Per-player retreat commitment (anti-flip-flop). Store only VALUES, never
// handles/refs — see docs/05-recipes.md §F.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Recall,
    Retreat,
}

/// player_id -> (commit_until_tick, mode)
fn commits() -> &'static Mutex<HashMap<usize, (usize, Mode)>> {
    static C: OnceLock<Mutex<HashMap<usize, (usize, Mode)>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

fn active_commit(pid: usize, now: usize) -> Option<Mode> {
    let map = commits().lock().unwrap();
    match map.get(&pid) {
        Some(&(until, mode)) if now < until => Some(mode),
        _ => None,
    }
}

fn set_commit(pid: usize, until: usize, mode: Mode) {
    commits().lock().unwrap().insert(pid, (until, mode));
}

fn clear_commit(pid: usize) {
    commits().lock().unwrap().remove(&pid);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Did the native AI pick an aggressive action this tick? (attack / any skill /
/// ult). Move/Return are passive and we never veto those.
fn is_aggressive(base: &Option<Input>) -> bool {
    matches!(
        base,
        Some(Input::Attack { .. } | Input::Skill { .. } | Input::Skill2 { .. } | Input::Ult { .. })
    )
}

/// Carries and supports are squishier / more positionally punished, so they
/// should bail earlier. Returns the hp-threshold bonus (percentage points).
fn position_bias(pos: Position, s: &Settings) -> usize {
    match pos {
        Position::Mid | Position::Bottom | Position::Support => s.carry_bias,
        _ => 0, // Top / Jungle: hold longer
    }
}

/// Fetch the concrete input for a committed mode, validated. Re-fetched fresh
/// each tick (never cached) so it can never go stale.
fn issue(ctx: &mut PlayerAiContext, mode: Mode) -> Option<Input> {
    let input = match mode {
        Mode::Recall => ctx.get_recall_input()?,
        Mode::Retreat => ctx.get_run_away_without_skill_input()?,
    };
    ctx.is_valid_input(&input).then_some(input)
}

// ---------------------------------------------------------------------------
// The hook
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct AiSurvival;

impl ModPlayerInputAi for AiSurvival {
    fn clone_box(&self) -> Box<dyn ModPlayerInputAi> {
        Box::new(self.clone())
    }

    fn id(&self) -> &str {
        MOD_ID
    }

    fn priority(&self) -> i32 {
        100
    }

    fn matches(&self, _ctx: &PlayerAiInitContext) -> bool {
        settings().enabled
    }

    fn think(&mut self, ctx: &mut PlayerAiContext, base_input: Option<Input>) -> PlayerInputDecision {
        let s = settings();
        let now = ctx.tick();
        let pid = ctx.player_id();

        let Some(hp) = ctx.hp_ratio_percent() else {
            return PlayerInputDecision::Pass;
        };

        let bias = position_bias(ctx.position(), s);
        let recall_hp = (s.recall_hp + bias).min(100);
        let caution_hp = (s.caution_hp + bias).min(100);
        let danger_hp = (s.danger_hp + bias).min(100);

        // 1) Honour an active retreat/recall commitment — disengage cleanly
        //    instead of oscillating with the native AI.
        if let Some(mode) = active_commit(pid, now) {
            if let Some(input) = issue(ctx, mode) {
                return PlayerInputDecision::Replace(input);
            }
            clear_commit(pid); // can't issue it anymore (e.g. recall no longer valid)
        }

        // 2) Danger: flee without spending a skill.
        if hp <= danger_hp {
            if let Some(run) = ctx.get_run_away_without_skill_input() {
                if ctx.is_valid_input(&run) {
                    set_commit(pid, now + s.commit_ticks, Mode::Retreat);
                    log_decision(ctx, hp, &base_input, "DANGER -> retreat(no-skill)");
                    return PlayerInputDecision::Replace(run);
                }
            }
        }

        // 3) Recall band: only when the engine itself deems it safe.
        if hp <= recall_hp && ctx.is_safe_to_recall() {
            if let Some(recall) = ctx.get_recall_input() {
                if ctx.is_valid_input(&recall) {
                    set_commit(pid, now + s.commit_ticks, Mode::Recall);
                    log_decision(ctx, hp, &base_input, "LOW+SAFE -> recall");
                    return PlayerInputDecision::Replace(recall);
                }
            }
        }

        // 4) Caution band: veto the chase only if the native AI is being
        //    aggressive. If it's already retreating/farming, don't fight it.
        if hp <= caution_hp && is_aggressive(&base_input) {
            if let Some(run) = ctx.get_run_away_without_skill_input() {
                if ctx.is_valid_input(&run) {
                    // Half-length commit: enough to break the chase, short enough
                    // to hand control back quickly if the situation clears.
                    set_commit(pid, now + s.commit_ticks / 2, Mode::Retreat);
                    log_decision(ctx, hp, &base_input, "CAUTION+aggressive -> disengage");
                    return PlayerInputDecision::Replace(run);
                }
            }
        }

        // 5) Healthy / native already passive — trust it.
        PlayerInputDecision::Pass
    }
}

fn log_decision(ctx: &PlayerAiContext, hp: usize, base: &Option<Input>, what: &str) {
    logger().capped(&format!(
        "tick={} player={} pos={} champ={} hp={}% base={:?} -> {what}",
        ctx.tick(),
        ctx.player_id(),
        ctx.position().to_string(),
        ctx.champion_name(),
        hp,
        base,
    ));
}

fn init(_ctx: &GameCtx) -> ModRegistration {
    logger().line("=== ai_survival init ===");
    let _ = settings();
    let mut reg = ModRegistration::new(MOD_ID);
    reg.add_player_input_ai(AiSurvival);
    reg
}

declare_mod!(init);
