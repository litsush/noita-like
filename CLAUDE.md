# CLAUDE.md

## Prompt files: NEW or OLD

Work for this project is handed over in prompt files such as `prompt.txt` in the repo root. The first line of every prompt file is a status label:

- `STATUS: NEW`: not done yet. Carry out the prompt.
- `STATUS: OLD`: already done. Don't redo it. Only use it for context.

Whenever you get a new prompt, or are asked to work from a prompt file, check its status line first:

1. If it is `NEW`, do the work it describes.
2. When you have finished all of it, change the line to `STATUS: OLD`, then commit and push that change with the rest of the work so the next session won't redo it.
3. If it is `OLD`, don't redo it. Tell the user it is already marked done and ask what they want.
4. If a file has no status line, ask the user before acting on it.

Only mark a prompt `OLD` when it is actually finished. If you stop partway through, leave it `NEW` and note what is left in the prompt's plan or progress log.

## Project

A Noita-style falling-sand roguelike in Rust (Bevy 0.19, bevy_egui). See `README.md` for how to play and the dev flags, and `docs/roguelike-plan.md` for the design and progress log.

- `crates/sim`: simulation and world generation. No Bevy, deterministic from the seed, unit tested.
- `crates/net`: wire protocol and transports (Steam, TCP).
- `crates/game`: the Bevy app. The roguelike lives in `src/run/`.
- `tools/gen_art.py`, `tools/gen_audio.py`: generate every asset in `assets/`. Edit the scripts, rerun them, and commit the output. Don't hand-edit generated assets.

Before committing:

```sh
cargo fmt
cargo clippy --workspace --all-targets
cargo test --workspace
```

Set `SBCT_SAVE_DIR` to a temp folder when play-testing so the real save isn't touched.
