# CLAUDE.md

## IMPORTANT: start the Rust LSP at the beginning of every session

**After the first prompt of every session, before doing anything else, start the Rust LSP server.** Do this even if the first prompt does not look like a coding task.

1. Load the deferred tool: ToolSearch with `select:LSP`.
2. Make one LSP call on a Rust file to start rust-analyzer, for example `documentSymbol` on `games/long-way/src/main.rs`. If it answers that the server is starting, carry on with the user's request and retry before you first need it. Indexing Bevy takes about a minute.
3. Use the LSP (goToDefinition, hover, findReferences) for all Rust and Bevy source reading afterwards, including `bevy-src/` and `~/.cargo/registry/src/`. Do not use `cat`, `sed` or grep to read Rust symbols. Use grep only for text search in examples and docs.

## Purpose

A workspace for 2D pixel art games built with Bevy (Rust). Long Way is a real-time top-down RPG (`games/long-way/docs/adr.md` adr-lw7). Several games are developed side by side. Claude writes all code. The user makes architectural decisions and does not hand-write code.

Game designs are not defined yet. Ask the user for design details before inventing mechanics.

User context: knows common game engine architecture and mechanics, so do not explain basics. Prior engines: LOVE2D, Pyxel. Pixel art editor: Aseprite. Prefers Rust for its strict compiler and linter.

## Conventions

- **Git:** Do NOT commit unless the human confirms. Include commits as steps in plans, but ask first. Remote is `https://github.com/wernerpyke/bevy-games.git`, branch `main`. Do not push unless asked.
- **Languages:** Rust (game), Lua (Aseprite scripts), shell (launchers). No Python and no `uv`. Ask before adding any other language (`dev-docs/adr/WORKSPACE.md` adr-w2).
- **Diagrams:** Mermaid only. Rules and sequence diagram template in `dev-docs/diagrams.md`. Read it before writing a diagram.
- **Decision records:** Add an entry in `dev-docs/adr/<TOPIC>.md` for every architectural decision the user makes. Record why, not implementation details. Format in `dev-docs/adr/README.md`.

### Documentation

| Location | Purpose | Audience |
|---|---|---|
| `CLAUDE.md` | Machine-readable project reference | Claude |
| `docs/` | User-facing guides. `docs/README.md` is the single entry point: quick start, commands, all troubleshooting. | Humans |
| `dev-docs/` | ADRs, Bevy notes, extensions, roadmap, plans. Map in `dev-docs/README.md`. | Contributors |
| `games/<game>/docs/` | One game: values, object kinds, source layout, art pipeline | Contributors |

- Keep general rules in `dev-docs/extensions/` and game values (sizes, speeds, art pixels, object kinds) in the game's docs. Do not mix them in one file.
- Do not create a `notes/` folder. `dev-docs/` keeps developer material out of any shipped web build.

### Plans

- Write a PLAN to `dev-docs/PLAN-{description}.md`.
- Always add a final step that migrates lasting content: user guides and troubleshooting to `docs/`, technical decisions and context to `dev-docs/`.
- When a plan is completed, migrate the content, then delete the plan file. Git history keeps it.

### Roadmap

`dev-docs/ROADMAP.md` has two H2 headings. **Items for Future Implementation** is numbered sequentially (renumber and fix cross-references when re-ordering). **Completed Items** is not numbered (remove the number when moving an item). Each item has an H3 heading with paragraphs on goal, considerations and known issues or learnings.

## Games and workspace rules

| Game | Crate and folder |
|---|---|
| Long Way | `long-way`, `games/long-way/` |
| R-Town | `r-town`, `games/r-town/` |
| Shared code | `game-kit`, `crates/game-kit/` (a library, not a game) |

