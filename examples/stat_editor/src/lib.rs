//! stat_editor — STARTER: edit player stats from the management layer.
//!
//! The supported way to "change the AI's numbers". You cannot patch the in-match
//! decision pipeline (it lives in the shipped game binary, not the mod SDK), but
//! the engine *derives* all match behaviour from each athlete's raw `AthleteStat`.
//! So if you raise those stats, the built-in AI plays better — fewer misjudged
//! fights, sharper late-game, faster reactions — with zero engine internals.
//!
//! Two layers a mod can act on:
//!   • management / database layer → THIS mod (`ModServerExtension`): edit
//!     `Athlete.stat` directly. Persistent, applies to every match the save plays.
//!   • in-match per-tick layer     → `ModPlayerInputAi::think` (override `Input`);
//!     see `examples/ai_perf`, `examples/ai_survival`.
//!
//! Mechanism: apply once in `on_server_start` (optional, immediate) and again
//! after every management tick, so the engine's training/aging never washes the
//! tuning out. Stats are round-tripped through `serde_json`, so edits are by
//! field NAME and survive EA patches that add stat fields. Values clamp to 0..=100.
//!
//! Config: `<game>\mods\stat_editor\stat_editor.cfg` or `%TEMP%\stat_editor.cfg`.
//! Lines are `key = value`; `#` starts a comment:
//!
//!   enabled       = on
//!   team          = 0          # optional: only this team's athletes (omit = all)
//!   apply_on_load = on         # apply once immediately, not just on tick
//!   # per-stat ops — "<op>:<n>", op in {set, floor, cap, scale(%)}:
//!   judgement     = floor:80   # never below 80
//!   control_speed = floor:80
//!   skill_hit     = scale:120  # +20% of current
//!   ego           = cap:40     # never above 40 (team player)

use mod_api::*;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[path = "../../../shared/ai_common.rs"]
mod ai_common;
use ai_common::Logger;

const MOD_ID: &str = "stat_editor";

/// One edit applied to a named stat. `apply()` returns the new value, clamped.
#[derive(Debug, Clone, Copy)]
enum Op {
    Set(u64),
    Floor(u64),
    Cap(u64),
    Scale(u64), // percent: 120 => x1.20
}

impl Op {
    fn parse(spec: &str) -> Option<Op> {
        let (op, n) = spec.split_once(':')?;
        let n: u64 = n.trim().parse().ok()?;
        Some(match op.trim().to_ascii_lowercase().as_str() {
            "set" => Op::Set(n),
            "floor" | "min" => Op::Floor(n),
            "cap" | "max" => Op::Cap(n),
            "scale" | "mul" => Op::Scale(n),
            _ => return None,
        })
    }

    fn apply(self, cur: u64) -> u64 {
        let v = match self {
            Op::Set(n) => n,
            Op::Floor(n) => cur.max(n),
            Op::Cap(n) => cur.min(n),
            Op::Scale(p) => cur.saturating_mul(p) / 100,
        };
        v.min(100)
    }
}

/// The stat keys this mod recognises (= `AthleteStat` serde field names). A
/// config line whose key isn't here is ignored, so a typo can't corrupt stats.
const KNOWN_STATS: &[&str] = &[
    "last_hit", "skill_avoid", "skill_hit", "positioning", "control_speed",
    "concentration", "mental", "judgement", "order", "roaming", "aggressive",
    "ego", "stamina", "condition",
];

struct Settings {
    enabled: bool,
    team: Option<usize>,
    apply_on_load: bool,
    /// stat key (serde field name) -> op
    ops: BTreeMap<String, Op>,
}

fn settings() -> &'static Settings {
    static S: OnceLock<Settings> = OnceLock::new();
    S.get_or_init(|| {
        let c = ai_common::Cfg::load(MOD_ID);
        let mut ops = BTreeMap::new();
        for &stat in KNOWN_STATS {
            if let Some(spec) = c.string(stat) {
                match Op::parse(spec) {
                    Some(op) => {
                        ops.insert(stat.to_string(), op);
                    }
                    None => logger().line(&format!(
                        "ignoring bad op for '{stat}': '{spec}' (use set:/floor:/cap:/scale:N)"
                    )),
                }
            }
        }
        let s = Settings {
            enabled: c.bool("enabled", true),
            team: c.string("team").and_then(|v| v.parse().ok()),
            apply_on_load: c.bool("apply_on_load", true),
            ops,
        };
        logger().line(&format!(
            "config: enabled={} team={:?} apply_on_load={} ops={:?}",
            s.enabled, s.team, s.apply_on_load, s.ops
        ));
        s
    })
}

fn logger() -> &'static Logger {
    static L: OnceLock<Logger> = OnceLock::new();
    L.get_or_init(|| Logger::new(MOD_ID, 2000))
}

/// Apply the configured ops to one athlete's `stat`. Returns the count of stat
/// fields actually changed (0 if nothing matched / nothing moved).
fn edit_athlete(a: &mut Athlete, s: &Settings) -> usize {
    // Round-trip the stat block to JSON so we can edit by field name without
    // depending on AthleteStat's concrete field set (survives EA stat additions).
    let Ok(mut v) = serde_json::to_value(&a.stat) else {
        return 0;
    };
    let Value::Object(map) = &mut v else { return 0 };

    let mut changed = 0;
    for (key, op) in &s.ops {
        let Some(slot) = map.get_mut(key) else { continue };
        let Some(cur) = slot.as_u64() else { continue }; // stat fields are integers
        let next = op.apply(cur);
        if next != cur {
            *slot = Value::from(next);
            changed += 1;
        }
    }
    if changed > 0 {
        match serde_json::from_value(v) {
            Ok(stat) => a.stat = stat,
            Err(_) => return 0, // deserialize failed → leave the athlete untouched
        }
    }
    changed
}

/// Walk every athlete in the database, applying edits to those that pass the
/// optional team filter. Returns (athletes_touched, fields_changed).
fn edit_all(ctx: &mut ServerModContext, s: &Settings) -> (usize, usize) {
    let (mut athletes, mut fields) = (0, 0);
    for a in ctx.database.athletes.iter_mut() {
        if let Some(team) = s.team {
            if !a.with(team) {
                continue;
            }
        }
        let n = edit_athlete(a, s);
        if n > 0 {
            athletes += 1;
            fields += n;
        }
    }
    (athletes, fields)
}

#[derive(Debug)]
struct StatEditor;

impl ModServerExtension for StatEditor {
    /// Apply the moment the save loads, so the effect is visible without waiting
    /// for the first management tick.
    fn on_server_start(&self, ctx: &mut ServerModContext) {
        let s = settings();
        if !s.enabled || s.ops.is_empty() || !s.apply_on_load {
            return;
        }
        let (athletes, fields) = edit_all(ctx, s);
        logger().line(&format!(
            "on_server_start: edited {fields} field(s) across {athletes} athlete(s)"
        ));
    }

    /// Re-apply after each management tick so training/aging never erases it.
    fn after_management_tick(&self, ctx: &mut ServerModContext) {
        let s = settings();
        if !s.enabled || s.ops.is_empty() {
            return;
        }
        let (athletes, fields) = edit_all(ctx, s);
        if fields > 0 {
            logger().line(&format!(
                "management tick: re-edited {fields} field(s) across {athletes} athlete(s)"
            ));
        }
    }
}

fn init(_ctx: &GameCtx) -> ModRegistration {
    logger().line("=== stat_editor init ===");
    let _ = settings(); // load + log config at startup
    let mut reg = ModRegistration::new(MOD_ID);
    reg.set_server_extension(StatEditor);
    reg
}

declare_mod!(init);
