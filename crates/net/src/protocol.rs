//! Wire messages. The host owns the simulation; clients send their own
//! player state and edit requests, and receive world chunks and other players.

use serde::{Deserialize, Serialize};

use crate::PeerId;

/// Bump whenever a message layout changes so mismatched builds refuse to connect.
pub const PROTOCOL_VERSION: u32 = 3;

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
}
