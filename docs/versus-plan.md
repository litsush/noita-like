# Alien Versus: design and progress

Branch `alien-versus`, built on top of `alien-ecosystem`.

## What it is

A competitive mode. Each player designs a species from a point budget, the
species are dropped into a generated arena, and they fight on their own while
everyone watches. Rounds end on a wipe, the clock, or a stand-off; the winner
of three rounds wins the match. After every round everyone gets more points
to evolve: upgrades can be added, never removed.

## Layout

- `crates/sim/src/versus/design.rs`: the upgrade catalogue (32 upgrades with
  prices, levels or choices, hints and requirements), `Design` (the sheet
  sent over the wire), and `build_species`/`build_teams`, which turn a sheet
  into a drawable `Species` plus a `Loadout` of combat numbers. Every upgrade
  changes the body; a test asserts it.
- `parts.rs`: body parts as hitboxes (head, each body segment, each limb
  instance) with their own integrity, `nearest_part` for contact tests, and
  `effects` for what a damaged body can still do.
- `combat.rs`: perception (sight vs light and camouflage, echolocation,
  feelers), target choice, tactics (rush, flank, kite, harass, ambush, dive,
  undermine, hold, retreat, search) and per-tactic learning scaled by
  intelligence; detours when stuck; escaping hazards.
- `mod.rs`: the `Arena`. Spawns teams on connected ground (regenerating the
  arena if a team can't reach another), runs strikes as swept segments that
  can clip terrain or land on a part, slams and grabs, projectiles, damage
  with armour and piercing, severing, drama timers (slow motion and
  hit-stop), the frenzy, round results, and `Snapshot`s.
- `mirror.rs`: the watcher's copy. Applies snapshots and events, smooths
  positions, animates limbs locally, spawns particles, gibs and callouts,
  keeps a history for the instant replay, and exposes camera cues.
- `crates/net/src/protocol.rs`: `VersusClientMsg`/`VersusHostMsg`. Lobbies
  carry a `mode` tag so joiners know whether they're entering a sandbox or
  a match.
- `crates/game/src/versus/`: `session.rs` (host/client/practice roles,
  roster, rounds, points), `view.rs` (texture, auto camera, portraits),
  `ui.rs` (designer, fight overlay, results).

## Decisions

- Host-authoritative with a shared mirror: the host also watches through
  snapshots, so host and clients run identical presentation code.
- Designs are the only thing exchanged before a round; species and arenas
  are rebuilt deterministically everywhere from the designs and a seed.
- Reach is measured from each weapon's own origin (mouth, arm root, tail
  root), and a bite sweeps from the chest forward, so big-headed creatures
  don't whiff over small targets.
- The arena picks a seed until every walking team has a real path (one that
  arrives and doesn't wade deep water) to every other team.
- A stand-off (no hits for 35 s) triggers the frenzy early; a frenzy with
  no hits for 45 s ends the round on health.

## Progress

- [x] Designer with 32 upgrades, costs, locks between rounds, auto-fill,
      live portrait and derived stats
- [x] Species built from designs; every upgrade visible on the sprite
- [x] Per-part hitboxes, severing with gibs, lost-function effects
- [x] Physics strikes (clip, miss, hit), clashes, slams, grabs, spit
- [x] Adaptive tactic AI with intelligence-scaled learning and weak-spot
      targeting
- [x] Arena generation with spawn connectivity, drama timers, frenzy
- [x] Snapshots, events, chunk streaming; host and client mirrors
- [x] Steam and LAN hosting/joining, practice vs an evolving AI
- [x] Auto camera, slow motion, letterbox, speed lines, callouts, replay
- [x] Results and evolution loop, match winner, rematch
- [ ] Sound
- [ ] Balance pass with real players
- [ ] Spectator chat / emotes
