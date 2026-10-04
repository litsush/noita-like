# The Last Apprentice

A Noita-style falling-sand roguelike in a dark fantasy world. You are an apprentice wizard sent down through a living world to complete the rite at its heart. Many apprentices went before you, and their remains, journals and half-finished rituals line the way. Everything runs on the pixel physics: every cell is sand, water, lava, acid, gas or one of 30 materials, and every spell changes how you interact with them. There is also a sandbox mode with Steam and LAN multiplayer.

## Quick start

```sh
cargo run                      # main menu
cargo run -- --run             # straight into a run (random seed)
cargo run -- --run 1234        # a specific seed
```

It builds from a clean clone with no extra steps. Every asset is generated, committed and loaded from `assets/`. On Linux you need the usual Bevy system libraries (ALSA, udev).

## Playing

Pick **New Run** from the menu. Choose your first school of magic and how you begin, and optionally enter a seed (leave it empty for a fresh world). Death is permanent. The summary screen shows how deep you got, what killed you, your schools, fusion and scrolls, the achievements you earned and the seed, so you can replay or share it.

### Controls

| Action | Default keys |
|---|---|
| Move | A / D (or arrows) |
| Jump; climb walls briefly; wall-jump | W / Space |
| Swim down, dig straight down (smart dig) | S / ↓ |
| Dash | Shift |
| Dig spell: a beam from your staff (speed depends on hardness) | Left mouse, aimed with the cursor |
| Toggle smart dig (dig whatever blocks you in the direction you move or aim; holding down digs straight down; the next cells are outlined) | Tab |
| Cast the selected spell (hold for channelled spells) | Right mouse |
| Switch spell | Q / E or mouse wheel |
| Cast a light orb (floats where you aim and lights the dark for a while; charges recover slowly) | T |
| Dim or relight your staff (hide in the dark) | L |
| Interact (chests, remains, altars, shrines, the heart) | F |
| Pause, settings, abandon run | Esc |

Every control can be rebound under **Settings → Controls**, from the main menu or the pause menu. Each action has two slots. If a key is already in use, you can swap the two actions or keep the key on both. Bindings are saved with your progress.

### Magic

**Schools.** There are eight schools, and each run you specialise in two. You choose the first before descending. The second comes from the **attunement altar** in a small sanctum early in the first layer: it offers one scroll from each of up to three other schools, and the one you take binds its school to you. The *Twin-Souled* start lets you pick both schools up front.

| School | Theme | Unlocked by |
|---|---|---|
| Pyromancy | Fire, lava and heat | start |
| Hydromancy | Water and steam | start |
| Terramancy | Stone, sand and gravel | start |
| Mycomancy | Fungus and rot | start |
| Cryomancy | Ice and frost | Obsidian Bridge |
| Aeromancy | Wind and gas | Long Fall |
| Fulmancy | Lightning, conducting through water and metal | Conductor |
| Alchemy | Acid, explosives, transmutation | Alchemist |

**Scrolls.** There are 37 scrolls: four or more per school, plus neutral ones (Beacon, Glass Focus, Iron Soles, Blink). They are actives that cost **mana** and have a cooldown, channelled spells, passives, and dig variants that change your dig beam (Magma Bore, Crumbling Touch, Arc Drill, Rot Touch, Glass Focus). Scrolls you find in chests, on altars and in shrines are from your schools or neutral. Scrolls from other schools, carried by fallen apprentices, **shatter into shards** when you take them.

**Fusions.** Once you know a scroll from each of your two schools, their **fusion** awakens: a stronger spell that combines both elements. Each of the 28 school pairs has its own:

| | Hydro | Terra | Cryo | Aero | Fulm | Alch | Myco |
|---|---|---|---|---|---|---|---|
| **Pyro** | Steam Burst | Magma Surge | Thermal Shock | Firestorm | Plasma Lance | Napalm | Spore Bomb |
| **Hydro** | | Mudslide | Glacier | Typhoon | Storm Flood | Acid Rain | Swamp Bloom |
| **Terra** | | | Frozen Earth | Sandstorm | Railshot | Petrify | Root Lattice |
| **Cryo** | | | | Blizzard | Cryo Arc | Shatter | Rimebloom |
| **Aero** | | | | | Thunderstorm | Miasma | Spore Gale |
| **Fulm** | | | | | | Electrolysis | Storm Spores |
| **Alch** | | | | | | | Decay Bloom |

