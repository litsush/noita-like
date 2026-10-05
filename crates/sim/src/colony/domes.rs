//! Domes: their shape in the cell world, what stands inside them, and the
//! state they carry.
//!
//! Geometry: a dome of size W×H is the upper half of an ellipse centred on
//! the middle of its floor with radii (W/2, H). The outer 2 cells are the
//! glass shell, a membrane that stops cells and creatures but not players.
//! Below it lies a plated foundation.

use serde::{Deserialize, Serialize};

use super::geom::{Rect, V2, v2};
use super::inventory::Inventory;
use super::items::DomeKind;
use super::plants::{Bed, SpeciesId};
use crate::material::Material;
use crate::world::World;

/// Domes must keep this many cells between their footprints.
pub const DOME_GAP: i32 = 8;
pub const SHELL: i32 = 2;
pub const FOUNDATION: i32 = 4;
pub const CHEST_SLOTS: usize = 40;

/// Something a player can walk up to and use inside a dome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Fixture {
    /// A bunk of two beds (index of the bunk).
    Bunk(u8),
    /// A chest (index into the dome's chests).
    Chest(u8),
    SuitRack,
    Synthesizer,
    Terminal,
    ModBay,
    /// A planter (index into the dome's planters).
    Planter(u8),
    Cooker,
    CompostVat,
    AssemblyRing,
    FishPool,
    Stall(u8),
    Condenser,
    Berths,
}

/// Where a fixture stands: `dx` from the middle of the floor, `dy` above it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    pub fixture: Fixture,
    pub dx: i32,
    pub dy: i32,
}

const fn at(fixture: Fixture, dx: i32) -> Placed {
    Placed { fixture, dx, dy: 0 }
}

const fn raised(fixture: Fixture, dx: i32, dy: i32) -> Placed {
    Placed { fixture, dx, dy }
}

use Fixture::*;

static STARTER: [Placed; 10] = [
    at(SuitRack, -92),
    at(Bunk(0), -72),
    at(Bunk(1), -52),
    at(Bunk(2), -32),
    at(Bunk(3), -12),
    at(Chest(0), 8),
    at(Chest(1), 26),
    at(Synthesizer, 48),
    at(Terminal, 72),
    at(ModBay, 94),
];
static GREEN: [Placed; 5] = [
    at(Planter(0), -36),
    at(Planter(1), -14),
    at(Planter(2), 8),
    at(Planter(3), 30),
    raised(Chest(0), 44, 0),
];
static STORAGE: [Placed; 6] = [
    at(Chest(0), -20),
    at(Chest(1), 0),
    at(Chest(2), 20),
    raised(Chest(3), -18, 15),
    raised(Chest(4), 0, 15),
    raised(Chest(5), 18, 15),
];
static BEDROOM: [Placed; 2] = [at(Bunk(0), -14), at(Bunk(1), 14)];
static KITCHEN: [Placed; 3] = [at(Chest(0), -24), at(Cooker, 0), at(Chest(1), 24)];
static DARK: [Placed; 7] = [
    at(Planter(0), -40),
    at(Planter(1), -22),
    at(Planter(2), -4),
    at(Planter(3), 14),
    at(CompostVat, 34),
    raised(Chest(0), 30, 22),
    at(Chest(1), 46),
];
static DOMEDOME: [Placed; 2] = [at(AssemblyRing, -8), at(Chest(0), 36)];
static AQUA: [Placed; 4] = [
    at(Planter(0), -38),
    at(FishPool, -6),
    at(Planter(1), 26),
    at(Chest(0), 44),
];
static APARTMENT: [Placed; 1] = [at(Berths, 0)];
static BARN: [Placed; 6] = [
    at(Stall(0), -42),
    at(Stall(1), -26),
    at(Stall(2), -10),
    at(Stall(3), 6),
    at(Chest(0), 26),
    at(Chest(1), 44),
];
static WATERGEN: [Placed; 1] = [at(Condenser, 0)];

