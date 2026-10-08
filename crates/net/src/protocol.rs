//! Wire messages. The host owns the simulation; clients send their own
//! player state and edit requests, and receive world chunks and other players.

use sbct_sim::versus::design::Design;
use sbct_sim::versus::{FightEvent, RoundResult, Snapshot};
use serde::{Deserialize, Serialize};

use crate::PeerId;

/// Bump whenever a message layout changes so mismatched builds refuse to connect.
pub const PROTOCOL_VERSION: u32 = 2;

/// Which game a session is running. Hosts and joiners must agree.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameMode {
    Sandbox,
    Versus,
}

impl GameMode {
    pub fn tag(self) -> &'static str {
        match self {
            GameMode::Sandbox => "sandbox",
            GameMode::Versus => "versus",
        }
    }

    pub fn from_tag(s: &str) -> GameMode {
        if s == "versus" {
            GameMode::Versus
        } else {
            GameMode::Sandbox
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerState {
    /// Position in cell coordinates (y grows downward), centre of the feet.
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
}

/// A request to paint or dig a disc of cells. `material == 0` digs.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Edit {
    pub x: i32,
    pub y: i32,
    pub radius: u8,
    pub material: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMsg {
    Hello { version: u32, name: String },
    State(PlayerState),
    Edit(Edit),
}

// ---- Alien Versus ----------------------------------------------------------

/// A player as the host lists them.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RosterEntry {
    pub id: PeerId,
    pub name: String,
    pub ready: bool,
    pub wins: u8,
    /// Joined mid-match; watches until the next match.
    pub spectator: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum VersusClientMsg {
    Hello {
        version: u32,
        name: String,
    },
    /// Ready with this design.
    Ready(Design),
    Unready,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum VersusHostMsg {
    Welcome {
        your_id: PeerId,
        round: u32,
        points: u32,
    },
    Reject {
        reason: String,
    },
    Roster(Vec<RosterEntry>),
    /// A round begins: everyone builds the same arena from `seed` and the
    /// designs, in team order.
    RoundStart {
        round: u32,
        seed: u64,
        teams: Vec<(PeerId, String, Design)>,
    },
    /// Run-length encoded chunk of the arena, see `World::encode_chunk`.
    Chunk {
        cx: u16,
        cy: u16,
        data: Vec<u8>,
    },
    Snapshot(Snapshot),
    Events(Vec<FightEvent>),
    RoundOver {
        result: RoundResult,
        /// Wins per team, in the round's team order.
        wins: Vec<u8>,
        /// Points each player has to spend next.
        points: u32,
        /// The player who won the match, if it's over.
        match_winner: Option<PeerId>,
    },
    /// Wins and designs are wiped; everyone starts again with `points`.
    NewMatch {
        points: u32,
    },
    PlayerLeft {
        id: PeerId,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum HostMsg {
    Welcome {
        your_id: PeerId,
        width: u32,
        height: u32,
        seed: u64,
        spawn: (f32, f32),
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
    PlayerJoined {
        id: PeerId,
        name: String,
    },
    PlayerLeft {
        id: PeerId,
    },
    Players(Vec<(PeerId, PlayerState)>),
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

    #[test]
    fn roundtrip() {
        let bytes = encode(&ClientMsg::Hello {
            version: PROTOCOL_VERSION,
            name: "a".into(),
        });
        match decode::<ClientMsg>(&bytes) {
            Some(ClientMsg::Hello { version, name }) => {
                assert_eq!(version, PROTOCOL_VERSION);
                assert_eq!(name, "a");
            }
            other => panic!("{other:?}"),
        }
        assert!(decode::<HostMsg>(&[255, 255, 255, 255]).is_none());
    }

    #[test]
    fn versus_messages_roundtrip() {
        let d = Design::new(7);
        let bytes = encode(&VersusClientMsg::Ready(d.clone()));
        match decode::<VersusClientMsg>(&bytes) {
            Some(VersusClientMsg::Ready(got)) => assert_eq!(got, d),
            other => panic!("{other:?}"),
        }
        let start = VersusHostMsg::RoundStart {
            round: 1,
            seed: 9,
            teams: vec![(1, "a".into(), d)],
        };
        assert!(decode::<VersusHostMsg>(&encode(&start)).is_some());
    }
}
