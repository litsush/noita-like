# Planet Terraforming

A co-op sandbox and automation game for up to 8 players, built on a falling-sand engine. You are terraformers sent from Earth to prepare an alien planet for human habitation: mine the caves, sell minerals to Earth, grow plants in biodomes, lay pipes, program robots, upgrade your body, and raise the oxygen until the colony ship can land.

The full design, every table (recipes, domes, species, the 53 body-mod sets) and the progress log are in [`docs/terraforming-plan.md`](docs/terraforming-plan.md).

## Running

```sh
cargo run                               # main menu
cargo run -- --singleplayer --seed 1    # straight into a new world
```

Worlds are saved to `<data dir>/sbct-terraforming/saves/` (every three minutes, when you leave, and on exit) and are loaded from **Load World** in the menu. Set `SBCT_SAVE_DIR` to keep test runs away from your real saves.

## How to play

You start in the starter biodome: eight beds, two chests, a Synthesizer, a Comm. Terminal and a Mod Bay. The air outside is 3% oxygen, so your suit seals when you step out and its tank drains.

1. **Mine.** Dig with the multitool, shoot predators with the laser (it recharges by itself, slowly if you drain it). Ore gets richer and the caves darker and nastier the deeper you go. If you black out, your raw resources stay in a pack where you fell.
2. **Sell and buy.** At the Comm. Terminal, sell ores and produce to Earth for credits (shared by the colony). Order seeds and animals (they arrive by drop pod), hire workers, and buy blueprints.
3. **Synthesize.** The Synthesizer makes glass, machines, dome kits, robots and body-mod tiers from what is in your inventory, the dome's chests and every Storage Dome.
4. **Build the base.** Place dome kits and machines where the ghost turns green. Domes can't touch. Green and Dark Domes grow crops; Kitchens, Barns and Aqua Domes make food; Water Generator Domes and lake Pumps make water; Apartment Domes house the colonists.
5. **Power.** Battery flowers sprout on the surface each dawn and grow in Green Domes. Load them into Charging Pylons: a pylon powers everything in range, and pylons in range of each other share. Press **B** to see it.
6. **Water.** Drag copper pipe between pumps, tanks and domes. Press **V** for the water overlay: every network in its own colour with supply, demand and what is wrong.
7. **Automate.** Robots run short programs written as sentences ("Harvest anything in Green Dome 1, then store everything in the nearest storage."). Sprinklers, workers, automated arms and drones take over the chores.
8. **Splice.** The Gene Splicer crosses any two seeds into a hybrid with genes from both parents; the window shows the result before you commit.
9. **Win.** The four Colony Readiness bars at the top of the screen (apartments, oxygen, food, water) are always visible. When all four are full, call the ship from the Comm. Terminal's Colony tab. The world carries on afterwards.

### Robots

Synthesize a Robot Kit (blueprint from Earth), place it, and press **E** on the robot. A program is up to eight sentences built from drop-down word blocks:

```
[WHEN condition,] step [UNTIL condition] {, then step [UNTIL condition]}.
```

Sentences are tried from the top every moment and the first one that can run is what the robot does, so put the urgent ones first ("WHEN battery is below 20%, charge at the nearest pylon."). The running sentence is green; a sentence that can't run is amber and says why. Robots also say what they are doing in a bubble. Areas for mining, scavenging and guarding are marked on the map (**M**, "+ Area", drag a box).

### Body mods

53 sets, each with tiers I, II, III and Legendary, synthesized at a Synthesizer or the Mod Bay. Six basic sets are known from the start; Earth sells blueprints for the rest. You can own everything but wear one set per body slot (head, eyes, torso, back, arms, hands, legs, feet), swapped at the Mod Bay in the starter dome. Every tier shows on your sprite, for everyone.

## Controls

Everything is rebindable under **Settings → Controls**, from the main menu and the pause menu. Defaults:

| Action | Key |
|---|---|
| Move, jump / swim up / fly, down | A, D, W or Space, S |
| Dig / use the selected item | Left mouse |
| Fire the laser (hold to charge with Bolt Enhancements) | Right mouse |
| Interact (chests, planters, machines, robots, teammates) | E |
| Hotbar | 1–0, mouse wheel, `[` `]`; X puts the item away |
| Inventory | Tab or I |
| Quick stack to nearby chests | Q |
| Drop the selected item | G (Ctrl+G drops the stack) |
| Map, Codex | M, C |
| Water overlay, Power overlay | V, B |
| Dash (Dash Pistons) | Shift |
| Grapple (Grapple Glove) | R |
| Ping a waypoint (Beacon Rack) | P |
| Rain dance (Weather Vane) | H |
| Scout camera (Zoom Goggles) | Z |
| Pause | Esc |

Inventory: drag to move, right-click to split, Shift-click to send a stack to the other inventory, Alt-click to favourite (favourites are skipped by sort, quick stack, deposit and trash). With a pipe in hand, left-drag lays pipe and right-drag takes it up.

## Multiplayer

Up to 8 players. The host simulates everything; clients send their own movement and their requests.

- **Steam:** choose **New World** or **Load World**, then *Steam co-op*. Friends join through the Steam overlay or an invite.
- **LAN / one machine, no Steam:**

  ```sh
  cargo run -- --host-lan --name Ada
  cargo run -- --join-lan 127.0.0.1 --name Grace
  ```

  `--name` gives each instance its own character so several can run on one machine. The same messages and code paths are used for Steam and LAN.

Players can join and leave at any time. Characters (inventory, body mods, spawn bed) are stored in the world save and handed back when that player returns. Credits, blueprints, the codex, map pins and robot areas belong to the colony; map fog is each player's own.

## Developer flags

`--singleplayer`, `--host-lan`, `--join-lan <addr>`, `--name <name>`, `--seed <n>`, `--world <save name>`, `--load <save name>`.

For unattended testing (see the top of `crates/game/src/dev.rs` for the full list): `--screenshot <dir>`, `--quit-after <secs>`, `--window WxH`, `--fps`, `--time <secs>`, `--weather <clear|mist|rain>`, `--goto x,y`, `--give Iron:50,Gold:10`, `--credits <n>`, `--o2 <percent>`, `--mods 0:2,46:4` (or `all:3`), `--setup farm` (a demo base with domes, a pylon, a Gene Splicer and two robots), `--open <inventory|chest|synth|terminal|modbay|map|codex|robot|splicer>`, `--press KeyW@6-6.2,KeyD@5-9` (scripted key presses).

## Layout

| Path | Purpose |
|---|---|
| `crates/sim` | Bevy-free and deterministic: the cell world, planet generation, and the colony rules in `colony/` (players, creatures, items, recipes, domes, farming, plants and splicing, power, water, robots, body mods, readiness, the save file) |
| `crates/net` | Wire protocol and the Steam and TCP transports |
| `crates/game` | Bevy client: session (host and client roles), rendering, lighting, input, UI, audio |
| `tools/` | Asset generators |
| `assets/` | Generated art, fonts and audio (committed) |

## Assets

All art and audio is generated by Python scripts (standard library only) and committed. Edit the script, rerun it from the repository root, and commit the output; don't edit the files in `assets/` by hand.

```sh
python3 tools/gen_art.py       # item icons, domes, props, effects, foliage, backdrops, UI, fonts
python3 tools/gen_sprites.py   # players, suit, body-mod attachments, robots, creatures, workers, animals
python3 tools/gen_audio.py     # sound effects, ambience and music
```

`gen_art.py` and `gen_sprites.py` take `--preview <dir>` to write enlarged previews. Plants in planters and outdoors are not in the sheets: they are drawn in the game from each species' genes, so hybrids get their own look.

## Checks

```sh
cargo fmt
cargo clippy --workspace --all-targets
cargo test --workspace
```
