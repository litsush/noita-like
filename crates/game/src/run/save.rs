//! Persistent meta progression and settings, as JSON in the platform data
//! directory. A missing file starts fresh; a corrupt one is backed up to
//! `save.json.bak` and replaced, so a bad file never blocks the game.

use std::collections::BTreeSet;
use std::path::PathBuf;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::achievements::{AchievementId, StartChoice};
use super::scrolls::{FusionId, School, ScrollId};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DigMode {
    /// Dig where the cursor points.
    #[default]
    Cursor,
    /// Dig whatever blocks you in the direction you move or aim.
    Smart,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub screen_shake: bool,
    pub dig_mode: DigMode,
    /// UI zoom (egui zoom factor).
    pub ui_scale: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            master_volume: 0.8,
            music_volume: 0.6,
            sfx_volume: 0.8,
            screen_shake: true,
            dig_mode: DigMode::Cursor,
            ui_scale: 1.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Stats {
    pub runs: u32,
    pub wins: u32,
    /// Deepest row reached in any run.
    pub best_depth: i32,
    /// Fastest win, in seconds.
    pub best_time: Option<f32>,
}

#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct SaveData {
    /// Stored by name and parsed leniently: names from older versions map
    /// forward, unknown ones are dropped, so old saves always load.
    #[serde(with = "named_achievements")]
    pub achievements: BTreeSet<AchievementId>,
    pub stats: Stats,
    pub settings: Settings,
    pub start_choice: StartChoice,
    /// The school picked last time, preselected on the wizard screen.
    pub last_school: Option<String>,
    #[serde(with = "named_fusions")]
    pub fusions_discovered: BTreeSet<FusionId>,
    /// Journal pages read, across all runs.
    pub journals_read: BTreeSet<u16>,
    pub bindings: crate::controls::Bindings,
}

impl SaveData {
    pub fn has(&self, a: AchievementId) -> bool {
        self.achievements.contains(&a)
    }

    pub fn school_unlocked(&self, s: School) -> bool {
        s.unlock().is_none_or(|a| self.has(a))
    }

    pub fn scroll_unlocked(&self, sc: ScrollId) -> bool {
        let def = sc.def();
        def.school.is_none_or(|s| self.school_unlocked(s)) && def.unlock.is_none_or(|a| self.has(a))
    }

    pub fn start_unlocked(&self, c: StartChoice) -> bool {
        c.unlock().is_none_or(|a| self.has(a))
    }

    pub fn unlocked_schools(&self) -> Vec<School> {
        School::ALL
            .into_iter()
            .filter(|&s| self.school_unlocked(s))
            .collect()
    }

    /// Unlocked scrolls of the given schools plus neutral ones.
    pub fn scrolls_for(&self, schools: &[School]) -> Vec<ScrollId> {
        ScrollId::all()
            .filter(|&sc| self.scroll_unlocked(sc))
            .filter(|sc| sc.def().school.is_none_or(|s| schools.contains(&s)))
            .collect()
    }

    /// Records an achievement; returns true if it's new.
    pub fn grant(&mut self, id: AchievementId) -> bool {
        self.achievements.insert(id)
    }
}

/// Serde helpers that store sets of enums by name, tolerating unknown names.
macro_rules! named_set {
    ($module:ident, $ty:ty, $parse:expr, $name:expr) => {
        mod $module {
            use std::collections::BTreeSet;

            use serde::{Deserialize, Deserializer, Serialize, Serializer};

            #[allow(unused_imports)]
            use super::*;

            pub fn serialize<S: Serializer>(set: &BTreeSet<$ty>, s: S) -> Result<S::Ok, S::Error> {
                let names: Vec<String> = set.iter().map(|v| $name(*v)).collect();
                names.serialize(s)
            }

            pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<BTreeSet<$ty>, D::Error> {
                let names = Vec::<serde_json::Value>::deserialize(d)?;
                Ok(names
                    .iter()
                    .filter_map(|v| v.as_str())
                    .filter_map($parse)
                    .collect())
            }
        }
    };
}

