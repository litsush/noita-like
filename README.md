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
| `crates/sim` | Engine-independent cell simulation: materials, chunked world with sleeping chunks, world generation, chunk run-length encoding. No Bevy, so it can run headless. |
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
