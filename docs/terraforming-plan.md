# Planet Terraforming: design and plan

A co-op sandbox and automation game for up to 8 players, built on the falling-sand engine in this repository. You are terraformers sent from Earth to prepare a similar planet for human habitation. Early on everything is done by hand. Over time the colony's domes, pipes, generators and robots do the work, and the numbers keep climbing while the players explore.

This document is the blueprint. The progress log at the end records what has been built.

## 1. Core loop and progression

**Loop:** mine underground → sell minerals to Earth for credits → buy blueprints, seeds, animals and workers; synthesize body mods, machines and domes → the base produces oxygen, food, water and housing → repeat until the colony is ready and the ship can be called.

**Win:** every Colony Readiness target is met at the same time, then a player calls the ship from the Comm. Terminal. The ship lands in an ending sequence and the world stays playable.

| Readiness target | Goal | How it is measured |
|---|---|---|
| Apartment Domes | 6 | Placed, powered and connected to water |
| Atmospheric oxygen | 19% | Global level (starts at 3%) |
| Food output | 240 food/day | Food produced by Kitchens, Barns and Aqua Domes over the last full day |
| Fresh water surplus | 600 L/day | Source capacity minus consumption across all water networks |

The Colony Readiness panel is always on the HUD (collapsed to four small bars, expandable).

