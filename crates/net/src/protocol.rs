//! Wire messages. The host owns the world and the colony; clients send their
//! own pose and their actions, and receive everything else.

use serde::{Deserialize, Serialize};

use sbct_sim::colony::actions::Action;
use sbct_sim::colony::{
    Colony, Ent, Event, Globals, Id, Motion, Player, PlayerKey, Pose, PublicPlayer, Vitals,
};

/// Bump whenever a message layout changes so mismatched builds refuse to connect.
pub const PROTOCOL_VERSION: u32 = 20;

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMsg {
    /// `key` identifies the player across sessions so their character is kept.
    Hello {
        version: u32,
        name: String,
        key: PlayerKey,
    },
    Pose(Pose),
    Action(Action),
}

/// A player's pose as relayed to everyone, with health and oxygen (0–255)
/// for name tags.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct PoseUpdate {
    pub key: PlayerKey,
    pub pose: Pose,
    pub hp: u8,
    pub o2: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum HostMsg {
    /// The whole colony as it is now; the cell world follows as chunks.
    Welcome {
        key: PlayerKey,
        width: u32,
        height: u32,
        seed: u64,
        colony: Box<Colony>,
    },
    Reject {
        reason: String,
    },
    /// Run-length encoded chunk, see `World::encode_chunk`.
    Chunk {
        cx: u16,
        cy: u16,
        data: Vec<u8>,
    },
    /// Everyone online. Sent whenever someone joins, leaves or changes how
    /// they look.
    Roster(Vec<PublicPlayer>),
    Poses(Vec<PoseUpdate>),
    /// The receiving player's full state (inventory, mods).
    You(Box<Player>),
    Vitals(Vitals),
    /// Entities that changed (whole snapshots) and ids that are gone.
    Ents {
        upserts: Vec<Ent>,
        removed: Vec<Id>,
    },
    Motion(Vec<Motion>),
    Globals(Globals),
    Events(Vec<Event>),
}

pub fn encode<T: Serialize>(msg: &T) -> Vec<u8> {
    bincode::serialize(msg).expect("protocol messages always serialize")
}

pub fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Option<T> {
    bincode::deserialize(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbct_sim::colony::actions::{Container, InvOp};
    use sbct_sim::colony::geom::v2;

    #[test]
    fn roundtrip() {
        let bytes = encode(&ClientMsg::Hello {
            version: PROTOCOL_VERSION,
            name: "a".into(),
            key: 77,
        });
        match decode::<ClientMsg>(&bytes) {
            Some(ClientMsg::Hello { version, name, key }) => {
                assert_eq!(version, PROTOCOL_VERSION);
                assert_eq!(name, "a");
                assert_eq!(key, 77);
            }
            other => panic!("{other:?}"),
        }
        assert!(decode::<HostMsg>(&[255, 255, 255, 255]).is_none());
    }

    #[test]
    fn actions_and_colony_survive_the_wire() {
        let actions = [
            Action::Dig { x: -3, y: 900 },
            Action::Fire {
                from: v2(1.5, 2.5),
                dir: v2(0.0, -1.0),
            },
            Action::Inv(InvOp::Move {
                from: (Container::DomeChest(4, 1), 3),
                to: (Container::Me, 12),
                count: 40,
            }),
        ];
        for a in actions {
            let bytes = encode(&ClientMsg::Action(a.clone()));
            let Some(ClientMsg::Action(back)) = decode::<ClientMsg>(&bytes) else {
                panic!("action lost")
            };
            assert_eq!(back, a);
        }

        let mut planet = sbct_sim::planetgen::generate_sized(1024, 512, 5);
        let mut colony = Colony::found(&mut planet);
        colony.join(9, "Ada".into());
        let msg = HostMsg::Welcome {
            key: 9,
            width: 1024,
            height: 512,
            seed: 5,
            colony: Box::new(colony.clone()),
        };
        let bytes = encode(&msg);
        let Some(HostMsg::Welcome { colony: back, .. }) = decode::<HostMsg>(&bytes) else {
            panic!("welcome lost")
        };
        assert_eq!(back.ents, colony.ents);
        assert_eq!(back.players[&9].name, "Ada");
        assert!(bytes.len() < 200_000, "welcome is {} bytes", bytes.len());
    }
}
