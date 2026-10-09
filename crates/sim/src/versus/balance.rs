//! Headless match running for balance work. Run the ignored tests with
//! `cargo test --release -p sbct_sim balance -- --ignored --nocapture`;
//! `VS_N` sets how many matches (or seeds per pairing).

use super::design::{Design, Upgrade};
use super::{Arena, FightEvent, ROUND_TIME};

#[derive(Clone, Debug)]
pub struct Outcome {
    /// Team index that won, if any.
    pub winner: Option<usize>,
    pub duration: f32,
    pub hits: u32,
    /// Health fractions at the end, per team.
    pub health: Vec<f32>,
    pub biome: String,
}

/// Fights `a` against `b` once in the arena `seed` picks.
pub fn run_match(seed: u64, a: &Design, b: &Design) -> Outcome {
    let mut arena = Arena::new(seed, &[("A".into(), a.clone()), ("B".into(), b.clone())]);
    let mut hits = 0;
    let mut t = 0;
    while !arena.over() && t < (ROUND_TIME * 60.0) as usize + 120 {
        arena.step();
        t += 1;
        for e in arena.take_events() {
            if matches!(e, FightEvent::Hit { .. }) {
                hits += 1;
            }
        }
    }
    let r = arena.result.clone();
    Outcome {
        winner: r.as_ref().and_then(|r| r.winner.map(|w| w as usize)),
        duration: r.as_ref().map_or(ROUND_TIME, |r| r.duration),
        hits,
        health: (0..2).map(|t| arena.team_health(t)).collect(),
        biome: format!("{:?}", arena.biome.kind),
    }
}

/// Hand-built builds covering the styles the game should support.
pub fn archetypes() -> Vec<(&'static str, Design)> {
    use Upgrade::*;
    let make = |name: &'static str, picks: &[(Upgrade, u8)]| {
        let mut d = Design::new(name.len() as u64 * 7919);
        for u in Upgrade::ALL {
            d.set(u, 0);
        }
        for &(u, l) in picks {
            d.set(u, l);
        }
        d.name = name.to_string();
        (name, d)
    };
    vec![
        make(
            "Brute",
            &[
                (Legs, 2),
                (Eyes, 2),
                (Teeth, 6),
                (Size, 4),
                (Vitality, 1),
                (Aggression, 2),
            ],
        ),
        make(
            "Tank",
            &[
                (Legs, 2),
                (Eyes, 2),
                (Teeth, 2),
                (Armor, 3),
                (Spines, 2),
                (Regeneration, 2),
                (Aggression, 2),
                (Caution, 1),
            ],
        ),
        make(
            "Pack",
            &[
                (Legs, 2),
                (Eyes, 2),
                (Teeth, 5),
                (Individuals, 2),
                (PackMind, 1),
                (Aggression, 3),
            ],
        ),
        make(
            "Spitters",
            &[
                (Legs, 2),
                (Eyes, 4),
                (Spit, 3),
                (Individuals, 2),
                (Caution, 2),
                (LongLegs, 1),
            ],
        ),
        make(
            "Flier",
            &[
                (Legs, 1),
                (Eyes, 3),
                (Teeth, 5),
                (Wings, 2),
                (Claws, 1),
                (Size, 1),
            ],
        ),
        make(
            "Grazer",
            &[
                (Legs, 2),
                (Eyes, 5),
                (Diet, 1),
                (Camouflage, 2),
                (Metabolism, 2),
                (Nose, 1),
                (Caution, 3),
                (LongLegs, 1),
            ],
        ),
        make(
            "Burrower",
            &[
                (Legs, 2),
                (Eyes, 1),
                (Burrow, 2),
                (Feelers, 1),
                (Tentacles, 2),
                (Venom, 1),
                (Teeth, 4),
            ],
        ),
        make(
            "Brain",
            &[
                (Legs, 1),
                (Eyes, 2),
                (Arms, 2),
                (Claws, 2),
                (Intelligence, 3),
                (Teeth, 2),
                (Size, 1),
                (Vitality, 1),
                (Aggression, 2),
            ],
        ),
        make(
            "Serpent",
            &[
                (Legs, 0),
                (Eyes, 2),
                (Teeth, 5),
                (Venom, 2),
                (Size, 2),
                (Camouflage, 1),
                (Tail, 1),
            ],
        ),
        make(
            "Swarm",
            &[(Legs, 2), (Eyes, 2), (Teeth, 1), (Individuals, 3), (PackMind, 1)],
        ),
    ]
}

fn workers() -> usize {
    std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .clamp(1, 32)
}

/// Runs `jobs` across threads, keeping their order.
/// A unit of work for [`run_all`].
pub type Job<T> = Box<dyn FnOnce() -> T + Send>;

