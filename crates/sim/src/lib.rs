//! Engine-agnostic falling-sand simulation. No rendering or networking here,
//! so it can run headless (tests, dedicated server) as well as in the client.

pub mod material;
pub mod rng;
pub mod world;
pub mod worldgen;

pub use material::{Kind, Material};
pub use world::{CHUNK_SIZE, Cell, World};