**Progression curve** (a small group, about 4–5 hours; more players go faster because costs don't scale):

| Phase | Time | What players do | Unlocks |
|---|---|---|---|
| 0. Landing | 0–20 min | Leave the starter dome in suits, harvest battery flowers, mine iron, copper and silicon near the surface, sell, synthesize first mods | Basic recipes |
| 1. First roots | 20–60 min | First Green Dome, hand farming, Storage Dome, Charging Pylon | Kitchen, Pump and Pipe blueprints |
| 2. Plumbing | 1–2 h | Lake pump, copper pipes, sprinklers, Kitchen, workers, deeper caves (gold, titanium) | Robot, Oxygen Generator, Dark/Aqua/Barn blueprints |
| 3. Automation | 2–3.5 h | Robots with programs, Oxygen Generators, Gene Splicer, Tier III mods | Dome Dome, Apartment blueprints |
| 4. Readiness | 3.5–5 h | Dome Domes building Apartments, hybrids tuned for oxygen and food, Legendary mods (aurorium from the deepest caves) | Calling the ship |

## 2. Architecture

The existing split stays: `sim` is Bevy-free and deterministic, `net` carries messages, `game` presents and takes input.

- **`crates/sim`**
  - Cell world (`world.rs`, `material.rs`, `color.rs`): the falling-sand engine, extended with ores, glass, plating, toxic spores and hardness-based digging that reports what was dug.
  - `planetgen.rs`: the planet (surface, lakes, cave networks, ore veins, ruins) from a seed.
  - `colony/`: all game rules as plain data and functions with no engine types, so they run headless in tests and identically on host and offline. Items and inventories, recipes, plants and genetics, domes, water, power, atmosphere, robots and their program interpreter, body mods, creatures, player actions and the save file.
- **`crates/net`**: the wire protocol (now carrying colony snapshots, deltas and player actions) and the Steam and TCP transports.
- **`crates/game`**: Bevy client. Session (host/client roles), rendering (tiled cell world, lighting, sprites, procedural plants, weather), input and bindings, UI (inventory, synthesizer, terminal, map, codex, sentence builder), audio.

The prompt asks for game rules in `game`. The rules that the host must run and replicate live in `sim::colony` instead, because the protocol in `net` has to name their types and `net` can't depend on `game`. `game` keeps everything about presentation, input and orchestration.

### Network model

The host is authoritative for everything except each player's own movement.

| State | Owner | How it is synced |
|---|---|---|
| Terrain cells | Host | Dirty 64×64 chunks, run-length encoded, reliable, 20 Hz (unchanged) |
| Player position and animation | Each client | `ClientMsg::State` 20 Hz unreliable; host relays to everyone |
| Colony entities (domes, machines, plants, robots, creatures, drops, bolts) | Host | Each entity has an id. Changed entities are sent as whole snapshots in `HostMsg::Entities { upserts, removed }`, reliable, up to 10 Hz |
| Motion of moving entities | Host | `HostMsg::Motion` (id, position, velocity, animation), unreliable, 15 Hz, only entities near each client |
| Pipes | Host | `HostMsg::Pipes` deltas (grid position, pipe or none), reliable |
| Globals (clock, weather, credits, atmosphere, readiness, ship) | Host | `HostMsg::Globals`, 4 Hz |
| Meta (species, codex, blueprints, map pins, robot areas, workers) | Host | Whole section when its revision changes |
| Your own player (inventory, mods, vitals) | Host | `HostMsg::You` when changed; vitals at 5 Hz |
| Other players' public info (name, colour, mods shown, suit, held item) | Host | `HostMsg::PlayerInfo` when changed |
| One-shot effects (sounds, bursts, toasts) | Host | `HostMsg::Events` |

- **Actions.** Everything a player does to the colony is an `Action` (dig, place, fire, interact, move an item, craft, buy, program a robot…). On the host or offline it is applied directly. On a client it is sent to the host as `ClientMsg::Action` and the result comes back through the normal sync. Digging is also applied locally at once so it feels responsive, as before.
- **Joining.** A client sends `Hello { version, name, key }`. The key is a persistent player id (the Steam id, or a random id stored in the client's settings for LAN), so the character, inventory and mods are restored on rejoin. The host answers with `Welcome`, the full cell world, and a full colony snapshot.
- **Steam and LAN** use the same `Transport` trait and the same messages; only the transport differs.
- **Persistence.** The host saves the world (all chunks, run-length encoded), the colony and every known player to `saves/<name>.sbct` on autosave (every 3 minutes), on closing the world, and on request. Worlds are loaded from the menu.
- **Simulation of far-away things.** The whole colony lives in memory and steps every tick regardless of where players are, so robots and domes keep working off-screen. Only the cell simulation is limited to chunks near players.

## 3. World and survival

- **Planet:** 5120×1536 cells (1 cell = 1 art pixel; the player is about 8×22 cells). A rolling surface with lakes, alien trees and ruins; contiguous cave networks with surface entrances; three depth bands.

| Band | Depth | Rock | Ores | Hazards |
|---|---|---|---|---|
| Shallows | 0–30% | Dirt, stone, sand | Iron, copper, silicon | Skitters, dim light |
| Deeps | 30–65% | Stone, basalt, crystal | Gold, titanium, more of the above | Webspinners, gloom leeches, spore gas, darkness |
| Abyss | 65–100% | Basalt, obsidian, lava pockets | Xenite, aurorium | Burrow maws, the Brood Mother, heat, darkness |

- **Day and night:** a day is 12 minutes (8 day, 4 night). Battery flowers sprout on the surface at each dawn. Nights are cold and some predators come to the surface.
- **Weather:** clear, mist and rain. Rain waters outdoor plants, raises Water Generator output and refills lakes slightly.
- **Suit and oxygen:** the suit tank holds 100 O₂ (about 3 minutes at the start). It refills inside domes and near Oxygen Generators. At 0 the player loses health. The drain slows as the atmosphere improves; at 16% atmosphere players can breathe outside and the suit comes off the sprite.
- **Health:** 100. Regenerates in domes. At 0 the player blacks out and wakes in their bed after a few seconds; raw resources they carried are left in a pack at the spot.
- **Cold:** at night on the surface and in frost caves, warmth drains; at 0 the player takes slow damage. Domes, lava and thermal mods warm.
- **Toxins:** spore gas in the Deeps damages unfiltered lungs.
- **The laser:** fires bolts for 12 battery each from a 100 battery. The battery recharges by itself after a short pause; if it hits 0 it locks out for 2.5 s first. Bolts hurt creatures and chip terrain.
- **Digging:** the multitool digs where you point. Speed depends on hardness. Dug cells become item drops that fly to nearby players (magnet radius).

### Creatures

| Creature | Where | Behaviour | Drops |
|---|---|---|---|
| Driftmoth | Surface | Harmless, flutters around plants and lights | — |
| Puffback | Surface | Harmless grazer | Bio-gel |
| Skitter | Shallows, surface at night | Spider that climbs walls and ceilings and lunges | Chitin, silk |
| Webspinner | Deeps | Keeps distance and spits webs that slow | Silk |
| Gloom Leech | Deeps, water | Latches on and drains oxygen | Bio-gel |
| Burrow Maw | Abyss | Hides in the ground and erupts under players | Chitin, xenite |
| Brood Mother | Abyss lairs | Large, slow, spawns skitters | Aurorium |

Predators can't pass dome shells. Each kind is recorded in the codex when first seen.

## 4. Items and resources

| Item | Source | Sell value |
|---|---|---|
| Stone, Dirt, Sand | Digging | 0 |
| Wood, Plant Fiber | Trees, Ironbark, Fiber Vine | 1 |
| Iron | Shallows and deeper | 4 |
| Copper | Shallows and deeper | 6 |
| Silicon | Shallows and deeper | 8 |
| Gold | Deeps | 30 |
| Titanium | Deeps | 55 |
| Xenite | Abyss | 140 |
| Aurorium | Abyss lairs | 450 |
| Chitin, Silk, Bio-gel | Creatures | 10–15 |
| Glass | Synthesized from sand | 2 |
| Fertilizer | Synthesized from plant matter; Compost Vat; Barn | 3 |
| Battery Flower | Wild at dawn; grown | 5 |
| Crops (per species) | Farming | 2–12 |
| Meal | Kitchen Dome | 15 |
| Egg, Milk, Fish | Barn, Aqua Dome | 8 |
| Seeds (per species) | Harvesting, Earth | — |
| Water Canister | Filled at lakes and taps | — |

Stacks hold 999 for raw resources and 99 for everything else unless noted.

## 5. Synthesizer, Earth and credits

The **Synthesizer** turns materials into items using recipes. It draws from the player's inventory, chests in the same dome and every Storage Dome (the colony stockpile). Players start with the basic recipes; the rest are blueprints bought from Earth.

The **Comm. Terminal** (in the starter dome) sells minerals and goods to Earth for credits, which are shared by the colony. Credits buy seeds, animals, workers and blueprints. The terminal also calls the ship.

### Recipes

| Output | Inputs | Blueprint |
|---|---|---|
| Glass ×2 | Sand 3 | Start |
| Fertilizer ×2 | Plant Fiber 2 or any crops 3 | Start |
| Chest | Wood 12, Iron 4 | Start |
| Watering Can | Iron 6, Copper 2 | Start |
| Lamp | Iron 2, Silicon 2, Battery Flower 1 | Start |
| Charging Pylon | Iron 12, Copper 8, Silicon 4 | Start |
| Green Dome kit | Glass 40, Iron 30, Dirt 40, Fertilizer 10, Wood 20 | Start |
| Storage Dome kit | Glass 15, Iron 40, Wood 20 | Start |
| Bedroom Dome kit | Glass 20, Iron 25, Wood 30, Plant Fiber 10 | Start |
| Copper Pipe ×8 | Copper 4 | 150 cr |
| Pump | Iron 15, Copper 15 | 150 cr (with pipes) |
| Water Tank | Iron 20, Glass 10 | 150 cr (with pipes) |
| Kitchen Dome kit | Glass 20, Iron 30, Copper 20, Silicon 10 | 400 cr |
| Water Generator Dome kit | Glass 20, Iron 40, Copper 40, Silicon 20 | 600 cr |
| Dark Dome kit | Stone 60, Iron 30, Silicon 10 | 500 cr |
| Aqua Dome kit | Glass 60, Iron 30, Copper 20, Sand 40 | 800 cr |
| Barn Dome kit | Glass 20, Iron 30, Wood 60, Plant Fiber 20 | 700 cr |
| Oxygen Generator | Iron 40, Silicon 25, Battery Flower 10 | 1200 cr |
| Robot | Iron 30, Silicon 20, Gold 8 | 1500 cr |
| Synthesizer | Iron 30, Copper 15, Silicon 15 | 500 cr |
| Gene Splicer | Iron 40, Gold 20, Silicon 40, Xenite 4 | 5000 cr |
| Dome Dome kit | Glass 40, Iron 60, Gold 20, Silicon 40, Titanium 10 | 3000 cr |
| Apartment Dome kit | Glass 80, Iron 80, Copper 40, Titanium 20, Wood 40 | 4000 cr |
| Body mod tiers | See section 10 | Per set |

### Earth catalogue

| Goods | Price |
|---|---|
| Seed packs (3 seeds of a base species) | 40–400 cr by species |
| Cluckbug (lays eggs) | 300 cr |
| Milk Grub (gives milk) | 500 cr |
| Pond fish fry ×5 | 250 cr |
| Human worker | 800 cr |
| Blueprints | As listed above and in section 10 |

**Workers** arrive by drop pod. Each needs a free bed in a Bedroom or Apartment Dome and eats 2 food and drinks 4 L of water a day from the colony stockpile. A worker is assigned to one production dome (Green, Dark, Aqua, Barn, Kitchen, Water Generator). Each worker raises that dome's output by 50% (two per dome at most) and harvests and replants its planters. Unfed workers stop working.

## 6. Domes

A dome is synthesized as a kit and placed on the ground. Placing levels a foundation, clears the interior and builds the shell. Domes can't touch each other (their footprints must be at least 8 cells apart). The shell is a membrane: players and robots pass through, but cells and predators don't. The inside is breathable.

| Dome | Size | Contents | Needs | Does |
|---|---|---|---|---|
| Starter Biodome | 216×80 | 8 beds, 2 chests, suit rack, Synthesizer, Comm. Terminal, Mod Bay | — (built-in generator) | Home. Spawn point |
| Green Dome | 104×52 | 4 planters, sprinkler port | Water for sprinklers (or by hand) | Grows land crops, including battery flowers; oxygen |
| Storage Dome | 72×44 | 6 chests (40 slots each) | — | Part of the colony stockpile that synthesizers and robots draw from |
| Bedroom Dome | 72×44 | 4 beds | — | Players sleep (all asleep skips the night) and set spawn; houses 4 workers |
| Kitchen Dome | 72×44 | Cooker with input and output chests | Power, water | Turns any 3 crops into a Meal (food) |
| Dark Dome | 104×52 | 4 dark planters, Compost Vat | — | Grows mushrooms and other dark species; composts plant matter into fertilizer |
| Dome Dome | 104×52 | Assembly ring | Power | Builds a chosen dome kit for free, slowly (1–3 days) |
| Aqua Dome | 104×52 | 2 aquatic planters, fish pool | Water | Aquatic plants (high oxygen); fish (food) |
| Apartment Dome | 136×64 | 20 colonist berths, 6 worker beds | Power, water | Housing for the colonists. Counts toward readiness only when supplied |
| Barn Dome | 104×52 | 4 animal stalls, feed trough, output chest | Water, crops as feed | Animals from Earth give eggs, milk and fertilizer |
| Water Generator Dome | 72×44 | Condenser | Power | 90 L/day of water into its pipe network (more in rain) |

Every dome has a tooltip stating what it does and what it is missing.

### Farming

A planter holds one species. Plants need water (by hand with the Watering Can, from a sprinkler if the dome is on a water network, or from rain outdoors) and grow faster with fertilizer. Growth stops when dry. A mature plant is harvested for its products and usually a seed, then regrows or is replanted depending on the species.

## 7. Water

- **Sources:** a Pump placed in a lake (120 L/day while submerged) and Water Generator Domes.
- **Pipes:** copper pipes are laid on a grid of 4×4-cell tiles and may cross terrain. Touching pipes form a network. A network connects to any dome or machine whose footprint touches one of its tiles.
- **Consumers:** sprinklers in Green and Dark Domes (30 L/day), Aqua Dome (60), Barn (20), Kitchen (20), Bedroom (10), Apartment (40), drinking taps, and coolant for Oxygen Generators and Synthesizers (10; cooled machines run 50% faster). Water Tanks buffer 500 L.
- **Flow:** each network compares supply with demand every second. If supply covers demand, every consumer is satisfied and the surplus counts toward readiness. If not, consumers are served in priority order (drinking, sprinklers, animals, kitchens, coolant) and the rest report "no water".
- **Readability:** pipes animate in the flow direction when water moves. The Water overlay colours each network, labels it with supply/demand in L/day, and marks problems: no source, not enough supply, dead ends and unsupplied consumers.

## 8. Power

- **Battery flowers** sprout on the surface each dawn and can be farmed. A harvested flower is an item worth 100 charge.
- **Charging Pylons** hold up to 20 flowers. Pylons within 60 cells of each other link into a grid. Every dome and machine within 48 cells of a pylon draws from the grid's stored charge. Robots recharge by flying to a pylon.
- **Consumers:** Kitchen (20 charge/day), Water Generator (40), Dome Dome (60), Apartment (30), Oxygen Generator (50), Gene Splicer (per splice), robots (while working).
- **Readability:** powered machines show a green lamp, unpowered ones flash amber. The Power overlay draws pylon ranges and links and labels each grid with stored charge and drain per day.
- The starter dome has a small built-in generator that powers its own fixtures.

## 9. Atmosphere and plants

**Oxygen.** The atmosphere starts at 3% oxygen. Every plant in a dome releases oxygen according to its species, maturity and health; outdoor plants release half as much. Oxygen Generators add a fixed amount while powered. The HUD shows the level and its rate of change per day.

### Species and traits

Each species has numeric genes in 0–10:

| Gene | Effect |
|---|---|
| Oxygen | Oxygen released when mature |
| Growth | Speed from seed to maturity |
| Yield | Units of product per harvest |
| Power | Charge per harvested flower |
| Food | Food value of the crop |
| Light | Light it needs: low grows only in the dark, high only in light |
| Thirst | Water used per day (0 means it makes its own) |
| Hardy | Survives outdoors and cold |

plus a **product** (flower, crop, wood, fiber, fertilizer, gel, water), a **habitat** (land or aquatic) and **appearance genes** (height, stem curve, leaf shape and count, bloom shape, two hues, glow) that drive the procedural drawing.

| # | Species | Product | Notable genes |
|---|---|---|---|
| 1 | Voltbloom (battery flower) | Flower | Power 6 |
| 2 | Breathfern | — | Oxygen 7, Growth 7 |
| 3 | Ration Root | Crop | Food 4, Yield 5 |
| 4 | Amber Wheat | Crop | Food 3, Growth 8 |
| 5 | Sugar Reed | Crop | Food 5, Thirst 8 |
| 6 | Pepper Pod | Crop | Food 7, Yield 2 (valuable) |
| 7 | Starfruit Tree | Crop | Food 8, Growth 2 |
| 8 | Ironbark Sapling | Wood | Yield 6, Hardy 7 |
| 9 | Fiber Vine | Fiber | Yield 6, Growth 6 |
| 10 | Lung Moss | — | Oxygen 5, Thirst 2, Hardy 6 |
| 11 | Sunspire | Flower | Power 9, Growth 2, Light 9 |
| 12 | Dew Bulb | Water | Thirst 0, Yield 4 |
| 13 | Thornshield | — | Hardy 10, Oxygen 3 |
| 14 | Honeycup | Crop | Food 3; speeds up neighbouring planters |
| 15 | Gel Cactus | Gel | Thirst 1, Hardy 8 |
| 16 | Glowcap (dark) | Crop | Food 4, Light 1, glows |
| 17 | Sporebloom (dark) | Fertilizer | Yield 6, Light 1 |
| 18 | Night Orchid (dark) | Flower | Power 5, Light 0 |
| 19 | Kelp Ribbon (aquatic) | — | Oxygen 9 |
| 20 | Bubble Lotus (aquatic) | Crop | Food 5, Oxygen 5 |

### Gene splicing

The Gene Splicer takes one seed each of two species and makes a seed of their hybrid.

- Each numeric gene is taken from one parent, or averaged, chosen per gene by a hash of the two parents, then mutated by −1 to +1. One gene per hybrid gets hybrid vigour (+2, up to 10).
- The product comes from either parent, and one hybrid in four yields both products at 60% each.
- Aquatic only if both parents are; light need is the average, so land × dark crosses give shade plants that grow in either dome at reduced speed.
- Appearance genes blend, so a hybrid looks like its parents. Sprites are drawn procedurally from the genes.
- Names are built from the parents' name parts ("Voltfern", "Sugarcap"), with a prefix when a gene is exceptional ("Greater", "Thirsty", "Radiant").
- The same two parents always give the same hybrid, and hybrids can be spliced again. The codex records every species the colony has grown, with its genes and parents.

## 10. Robots and the sentence builder

Robots are small hover drones made from iron, silicon and gold. They carry 12 item stacks, hold 100 charge (a battery flower's worth), and recharge at Charging Pylons. They fly around obstacles (path-finding on a coarse grid) and pass through dome shells.

A robot's program is a list of up to 8 sentences built from drop-down word blocks.

```
Sentence  = [ ("WHEN" | "IF") Condition "," ] Step { ", then" Step } [ "UNTIL" Condition ] "."
Condition = "battery" "is" ("below" | "above") Percent
          | "inventory" "is" ("full" | "empty" | "not empty")
          | "it" "is" ("day" | "night" | "raining")
          | "a predator" "is" ("nearby" | "not nearby")
          | Place "has" ("less than" | "more than") Number Item
Step      = "go to" Place
          | "flee to" Place
          | "charge at" Place
          | "harvest" Crop "in" (Place | Area)
          | "plant" Seed "in" Place
          | "water" "plants in" Place
          | "fertilize" "plants in" Place
          | "mine" Ore "in" Area
          | "scavenge" "in" Area
          | "store" Item "in" Place
          | "take" Item "from" Place
          | "guard" Area
          | "wait" Seconds
Place     = a named dome or machine | "nearest dome" | "nearest storage" | "nearest pylon"
Area      = a named box dragged on the map
Crop/Seed/Ore/Item = a specific kind, or "anything"
```

**How a program runs.** Sentences are checked from the top every tick. The first sentence whose condition holds (a sentence without one always holds) and whose current step can run is the active one, so sentences higher up interrupt lower ones: put "WHEN battery is below 20%, charge at nearest pylon" first. Within a sentence the steps run in order and then repeat. A step ends when its work is done (nothing left to harvest, inventory full, arrived). An UNTIL condition ends the current step early and moves to the next. If a step can't run (the place is gone, the area holds no ore, nothing to store), the sentence is skipped this tick and marked with the reason.

**Readability.** A speech bubble and a status line show what the robot is doing and which sentence caused it ("Harvesting battery flowers in Green Dome 2 — sentence 2"). In the editor, the active sentence is highlighted green and sentences that can't run are highlighted amber with their reason.

## 11. Body mods

There are 53 mod sets. Each has tiers I–III, bought in order, plus a Legendary tier. Blueprints for a set come from Earth (the three example sets and a few basics are known from the start); each tier is then synthesized.

**Slots.** The body has eight slots: Head, Eyes, Torso, Back, Arms, Hands, Legs, Feet. A player can own any number of sets but equips one per slot, changed at the Mod Bay in the starter dome. This makes loadouts a choice (a miner, a farmer and a medic look and play differently) and keeps the sprite readable: each slot shows the equipped set at its tier.

**Tier costs.** I: iron, copper, silicon. II: adds gold. III: adds titanium and xenite. Legendary: adds aurorium.

### Feet

| Set | I | II | III | Legendary |
|---|---|---|---|---|
| **Rocket Feet** | Booster Thrust: double jump with a rocket burst | Rocket Hover: hold after the second jump for 2 s of thrust | Rocket Flight: 5 s of flight per jump | Perma-thrust: infinite flight |
| **Sticky Soles** | Gecko Grip: slide slowly down walls and wall-jump | Wall Crawl: climb walls | Cling Wrap: climb fast and hang without sliding | Gravity Is A Suggestion: climb at running speed, never slip |
| **Spring Heels** | Bounce: +30% jump height | Boing: +60%; landing on a creature hurts it | Kangaroo Court: hold down to charge a triple-height jump | Moon Rules: low gravity all the time |
| **Hustle Treads** | Brisk Walk: +20% speed | Jog Protocol: +40% | Zoomies: +60% | Late For Work: double speed and you run across water |
| **Aqua Flippers** | Doggy Paddle+: swim 50% faster | Dolphin Kick: swim twice as fast | Torpedo: triple swim speed; water costs no extra oxygen | Certified Fish: breathe underwater |
| **Stompers** | Heavy Landing: landing from a height digs a small crater | Crater Maker: bigger crater, hurts creatures | Seismic Drop: press down in the air to ground-pound | Tectonic Plates: huge ground-pound that stuns everything nearby |

### Legs

| Set | I | II | III | Legendary |
|---|---|---|---|---|
| **Dash Pistons** | Sidestep: short dash | Double Dash: two charges | Phase Dash: dash through creatures unharmed | Blink And You Miss Me: dash passes through thin walls |
| **Cargo Pants** | Deep Pockets: +10 inventory slots | Deeper Pockets: +20 | Pocket Dimension: +30 | Bag Of Holding My Beer: open the colony stockpile from anywhere |
| **Thermal Leggings** | Warm Knees: cold drains 50% slower | Toasty: immune to cold | Space Heater: warms teammates nearby | Walking Summer: plants near you ignore night and cold |
| **Root Walkers** | Green Stride: grass grows where you walk | Seed Trail: planters you pass are replanted from your seeds | Harvest Stride: plants you pass are harvested | Johnny Applesprint: plants you pass are harvested, replanted and watered |
| **Shock Absorbers** | Brace: take 15% less damage | Brace Harder: 30% less, no knockback | Braced For Impact: 45% less | Unbothered: 60% less; immune while standing still |
| **Jackhammer Knees** | Kneel Drill: hold down to dig beneath you | Power Squat: digs twice as fast | Free Fall Drill: keeps digging while you fall | Elevator Going Down: plunges at full speed and collects everything |

### Back

| Set | I | II | III | Legendary |
|---|---|---|---|---|
| **O2 Backpack** | Spare Lung: +50% oxygen | Scuba Flex: +100% | Tank Top: +200%, refills twice as fast | Photosynthesis, Baby: no drain in daylight |
| **Glide Wings** | Flappy: hold jump to fall slowly | Wingsuit: glide forward fast | Updraft: gain height while gliding in rain or wind | Technically Flying: glide without losing height |
| **Shoulder Turret** | Lil' Pew: auto-fires at the nearest predator | Pew Pew: fires faster | Pew Cubed: three targets | Pew Infinity: homing shots that never miss |
| **Battery Pack** | Extra Juice: +50% battery | Double A: +100% | Power Bank: +200%, no lockout when drained | Infinite Scroll: triple recharge |
| **Sprinkler Pack** | Mist Me: waters plants close to you | Drizzle: wider | Monsoon: wider, and plants grow 25% faster | Personal Raincloud: a cloud follows you; plants under it grow twice as fast |
| **Pack Mule Frame** | Lint Roller: pickup radius ×2 | Vacuum: ×3 | Hoover Dam: ×5, quick-stacks when you enter a dome | Everything Comes To Me: pickup across the screen, through walls |
| **Drone Dock** | Buddy: a drone follows and picks up drops | Buddy Pro: it also mines ore near you | Buddy Squad: two drones | Union Busted: four drones |
| **Beacon Rack** | Ping: drop a waypoint all teammates see | Recall: teleport home (cooldown) | Rally: teammates can teleport to you | Group Chat: teleport to any teammate or dome |

### Torso

| Set | I | II | III | Legendary |
|---|---|---|---|---|
| **Plating** | Tin Vest: +25 health | Steel Hoodie: +50 | Titanium Turtleneck: +100 | Legally A Tank: +200; attackers take damage |
| **Filter Lungs** | Sniff Test: spore gas hurts 50% less | Nose Plugs: immune to spore gas | Clean Air Act: clears gas around you | Breathes In Menacingly: gas you clear becomes oxygen for the planet |
| **Medi-Core** | Self-Care: slow health regeneration anywhere | Group Hug: heals teammates nearby | Defib: revives downed teammates nearby | HMO Premium Plus: the whole team regenerates anywhere |
| **Reactor Heart** | Pocket Reactor: slowly charges pylons near you | Walking Outlet: you power machines near you | Grid Daddy: larger radius | I Am The Power Plant: domes near you need no power |
| **Camo Skin** | Beige Mode: predators notice you from half as far | Ghosting: unseen while standing still | Left On Read: predators ignore you unless attacked | Do Not Disturb: predators flee from you |
| **Synth Belly** | Snack Crafting: synthesize start recipes anywhere | Bulk Order: recipes cost 15% less | Supply Chain: 30% less, drawing on the stockpile from anywhere | Free Shipping: half your crafts come out doubled |
| **Buddy Breather** | Share Air: teammates near you drain oxygen 50% slower | Air Supply: they don't drain at all | Lung Lease: you refill their tanks | Atmosphere Subscription: you count as a dome; air around you is breathable |

### Head

| Set | I | II | III | Legendary |
|---|---|---|---|---|
| **Antenna Array** | Bars: map reveals 50% further | Full Bars: caves near you appear on the map | 5G: ore veins appear on the map | Satellite Uplink: the whole map is revealed |
| **Trade Chip** | Haggle: sell for 10% more | Hard Bargain: 20% | Remote Work: 35%, and sell from anywhere | Insider Trading: 75% more |
| **Hive Mind** | Middle Manager: robots near you work 25% faster | Micromanager: 50%; edit programs from anywhere | Synergy: 100% | Hostile Takeover: every robot works twice as fast and uses no power |
| **Botanist's Bonnet** | Green Thumb: +25% yield harvesting by hand | Greener Thumb: +50%; see plant genes on hover | Greenest Thumb: double yield, extra seeds | Photosynthesis Hat: hybrids you splice get +1 to every gene |
| **Headlamp** | Flashlight: a cone of light | High Beams: wider and longer | Floodlight: lights the whole screen | Second Sun: light-loving plants grow near you even in caves |
| **Dome Brain** | Blueprint Memory: dome kits cost 10% less | Efficient Layouts: 20% | Arcology: 30%; Dome Domes near you build twice as fast | Big Dome Energy: domes you place produce 25% more |
| **Weather Vane** | Forecast: see the coming weather | Rain Dance: call rain (long cooldown) | Storm Chaser: you move and recharge faster in rain | Cloud Computing: it always drizzles on your domes; Water Generators +50% |

### Eyes

| Set | I | II | III | Legendary |
|---|---|---|---|---|
| **X-Ray Specs** | Squint: ore near you sparkles through rock | Stare: further, and shows creatures | Glare: further still | I Can See Your Skeleton: everything on screen |
| **Night Vision** | Carrot Diet: darkness is less dark | Cat Mode: much less | Owl Mode: nearly bright | Dark Mode Disabled: nothing is ever dark |
| **Threat Lens** | Lucky Shot: 10% of bolts crit for double | Weak Points: 20%; see creature health | Bullseye: 30%; crits triple | Aimbot (Legal): bolts curve toward predators |
| **Appraisal Monocle** | Finder's Fee: creatures drop 25% more | Good Eye: 50% | Connoisseur: rare drops appear | Loot Goblin: double drops |
| **Zoom Goggles** | Step Back: see 15% further | Wide Angle: 30% | Panorama: 50% | Eagle Eye: detach the camera to scout |
| **Geo Visor** | Long Reach Planning: place things 50% further away | Site Survey: valid dome sites are outlined | Remote Build: place from three times as far | Sim City Mode: place domes from the map |

### Arms

| Set | I | II | III | Legendary |
|---|---|---|---|---|
| **Extendo-Arms** | Arm++: longer reach for everything | Arm++ Premium: an extra arm that does a nearby task of your choice | Arm++ Ultra: a second automated arm; more reach | Armnipresence: a free-flying arm sent anywhere on the revealed map |
| **Drill Arms** | Dig Dug: dig 30% faster | Bore Dom: 60%, wider | Tunnel Vision: twice as fast, wider still | Mine Craft: rock vaporizes instantly |
| **Power Lifters** | Lift With Your Legs: stacks hold 50% more | Two Trips: double | One Trip: triple | Do You Even Lift: ten times |
| **Shield Arm** | Parry: a shield absorbs 20 damage, then recharges | Bubble: 40 | Deflector: 70, and reflects webs | Force Field Trip: covers teammates near you |
| **Farm Hands** | Quick Pick: harvest a whole dome at once | Quick Plant: plant a whole dome at once | Quick Everything: also waters and fertilizes | Agricultural Revolution: works on every dome in view |
| **Welder Arms** | Tack Weld: pipes go twice as far per item | Pipe Dream: four times | Hot Swap: dismantling refunds everything | Unionized: machines you place run 25% faster |

### Hands

| Set | I | II | III | Legendary |
|---|---|---|---|---|
| **Bolt Enhancements** | Overcharge: hold to charge a bigger, stronger bolt that uses more battery | Jack Up: bolts are larger and hit harder | Rug Pull: every shot fires two bolts for the price of one | Straight Up Scammed: bolts use 80% less energy |
| **Midas Mitts** | Sticky Fingers: +15% ore from digging | Gold Digger: +30% | Fool's Gold: +50%; stone sometimes yields gold | Everything I Touch: stone sometimes yields any ore |
| **Cryo Palms** | Chill Pill: bolts slow creatures | Brain Freeze: bolts freeze water into ice | Ice Age: frozen creatures shatter for extra damage | Absolute Zero Chill: a freezing aura around you |
| **Boom Mitts** | Pop: bolts burst on impact, digging a little | Bang: bigger bursts | Kaboom: bigger still, and sets things alight | Controlled Demolition: huge bursts that never hurt you or teammates |
| **Green Fingers** | Tickle: touch a plant to advance its growth (cooldown) | Poke: shorter cooldown | Coax: touched plants mature at once | Miracle Grow: plants near you grow three times as fast |
| **Grapple Glove** | Yoink: fire a short grapple line and pull yourself in | Long Yoink: longer; pulls drops and creatures to you | Swing State: swing on the line | Spider, Man: unlimited range |
| **Healing Hands** | High Five: touch a teammate to heal them | Fist Bump: revive downed teammates quickly | Finger Guns: a healing beam at range | Thoughts And Prayers (Effective): heal the whole team from anywhere |

### On the sprite

The player sprite is drawn in layers: body, suit, then one attachment per slot. Every set has its own attachment at each tier (212 in total), generated with the rest of the art. Tiers get larger and more elaborate; Legendary attachments glow and animate. Attachments follow per-frame anchor points on the body animation, so they stay attached while running, jumping and reaching, and they are shown on every player in multiplayer.

## 12. Inventory and quality of life

- 40-slot inventory grid with a 10-slot hotbar; stack sizes; drag and drop; right-click to split.
- Shift-click moves a stack between the inventory and the open chest or machine.
- Sort, quick stack to nearby chests, deposit all, search filter, tooltips, trash slot.
- Auto pickup with a magnet radius. Favourite an item (Alt-click) to lock it against quick stack, deposit, sort and trash.
- Map with fog of war, markers for domes, robots, teammates and pins, and robot areas drawn by dragging.
- Codex for plants, creatures and recipes.
- Every control is listed and rebindable under Settings → Controls, from the main menu and the pause menu.

## 13. Art and audio

Everything is generated by scripts in `tools/` and committed.

- **Look:** plant punk. Overgrown industrial ruins, moody light, rain and mist, cold machinery under lush alien growth. A muted teal, rust and moss palette with luminous accents.
- **Plants** are drawn procedurally in the game from their genes, so hybrids get their own look. Foliage sways in wind and bends away from players.
- **Sprites:** layered player with suit and mod attachments, domes (back wall, fixtures and glass front), machines, robots, creatures, item icons, UI ornaments and backgrounds.
- **Audio:** rain, wind, alien wildlife, dripping caves; moody ambient music by day, night and depth; machine hum, laser, digging per material, UI.

## 14. Milestones

1. World generation (surface, lakes, caves, ores), day/night and weather
2. Player, suit oxygen, starter dome, laser and battery, creatures
3. Inventory, chests, synthesizer, Comm. Terminal, credits
4. Domes and farming; battery flowers and power
5. Copper pipes and water
6. Atmosphere and oxygen generators; Colony Readiness and the win condition
7. Robots and the sentence builder
8. Gene splicer and plant genetics
9. Body mods: the 3 example sets, then the other 50
10. Art, animation, UI and audio polish
11. 8-player test, balancing and bug fixing

Each milestone is committed and pushed, builds and runs, and works with at least two players over LAN.

## 15. Progress log

- **Plan:** this document.
- **Milestone 1 (world, day/night, weather):**
  - The cell engine gained ores, dome glass and plating, ruins, spore gas and grass that spreads as the air improves.
  - `planetgen.rs` builds the 5120×1536 planet: rolling surface, lakes, a starter plateau, cave networks with surface entrances, three depth bands with richer ores further down, ruin vaults, Brood Mother lairs, alien trees and overgrown ruins.
  - The colony core (`sim::colony`) runs the clock, weather, players' vitals, digging and drops, the laser, creatures, inventories and chests, with tests for each.
  - The client was rebuilt on it: tiled world rendering, lighting that follows the sun, sky and horizon layers, rain and mist, layered player sprites, HUD, menus with new/load/join, world saves.
  - Two players over LAN see the same world, time and weather, each other, creatures and drops.
- **Milestones 2 and 3 (player, suit, laser, creatures; inventory, Synthesizer, Comm. Terminal, credits):**
  - Suit oxygen, cold, spore gas, health, blackout and respawn with a recoverable pack; the laser with its battery and lockout; seven creatures with their own behaviours and a spawner that follows depth and time of day.
  - Inventory window with drag and drop, split, shift-click, favourites, trash, sort, quick stack, deposit all, loot all and search; chests in domes, standalone chests, caches, packs and pods.
  - Synthesizer with recipes and blueprints, drawing on the player's inventory, the dome's chests and every Storage Dome.
  - Comm. Terminal: sell ores and goods, order seeds and animals (delivered by drop pod), buy blueprints, hire and assign workers.
  - Interaction prompts, the generated sprites for players (body, suit, team accents), creatures, props and domes.