Rules (`dev-docs/adr/WORKSPACE.md` adr-w4 to adr-w6):
- Every game is a workspace member in `games/<name>/` with its own `src/` and `assets/`. Crate name and folder name are the same, in kebab-case.
- Games are self-contained. A game never depends on another game's crate or reads another game's assets.
- Art is copied into the game that uses it. A palette or sprite used by two games is copied twice.
- Shared code lives in `crates/game-kit/` (`adr-x12`): collision, outline, depth sort, layers, map parser, camera, debug tools and the script runner. Move code there only when two games use it, and ask first. A game keeps its own values, object kinds, art and rules. The kit has no assets and no game types. Build and test it alone with `./dev-scripts/rust.sh test game-kit`.
- Dependency versions are set once in the root `Cargo.toml` under `[workspace.dependencies]`. Games use `bevy.workspace = true`. Build profiles are set only in the root `Cargo.toml`.
- All commands name the game. When a request does not say which game, ask.

## Targets and scope

- macOS (native) and Web (WebAssembly). Both must keep working. Do not add a dependency that breaks `wasm32-unknown-unknown` without telling the user first.
- 2D only. Disable unused Bevy default features where practical.
- Pixel art rendering: nearest-neighbour sampling, integer scaling, fixed low-resolution camera.
- Long Way is real-time: movement and collision use `time.delta_secs()`. It is not turn-based or grid-locked (adr-lw7). R-Town is also real-time (its `docs/SPEC.md`). Other games state their own timing model, so ask.

## Working rules

1. Ask before making an architectural decision (state layout, plugin boundaries, save format, crate choice). Give a recommendation with a short trade-off.
2. Default to a maintained crate when one exists and supports the pinned Bevy version and wasm. Write custom code when a crate is stale or too heavy. Say which you chose and why. Crate versions are in `dev-docs/adr/ENGINE.md` adr-e4. Long Way uses its own collision (adr-lw1). Do not add a physics crate before roadmap item 3 is decided.
3. Bevy is pinned to 0.20.1 in the root `Cargo.toml`. Check every Bevy ecosystem crate for compatibility with that version before adding it.
4. Bevy APIs change between releases. Do not rely on memory or older examples. Bundles such as `SpriteBundle` are removed. Use required components.
5. Bevy reference order: (a) `bevy-src/` (git-ignored shallow clone of tag `v0.20.1`; update it when the version changes), especially `examples/` and `_release-content/migration-guides/`; (b) the LSP on dependency source; (c) docs.rs for the pinned version. Search for the exact API. Do not read whole files.
6. Read `dev-docs/bevy/notes.md` (verified patterns) before searching again, and record new verified patterns there. Keep it short. When the project uses a new Bevy concept for the first time, add its section to `dev-docs/bevy/concepts.md` (plain language, project code as the example). Put our own reusable, non-standard designs (such as `allow_sliding`) in `dev-docs/extensions/`, and link their ADR.
7. Run `./dev-scripts/rust.sh lint` before reporting work as done, and `test` for each crate you changed that has tests (`game-kit` has them; a game may have none).
8. Run `./dev-scripts/rust.sh web` after adding dependencies, not only the native build.
9. When behaviour does not work in the real game, reproduce it with `./dev-scripts/rust.sh script` first, write a failing test for the cause, fix it, then rerun the script. Do not report a fix as working from unit tests alone. Say which parts you did not see on screen.
10. Match the style of surrounding code. Keep comments sparse.

## Architecture defaults (user may override)

- One Bevy plugin per feature area. Each plugin owns its components, resources, events and systems.
- Use Bevy `States` for app flow (loading, menu, playing, game over) and a sub-state or resource for play mode.
- Use events or messages between systems. Do not call across plugins directly.
- Keep game rules in plain Rust functions and types that do not need Bevy, so they are unit testable.
- Tests (`adr-w7`): code in `game-kit` is tested in `game-kit`. A game adds unit tests only for mechanics that are not in the kit, inline in the file they test. Do not test kit behaviour (collision, map parser, depth sort) or the bundled map file in a game, so maps can change freely. When a game rule moves to the kit, move its tests too.
- Draw order comes from `crates/game-kit/src/layers.rs` and a game's own overrides (adr-x9, adr-x14). Do not write bare z numbers or `GlobalZIndex` values elsewhere. Debug drawings are sprites on the right layer, not gizmos.
- Use `bevy_asset_loader` style loading states so the web build does not show missing assets.

