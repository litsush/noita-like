//! Persistent meta progression and settings, as JSON in the platform data
//! directory. A missing file starts fresh; a corrupt one is backed up to
//! `save.json.bak` and replaced, so a bad file never blocks the game.

use std::collections::BTreeSet;
use std::path::PathBuf;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::achievements::{AchievementId, Loadout};
use super::items::ItemId;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub screen_shake: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            master_volume: 0.8,
            music_volume: 0.6,
            sfx_volume: 0.8,
            screen_shake: true,
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
    pub achievements: BTreeSet<AchievementId>,
    pub stats: Stats,
    pub settings: Settings,
    pub loadout: Loadout,
}

impl SaveData {
    pub fn is_unlocked(&self, item: ItemId) -> bool {
        item.def().unlock.is_none_or(|a| self.achievements.contains(&a))
    }

    pub fn loadout_unlocked(&self, loadout: Loadout) -> bool {
        loadout.unlock().is_none_or(|a| self.achievements.contains(&a))
    }

    pub fn unlocked_items(&self) -> Vec<ItemId> {
        ItemId::ALL.into_iter().filter(|&i| self.is_unlocked(i)).collect()
    }

    /// Records an achievement; returns true if it's new.
    pub fn grant(&mut self, id: AchievementId) -> bool {
        self.achievements.insert(id)
    }
}

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
        data.stats.runs = 3;
        store(&data);
        assert_eq!(load(), data);
        assert!(load().is_unlocked(ItemId::BlastCharges));
        assert!(!load().is_unlocked(ItemId::GillMask));

        std::fs::write(dir.join("save.json"), "{ not json").unwrap();
        assert_eq!(load(), SaveData::default(), "corrupt file starts fresh");
        assert!(dir.join("save.json.bak").exists(), "corrupt file is backed up");

        // Unknown and missing fields are fine (forward/backward compatible).
        std::fs::write(dir.join("save.json"), r#"{"stats":{"runs":7},"future_field":1}"#).unwrap();
        assert_eq!(load().stats.runs, 7);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
