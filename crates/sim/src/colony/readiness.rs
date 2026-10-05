//! Colony Readiness: the four targets that decide when the colonists can
//! come, and the ship that brings them.

use serde::{Deserialize, Serialize};

use super::domes::Fixture;
use super::geom::{V2, v2};
use super::items::DomeKind;
use super::{Colony, Fx, PlayerKey};

/// One reading of the four targets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Readiness {
    /// Apartment Domes with power and water.
    pub apartments: f32,
    /// Atmospheric oxygen, percent.
    pub oxygen: f32,
    /// Food produced over the last full day (or today so far, if more).
    pub food: f32,
    /// Fresh-water surplus across all pipe networks, litres per day.
    pub water: f32,
}

pub const TARGETS: Readiness = Readiness {
    apartments: 6.0,
    oxygen: 19.0,
    food: 240.0,
    water: 600.0,
};

/// Seconds from calling the ship to touchdown.
pub const LANDING_SECS: f32 = 30.0;
pub const COLONISTS_PER_APARTMENT: u32 = 20;

impl Readiness {
    pub const LABELS: [&'static str; 4] = [
        "Apartment Domes",
        "Oxygen",
        "Food per day",
        "Water surplus per day",
    ];

    pub fn values(&self) -> [f32; 4] {
        [self.apartments, self.oxygen, self.food, self.water]
    }

    /// Which targets are met.
    pub fn met(&self) -> [bool; 4] {
        let (v, t) = (self.values(), TARGETS.values());
        [v[0] >= t[0], v[1] >= t[1], v[2] >= t[2], v[3] >= t[3]]
    }

    pub fn all_met(&self) -> bool {
        self.met().iter().all(|m| *m)
    }

