# Descent to the Core

A Noita-style falling-sand roguelike. You dig down through a fully simulated planet to touch its core. Everything is built on the pixel physics: every cell is sand, water, lava, acid, gas or one of 31 materials, and every upgrade changes how you interact with them. It also has a sandbox mode with Steam and LAN multiplayer.

## Quick start

```sh
cargo run                      # main menu
cargo run -- --run             # straight into a run (random seed)
cargo run -- --run 1234        # a specific seed
```

It builds from a clean clone with no extra steps. Every asset is generated, committed and loaded from `assets/`. On Linux you need the usual Bevy system libraries (ALSA, udev).

## Playing the roguelike

Pick **New Run** from the menu, choose a loadout, and optionally enter a seed (leave it empty for a fresh planet). Death is permanent. The summary screen shows how deep you got, what killed you, your items, the achievements you earned and the seed, so you can replay or share it.

### Controls

| Action | Keys |
|---|---|
| Move | A / D (or arrows) |
| Jump; climb walls briefly; climb ropes | W / Space (hold against a wall to climb; wall-jump off walls) |
| Dash | Shift |
| Dig (speed depends on hardness) | Left mouse, aimed with the cursor |
| Toggle smart dig (dig whatever blocks you in the direction you move or aim; holding down digs straight down; the next cells are outlined) | Tab |
| Use selected active item | Right mouse |
| Switch active item | Q / E or mouse wheel |
| Throw rope upward (climb back up) | R |
| Throw a torch (lights the dark, ignites flammables) | T |
| Interact (chests, altars, shrines, the core) | F |
| Pause, settings, abandon run | Esc |

Every control can be rebound under **Settings → Controls**, from the main menu or the pause menu. Each action has two slots. If a key is already in use, you can swap the two actions or keep the key on both. Bindings are saved with your progress.

### Surviving

- **Hearts** drop from fire, lava, molten metal, acid, explosions, electricity, creatures, drowning, toxic gas and being crushed. Big hits give brief invulnerability.
- **Heat meter** rises near hot materials, especially deep down. When it maxes out you take damage. Water and ice cool you.
- **Breath meter** drains underwater and in toxic gas.
- **Catching fire** keeps burning you until it runs out or you jump into water.
- **Darkness:** the deep caves are dark. Your headlamp, torches, lava, fungus, crystals and the core give light.

### The world

The planet is a 512×4096-cell shaft with five layers. A banner, a new ambience and a new background mark each one.

| Layer | What's there |
|---|---|
| The Crust | Dirt, stone, roots, water pockets, sand, oil seams. Flooded ruins and an abandoned mine full of explosive crates. |
| Upper Mantle | Basalt, lava pools, steam vents, unstable gravel ceilings (dig under them and they cave in), gas pockets, a magma chamber with a fragile bridge. |
| Deep Mantle | Crystals, glowing fungus that spreads and burns, acid pools, explosive gas chambers, gravel-filled pressure fractures. |
| Outer Core | Ferrite, molten metal that throws sparks, electrified pools, extreme heat, a foundry. |
| The Core | A core-shell sphere with one way in. Touch the core to win. |

Each layer has a free item on an altar and chests scattered through its caves. From the Upper Mantle down, shrines sell items for ore, which you get by digging ore veins. Everything except the core shell can be dug, so there is always a way down.

### Items and unlocks

There are 17 items. Six are available from the start; the other 11 unlock through achievements that teach the physics: flood a chamber you're in and survive, ignite a gas pocket, turn lava into obsidian, and so on. Locked entries appear as silhouettes with hints under **Unlocks & Achievements**. Two achievements unlock new starting loadouts instead (Gifted and Excavator).

Progress and settings are saved as JSON in `$XDG_DATA_HOME/sbct/save.json` (on Windows `%APPDATA%\sbct`, on macOS `~/Library/Application Support/sbct`). A corrupt save is backed up to `save.json.bak` and the game starts fresh.

## Sandbox and multiplayer

**Multiplayer & Sandbox** in the menu opens the free-build mode: paint and dig with any material, alone or together.

```sh
cargo run -- --singleplayer                    # offline sandbox
cargo run -- --host-lan --name Alice           # two windows on one machine (LAN)
cargo run -- --join-lan 127.0.0.1 --name Bob
```

Other sandbox flags: `--seed <n>`, `--host-lan <port>`, `--join-lan <host:port>`.

### Steam

With the Steam client running, the multiplayer menu offers Steam hosting and joining:

- **Host:** creates a lobby (friends-only, public or invite-only). Invite people from the Esc menu or the Steam overlay.
- **Join:** lists friends currently in a game and public lobbies, or joins by lobby ID. Accepting a Steam invite also works when the game isn't running (`+connect_lobby`).

Traffic is relayed through Steam (`ISteamNetworkingMessages`), so nobody needs to forward ports. The game uses Valve's test app **480 (Spacewar)** until it has its own app ID: change `APP_ID` in `crates/net/src/steam.rs`. Without Steam the game still runs, with LAN and singleplayer only.

