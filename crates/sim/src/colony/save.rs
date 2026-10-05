//! Saved worlds: every chunk of the cell world plus the whole colony
//! (including every player who has ever joined), in one file.

use serde::{Deserialize, Serialize};

use super::Colony;
use crate::world::World;

const MAGIC: [u8; 4] = *b"SBTF";
/// Bump when the layout changes; older files are refused with a clear error.
pub const SAVE_VERSION: u32 = 2;

#[derive(Serialize, Deserialize)]
pub struct SaveFile {
    pub width: u32,
    pub height: u32,
    pub seed: u64,
    /// Run-length encoded chunks in row order.
    pub chunks: Vec<Vec<u8>>,
    pub colony: Colony,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LoadError {
    NotASave,
    /// The file was written by a different version of the game.
    Version(u32),
    Corrupt,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::NotASave => write!(f, "not a saved world"),
            LoadError::Version(v) => write!(
                f,
                "saved by a different version of the game (format {v}, this build reads {SAVE_VERSION})"
            ),
            LoadError::Corrupt => write!(f, "the file is damaged"),
        }
    }
}

impl SaveFile {
    pub fn capture(world: &World, colony: &Colony) -> SaveFile {
        let mut chunks = Vec::with_capacity(world.chunks_x() * world.chunks_y());
        for cy in 0..world.chunks_y() {
            for cx in 0..world.chunks_x() {
                chunks.push(world.encode_chunk(cx, cy));
            }
        }
        SaveFile {
            width: world.width() as u32,
            height: world.height() as u32,
            seed: world.seed(),
            chunks,
            colony: colony.clone(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        out.extend_from_slice(&SAVE_VERSION.to_le_bytes());
        out.extend(bincode::serialize(self).expect("saves always serialize"));
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<SaveFile, LoadError> {
        if bytes.len() < 8 || bytes[..4] != MAGIC {
            return Err(LoadError::NotASave);
        }
        let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        if version != SAVE_VERSION {
            return Err(LoadError::Version(version));
        }
        bincode::deserialize(&bytes[8..]).map_err(|_| LoadError::Corrupt)
    }

    /// Rebuilds the world and colony. Everyone starts offline.
    pub fn restore(self) -> Result<(World, Colony), LoadError> {
        let mut world = World::new(self.width as usize, self.height as usize, self.seed);
        if self.chunks.len() != world.chunks_x() * world.chunks_y() {
            return Err(LoadError::Corrupt);
        }
        let cx_n = world.chunks_x();
        for (i, data) in self.chunks.iter().enumerate() {
            world
                .decode_chunk(i % cx_n, i / cx_n, data)
                .map_err(|_| LoadError::Corrupt)?;
        }
        world.take_net_dirty();
        Ok((world, self.colony))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colony::items::Item;
    use crate::colony::testing::*;
    use crate::material::Material;

    #[test]
    fn worlds_roundtrip_with_players_and_progress() {
        let (mut world, mut colony, key) = colony();
        colony.credits = 1234;
        colony.atmosphere.o2 = 7.5;
        colony.players.get_mut(&key).unwrap().inv.add(Item::Gold, 9, 1.0);
        world.set(100, 100, Material::Lava);
        run(&mut colony, &mut world, 1.0);
        // Robots and their programs, areas, pins, hybrids and body mods.
        use crate::colony::geom::{Rect, v2};
        use crate::colony::mods::{Slot, set};
        use crate::colony::robots::{Area, Cond, Place, Robot, Sentence, Step};
        let mut robot = Robot::new("Rusty".into(), 3);
        robot.charge = 41.0;
        robot.program = vec![Sentence {
            when: Some(Cond::Battery {
                below: true,
                percent: 20,
            }),
            steps: vec![Step::ChargeAt(Place::NearestPylon).into()],
        }];
        robot.inv.add(Item::Iron, 12, 1.0);
        let robot_id = colony.spawn(v2(300.0, 200.0), crate::colony::EntKind::Robot(robot.clone()));
        colony.areas.push(Area {
            id: 0,
            name: "North vein".into(),
            rect: Rect::new(10, 20, 110, 90),
        });
        colony.pins.push(crate::colony::modfx::Pin {
            id: 0,
            name: "Lake".into(),
            pos: v2(50.0, 60.0),
            color: 2,
            ping: None,
        });
        let hybrid = crate::colony::plants::splice(
            colony.seed,
            &colony.species[0],
            &colony.species[1],
            0,
            &colony.species,
        );
        colony.species.push(crate::colony::plants::Species {
            id: colony.species.len() as u16,
            ..hybrid
        });
        let p = colony.players.get_mut(&key).unwrap();
        p.mods.insert(set::ROCKET_FEET, 3);
        p.equipped[Slot::Feet as usize] = Some(set::ROCKET_FEET);
        p.gear.arm_tasks[0] = crate::colony::mods::ArmTask::Mine;

        let bytes = SaveFile::capture(&world, &colony).to_bytes();
        let (world2, mut colony2) = SaveFile::from_bytes(&bytes).unwrap().restore().unwrap();
        assert_eq!(world2.material(100, 100), Material::Lava);
        for cy in 0..world.chunks_y() {
            for cx in 0..world.chunks_x() {
                assert_eq!(world.encode_chunk(cx, cy), world2.encode_chunk(cx, cy));
            }
        }
        assert_eq!(colony2.credits, 1234);
        assert_eq!(colony2.ents, colony.ents);
        assert_eq!(colony2.clock, colony.clock);
        assert_eq!(colony2.online().count(), 0, "nobody is online after loading");
        // The player gets their character back on joining.
        colony2.join(key, "Ada".into());
        assert_eq!(colony2.players[&key].inv.count(Item::Gold), 9);
        assert_eq!(colony2.players[&key].stats().thrust_secs, 5.0);
        assert_eq!(
            colony2.players[&key].gear.arm_tasks[0],
            crate::colony::mods::ArmTask::Mine
        );
        assert_eq!(colony2.ents[&robot_id].kind, crate::colony::EntKind::Robot(robot));
        assert_eq!(colony2.areas, colony.areas);
        assert_eq!(colony2.pins, colony.pins);
        assert_eq!(colony2.species, colony.species);
        assert_eq!(colony2.species.last().unwrap().parents, Some((0, 1)));
        // And the loaded world keeps running.
        let mut world2 = world2;
        run(&mut colony2, &mut world2, 1.0);
    }

    #[test]
    fn bad_files_are_refused() {
        assert_eq!(SaveFile::from_bytes(b"hello").err(), Some(LoadError::NotASave));
        let (world, colony, _) = colony();
        let mut bytes = SaveFile::capture(&world, &colony).to_bytes();
        let mut other = bytes.clone();
        other[4] = 99;
        assert_eq!(SaveFile::from_bytes(&other).err(), Some(LoadError::Version(99)));
        bytes.truncate(bytes.len() / 2);
        assert_eq!(SaveFile::from_bytes(&bytes).err(), Some(LoadError::Corrupt));
    }
}
