# Workspace: decisions

## adr-w1. Folder layout and documentation structure (2026-10-30)

**Decision.** Use `docs/` for human docs, `dev-docs/` for developer docs and `dev-scripts/` for dev tooling. Keep `ROADMAP.md`, `ROADMAP-completed.md`, plans and ADRs in `dev-docs/`. The rules are in `CLAUDE.md` under Conventions.

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

## adr-w6. No shared code crate until a second game needs it (2026-10-31, applied 2026-11-02)

**Decision.** Each game keeps its own camera, player and other code. A shared crate (`crates/<name>/`) is created only when a second game needs code that an existing game already has, and only after asking the developer.

**Rationale.** A shared API designed before a second game exists is likely to have the wrong shape. Copying and then extracting keeps the shared crate based on real use.

**How to apply.** When starting a second game, list the code it could reuse from Long Way and propose what to extract. Applied for R-Town: the shared crate is `crates/game-kit` (`adr-x12`). A game keeps its own values, kinds, art and rules, and the kit holds only what both games use.


## adr-w7. Shared rules are tested in `game-kit`, games test only their own mechanics (2026-11-06)

**Decision.** Code in `crates/game-kit/` is tested in `game-kit`. A game has unit tests only for mechanics that are not in the kit. A game has no tests for collision, map parsing, depth sort or other kit behaviour. A game does not test its bundled map file (its size, object count or cells), so the map can change freely.

**Rationale.** A rule tested in two places needs two updates. Tests of the map file break on every edit while the developer experiments with layouts. Kit code is the shared contract, so it carries the tests.

**How to apply.** When a game rule moves into the kit, move its tests with it (ask first, as for any move to the kit). Game tests stay inline in a `#[cfg(test)] mod tests` at the bottom of the file they test. Applied to Long Way on 2026-11-06 (`adr-lw17`).

## adr-w8. Roadmap items have stable IDs, and completed items move to their own file (2026-11-09)

**Decision.** Each roadmap item has an ID `rm<n>` that is assigned once and never reused or renumbered. The priority is a separate number in the item heading, and the list is sorted by it. Other documents refer to an item by its ID. Completed items move to `dev-docs/ROADMAP-completed.md`, where they have no ID.

**Rationale.** Priority changes often, so a number that means both position and identity broke every reference to it when items were re-ordered. About 25 documents cited items by number. A stable ID is the same idea as an ADR ID (`dev-docs/adr/README.md`). The completed items were 17 KB of history that every read of the roadmap paid for.

**How to apply.** Add an item with the next free `rm<n>` and a priority. To re-prioritise, change the priority number and move the item, never the ID. When an item is done, move it to `ROADMAP-completed.md` with its date and remove its ID, and update documents that cite the ID. An item closed without being done moves there too, with the reason. To merge items, the merged item keeps the lowest ID and its heading lists the absorbed IDs ("absorbs rm4 and rm6"); an absorbed ID is retired, never reused, and the documents that cite it are updated (the user, 2026-11-14). New IDs start at `rm101` (`rm100` is the highest ever used).
