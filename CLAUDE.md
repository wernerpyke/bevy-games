# CLAUDE.md

## IMPORTANT: start the Rust LSP at the beginning of every session

**After the first prompt of every session, before doing anything else, start the Rust LSP server.** Do this even if the first prompt does not look like a coding task.

1. Load the deferred tool: ToolSearch with `select:LSP`.
2. Make one LSP call on a Rust file to start rust-analyzer, for example `documentSymbol` on `games/long-way/src/main.rs`. If it answers that the server is starting, carry on with the user's request and retry before you first need it. Indexing Bevy takes about a minute.
3. Use the LSP (goToDefinition, hover, findReferences) for all Rust and Bevy source reading afterwards, including source in `bevy-src/` and `~/.cargo/registry/src/`. Do not use `cat`, `sed` or grep to read Rust symbols when the LSP can answer (working rule 5).

Starting it late wastes the indexing time and has led to reading Bevy source with grep by mistake.

## Conventions

### Git workflow

Claude does NOT commit to git unless the human confirms. Claude includes commits as steps in plan documents, but either asks the human to confirm the commit or commits only when instructed. This keeps every commit intentional and reviewed.

The remote is `https://github.com/wernerpyke/bevy-games.git`, branch `main`. Do not push unless the user asks.

### Languages

Rust (game), Lua (Aseprite scripts), shell (launchers). No Python and no `uv`. Ask before adding any other language (`dev-docs/adr/WORKSPACE.md` adr-w2).

### Documentation structure

| Location | Purpose | Audience |
|---|---|---|
| `CLAUDE.md` | Machine-readable project reference | Claude |
| `docs/` | User-facing guides, quick reference, consolidated troubleshooting | Humans |
| `dev-docs/` | Architecture decisions, roadmap, plans, internal context | Contributors |

- One file in `docs/` is the single entry point for human readers: quick start, common commands and all troubleshooting. `docs/README.md` is that file. Extra `docs/` files cover specific topics in depth.
- `dev-docs/` is used instead of `notes/` so developer material stays out of any shipped web build. Do not create a `notes/` folder.

### Plan documents

When asked to create a PLAN document:
- Write it to `dev-docs/PLAN-{description}.md`, for example `PLAN-implementation-strategy.md`.
- Always add a final step that migrates lasting content into long-lived docs: user guides and troubleshooting go to `docs/`, technical decisions and context go to `dev-docs/`.

When a plan is completed, migrate the useful content as above, then delete the plan file. Git history keeps it.

### Roadmap

`dev-docs/ROADMAP.md` lists future and completed work, under two H2 headings:
- **Items for Future Implementation**: active items, numbered sequentially. When re-ordering, renumber and update cross-references.
- **Completed Items**: not numbered. Remove the leading number when moving an item here.

Each item has its own H3 heading with paragraphs on the goal, considerations and known issues or learnings.

### Decision records

Decisions go in `dev-docs/adr/<TOPIC>.md`, one file per topic, in the format described in `dev-docs/adr/README.md` (`adr-<prefix><n>` IDs, with Decision, Rationale and How to apply). Record why a decision was made, not implementation details (git history holds those). Add an entry for every architectural decision the user makes.

### Diagrams

Always use Mermaid. No PlantUML or other tools. Embed diagrams inline in any `.md` file where they help. Do not commit rendered files (SVG, PNG).

Every sequence diagram uses this frontmatter:

```yaml
---
title: <title>
config:
  theme: neutral
  sequence:
    useMaxWidth: true
    wrap: false
    mirrorActors: false
    showSequenceNumbers: true
---
```

To highlight a message or range, wrap it in a `rect rgb(...)` block (nesting is allowed):

```
rect rgb(220, 245, 220)
  A->>B: highlighted message
end
```

When the user says "mark number N as <colour>", wrap the message with that rendered sequence number in a `rect` using the light shade below. For a range, wrap all of them in one `rect`.

| Colour | rgb |
|---|---|
| green | `rgb(220, 245, 220)` |
| red | `rgb(255, 230, 230)` |
| yellow | `rgb(255, 250, 205)` |
| blue | `rgb(220, 235, 255)` |
| orange | `rgb(255, 235, 205)` |
| grey | `rgb(235, 235, 235)` |

Mermaid draws `X->>X: ...` as a small curved loop that cannot become a box. For a rectangular internal action, use `note over X: ...` instead. The trade-off is that sequence numbering does not count notes.