Fusions you've awakened are recorded under **Unlocks & Achievements → Fusions**.

**Shards** (aether shards) come from digging glowing shard veins, from fallen apprentices, from creatures and from shattered scrolls. Shrines sell scrolls for them.

### Surviving

- **Hearts** drop from fire, lava, molten metal, acid, explosions, electricity, creatures, drowning, toxic gas and being crushed. Big hits give brief invulnerability.
- **Heat meter** rises near hot materials, especially deep down. When it maxes out you take damage. Water and ice cool you.
- **Breath meter** drains underwater and in toxic gas.
- **Catching fire** keeps burning you until it runs out or you jump into water.
- **Darkness.** The deep caves are dark, and some things down there hunt by light. Your staff, light orbs, lava, fungus, crystals and the heart give light. Dimming your staff hides you but blinds you too.
- **The Hollow Stalker.** If you linger deep down, something tall and faceless starts to follow. It cannot be hurt and it never digs. It won't cross water or any other liquid, so a moat buys you time, and if you get far enough ahead for long enough it loses you. You can also trap it with the world itself: bury it under collapsing gravel or sand, or freeze it into the ice.

### The world

The world is a 512×4096-cell shaft with five layers. A banner, a new ambience and a new background mark each one.

| Layer | What's there |
|---|---|
| The Whispering Crust | Dirt, stone, roots, water pockets, unstable gravel ceilings. The attunement sanctum, ritual circles, flooded ruins, an old mine full of explosive crates. Gnawlings and the Hollowed. |
| The Drowned Halls | Vast flooded halls of arches and pillars, basalt, lava pools, steam vents, gas pockets, a lava lake crossed by a rickety bridge. Blind wyrms that hunt by the sound of digging. |
| The Fungal Abyss | Giant glowing mushrooms that spread and burn, crystals, acid pools, explosive gas chambers. Spore puppets (apprentices once) and spore drifters. |
| The Molten Sanctum | Ferrite halls, molten metal that throws sparks, electrified pools, extreme heat. Cinder wraiths and lightseekers that dive at your light. |
| The Heart of the World | A shell around the heart with one way in. Touch the heart to complete the rite. |

Each layer holds chests (some of them **mimics**), altars with a free scroll, shrines, and **fallen apprentices** whose remains you can search for shards, scrolls and **journal pages**. There are 30 pages to collect, and which ones you've read is saved across runs. Everything except the heart's shell can be dug, so there is always a way down.

### Achievements and unlocks

22 achievements teach the physics: survive a flood, ignite a gas pocket, turn lava into obsidian, stay in the dark for a minute, search five fallen apprentices, escape the Stalker, and so on. Each one unlocks a school, a scroll or a starting choice. Locked entries show hints under **Unlocks & Achievements**.

| Start | Effect | Unlocked by |
|---|---|---|
| Initiate | A staff, three light orbs and your wits | start |
| Prodigy | Begin knowing a scroll of your first school | Untouched |
| Scavenger | 40 shards and four light orbs | Grave Robber |
| Archmage's Heir | A first-school scroll and Blink | Rite Complete |
| Twin-Souled | Choose both schools before descending | Twin Mastery |

Progress and settings are saved as JSON in `$XDG_DATA_HOME/sbct/save.json` (on Windows `%APPDATA%\sbct`, on macOS `~/Library/Application Support/sbct`). Saves from before the wizard rework load safely: matching achievements carry over (Core Breaker becomes Rite Complete) and old items and loadouts are dropped. A corrupt save is backed up to `save.json.bak` and the game starts fresh.

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
- **Spawns and entities.** World generation returns chests, remains, altars, shrines, creatures and the heart as a plain `Vec<Spawn>`. Entity state (chest opened, remains searched, altar taken, shrine bought, creature positions and health, the Stalker, projectiles, light orbs) would need protocol messages, host-authoritative like cells.
- **Simulation region.** The run simulates only chunks near the player (`World::set_active_region` already takes several centres); the host would pass every player's position.
- **Per-player run state.** Health, mana, schools, scrolls, cooldowns, light-orb charges and shards live in `Run` and `RunPlayer` resources. These would move to per-player components keyed by `PeerId`. Clients would send their dig, cast and interact actions as requests, the same way the sandbox sends edits today.
- **Shared-run rules:** a shared seed, what happens when one player dies (spectate or respawn at a checkpoint), who gets scrolls from altars, whether each player attunes separately, and how the Stalker picks its prey.

