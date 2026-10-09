# Alien Versus: design and progress

Branch `alien-versus`, built on top of `alien-ecosystem`.

## What it is

A competitive mode. Each player designs a species from a point budget, the
species are dropped into a generated arena, and they fight on their own while
everyone watches. Rounds end on a wipe, the clock, or a stand-off; the host
picks how many rounds a match has and the most wins takes it. After every
round everyone gets more points to evolve: upgrades can be added, never
removed.

Fights are also about energy. Everything burns it (hunting gear, size and
aggression quickly; grazing, camouflage and caution slowly), carnivores
refill on what they bite and kill, herbivores graze fruit that every arena
grows and regrows, and a starving animal slows and bleeds health. A species
with no weapons can still win by hiding, foraging and outlasting.

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
  feelers, nose), target choice, tactics (rush, flank, kite, harass, ambush,
  dive, undermine, hold, retreat, search, forage, evade, hide) and per-tactic
  learning scaled by intelligence; the ambush bonus for spotting first; pack
  spreading; search that drifts towards the enemy; detours when stuck;
  escaping hazards.
- `balance.rs`: headless match running, ten archetype builds, and the
  ignored sweeps (`balance_archetypes`, `balance_upgrades`, `trace_pair`).
- `mod.rs`: the `Arena`. Spawns teams on connected ground (regenerating the
  arena if a team can't reach another), runs strikes as swept segments that
  can clip terrain or land on a part, slams and grabs, projectiles, damage
  with armour and piercing, severing, drama timers (slow motion and
  hit-stop), the frenzy, round results, and `Snapshot`s.
- `mirror.rs`: the watcher's copy. Applies snapshots and events (including
  plants and energy), smooths positions, animates limbs locally, spawns
  particles, gibs and callouts, queues sound cues, and exposes camera cues.
- `crates/net/src/protocol.rs`: `VersusClientMsg`/`VersusHostMsg`. Lobbies
  carry a `mode` tag so joiners know whether they're entering a sandbox or
  a match.
- `crates/game/src/versus/`: `session.rs` (host/client/practice roles,
  roster, round settings, points, rejoin memory, keepalives, the evolution
  log), `view.rs` (texture, auto camera, portraits), `ui.rs` (designer,
  fight overlay, results), `icons.rs` (an 8×8 pixel icon per upgrade).
- `crates/game/src/theme.rs` and `audio.rs`: the pixel look (Press Start
  2P, Silkscreen, VT323; square corners, 2 px borders, one palette) and the
  generated chiptune effects and tracks from `tools/gen_audio.py`.

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
- A stand-off (no hits for 60 s) triggers the frenzy early; a frenzy with
  no hits for 40 s ends the round on health, then energy as the tie-break.
- Bodies are drawn and collided 1.4× bigger than the size the numbers are
  balanced around, so fights read clearly.
- Only the finishing blow gets slow motion and effects; everything else is
  plain so the fight stays legible.
- The host pings between rounds. Clients used to time out on the results
  screen and rejoin as strangers, which reset their stats.
- Balance targets come from the archetype matrix: ten hand-built styles
  (brute, tank, pack, spitters, flier, grazer, burrower, brain, serpent,
  swarm) each win 38–65% of a round-robin, with the hard counters (fliers
  and spitters beat grazers, brutes beat packs, grazers outlast brutes)
  intact.

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
- [x] Auto camera, callouts; the finishing blow alone gets slow motion,
      letterbox and speed lines
- [x] Results and evolution loop, match winner, rematch
- [x] Energy, diet, metabolism, nose, fruit on every arena, foraging and
      evading
- [x] Flight stamina, wing tiers by body size, individuals priced by
      investment
- [x] Ambush bonus for spotting first, pack spreading, search drift
- [x] Host-set round count, rejoin memory, keepalives
- [x] Pixel theme and fonts, per-upgrade icons, evolution log
- [x] Generated chiptune effects and music
- [x] Headless balance sweeps and a first tuning pass
- [ ] Balance pass with real players
- [ ] Spectator chat / emotes