## Art

Pipeline, palette rules and the list of copied files: `games/long-way/docs/art-pipeline.md`. Read it before any art work. Key rules:
- `.aseprite` files are the source of truth. Exports in `assets/generated/` are never edited by hand.
- **Do NOT copy, import, convert or reference any file from `~/Dark/graveyard_shift/diodrama-assets/` until the user names that file.** Never modify or delete anything there.
- Mark Claude-drawn placeholders with the `_placeholder` suffix.

## Commands

Use the wrapper so one allowlist rule `Bash(./dev-scripts/rust.sh:*)` covers all of them. Call it bare: no pipes, `;` or `$(...)`.

- Build, test: `./dev-scripts/rust.sh build|test [game]` (no game means all games)
- Run: `./dev-scripts/rust.sh run <game>` (add `--debug` to start with the F2 info box and the F3 grid visible; the flag is not passed to cargo)
- Scripted run: `./dev-scripts/rust.sh script <game> "<commands>" [map-file]` (the optional map file starts the game in that map, for example `2.txt`), for example `script long-way "teleport:225:-150;hold:up:3;quit"`. **When testing or taking screenshots, always add `--debug`** (for example `script long-way "wait:0.5;screenshot:start;quit" --debug`), so the F2 info box, the F3 grid and the collision and movement overlays are drawn. Without the flag they start hidden (F2 and F3 still toggle them). Add `screenshot:<name>` to save the window to `target/screenshots/<name>.png` (native only). Commands are in `crates/game-kit/src/script.rs` (adr-x5).
- Zoomed screenshot: `./dev-scripts/shot.sh <game> <name> "<commands>" cx=<x> cy=<y> w=<w> h=<h> scale=<n>` (world units, saves `target/screenshots/<name>_crop.png`). Aseprite checks: `./dev-scripts/aseprite.sh <check|frame_grid|sheet_analysis|crop> <file> key=value ...`. Details: `dev-docs/extensions/dev-scripts.md`. Allowlist: `Bash(./dev-scripts/shot.sh:*)`, `Bash(./dev-scripts/aseprite.sh:*)`.
- Lint (fmt check and clippy with `-D warnings`): `./dev-scripts/rust.sh lint`
- Web compile check: `./dev-scripts/rust.sh web`
- Fast dev builds: `./dev-scripts/rust.sh run long-way --features bevy/dynamic_linking` (native only, never for release or web)
- Web release build (to define): record the chosen method in `dev-docs/adr/ENGINE.md`.
- Disk use: `./dev-scripts/rust.sh size` (sizes of `target/`) and `./dev-scripts/rust.sh prune` (see "Disk use")

## Disk use

`target/` must stay under **8 GB**. A clean build of both games is 2.2 GB, and 3.2 GB with `lint`, `test` and `web` (`dev-docs/adr/ENGINE.md` adr-e8). Run this routine at the start of a session that builds Rust, and after any change to `Cargo.toml`.

1. Run `./dev-scripts/rust.sh size`.
2. Over 8 GB: tell the user, then run `./dev-scripts/rust.sh prune`.
3. Still over 8 GB after the prune: ask before `cargo clean`. A clean build takes about 10 minutes.
4. After a Bevy version or feature change, run `cargo clean` once. The old outputs are never used again.
5. Do not add `debug = true` or `bevy/dynamic_linking` as a default to a profile without asking. The user does not debug Rust by hand, so dependencies have no debug info and our crates have line tables only.
6. Delete files in `target/screenshots/` when the check that needed them is done.

`cargo-sweep` is optional and not installed. Other savings are roadmap item 90.