named_set!(
    named_achievements,
    AchievementId,
    AchievementId::from_name,
    AchievementId::name_id
);
named_set!(named_fusions, FusionId, FusionId::from_name, FusionId::name_id);

/// Where the save file lives. `SBCT_SAVE_DIR` overrides (handy for testing).
pub fn save_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("SBCT_SAVE_DIR") {
        return PathBuf::from(dir);
    }
    let env = |k: &str| std::env::var_os(k).map(PathBuf::from);
    let base = if cfg!(windows) {
        env("APPDATA")
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library/Application Support"))
    } else {
        env("XDG_DATA_HOME").or_else(|| env("HOME").map(|h| h.join(".local/share")))
    };
    base.unwrap_or_else(|| PathBuf::from(".")).join("sbct")
}

pub fn load() -> SaveData {
    let path = save_dir().join("save.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return SaveData::default();
    };
    match serde_json::from_str(&text) {
        Ok(data) => data,
        Err(e) => {
            warn!(
                "save file {} is corrupt ({e}); backing it up and starting fresh",
                path.display()
            );
            let _ = std::fs::rename(&path, path.with_extension("json.bak"));
            SaveData::default()
        }
    }
}

/// Writes atomically (temp file + rename) so a crash mid-write can't corrupt it.
pub fn store(data: &SaveData) {
    let dir = save_dir();
    let result = (|| -> std::io::Result<()> {
        std::fs::create_dir_all(&dir)?;
        let tmp = dir.join("save.json.tmp");
        std::fs::write(
            &tmp,
            serde_json::to_string_pretty(data).expect("save data serializes"),
        )?;
        std::fs::rename(tmp, dir.join("save.json"))
    })();
    if let Err(e) = result {
        warn!("couldn't write save file in {}: {e}", dir.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_and_tolerates_bad_files() {
        let dir = std::env::temp_dir().join(format!("sbct-save-test-{}", std::process::id()));
        // SAFETY: tests in this module are the only users of this variable.
        unsafe { std::env::set_var("SBCT_SAVE_DIR", &dir) };

        assert_eq!(load(), SaveData::default(), "missing file starts fresh");

        let mut data = SaveData::default();
        data.grant(AchievementId::Pyromaniac);
        data.fusions_discovered.insert(FusionId::SteamBurst);
        data.journals_read.insert(3);
        data.stats.runs = 3;
        store(&data);
        assert_eq!(load(), data);
        assert!(
            load().scroll_unlocked(ScrollId::AlchemistsCharge) == load().school_unlocked(School::Alchemy)
        );
        assert!(!load().scroll_unlocked(ScrollId::GillsOfTheDeep));

        std::fs::write(dir.join("save.json"), "{ not json").unwrap();
        assert_eq!(load(), SaveData::default(), "corrupt file starts fresh");
        assert!(dir.join("save.json.bak").exists(), "corrupt file is backed up");

        // Unknown and missing fields are fine (forward/backward compatible).
        std::fs::write(dir.join("save.json"), r#"{"stats":{"runs":7},"future_field":1}"#).unwrap();
        assert_eq!(load().stats.runs, 7);

        // A save from before the wizard rework: old achievement names map
        // forward, unknown ones and the old loadout are dropped, nothing fails.
        std::fs::write(
            dir.join("save.json"),
            r#"{"achievements":["Pyromaniac","CoreBreaker","GoneAchievement"],"loadout":"Excavator",
                "stats":{"runs":12,"wins":1,"best_depth":3988},"settings":{"master_volume":0.5}}"#,
        )
        .unwrap();
        let old = load();
        assert!(old.has(AchievementId::Pyromaniac));
        assert!(old.has(AchievementId::RiteComplete));
        assert_eq!(old.achievements.len(), 2);
        assert_eq!(old.stats.wins, 1);
        assert_eq!(old.settings.master_volume, 0.5);
        assert_eq!(old.start_choice, StartChoice::Initiate);
        assert!(old.start_unlocked(StartChoice::ArchmagesHeir));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
