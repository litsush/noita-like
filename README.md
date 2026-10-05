# SBCT

A Noita-style falling-sand game with Steam multiplayer, heading toward Terraria-like gameplay.

## Running

```sh
cargo run                      # main menu
cargo run -- --singleplayer    # skip the menu
```

Test multiplayer on one machine without two Steam accounts (LAN transport):

```sh
cargo run -- --host-lan --name Alice
cargo run -- --join-lan 127.0.0.1 --name Bob
```

Other flags: `--seed <n>`, `--host-lan <port>`, `--join-lan <host:port>`.

Controls: A/D move, W/Space jump, left mouse dig, right mouse place, 1–9 or wheel to pick a material, Esc for the menu.

## Alien Ecosystem

**Alien Ecosystem** in the main menu (or `cargo run -- --ecosystem`) opens a sandbox where a procedurally generated alien world is populated by a generated predator species (3–5 of them) and the prey species it hunts (10 of them), feeding on generated plants. Every scene is new: press **New Scene** (or N) to roll another planet.

- **Biomes**: Verdant Highlands, Fungal Hollow, Volcanic Rift, Tidal Archipelago, Acid Marsh, Dune Sea, Spire Canyon and Sky Isles, each with an alien palette, trees, liquids (water, lava, acid), weather (rain, acid rain, ash fall, spores, sandstorms), a day/night cycle and its own gravity.
- **Species** are built from the biome they live in. Habitat (ground, burrower, tree-dweller, flier, swimmer, amphibian, lava-dweller, cave-clinger) decides locomotion and body plan: 0 to many legs, arms, tentacles, wings, fins, tails, antennae, eye stalks, shells, crests and gas bladders. Abilities follow from that and from the food chain: biting, claws, venom, constriction, spitting acid/fire/venom/silk, digging and sand-swimming, leaping, climbing, swimming in water or lava, camouflage, armour, spines, toxic flesh, ink clouds, playing dead, shelter building, bioluminescence and lures. Colours come from the environment (camouflage) or warn of toxins.
- **AI** is a utility system made of behaviour modules (flee, freeze, shelter, graze, hunt, ambush, set traps, scavenge, rest, build nests, reproduce, flock, mob, patrol, investigate, escape hazards, wander). Each species gets its own set of modules and weights, plus hunting styles (chase, stalk-and-pounce, ambush, web traps, pit traps, glowing lures, ranged spitting, flanking packs) and escape styles. Movement uses pathfinding that understands each species' locomotion.
- **Ecosystem**: hunger, energy, breath, ageing, mating (live young or eggs), juveniles, carcasses, scavenging, learned aversion to toxic prey, plants that grow, fruit and seed, and migrants when a species dies out.

Click a creature to inspect its needs, current goal, live utility scores and senses. Right-drag or WASD pans, the wheel zooms, Space pauses, 1–4 set the speed, L toggles behaviour labels, F follows the selected creature.

Dev flags: `--ecosystem`, `--seed <n>`, `--biome <name>`, `--speed <1-4>`, `--select [prey]`, `--zoom <px per cell>`, `--screenshot <file> --after <secs>`, `--quit-after <secs>`.

## Steam

With the Steam client running, the menu offers Steam hosting and joining:

- **Host**: creates a lobby (friends-only, public, or invite-only). Invite people from the Esc menu or with the Steam overlay (Shift+Tab).
- **Join**: lists friends currently in a game, lists public lobbies, or joins by lobby ID. Accepting a Steam invite also works, even when the game isn't running yet (`+connect_lobby`).

Traffic goes over `ISteamNetworkingMessages`, which Steam relays, so nobody needs to forward ports.

The game uses Valve's test app **480 (Spacewar)** until it has its own app ID. Change `APP_ID` in `crates/net/src/steam.rs`. Lobbies are tagged so they don't mix with other developers' lobbies on 480. Testing Steam multiplayer needs two machines, each signed in to a different Steam account.

Without Steam, the game still runs with LAN and singleplayer only.

## Layout

| Crate | Purpose |
|---|---|
| `crates/sim` | Engine-independent cell simulation: materials, chunked world with sleeping chunks, world generation, chunk run-length encoding. No Bevy, so it can run headless. `sim/src/eco/` holds the alien ecosystem: biomes, flora, species genomes, AI, navigation, physics, animation and its CPU renderer. |
| `crates/net` | Wire protocol (serde/bincode), a `Transport` trait, Steam transport (lobbies + relayed P2P), TCP transport (LAN/dev). No Bevy. |
| `crates/game` | Bevy client: menu, session (host/client logic), rendering, player, HUD. |

## Network model

The host is authoritative. It runs the simulation at 60 Hz and applies edits from clients. At 20 Hz it sends changed 64×64 chunks, run-length encoded and reliable, plus everyone's positions (unreliable). Clients don't simulate cells. Each client controls its own player's movement and applies its own edits immediately (optimistic), and the host's chunk updates overwrite them if they disagree. On joining, a client downloads the full world.

## Roadmap ideas

- Multithreaded simulation (checkerboard chunk update passes)
- Rigid bodies, spells/projectiles, and particles
- Inventory, mining drops, crafting (Terraria side)
- Infinite/streamed world with chunk interest management per player
- Client-side prediction of the sim near players; delta-compressed chunk updates
- Dedicated headless server binary reusing `sim` + `net`