## Development

```sh
cargo test --workspace         # simulation, world generation, protocol, save migration, schools and fusions, smart dig, bindings, sounds
cargo clippy --workspace --all-targets
cargo run -p sbct_sim --release --example descent_map -- <seed> <out-dir>   # render a world to PNGs
```

### Dev flags for unattended play-testing

| Flag | Effect |
|---|---|
| `--autoplay` | A bot plays runs (digs toward the heart, opens chests, casts spells) |
| `--layer <0-5>` | Start runs in a deeper layer (5 = right beside the heart) |
| `--give <all\|ScrollA,ScrollB>` | Start runs knowing scrolls, e.g. `--give SparkBolt,IceLance` (scrolls outside your schools are learned anyway) |
| `--schools <First[,Second]>` | Pick the run's schools, e.g. `--schools Pyromancy,Cryomancy` (the fusion awakens once you know a scroll of each) |
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

- **Sprite sheets** (1 cell = 1 pixel, a darkened Endesga-32 palette):
  - `wizard_robe.png`, `wizard_trim.png`, `wizard_base.png`: three aligned 16×16 sheets (8×11 frames: idle, run, jump, fall, climb, cast side/down/up, cast spell, hurt, death). The game tints the robe by your first school and the trim by your second, with the base drawn on top.
  - `icons.png`: 16×16 icons in a 16×8 grid. School sigils, the 37 scrolls, the 28 fusions, HUD icons and start choices. Indices are in `crates/game/src/run/scrolls.rs`.
  - `props.png`: light orb, chests, altar, shrine, remains, journal, mimic, shard, rune stone.
  - `spells.png`: an 8×8 projectile and impact animation per school.
  - `creatures.png`: gnawling, hollowed apprentice, blind wyrm, spore puppet, spore drifter, cinder wraith, lightseeker, mimic.
  - `stalker.png`: the Hollow Stalker, 16×32 frames.
  - `core.png`: 8 frames of 48×48, the heart of the world.
  - `ui.png`: grimoire panel ornaments (corners, badge, dividers).
- **Backgrounds:** `backgrounds/layer{0-4}_{far,near}.png` are 256×256 and tile seamlessly.
- **Fonts:** `fonts/pixel.ttf` is the body pixel font and `fonts/title.ttf` the ornate title face. Both are generated and crisp at multiples of 8 px.
- **Audio:** 16-bit mono 22 kHz WAVs. Footsteps per surface, the dig beam per material family, per-school cast and impact sounds, fusions, creature voices, the Stalker, whispers and stings, hazards, UI, five layer ambiences, menu music, and victory and death themes.

The docstring at the top of each script documents every file in detail.

### Layout

| Crate | Purpose |
|---|---|
| `crates/sim` | Engine-independent simulation, no Bevy:<br>• materials (`material.rs`, property-driven) and cell rules (`world.rs`: powders, liquids, gases, fire, electricity, acid, frost, gravel cave-ins, explosions, particles, random ticks for slow growth and melting)<br>• cell colours (`color.rs`)<br>• sandbox world generation (`worldgen.rs`) and roguelike generation (`descent.rs`) |
| `crates/net` | Wire protocol, `Transport` trait, Steam and TCP transports. No Bevy. |
| `crates/game` | Bevy app:<br>• `run/`: the roguelike, i.e. player, schools and scrolls (`scrolls.rs`), spells and fusions (`spells.rs`), hazards, achievements, save, entities, creatures and the Stalker, lore, smart dig, lighting, dread, backgrounds, HUD<br>• `controls.rs`, `settings.rs`: rebindable controls and the settings pages<br>• `session.rs`: sandbox and multiplayer<br>• `menu.rs`, `render.rs`, `audio.rs`, `fx.rs`, `ui.rs`, `dev.rs` |

The implementation plan and progress log are in `docs/roguelike-plan.md`.