    /// Overall progress, 0–1: the average of each target's share.
    pub fn progress(&self) -> f32 {
        let (v, t) = (self.values(), TARGETS.values());
        (0..4).map(|i| (v[i] / t[i]).clamp(0.0, 1.0)).sum::<f32>() / 4.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Ship {
    /// Waiting in orbit for the call.
    #[default]
    Away,
    /// On its way down; seconds since the call.
    Landing(f32),
    /// Down, since this day. The colony is established.
    Landed(u32),
}

impl Colony {
    /// Measures the four targets now.
    pub fn measure(&self) -> Readiness {
        Readiness {
            apartments: self
                .domes()
                .filter(|(_, d)| d.kind == DomeKind::Apartment && d.powered && d.watered)
                .count() as f32,
            oxygen: self.atmosphere.o2,
            food: self.stats.food_yesterday.max(self.stats.food_today),
            water: self.water_surplus(),
        }
    }

    /// Where the ship sets down: open ground to the left of the starter dome.
    pub fn landing_site(&self) -> V2 {
        let home = self.ents.get(&self.home).map_or(V2::ZERO, |e| e.pos);
        let x = home.x - DomeKind::Starter.size().0 as f32 / 2.0 - 130.0;
        v2(x, self.profile.surface_at(x as i32) as f32)
    }

    pub(super) fn call_ship(&mut self, key: PlayerKey) -> Result<(), &'static str> {
        let p = &self.players[&key];
        let at_terminal = p
            .in_dome
            .and_then(|d| self.dome(d))
            .is_some_and(|(_, d)| d.kind.fixtures().iter().any(|f| f.fixture == Fixture::Terminal));
        if !at_terminal {
            return Err("Use the Comm. Terminal");
        }
        if self.ship != Ship::Away {
            return Err("The ship has already been called");
        }
        if !self.readiness.all_met() {
            return Err("The colony isn't ready: every target must be met");
        }
        self.ship = Ship::Landing(0.0);
        let name = p.name.clone();
        self.fx(Fx::ShipCalled, self.landing_site());
        self.toast(
            None,
            format!("{name} called the ship. It is on its way down!"),
            true,
        );
        Ok(())
    }

    /// Re-measures readiness, announces newly met targets and flies the ship.
    pub(super) fn step_readiness(&mut self, dt: f32) {
        let before = self.readiness.met();
        self.readiness = self.measure();
        let now = self.readiness.met();
        if self.ship == Ship::Away {
            for i in 0..4 {
                if now[i] && !before[i] && !self.announced[i] {
                    self.announced[i] = true;
                    self.fx(Fx::TargetMet, V2::ZERO);
                    self.toast(
                        None,
                        format!("Readiness target met: {}", Readiness::LABELS[i]),
                        true,
                    );
                }
            }
            if now.iter().all(|m| *m) && !before.iter().all(|m| *m) {
                self.fx(Fx::AllMet, V2::ZERO);
                self.toast(
                    None,
                    "The colony is ready. Call the ship from the Comm. Terminal!",
                    true,
                );
            }
        }
        if let Ship::Landing(t) = &mut self.ship {
            *t += dt;
            if *t >= LANDING_SECS {
                self.ship = Ship::Landed(self.clock.day);
                let colonists = self.readiness.apartments as u32 * COLONISTS_PER_APARTMENT;
                self.fx(Fx::ShipLand, self.landing_site());
                self.toast(
                    None,
                    format!(
                        "Touchdown on day {}. {colonists} colonists step out onto a living world.",
                        self.clock.day
                    ),
                    true,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colony::EntKind;
    use crate::colony::actions::Action;
    use crate::colony::domes::Dome;
    use crate::colony::testing::*;

    #[test]
    fn targets_measure_progress() {
        let r = Readiness {
            apartments: 3.0,
            oxygen: 19.0,
            food: 480.0,
            water: 0.0,
        };
        assert_eq!(r.met(), [false, true, true, false]);
        assert!(!r.all_met());
        assert!((r.progress() - (0.5 + 1.0 + 1.0 + 0.0) / 4.0).abs() < 1e-6);
        assert!(TARGETS.all_met());
    }

    #[test]
    fn the_ship_comes_only_when_everything_is_ready() {
        let (mut world, mut colony, key) = colony();
        // At the terminal.
        let home = colony.home;
        colony.players.get_mut(&key).unwrap().in_dome = Some(home);
        assert_eq!(
            colony.apply(&mut world, key, Action::CallShip),
            Err("The colony isn't ready: every target must be met")
        );
        run(&mut colony, &mut world, 1.5);
        assert_eq!(colony.readiness.oxygen, colony.atmosphere.o2);
        assert_eq!(colony.readiness.apartments, 0.0);

        // Build it all (by decree).
        let pos = colony.ents[&home].pos;
        for i in 0..6 {
            let mut d = Dome::new(DomeKind::Apartment, format!("Apartment {i}"));
            d.powered = true;
            d.watered = true;
            d.reserve = 1.0e6;
            colony.spawn(v2(pos.x + 400.0 + i as f32 * 200.0, pos.y), EntKind::Dome(d));
        }
        colony.atmosphere.o2 = 19.5;
        colony.stats.food_yesterday = 300.0;
        assert_eq!(
            colony.measure().met(),
            [true, true, true, false],
            "only water is short"
        );
        // Decree the rest: a readiness reading with everything met.
        colony.readiness = TARGETS;
        colony.apply(&mut world, key, Action::CallShip).unwrap();
        assert_eq!(colony.ship, Ship::Landing(0.0));
        assert_eq!(
            colony.apply(&mut world, key, Action::CallShip),
            Err("The ship has already been called")
        );
        run(&mut colony, &mut world, LANDING_SECS + 1.0);
        assert!(matches!(colony.ship, Ship::Landed(1)));
        // The world keeps running afterwards.
        run(&mut colony, &mut world, 2.0);
        assert!(colony.dome_at(colony.landing_site()).is_none());
    }

    #[test]
    fn meeting_a_target_is_announced_once() {
        let (mut world, mut colony, _) = colony();
        colony.atmosphere.o2 = 19.2;
        colony.take_events();
        run(&mut colony, &mut world, 1.5);
        let count = |c: &mut Colony| {
            c.take_events()
                .iter()
                .filter(|e| {
                    matches!(
                        e,
                        crate::colony::Event::Fx {
                            fx: Fx::TargetMet,
                            ..
                        }
                    )
                })
                .count()
        };
        assert_eq!(count(&mut colony), 1);
        colony.atmosphere.o2 = 18.0;
        run(&mut colony, &mut world, 1.5);
        colony.atmosphere.o2 = 19.2;
        run(&mut colony, &mut world, 1.5);
        assert_eq!(count(&mut colony), 0, "not announced again");
    }
}