impl DomeKind {
    pub fn fixtures(self) -> &'static [Placed] {
        match self {
            DomeKind::Starter => &STARTER,
            DomeKind::Green => &GREEN,
            DomeKind::Storage => &STORAGE,
            DomeKind::Bedroom => &BEDROOM,
            DomeKind::Kitchen => &KITCHEN,
            DomeKind::Dark => &DARK,
            DomeKind::DomeDome => &DOMEDOME,
            DomeKind::Aqua => &AQUA,
            DomeKind::Apartment => &APARTMENT,
            DomeKind::Barn => &BARN,
            DomeKind::WaterGen => &WATERGEN,
        }
    }

    pub fn planters(self) -> usize {
        match self {
            DomeKind::Green | DomeKind::Dark => 4,
            DomeKind::Aqua => 2,
            _ => 0,
        }
    }

    pub fn chests(self) -> usize {
        match self {
            DomeKind::Starter | DomeKind::Kitchen | DomeKind::Dark | DomeKind::Barn => 2,
            DomeKind::Storage => 6,
            DomeKind::Green | DomeKind::DomeDome | DomeKind::Aqua => 1,
            _ => 0,
        }
    }

    /// What each chest is for, shown as its title.
    pub fn chest_label(self, index: usize) -> &'static str {
        match (self, index) {
            (DomeKind::Kitchen, 0) => "Crops to cook",
            (DomeKind::Kitchen, 1) => "Meals",
            (DomeKind::Dark, 0) => "Plant matter to compost",
            (DomeKind::Dark, 1) => "Harvest and fertilizer",
            (DomeKind::Barn, 0) => "Feed trough",
            (DomeKind::Barn, 1) => "Eggs, milk and fertilizer",
            (DomeKind::Green | DomeKind::Aqua, _) => "Harvest",
            (DomeKind::DomeDome, _) => "Finished kits",
            _ => "Chest",
        }
    }

    /// Beds for players to sleep in and set their spawn.
    pub fn beds(self) -> u32 {
        match self {
            DomeKind::Starter => 8,
            DomeKind::Bedroom => 4,
            _ => 0,
        }
    }

    /// Workers the dome can house.
    pub fn worker_beds(self) -> u32 {
        match self {
            DomeKind::Bedroom => 4,
            DomeKind::Apartment => 6,
            _ => 0,
        }
    }

    /// Whether workers can be assigned here to boost output.
    pub fn employs(self) -> bool {
        matches!(
            self,
            DomeKind::Green
                | DomeKind::Dark
                | DomeKind::Aqua
                | DomeKind::Barn
                | DomeKind::Kitchen
                | DomeKind::WaterGen
        )
    }

    /// Charge used per day while running.
    pub fn power_per_day(self) -> f32 {
        match self {
            DomeKind::Kitchen => 20.0,
            DomeKind::WaterGen => 40.0,
            DomeKind::DomeDome => 60.0,
            DomeKind::Apartment => 30.0,
            _ => 0.0,
        }
    }

    /// Litres of water used per day.
    pub fn water_per_day(self) -> f32 {
        match self {
            DomeKind::Green | DomeKind::Dark => 30.0,
            DomeKind::Aqua => 60.0,
            DomeKind::Barn | DomeKind::Kitchen => 20.0,
            DomeKind::Bedroom => 10.0,
            DomeKind::Apartment => 40.0,
            _ => 0.0,
        }
    }

    /// The bed type of this dome's planters.
    pub fn bed(self) -> Bed {
        match self {
            DomeKind::Dark => Bed::Dark,
            DomeKind::Aqua => Bed::Water,
            _ => Bed::Light,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Planter {
    pub species: Option<SpeciesId>,
    /// 0 = just sown, 1 = mature.
    pub growth: f32,
    /// 0 = dry (growth stops), 1 = soaked.
    pub moisture: f32,
    /// Seconds of faster growth left.
    pub fertilizer: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnimalKind {
    Cluckbug,
    MilkGrub,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Animal {
    pub kind: AnimalKind,
    /// Progress toward the next egg or bottle of milk, 0–1.
    pub progress: f32,
    pub fed: bool,
}

/// Why a dome isn't working, as bit flags.
pub mod issue {
    pub const NO_POWER: u8 = 1;
    pub const NO_WATER: u8 = 2;
    pub const NO_INPUT: u8 = 4;
    pub const OUTPUT_FULL: u8 = 8;
    pub const NO_TARGET: u8 = 16;
    pub const NOT_SUBMERGED: u8 = 32;

    pub fn describe(flags: u8) -> Vec<&'static str> {
        let mut out = Vec::new();
        for (bit, text) in [
            (
                NO_POWER,
                "No power: build a Charging Pylon within 48 cells and load battery flowers",
            ),
            (
                NO_WATER,
                "No water: water by hand or connect a pipe network with enough supply",
            ),
            (NO_INPUT, "Nothing to work on: fill its input chest"),
            (OUTPUT_FULL, "Output chest is full"),
            (NO_TARGET, "Choose what to build"),
            (NOT_SUBMERGED, "Must stand in water"),
        ] {
            if flags & bit != 0 {
                out.push(text);
            }
        }
        out
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Dome {
    pub kind: DomeKind,
    pub name: String,
    pub planters: Vec<Planter>,
    pub chests: Vec<Inventory>,
    pub animals: Vec<Animal>,
    /// Fish in the pool (Aqua Dome).
    pub fish: u32,
    /// Progress of the cooker, vat, assembly ring or fish breeding, 0–1.
    pub progress: f32,
    /// What a Dome Dome is building.
    pub target: Option<DomeKind>,
    /// Litres of water on hand. Pipes refill it; some domes can be topped
    /// up by hand.
    pub reserve: f32,
    /// Set by the production tick.
    pub powered: bool,
    pub watered: bool,
    pub issues: u8,
    /// Output multiplier from whoever placed it (mods).
    pub boost: f32,
}

impl Dome {
    pub fn new(kind: DomeKind, name: String) -> Dome {
        Dome {
            kind,
            name,
            planters: vec![Planter::default(); kind.planters()],
            chests: vec![Inventory::new(CHEST_SLOTS); kind.chests()],
            animals: Vec::new(),
            fish: 0,
            progress: 0.0,
            target: None,
            reserve: 0.0,
            powered: kind.power_per_day() == 0.0,
            watered: kind.water_per_day() == 0.0,
            issues: 0,
            boost: 1.0,
        }
    }

    /// The chest that production and workers put things into.
    pub fn output_chest(&mut self) -> Option<&mut Inventory> {
        self.chests.last_mut()
    }

    /// Most water the dome holds: half a day of its own use.
    pub fn reserve_cap(&self) -> f32 {
        self.kind.water_per_day() * 0.5
    }

    /// A digest of everything about the dome that players can see, so the
    /// host only resends it when something visible changed.
    pub fn display_key(&self) -> u64 {
        let mut k = (self.powered as u64) | ((self.watered as u64) << 1) | ((self.issues as u64) << 2);
        k = k.wrapping_mul(31).wrapping_add((self.progress * 20.0) as u64);
        k = k.wrapping_mul(31).wrapping_add(self.fish as u64);
        k = k
            .wrapping_mul(31)
            .wrapping_add((self.reserve / self.reserve_cap().max(1.0) * 8.0) as u64);
        for p in &self.planters {
            let species = p.species.map_or(0, |s| s as u64 + 1);
            let stage = (p.growth.min(1.0) * 8.0) as u64;
            let wet = (p.moisture * 4.0).ceil() as u64;
            k = k
                .wrapping_mul(131)
                .wrapping_add(species * 1000 + stage * 100 + wet * 10 + (p.fertilizer > 0.0) as u64);
        }
        for a in &self.animals {
            k = k
                .wrapping_mul(31)
                .wrapping_add((a.progress * 8.0) as u64 + a.fed as u64 * 16);
        }
        k
    }
}

/// Bounding box of a dome whose floor is centred at `pos` (the floor row is
/// the first row of the foundation).
pub fn footprint(kind: DomeKind, pos: V2) -> Rect {
    let (w, h) = kind.size();
    let (cx, fy) = pos.cell();
    Rect {
        x0: cx - w / 2,
        y0: fy - h,
        x1: cx + w / 2,
        y1: fy + FOUNDATION,
    }
}

/// 0 outside the dome, 1 in the shell, 2 in the interior, for a cell given
/// relative to the left edge and the top of the dome's box.
fn zone(kind: DomeKind, lx: i32, ly: i32) -> u8 {
    let (w, h) = kind.size();
    let (a, b) = (w as f32 / 2.0, h as f32);
    let (dx, dy) = (lx as f32 + 0.5 - a, ly as f32 + 0.5 - b);
    let d = |a: f32, b: f32| (dx / a).powi(2) + (dy / b).powi(2);
    if ly < 0 || ly >= h || d(a, b) > 1.0 {
        0
    } else if d(a - SHELL as f32, b - SHELL as f32) > 1.0 {
        1
    } else {
        2
    }
}

/// Whether a point is inside a dome (shell included), i.e. in breathable air.
pub fn contains(kind: DomeKind, pos: V2, p: V2) -> bool {
    let r = footprint(kind, pos);
    let (x, y) = p.cell();
    zone(kind, x - r.x0, y - r.y0) != 0
}

/// Where a fixture stands in the world (the middle of its base).
pub fn fixture_pos(pos: V2, f: &Placed) -> V2 {
    v2(pos.x.floor() + f.dx as f32, pos.y.floor() - f.dy as f32)
}

/// Why a dome can't go here, if it can't.
pub fn placement_error(world: &World, kind: DomeKind, pos: V2, others: &[Rect]) -> Option<&'static str> {
    let r = footprint(kind, pos);
    if r.x0 < 6 || r.y0 < 4 || r.x1 > world.width() as i32 - 6 || r.y1 > world.height() as i32 - 10 {
        return Some("Too close to the edge of the world");
    }
    if others.iter().any(|o| o.grown(DOME_GAP).overlaps(&r)) {
        return Some("Domes can't touch: leave a gap of 8 cells");
    }
    // It has to rest on something: most of the floor line must be ground.
    let (_, fy) = pos.cell();
    let supported = (r.x0..r.x1)
        .filter(|&x| (0..10).any(|d| world.material(x, fy + d).is_solid_for_player()))
        .count();
    if supported * 10 < (r.x1 - r.x0) as usize * 6 {
        return Some("Needs ground underneath");
    }
    for y in r.y0..r.y1 {
        for x in r.x0..r.x1 {
            if matches!(
                world.material(x, y),
                Material::Bedrock | Material::Glass | Material::Plating
            ) {
                return Some("Something indestructible is in the way");
            }
        }
    }
    None
}

/// Writes a dome into the cell world: clears the interior, builds the shell
/// and the foundation, and props up any overhang with dirt.
pub fn build_cells(world: &mut World, kind: DomeKind, pos: V2) {
    let r = footprint(kind, pos);
    let (_, fy) = pos.cell();
    for y in r.y0..fy {
        for x in r.x0..r.x1 {
            match zone(kind, x - r.x0, y - r.y0) {
                1 => world.set(x, y, Material::Glass),
                2 if world.material(x, y) != Material::Empty => world.set(x, y, Material::Empty),
                _ => {}
            }
        }
    }
    for x in r.x0..r.x1 {
        for y in fy..fy + FOUNDATION {
            world.set(x, y, Material::Plating);
        }
        // Fill any gap below so the dome doesn't hang in the air.
        for y in fy + FOUNDATION..fy + FOUNDATION + 40 {
            if world.material(x, y).is_solid_for_player() {
                break;
            }
            world.set(x, y, Material::Dirt);
        }
    }
}

/// Removes a dome's shell and foundation from the cell world.
pub fn clear_cells(world: &mut World, kind: DomeKind, pos: V2) {
    let r = footprint(kind, pos);
    for y in r.y0..r.y1 {
        for x in r.x0..r.x1 {
            if matches!(world.material(x, y), Material::Glass | Material::Plating) {
                world.set(x, y, Material::Empty);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ground() -> World {
        let mut w = World::new(512, 256, 1);
        for y in 150..256 {
            for x in 0..512 {
                w.set(x, y, Material::Stone);
            }
        }
        w
    }

    #[test]
    fn layouts_fit_inside_their_domes() {
        for kind in [DomeKind::Starter].into_iter().chain(DomeKind::KITS) {
            let (w, h) = kind.size();
            let pos = v2(300.0, 150.0);
            let mut chests = std::collections::HashSet::new();
            let mut planters = std::collections::HashSet::new();
            for f in kind.fixtures() {
                assert!(f.dx.abs() < w / 2 - 4, "{kind:?} {f:?} outside");
                // The spot 10 cells above the fixture's base is inside the dome.
                let p = fixture_pos(pos, f) + v2(0.0, -10.0);
                assert!(contains(kind, pos, p), "{kind:?} {f:?} pokes out");
                match f.fixture {
                    Fixture::Chest(i) => assert!(chests.insert(i) && (i as usize) < kind.chests()),
                    Fixture::Planter(i) => assert!(planters.insert(i) && (i as usize) < kind.planters()),
                    _ => {}
                }
            }
            assert_eq!(chests.len(), kind.chests(), "{kind:?} chests");
            assert_eq!(planters.len(), kind.planters(), "{kind:?} planters");
            assert!(h >= 44);
        }
        assert_eq!(
            DomeKind::Starter
                .fixtures()
                .iter()
                .filter(|f| matches!(f.fixture, Fixture::Bunk(_)))
                .count()
                * 2,
            8,
            "eight beds"
        );
    }

    #[test]
    fn building_makes_a_sealed_breathable_shell() {
        let mut w = ground();
        // Rubble where the dome will stand.
        for y in 120..150 {
            for x in 280..320 {
                w.set(x, y, Material::Dirt);
            }
        }
        let pos = v2(300.0, 150.0);
        let kind = DomeKind::Green;
        assert_eq!(placement_error(&w, kind, pos, &[]), None);
        build_cells(&mut w, kind, pos);
        let r = footprint(kind, pos);
        // Interior cleared, floor plated, top of the arc is glass.
        assert_eq!(w.material(300, 140), Material::Empty);
        assert_eq!(w.material(300, 150), Material::Plating);
        assert_eq!(w.material(300, r.y0), Material::Glass);
        assert!(contains(kind, pos, v2(300.5, 130.5)));
        assert!(
            !contains(kind, pos, v2(r.x0 as f32 + 1.5, r.y0 as f32 + 1.5)),
            "corner is outside"
        );
        // Sealed: every interior cell next to the outside is glass.
        for y in r.y0..150 {
            for x in r.x0..r.x1 {
                if zone(kind, x - r.x0, y - r.y0) == 2 {
                    for (dx, dy) in [(1, 0), (-1, 0), (0, -1)] {
                        assert_ne!(zone(kind, x + dx - r.x0, y + dy - r.y0), 0, "leak at {x},{y}");
                    }
                }
            }
        }
        // Players pass the shell, creatures don't.
        assert!(!Material::Glass.is_solid_for_player());
        clear_cells(&mut w, kind, pos);
        assert_eq!(w.material(300, 150), Material::Empty);
    }

    #[test]
    fn domes_cannot_touch_or_float() {
        let w = ground();
        let a = footprint(DomeKind::Green, v2(200.0, 150.0));
        // Right next to another dome.
        assert!(placement_error(&w, DomeKind::Storage, v2(200.0 + 52.0 + 36.0 + 4.0, 150.0), &[a]).is_some());
        assert!(placement_error(&w, DomeKind::Storage, v2(200.0 + 52.0 + 36.0 + 9.0, 150.0), &[a]).is_none());
        // In mid-air.
        assert_eq!(
            placement_error(&w, DomeKind::Storage, v2(300.0, 100.0), &[]),
            Some("Needs ground underneath")
        );
    }
}
