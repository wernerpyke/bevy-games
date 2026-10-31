# Workspace: decisions

## adr-w1. Folder layout and documentation structure (2026-10-30)

**Decision.** Use `docs/` for human docs, `dev-docs/` for developer docs and `dev-scripts/` for dev tooling. Keep `ROADMAP.md`, plans and ADRs in `dev-docs/`. The rules are in `CLAUDE.md` under Conventions.

**Rationale.** Keeping the three folders separate makes the audience of each file clear. `dev-docs/` is used instead of `notes/` so developer material stays out of any shipped web build. Decisions are one file per topic in `dev-docs/adr/` so they stay easy to find as they grow.

**How to apply.** Plans are `dev-docs/PLAN-*.md` and are deleted when done. New decision topics get a new file and a prefix in `dev-docs/adr/README.md`.

## adr-w2. No Python; Rust, Lua and shell only (2026-10-30)

**Decision.** The game is Rust. Aseprite scripts are Lua. Launcher scripts are shell. No Python and no `uv`.

**Rationale.** Aseprite requires Lua, so Python would be a third language for no gain.

**How to apply.** Ask before adding any other language.

## adr-w3. Cargo wrapper for allowlisting (2026-10-30)

**Decision.** Run cargo through `dev-scripts/rust.sh`. One allowlist rule, `Bash(./dev-scripts/rust.sh:*)`, covers build, test, run, lint and web check.

**Rationale.** One stable prefix means fewer permission prompts.

**How to apply.** Call it bare, without pipes, `;` or `$(...)`, or the rule does not match.

## adr-w4. Cargo workspace with one crate per game in `games/` (2026-10-31)

**Decision.** The repo is a Cargo workspace. The root `Cargo.toml` has `members = ["games/*"]`. Each game is a crate in `games/<name>/` with its own `src/`, `assets/` and `tests/`. The first game, Long Way, was renamed from the placeholder crate `game` and moved to `games/long-way/`.

**Rationale.** The developer wants to work on several games at once. Workspace members share one `target/` folder, so Bevy compiles once (about 8 minutes) instead of once per game. Dependency versions are set in one place, so every game uses the same Bevy version and the single `bevy-src/` reference stays correct for all of them.

**How to apply.** Add a game by creating `games/<name>/Cargo.toml` with `bevy.workspace = true`. Tell Claude which game each request is about. Profiles live only in the root `Cargo.toml`.

## adr-w5. Assets belong to a game and are copied when needed (2026-10-31)

**Decision.** Each game owns `games/<name>/assets/`. Art used by two games is copied into both. There is no shared assets folder.

**Rationale.** Bevy loads assets from the game crate's folder, so this needs no extra configuration. A change to one game's art cannot break another game. It also fits the copy-on-request rule for the legacy library (`dev-docs/adr/ART.md` adr-a3).

**How to apply.** Accept some duplication. If it becomes painful, revisit with a shared assets decision.

## adr-w6. No shared code crate until a second game needs it (2026-10-31)

**Decision.** Each game keeps its own camera, player and other code. A shared crate (`crates/<name>/`) is created only when a second game needs code that an existing game already has, and only after asking the developer.

**Rationale.** A shared API designed before a second game exists is likely to have the wrong shape. Copying and then extracting keeps the shared crate based on real use.

**How to apply.** When starting a second game, list the code it could reuse from Long Way and propose what to extract.