pub fn run_all<T: Send>(jobs: Vec<Job<T>>) -> Vec<T> {
    let n = jobs.len();
    let mut slots: Vec<Option<T>> = (0..n).map(|_| None).collect();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let jobs: Vec<std::sync::Mutex<Option<Job<T>>>> =
        jobs.into_iter().map(|j| std::sync::Mutex::new(Some(j))).collect();
    let results = std::sync::Mutex::new(Vec::<(usize, T)>::new());
    std::thread::scope(|s| {
        for _ in 0..workers().min(n.max(1)) {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if i >= n {
                        break;
                    }
                    let job = jobs[i].lock().unwrap().take().unwrap();
                    let r = job();
                    results.lock().unwrap().push((i, r));
                }
            });
        }
    });
    for (i, r) in results.into_inner().unwrap() {
        slots[i] = Some(r);
    }
    slots.into_iter().map(|s| s.unwrap()).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::rng::Rng;
    use crate::versus::design::START_POINTS;

    fn env_n(default: usize) -> usize {
        std::env::var("VS_N")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(default)
    }

    #[test]
    fn archetypes_fit_the_budget() {
        let mut bad = Vec::new();
        for (name, d) in archetypes() {
            if d.cost() > START_POINTS {
                bad.push(format!("{name} costs {}", d.cost()));
            }
            for u in Upgrade::ALL {
                if u.requires(&d).is_some() && d.level(u) > 0 {
                    bad.push(format!("{name}: {u:?} {}", u.requires(&d).unwrap()));
                }
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    /// One archetype pairing, narrated: `VS_A=Brute VS_B=Grazer VS_SEED=3`.
    #[test]
    #[ignore]
    fn trace_pair() {
        let arch = archetypes();
        let pick = |var: &str, default: &str| {
            let name = std::env::var(var).unwrap_or_else(|_| default.to_string());
            arch.iter()
                .find(|(n, _)| *n == name)
                .map(|(_, d)| d.clone())
                .expect("archetype")
        };
        let a = pick("VS_A", "Brute");
        let b = pick("VS_B", "Grazer");
        let seed: u64 = std::env::var("VS_SEED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let mut arena = Arena::new(seed, &[("A".into(), a), ("B".into(), b)]);
        println!(
            "{:?} seed {} plants {}",
            arena.biome.kind,
            arena.seed,
            arena.plants.len()
        );
        for t in 0..(ROUND_TIME * 60.0) as usize + 120 {
            arena.step();
            for e in arena.take_events() {
                if !matches!(e, FightEvent::Miss { .. }) {
                    println!("{:6.2} {e:?}", t as f32 / 60.0);
                }
            }
            if t % 120 == 0 {
                for f in &arena.fighters {
                    println!(
                        "  t{:3} id{} ({:4.0},{:4.0}) hp {:5.1} en {:.2} st {:.1} {:?} '{}' tgt {:?} seen {} fly {} ground {} stuck {:.0}",
                        t / 60,
                        f.c.id,
                        f.c.pos.x,
                        f.c.pos.y,
                        f.c.health,
                        f.c.energy,
                        f.stamina,
                        f.brain.tactic,
                        f.brain.label,
                        f.brain.target,
                        f.brain
                            .last_seen
                            .map_or("-".to_string(), |(p, _)| format!("({:.0},{:.0})", p.x, p.y)),
                        f.c.flying,
                        f.c.on_ground,
                        f.c.brain.stuck
                    );
                }
            }
            if arena.over() {
                println!("over at {:.1}: {:?}", arena.clock, arena.result);
                break;
            }
        }
    }

    /// Every archetype against every other over several arenas.

    #[test]
    #[ignore]
    fn balance_archetypes() {
        let seeds = env_n(6) as u64;
        let arch = archetypes();
        let n = arch.len();
        let mut jobs: Vec<Job<(usize, usize, Outcome)>> = Vec::new();
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }
                for s in 1..=seeds {
                    let a = arch[i].1.clone();
                    let b = arch[j].1.clone();
                    jobs.push(Box::new(move || (i, j, run_match(s * 31 + i as u64, &a, &b))));
                }
            }
        }
        let results = run_all(jobs);
        let mut wins = vec![vec![0.0f32; n]; n];
        let mut played = vec![vec![0u32; n]; n];
        let mut lengths = vec![0.0f32; n];
        let mut draws = 0;
        for (i, j, o) in &results {
            played[*i][*j] += 1;
            match o.winner {
                Some(0) => wins[*i][*j] += 1.0,
                Some(1) => wins[*j][*i] += 1.0,
                _ => {
                    wins[*i][*j] += 0.5;
                    wins[*j][*i] += 0.5;
                    draws += 1;
                }
            }
            lengths[*i] += o.duration;
        }
        println!(
            "\n{:>9} |{}",
            "",
            arch.iter()
                .map(|(n, _)| format!("{:>7}", &n[..n.len().min(7)]))
                .collect::<String>()
        );
        let mut totals: Vec<(f32, &str)> = Vec::new();
        for i in 0..n {
            let mut row = String::new();
            let mut total = 0.0;
            let mut games = 0;
            for j in 0..n {
                if i == j {
                    row.push_str("      -");
                    continue;
                }
                let g = played[i][j] + played[j][i];
                let w = wins[i][j];
                total += w;
                games += g;
                row.push_str(&format!("{:>6.0}%", 100.0 * w / g.max(1) as f32));
            }
            let rate = 100.0 * total / games.max(1) as f32;
            println!(
                "{:>9} |{row}  = {rate:4.0}%  avg {:.0}s",
                arch[i].0,
                lengths[i] / played[i].iter().sum::<u32>().max(1) as f32
            );
            totals.push((rate, arch[i].0));
        }
        totals.sort_by(|a, b| b.0.total_cmp(&a.0));
        println!("draws {draws} of {}", results.len());
        println!(
            "spread {:.0}% ({}) to {:.0}% ({})",
            totals[0].0,
            totals[0].1,
            totals[n - 1].0,
            totals[n - 1].1
        );
    }

    /// Random designs against each other; how much each upgrade shifts the
    /// odds when a design has it.
    #[test]
    #[ignore]
    fn balance_upgrades() {
        let matches = env_n(300);
        let mut jobs: Vec<Job<(Design, Design, Outcome)>> = Vec::new();
        for m in 0..matches as u64 {
            jobs.push(Box::new(move || {
                let mut rng = Rng::new(m * 977 + 13);
                let a = Design::random(&mut rng, START_POINTS, None);
                let b = Design::random(&mut rng, START_POINTS, None);
                let o = run_match(m + 1, &a, &b);
                (a, b, o)
            }));
        }
        let results = run_all(jobs);
        // Per upgrade: (wins, games) with it and without it.
        let mut with: BTreeMap<Upgrade, (f32, u32)> = BTreeMap::new();
        let mut without: BTreeMap<Upgrade, (f32, u32)> = BTreeMap::new();
        let mut by_level: BTreeMap<(Upgrade, u8), (f32, u32)> = BTreeMap::new();
        let mut draws = 0;
        let mut total_len = 0.0;
        let mut by_biome: BTreeMap<String, (u32, u32)> = BTreeMap::new();
        let mut by_count: BTreeMap<usize, (f32, u32)> = BTreeMap::new();
        for (a, b, o) in &results {
            total_len += o.duration;
            let e = by_biome.entry(o.biome.clone()).or_default();
            e.1 += 1;
            if o.winner.is_none() {
                draws += 1;
                e.0 += 1;
            }
            for (ti, d) in [(0usize, a), (1usize, b)] {
                let score = match o.winner {
                    Some(w) if w == ti => 1.0,
                    Some(_) => 0.0,
                    None => 0.5,
                };
                let c = by_count.entry(d.count()).or_default();
                c.0 += score;
                c.1 += 1;
                for u in Upgrade::ALL {
                    let l = d.level(u);
                    let slot = if l > 0 { with.entry(u) } else { without.entry(u) }.or_default();
                    slot.0 += score;
                    slot.1 += 1;
                    let bl = by_level.entry((u, l)).or_default();
                    bl.0 += score;
                    bl.1 += 1;
                }
            }
        }
        let mut rows: Vec<(f32, String)> = Vec::new();
        for u in Upgrade::ALL {
            let (ww, wn) = with.get(&u).copied().unwrap_or((0.0, 0));
            let (lw, ln) = without.get(&u).copied().unwrap_or((0.0, 0));
            if wn == 0 || ln == 0 {
                continue;
            }
            let a = ww / wn as f32;
            let b = lw / ln as f32;
            let levels: String = (0..=u.max())
                .filter_map(|l| {
                    by_level
                        .get(&(u, l))
                        .map(|(w, n)| format!(" L{l}:{:.0}%({n})", 100.0 * w / *n as f32))
                })
                .collect();
            rows.push((
                a - b,
                format!(
                    "{:+5.1}%  {:<16} with {:3.0}% ({wn:3})  without {:3.0}% ({ln:3}) |{levels}",
                    100.0 * (a - b),
                    u.label(),
                    100.0 * a,
                    100.0 * b
                ),
            ));
        }
        rows.sort_by(|x, y| y.0.total_cmp(&x.0));
        println!();
        for (_, r) in rows {
            println!("{r}");
        }
        println!(
            "\n{} matches, {draws} draws, avg {:.0}s",
            results.len(),
            total_len / results.len().max(1) as f32
        );
        for (k, (w, n)) in by_count {
            println!("  {k} individuals: {:.0}% over {n}", 100.0 * w / n as f32);
        }
        for (b, (d, n)) in by_biome {
            println!("  {b}: {n} fights, {d} draws");
        }
    }
}