## Directory structure

`docs/`, `dev-docs/` and `dev-scripts/` are kept separate.

```
bevy/
├── CLAUDE.md                # This file
├── Cargo.toml               # Workspace root: members games/*, shared dependency versions, build profiles
├── rust-toolchain.toml      # Pinned Rust version
├── games/                   # One crate per game (workspace members)
│   └── long-way/            # First game
│       ├── Cargo.toml
│       ├── src/             # Game code (Rust)
│       ├── assets/          # This game's runtime assets (Bevy loads from here)
│       │   └── art/         # .aseprite sources and palette
│       └── tests/           # Integration tests
├── docs/                    # Human-facing docs (README.md is the entry point)
├── dev-docs/                # Developer docs
│   ├── ROADMAP.md           # Future and completed work
│   ├── adr/                 # Decision records: README, ENGINE, ART, WORKSPACE, LONG-WAY
│   ├── bevy-notes.md        # Verified Bevy 0.20 patterns
│   ├── BEVY-CONCEPTS.md     # Human-friendly guide to how Bevy works
│   ├── BEVY-ADDITIONS.md    # Our own reusable design decisions built on Bevy
│   ├── art-tags.md          # Aseprite animation tag names
│   └── PLAN-*.md            # Temporary plans
├── dev-scripts/             # Dev tooling
│   ├── rust.sh              # cargo wrapper (allowlistable prefix)
│   └── aseprite/            # Lua scripts: lib.lua and per-sprite scripts
└── bevy-src/                # Bevy repo at v0.20.1 (git-ignored reference, not a dependency)
```

## Purpose

A workspace for 2D pixel art, turn-based games built with Bevy (Rust). Several games are developed side by side. Claude writes all code. The user makes architectural decisions and does not hand-write code.

Game designs are not defined yet. Ask the user for design details before inventing mechanics.

## Games

| Game | Crate and folder | Status |
|---|---|---|
| Long Way | `long-way`, `games/long-way/` | Walking test: Guy walks with the arrow keys, stays on screen, and a red and a green tree animate nearby and block him (F1 shows collision zones), with forgiving sliding past trunk edges |

Rules for the workspace (`dev-docs/adr/WORKSPACE.md` adr-w4 to adr-w6):
- Every game is a workspace member in `games/<name>/`, with its own `src/` and `assets/`. Crate name and folder name are the same, in kebab-case.
- Games are self-contained. A game never depends on another game's crate or reads another game's assets.
- Art is copied into the game that uses it. A palette or sprite used by two games is copied twice.
- Shared code stays in each game until a second game needs it. Then ask the user before creating a shared crate (`crates/<name>/`).
- Dependency versions (Bevy, plugins) are set once in the root `Cargo.toml` under `[workspace.dependencies]`. Games use `bevy.workspace = true`. Build profiles are set only in the root `Cargo.toml`.
- All commands name the game: `./dev-scripts/rust.sh run long-way`. When a request does not say which game, ask.

## Targets

- macOS (native)
- Web (WebAssembly, runs in browser)

Both targets must keep working. Do not add a dependency that breaks `wasm32-unknown-unknown` without telling the user first.

## Scope

- 2D only. No 3D features. Disable unused Bevy default features where practical to keep the web build small.
- Pixel art rendering: nearest-neighbour sampling, integer scaling, fixed low-resolution camera.
- Turn-based: game logic advances by turns, not by frame time. Keep turn logic separate from rendering and animation.

## User context

- Knows common game engine architecture and game mechanics. Do not explain basics.
- Prior engines: LOVE2D (Lua), Pyxel (Python).
- Pixel art editor: Aseprite. Prefer `.aseprite` files as the art source of truth.
- Prefers Rust for its strict compiler and linter.

## Working rules

