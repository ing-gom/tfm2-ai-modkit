//! ai_common — reusable helpers for TFM2 AI mods (ai-modkit).
//!
//! Dependency-free (std only). Included into an example crate as a MODULE via:
//!
//! ```ignore
//! #[path = "../../../shared/ai_common.rs"]
//! mod ai_common;
//! ```
//!
//! Because it compiles as part of the cdylib crate (not a separate crate), the
//! `mod_api` types injected by the SDK build (`--extern mod_api`) are in scope
//! here without any extra wiring. Keep this file std-only so every example
//! stays a fully offline build.

#![allow(dead_code)]

use mod_api::*;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

// ---------------------------------------------------------------------------
// Logging — bounded, append-only file under %TEMP%.
// ---------------------------------------------------------------------------

/// Append-only logger. Construct once (e.g. behind a `OnceLock`) and reuse.
/// `capped()` bounds total lines so a long match never floods the disk; use
/// `line()` for unbounded one-shot messages (init banner, config dump).
pub struct Logger {
    path: PathBuf,
    cap: usize,
    n: AtomicUsize,
}

impl Logger {
    /// Logs to `%TEMP%\<file_stem>.log`. `cap` bounds `capped()` writes.
    pub fn new(file_stem: &str, cap: usize) -> Self {
        Logger {
            path: std::env::temp_dir().join(format!("{file_stem}.log")),
            cap,
            n: AtomicUsize::new(0),
        }
    }

    /// Always writes (no cap). For init / rare events.
    pub fn line(&self, s: &str) {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&self.path) {
            let _ = writeln!(f, "{s}");
        }
    }

    /// Writes only while under `cap`. Returns `false` once the cap is reached,
    /// so callers can cheaply stop formatting further lines.
    pub fn capped(&self, s: &str) -> bool {
        if self.n.fetch_add(1, Ordering::Relaxed) >= self.cap {
            return false;
        }
        self.line(s);
        true
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

// ---------------------------------------------------------------------------
// Config — dependency-free `key = value` reader (no serde → offline build).
// ---------------------------------------------------------------------------

/// Reads `<game>\mods\<mod_id>\<mod_id>.cfg`, falling back to
/// `%TEMP%\<mod_id>.cfg`. Lines are `key = value`; `#` starts a comment.
/// At runtime the game's working directory is the game root, so the first
/// path resolves to the mod's own folder.
pub struct Cfg {
    map: HashMap<String, String>,
}

impl Cfg {
    pub fn load(mod_id: &str) -> Self {
        let mut map = HashMap::new();
        let candidates = [
            std::env::current_dir()
                .ok()
                .map(|d| d.join("mods").join(mod_id).join(format!("{mod_id}.cfg"))),
            Some(std::env::temp_dir().join(format!("{mod_id}.cfg"))),
        ];
        for path in candidates.into_iter().flatten() {
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    map.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
            break;
        }
        Cfg { map }
    }

    pub fn bool(&self, key: &str, default: bool) -> bool {
        match self.map.get(key) {
            Some(v) => matches!(v.to_ascii_lowercase().as_str(), "on" | "true" | "1" | "yes"),
            None => default,
        }
    }

    pub fn usize(&self, key: &str, default: usize) -> usize {
        self.map.get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
    }

    pub fn f32(&self, key: &str, default: f32) -> f32 {
        self.map.get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
    }

    pub fn string(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(|s| s.as_str())
    }
}

// ---------------------------------------------------------------------------
// Input constructors — read intent-first in behaviour code.
// (Input / InputTarget are public mod_api enums.)
// ---------------------------------------------------------------------------

pub mod input {
    use super::*;

    pub fn move_to(x: u64, y: u64) -> Input {
        Input::Move { x, y }
    }
    /// Recall to base (the engine's "Return").
    pub fn recall() -> Input {
        Input::Return
    }
    pub fn attack(target_id: usize) -> Input {
        Input::Attack { target: InputTarget::Target { target_id } }
    }
    pub fn skill_at(x: u64, y: u64) -> Input {
        Input::Skill { target: InputTarget::Pos { x, y } }
    }
    pub fn skill_dir(dir_x: i64, dir_y: i64) -> Input {
        Input::Skill { target: InputTarget::Dir { dir_x, dir_y } }
    }
    pub fn skill_on(target_id: usize) -> Input {
        Input::Skill { target: InputTarget::Target { target_id } }
    }
    pub fn ult_on(target_id: usize) -> Input {
        Input::Ult { target: InputTarget::Target { target_id } }
    }

    /// True if the native AI's chosen input is an aggressive action (attack /
    /// any skill / ult). Move/Return are passive. Handy for "veto the chase"
    /// behaviour: only override when the engine is committing to a fight.
    pub fn is_aggressive(base: &Option<Input>) -> bool {
        matches!(
            base,
            Some(Input::Attack { .. } | Input::Skill { .. } | Input::Skill2 { .. } | Input::Ult { .. })
        )
    }
}

// ---------------------------------------------------------------------------
// Diagnostics.
// ---------------------------------------------------------------------------

/// One-line snapshot of a PlayerAiContext + the native AI's chosen input.
/// Handy inside `think()` to observe how the built-in AI decides.
pub fn ctx_snapshot(ctx: &PlayerAiContext, base_input: &Option<Input>) -> String {
    format!(
        "tick={} player={} team={} pos={} champ={} hp={:?}% base={:?}",
        ctx.tick(),
        ctx.player_id(),
        ctx.team(),
        ctx.position().to_string(),
        ctx.champion_name(),
        ctx.hp_ratio_percent(),
        base_input,
    )
}
