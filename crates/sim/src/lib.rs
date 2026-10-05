//! Engine-agnostic simulation: the falling-sand cell world, planet
//! generation, and the colony rules. No rendering or networking here, so it
//! runs headless (tests, dedicated server) as well as in the client.

pub mod colony;
pub mod color;
pub mod material;
pub mod planetgen;
pub mod rng;
pub mod world;

pub use material::{Kind, Material};
pub use world::{CHUNK_SIZE, Cell, SimEvent, World};