1. Ask before making an architectural decision (state layout, plugin boundaries, save format, crate choice). Give a recommendation with a short trade-off.
2. Libraries versus custom code is open. Default to a maintained crate when one exists and supports the pinned Bevy version and wasm. Write custom code when a crate is stale or too heavy. Say which you chose and why.
3. Bevy is pinned to 0.20.1 (latest stable as of 2026-10-30) in the root `Cargo.toml` (`[workspace.dependencies]`). Check every Bevy ecosystem crate for compatibility with that version before adding it. Most Bevy plugins release one version per Bevy release.
4. Bevy APIs change between releases. Do not rely on memory or older examples. Check docs.rs and the official migration guide for the pinned version. Bundles such as `SpriteBundle` were removed in newer versions. Use required components.
5. Use the Rust LSP (rust-analyzer) through the LSP tool for code navigation and checks: go to definition, find references, hover for types and signatures, and diagnostics. Prefer it over grep for Rust symbols in this project and in dependencies. Use grep for text search in examples and docs. The LSP tool is deferred: load it at the start of work with ToolSearch (`select:LSP`). The server needs about a minute to index on first use, so retry once if it says it is starting. Use it (goToDefinition, hover) to read Bevy and plugin source in `~/.cargo/registry/src/`, not `cat` or `sed`.
6. Bevy reference material, in this order: (a) Bevy repo checked out at the pinned version's tag in `bevy-src/` (git-ignored, shallow clone of tag `v0.20.1`; update it when the Bevy version changes), especially `examples/` and `_release-content/migration-guides/`; (b) the LSP on the dependency source; (c) docs.rs for the pinned version. Search these for the exact API needed. Do not read whole files.
7. Record verified Bevy patterns for the pinned version in `dev-docs/bevy-notes.md` and read it before searching again. Keep it short. When the project uses a new Bevy concept for the first time, add or update its section in `dev-docs/BEVY-CONCEPTS.md` (plain language, project code as the example). The user is learning Bevy. Keep our own reusable, non-standard design decisions (such as `allow_sliding`) in `dev-docs/BEVY-ADDITIONS.md`, separate from plain Bevy concepts. Add an entry when we make such a decision, and link its ADR.
8. Run `./dev-scripts/rust.sh lint` before reporting work as done. Run `./dev-scripts/rust.sh test` when tests exist.
9. Run `./dev-scripts/rust.sh web` after adding dependencies, not only the native build.
10. When behaviour does not work in the real game, reproduce it with `./dev-scripts/rust.sh script` first, write a failing test for the cause, fix it, then rerun the script. Do not report a fix as working from unit tests alone. Say which parts you did not see on screen.
11. Match the style of surrounding code. Keep comments sparse.

## Architecture guidance (starting defaults, user may override)

- One Bevy plugin per feature area. Each plugin owns its components, resources, events and systems.
- Use Bevy `States` for app flow (loading, menu, playing, game over) and a sub-state or resource for turn phase.
- Use events or messages to pass actions between systems. Do not call across plugins directly.
- Keep game rules in plain Rust functions and types that do not need Bevy, so they are unit testable.
- Use `bevy_asset_loader` style loading states so the web build does not show missing assets.

## Candidate crates

Versions verified for Bevy 0.20 in `dev-docs/adr/ENGINE.md` adr-e4. Verify wasm builds when adding each crate.

| Need | Candidate |
|---|---|
| Aseprite import and animation | `bevy_aseprite_ultra` |
| Tilemaps | `bevy_ecs_tilemap`, `bevy_ecs_tiled` (Tiled editor maps) |
| Pathfinding | `bevy_northstar`, or `pathfinding` crate (engine independent) |
| Input mapping | `leafwing-input-manager` |
| Asset loading states | `bevy_asset_loader` |
| Debug UI | `bevy_egui`, `bevy-inspector-egui` |
| Physics | Not needed for turn-based. Use grid logic. Avoid `bevy_rapier2d` unless required. |

## Art pipeline

### Existing asset library (copy only on request)

Starting-point art lives in `~/Dark/graveyard_shift/diodrama-assets/` (55 `.aseprite` files, plus `.xcf`, `.tps` and `.jpg` files). It is a read-only source library.

- **Do NOT copy, import, convert or reference any file from it until the user names that file.** The user will say which file to copy across and when. Do not bulk copy the folder, and do not copy "related" files (palettes, other animations, `.xcf`, `.tps`) on your own initiative.
- Never modify or delete anything in the source library. Copy only.
- When the user names a file, ask which game it is for if that is not clear, copy it into that game's `assets/art/` (characters in `assets/art/characters/`, scenery in `assets/art/features/`), keeping its file name, then record it in that game's list below.
- Browsing the library to answer a question is allowed. Reading a file with the Aseprite CLI to inspect it is allowed. Writing outside the project is not.
- Licence: the game is for personal use. The user owns a licence for "Forest Graveyard" (GuttyKreum, GameDev Market), the source of the environment sprites. No redistribution is planned.

