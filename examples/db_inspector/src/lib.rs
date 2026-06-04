//! db_inspector — STARTER: read the client database + persist mod data (SDK 0.4.8).
//!
//! Showcases the capability the 0.4.8 SDK opened to mods. A `ModExtension`
//! frame hook is handed `&mut Scene`; when the game is in a match the
//! `Scene::InGame { data }` variant gives you `ClientData` — the whole client
//! database (teams, athletes, staff, leagues, matches …) PLUS `mod_save_data`,
//! a per-mod key/value store written into the game's save file.
//!
//! On each transition INTO an in-game scene this example:
//!   1. reads the DB (athlete / team counts) — read-only, safe.
//!   2. bumps a persistent "times this save entered a match" counter via
//!      `mod_save_data`, demonstrating cross-session persistence.
//! It logs to `%TEMP%\db_inspector.log`.
//!
//! WRITES — important: reading the DB and writing `mod_save_data` are the
//! verified, intended uses. Writing game-state collections (athletes, teams)
//! through `db_mut()` compiles, but whether such edits take effect in the
//! running game is UNVERIFIED. To durably edit player stats, use
//! `ModServerExtension` instead — see docs/03 §A and docs/01 §ModServerExtension.
//!
//! Config (optional): `%TEMP%\db_inspector.cfg` or
//! `<game>\mods\db_inspector\db_inspector.cfg`
//!   enabled = on
//!   persist = on      # bump the persistent match-entry counter

use mod_api::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

#[path = "../../../shared/ai_common.rs"]
mod ai_common;
use ai_common::Logger;

const MOD_ID: &str = "db_inspector";
const KEY_ENTRIES: &str = "ingame_entries";
const SAVE_VERSION: usize = 1;

struct Settings {
    enabled: bool,
    persist: bool,
}

fn settings() -> &'static Settings {
    static S: OnceLock<Settings> = OnceLock::new();
    S.get_or_init(|| {
        let c = ai_common::Cfg::load(MOD_ID);
        let s = Settings {
            enabled: c.bool("enabled", true),
            persist: c.bool("persist", true),
        };
        logger().line(&format!("config: enabled={} persist={}", s.enabled, s.persist));
        s
    })
}

fn logger() -> &'static Logger {
    static L: OnceLock<Logger> = OnceLock::new();
    L.get_or_init(|| Logger::new(MOD_ID, 4000))
}

/// `in_game` debounces the per-frame hook to one action per match entry.
/// `ModExtension` methods take `&self`, so session state lives in an atomic —
/// store VALUES only here, never an engine handle across calls.
struct DbInspector {
    in_game: AtomicBool,
}

impl ModExtension for DbInspector {
    fn pre_update(
        &self,
        scene: &mut Scene,
        _ui: &mut UI<(), UIOutEvent>,
        _assets: &mut Assets,
        _dt: f32,
    ) {
        if !settings().enabled {
            return;
        }
        match scene {
            // In a match: ClientData -> the whole client DB + mod_save_data.
            Scene::InGame { data } => {
                // Act once per entry, not every frame.
                if self.in_game.swap(true, Ordering::Relaxed) {
                    return;
                }

                // 1) READ the database (immutable borrow — drop it before writing).
                let (athletes, teams) = {
                    let db = data.db();
                    (db.athletes.len(), db.teams.len())
                };

                // 2) PERSIST: bump a counter stored in the save via mod_save_data.
                let entries_msg = if settings().persist {
                    let mut db = data.db_mut();
                    let prev = db
                        .mod_save_data
                        .get_string(MOD_ID, KEY_ENTRIES)
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0);
                    let next = prev + 1;
                    db.mod_save_data
                        .set_string(MOD_ID, KEY_ENTRIES, next.to_string());
                    db.mod_save_data.set_version(MOD_ID, SAVE_VERSION);
                    format!("ingame_entries(persisted)={prev}->{next}")
                } else {
                    String::from("persist=off")
                };

                logger().line(&format!(
                    "InGame: athletes={athletes} teams={teams} | {entries_msg}"
                ));
            }
            // Left the match — re-arm for the next entry.
            _ => {
                self.in_game.store(false, Ordering::Relaxed);
            }
        }
    }
}

fn init(_ctx: &GameCtx) -> ModRegistration {
    logger().line("=== db_inspector init ===");
    let _ = settings(); // load + log config at startup
    let mut reg = ModRegistration::new(MOD_ID);
    reg.set_extension(DbInspector {
        in_game: AtomicBool::new(false),
    });
    reg
}

declare_mod!(init);