### Network model

The host is authoritative. It simulates at 60 Hz and applies edits from clients. At 20 Hz it sends changed 64×64 chunks (run-length encoded, reliable) and player positions (unreliable). Clients don't simulate cells; they move their own player and apply their own edits immediately. On joining, a client downloads the whole world.

### What multiplayer needs before the roguelike can use it

The roguelike is singleplayer for now, but its state was designed to be synced:

- **Simulation events and particles.** The world already produces both as plain data (`World::take_events`, `World::particles`). The host would forward events (explosions, sizzles, collapses) so clients can play sounds and particles. In-flight particles would need replicating, or clients would simulate them locally from spawn messages.
- **Spawns and entities.** World generation returns chests, altars, shrines, creatures and the core as a plain `Vec<Spawn>`. Entity state (chest opened, altar taken, shrine bought, creature positions and health, projectiles, bombs, torches) would need protocol messages, host-authoritative like cells.
- **Simulation region.** The run simulates only chunks near the player (`World::set_active_region` already takes several centres); the host would pass every player's position.
- **Per-player run state.** Health, items, charges and ore live in `Run` and `RunPlayer` resources. These would move to per-player components keyed by `PeerId`. Clients would send their pick, rope and item actions as requests, the same way the sandbox sends edits today.
- **Shared-run rules:** a shared seed, what happens when one player dies (spectate or respawn at a checkpoint), and who gets items from altars.

## Development

```sh
cargo test --workspace         # simulation, world generation, protocol, save file, items
cargo clippy --workspace --all-targets
cargo run -p sbct_sim --release --example descent_map -- <seed> <out-dir>   # render a world to PNGs
```

### Dev flags for unattended play-testing

| Flag | Effect |
|---|---|
| `--autoplay` | A bot plays runs (digs toward the core, grabs items, uses actives) |
| `--layer <0-5>` | Start runs in a deeper layer (5 = right beside the core) |
| `--give <all\|ItemA,ItemB>` | Start runs with items, e.g. `--give SparkRod,FrostSeed` |
| `--screenshot <dir>` | Save a screenshot every 4 seconds |
| `--quit-after <secs>` | Exit after that long |
| `--fps` | Log the frame rate |
| `--menu-screen <new-run\|unlocks\|items\|settings\|controls\|multiplayer>` | Open a menu page |
| `--window <WxH>`, `--ui-scale <x>` | Window size and UI scale |
| `--test-toasts` | Show a few achievement toasts at the start of a run |

Set `SBCT_SAVE_DIR=/some/tmp/dir` while testing so the bot's achievements don't go into your real save. For example:

```sh
SBCT_SAVE_DIR=/tmp/sbct cargo run -- --run 42 --autoplay --layer 2 --screenshot /tmp/shots --quit-after 60
```

### Generated assets

All art and audio is generated by two stdlib-only, deterministic Python scripts. Rerun them after changing them, and commit the output:

```sh
python3 tools/gen_art.py      # sprite sheets, backgrounds, pixel font -> assets/
python3 tools/gen_audio.py    # sound effects, ambience, music -> assets/audio/
```

- **Sprite sheets** (1 cell = 1 pixel, Endesga-32 palette):
  - `player.png`: 16×16 frames, rows idle, run, jump, fall, climb, dig side/down/up, hurt, death.
  - `items.png`: 16×16 icons, 8 per row, indices match `crates/game/src/run/items.rs`.
  - `props.png`: torch, chests, altar, shrine.
  - `core.png`: 8 frames of 48×48.
  - `creatures.png`: 4 frames each for the crawler, magma slug and spore drifter.
- **Backgrounds:** `backgrounds/layer{0-4}_{far,near}.png` are 256×256 and tile seamlessly.
- **Font:** `fonts/pixel.ttf` is a generated pixel font, crisp at multiples of 8 px.
- **Audio:** 16-bit mono 22 kHz WAVs: footsteps per surface, digging per material family, hazards, UI, five layer ambience loops, a victory theme and menu music.

The docstring at the top of each script documents every file in detail.

### Layout

| Crate | Purpose |
|---|---|
| `crates/sim` | Engine-independent simulation, no Bevy:<br>• materials (`material.rs`, property-driven) and cell rules (`world.rs`: powders, liquids, gases, fire, electricity, acid, frost, gravel cave-ins, explosions, particles, random ticks for slow growth and melting)<br>• cell colours (`color.rs`)<br>• sandbox world generation (`worldgen.rs`) and roguelike generation (`descent.rs`) |
| `crates/net` | Wire protocol, `Transport` trait, Steam and TCP transports. No Bevy. |
| `crates/game` | Bevy app:<br>• `run/`: the roguelike, i.e. player, hazards, items, achievements, save, entities, creatures, lighting, backgrounds, HUD<br>• `session.rs`: sandbox and multiplayer<br>• `menu.rs`, `render.rs`, `audio.rs`, `fx.rs`, `ui.rs`, `dev.rs` |

The implementation plan and progress log are in `docs/roguelike-plan.md`.