Files copied so far, by game:

Long Way (`games/long-way/assets/art/`):
- `diodrama_palette.aseprite` (from `diodrama_palette.aseprite`, 19 colours). The game's palette.
- `characters/Guy.aseprite` (from `characters/guy/Guy.aseprite`, 60x60, 41 frames, tags: Idle, Walk, Up, Down, Block, Win)
- `features/Green tree.aseprite` (from `features/`, resized in our copy from 512x512 to 128x128 with `dev-scripts/aseprite/resize.lua`, 8 frames, no tags, 12 colour palette, not the game palette)
- `features/Red tree.aseprite` (from `features/`, resized in our copy from 64x64 to 128x128 with `dev-scripts/aseprite/resize.lua`, 8 frames, no tags, 6 colour palette, not the game palette)

### Pipeline

Source of truth: `.aseprite` files in each game's `assets/art/` (`games/<name>/assets/art/`). Exported files go in `games/<name>/assets/generated/` and are generated, not edited by hand. Lua scripts in `dev-scripts/aseprite/` are shared by all games and take the game as an argument.

Tools:
- Aseprite CLI, run as `aseprite --batch --script <script.lua>`. Binary: `/Applications/Aseprite.app/Contents/MacOS/aseprite`.
- Lua scripts in `dev-scripts/aseprite/`. Claude writes and runs them.

Rules:
1. Write one shared helper first (`dev-scripts/aseprite/lib.lua`): new sprite, load palette, draw tile, outline, shade, tag frames, save. Sprite scripts call the helper. Do not repeat drawing code per sprite.
2. Use one fixed palette per game. Long Way: `games/long-way/assets/art/diodrama_palette.aseprite` (19 colours, index 0 is transparent black `#000000a0`). Scripts must not use colours outside it.
3. Use one tile and sprite size grid (open question: size not chosen, for example 16x16 or 32x32). All sprites are multiples of it.
4. Check each sprite visually: export a PNG scaled up with `--scale`, view it, fix, repeat. Stop after 3 rounds and ask the user if it still looks wrong.
5. Claude draws placeholders, tiles, icons, UI, effects and simple animation. The user draws or refines characters and key art. Mark placeholders with the suffix `_placeholder` in the file name so they are easy to find.
6. Load `.aseprite` files in the game through `bevy_aseprite_ultra` (verify it supports the pinned Bevy version). If it does not, export PNG sheets and JSON with `--sheet` and `--data` and load those.
7. Use layers and animation tags as the contract between art and code. Keep a list of tag names in `dev-docs/art-tags.md`.

## Skill

A community skill exists: https://www.skillsdirectory.com/skills/laurigates-bevy-game-engine

Install: `npx -y skills add laurigates/claude-plugins --skill bevy-game-engine --agent claude-code`

Caution: its sample code uses `SpriteBundle` and `delta_seconds()`, which are old APIs. It has no web/WASM guidance. Treat it as a general ECS reference only and verify all API use against the pinned Bevy version.

## Commands

Use the wrapper so one allowlist rule `Bash(./dev-scripts/rust.sh:*)` covers all of them. Call it bare: no pipes, `;` or `$(...)`.

- Build, test: `./dev-scripts/rust.sh build|test [game]` (no game means all games)
- Run: `./dev-scripts/rust.sh run <game>`, for example `run long-way`
- Run a game with scripted key presses and print its log: `./dev-scripts/rust.sh script <game> "<commands>"`, for example `script long-way "teleport:225:-150;hold:up:3;quit"`. Add `screenshot:<name>` to save the window to `target/screenshots/<name>.png` (native only), for example `script long-way "wait:0.5;screenshot:start;quit"`. Commands are in `games/long-way/src/script.rs`. Use it to check behaviour in the real game (`dev-docs/adr/LONG-WAY.md` adr-lw5).
- Lint (fmt check and clippy with `-D warnings`): `./dev-scripts/rust.sh lint`
- Web compile check: `./dev-scripts/rust.sh web`
- Fast dev builds: `./dev-scripts/rust.sh run long-way --features bevy/dynamic_linking` (native only, never for release or web)
- Web release build (to define): `cargo build --target wasm32-unknown-unknown --release`, then `wasm-bindgen` or a runner such as `trunk` or `bevy_cli`. Record the chosen method in `dev-docs/adr/ENGINE.md`.
