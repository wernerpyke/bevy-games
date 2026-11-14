# Roadmap

Items that are done are in [ROADMAP-completed.md](ROADMAP-completed.md).

## Items for Future Implementation

Each item has a stable ID (`rm<n>`) that is assigned once and never reused or renumbered, the same rule as an ADR ID (`dev-docs/adr/WORKSPACE.md` adr-w8). Other documents refer to an item by its ID. The priority is the number in the heading, and the items are listed in priority order (the lowest number is next). To re-prioritise, change the priority number and move the item. Never change the ID. When items are merged, the merged item keeps the lowest ID and its heading lists the IDs it absorbed. An absorbed ID is retired and never reused.

### rm1. Art pipeline (priority 1)

Done: the palette is chosen and grows by appending colours (`adr-a4`, `adr-a5`, `adr-a8`, `adr-a10`); the map cell is 64 px and art is drawn at 1x or at a whole-number scale; the player is the Hunter, built from a sprite pack (`adr-lw15`, `games/long-way/docs/HUNTER-sprites.md`), with `Up` and `Down` drawn by the user and `Hurt` played when the player loses HP (`adr-lw28`); `Guy.aseprite` is deleted and the unused `Block` tag is gone. Existing assets in `~/Dark/graveyard_shift/diodrama-assets/` are still copied one file at a time on request (`adr-a3`).

Open:
- Decide whether the sprite sizes need one grid rule (now: Hunter 128x128 at 1x, trees 128x128, rocks 32x32 at 4x).
- Write the shared helper `dev-scripts/aseprite/lib.lua` from `art-pipeline.md` rule 1, or drop the rule: the scripts are standalone and work.
- An idle for each direction (only the side view has `Idle`).
- Play `Die` when the player dies. It waits for the death flow (`rm12`).
- Attack sheets from the pack (attack, charge, special, dash) and splitting their baked-in effects. Long Way's weapons are spells cast from the HUD, so ask the user whether the Hunter still needs melee or cast animations before doing this.

### rm3. Map format and tooling (priority 2, absorbs rm4 and rm6)

Maps are text files with front matter and one drawn grid, one character per cell (`dev-docs/extensions/map-files.md`, `games/long-way/docs/maps.md`, `adr-x18`), bundled with `include_str!` and parsed by our own code (`adr-lw8`). Done since this item was written: three Long Way maps joined by exits, entries and seamless doorways with a registry and a link check (`adr-x30`, `adr-x31`); an occupancy grid and A* path finding from the collision outlines (`adr-x23`); F1 reloads the map from disk on native (`reload.rs`, `adr-rt8`); the front matter holds nested blocks (`exits:`, `entries:`, `background:`), and a legend entry can carry settings (`adr-x25`, spawners).

Open, in three parts:
1. **Loading.** A Bevy asset loader with hot reload, so an edited map shows without a rebuild, also on the web build (F1 is native only). Validation with clear errors and line numbers. Dynamic obstacles (doors, destroyed objects) that rebuild one cluster (the kit has temporary blocked areas, `adr-x48`). A broad phase if many moving bodies come.
2. **Data beyond one grid.** Several layers (ground and objects), objects with their own settings, areas with triggers or dialogue, enemy paths, and typed values (numbers, lists, nested data). Options: more grids in one file (a `layer:` name per grid), a companion YAML or TOML file, chunk files for large maps, or seeded generated maps that return the same `Map` as the parser. Keep the parser behind one function (`parse`) so the format can change without touching the spawn code. Check any parser crate for wasm and the pinned Rust and Bevy.
3. **A visual editor.** Edit a map by clicking cells and save it in the text format, which stays the source of truth: a round trip must not change what the user did not edit (comments, rulers, row labels, blank footprint cells, front matter). The parser does not keep these, so the editor needs a writer that edits the text in place. Options: an in-game editor on the F3 grid and debug tools (Rust, native and web, draws the real art and footprints), or a separate tool (the language rule: Rust, Lua and shell only). Features: paint and erase with a legend kind, undo, live footprints and groups (`rm7`), the player start, resizing, errors from `parse`, and keeping `origin` and the row labels right. Existing editors (Tiled, LDtk, Ogmo) need their own formats, so they are not a fit for now. Build it after parts 1 and 2 settle.

Decide each part when a map needs it. Tiled maps (`bevy_ecs_tiled`) would touch all three and the ground layer (`rm5`).

### rm5. Ground and backgrounds (priority 3, absorbs rm14)

Done: a decorative ground, a colour and random tile images per map (`adr-x36`, `adr-lw26`). R-Town still draws one flat colour (`GROUND_COLOR` in `games/r-town/src/map.rs`).

Open:
- **A tile layer** with blocking tiles, so maps can draw grass, paths, stone and water. The R-Town pack's `0-tileset.png` (a 16 px grid of 20x12 tiles) is the first source. Considerations: how a map file describes the layer (ties in with `rm3`); a maintained crate that fits the pinned Bevy version and wasm (for example `bevy_ecs_tilemap`, to be checked) or custom code, with chunked drawing for a 60x60 map; the draw order (`BACKGROUND_Z` and its sublayers); blocking tiles such as water feeding `CollisionWorld` (`adr-rt5`) in the same clusters as objects; the terrain tiles packed into an `.aseprite` file with tags (`adr-rt1`, `adr-rt6`); and whether the kit or the game owns the layer.
- **The `pattern` mode** of the `background:` block. It is reserved and the parser rejects it (`adr-x36`). Decide how a pattern is written (a short repeat of image names, or a tile image per cell row) and how it combines with `jitter` and `scale`. It may be enough for a tiled R-Town ground. Ask the user what a pattern should look like before inventing the format.

### rm7. Collision feel by play (priority 4, absorbs rm11)

Goal: blocked areas match what the player sees, and the developer can tell why. Two parts.

**Groups.** Every kind has a rectangular footprint (`Kind::footprint`), and objects are grouped when footprints overlap or touch, also at a corner (`adr-lw14`, `adr-lw23`). A member of a group blocks its anchor cell (a tree: its trunk cell, `adr-lw18`) and a lone object blocks a thin band (`adr-x20`, `adr-rt9`). Weak points: the canopy counts as much as the trunk, and the rule is invisible. Steps, in the suggested order:
1. Show groups in the debug tools: draw each footprint, colour it by group, print the group size, and let a scripted run print the groups of a map.
2. Check by playing whether the rules feel right: a thinner footprint for tall trees, a deliberate gap size, the Long Way top inset dropped under a neighbour (`Group::neighbour_above`), and the 12.8 px band of a lone rock.
3. A solid shape per kind apart from its footprint (the cells that hold the trunk or base), preferably from a named Aseprite slice such as `solid` (check that `bevy_aseprite_ultra` reads slices), else a table per kind. Group on the solid cells and keep the footprint for placement and sorting.
4. Decide by gap width, not cell adjacency: the player is 28 px wide. A solid hedge needs its own kind or a legend parameter.
5. Keep corner contact for solid cells, drop it for canopy cells.

Known issues: a shrub diagonal to the empty canopy cell of a medium green tree (`1.txt`, col 6 row 1) becomes a full-cell block, as the rule says. The kit groups items of one size only, so the game expands footprints into cells first; a per-item footprint in `game_kit::groups` would be cleaner, and R-Town would follow.

**Markers.** A marker has a blocked area (the middle 80% of its width and the bottom half of its height, `adr-lw20`) and, with `exit_to`, a touch area 52 by 44 px (`markers.rs`, `adr-lw21`, `adr-x33`). One larger blocked area may be simpler (the player touches its edge to exit), at the cost of more blocked ground and different merging with trees and rocks. Questions: whether lanterns keep the small box, whether the arrival next to a marker still works (the `armed` rule, `adr-x33`), and whether the debug overlay draws the touch area. Compare by playing.

### rm8. Input and controls (priority 5, absorbs rm100)

Done: the kit turns a mouse click into a world position (`pointer.rs`, `WorldClick`, `adr-x42`); Long Way walks by click and by the arrow keys, opens a drawer with 1 to 4 and casts with Z, X, C and V (`adr-lw30`, `adr-lw38`); a cast key does nothing while its button is disabled for lack of energy (`adr-lw39`).

Open, in two parts:
1. **Long Way keyboard-only play** (wanted by the user, 2026-11-13):
   - Small key labels on each active weapon button (Z, X, C, V) and each gauge (1 to 4). Make them from `WEAPON_KEYS` (`hud.rs`), not fixed strings, so rebinding changes them. `KeyCode` is the physical key: on AZERTY the key at Z shows W; Bevy's logical `Key` gives the right label (check on the web too). The text must be readable at the window scale and must not cover the icon, the cost overlay or the gauge fill. Optional: show them only after the first key press.
   - Loading a weapon from the open drawer by keyboard, for example left and right to highlight and Space or Enter to load. The game does not pause while the drawer is open (`adr-lw31`), so decide first whether the drawer takes the arrow keys (the player stands), pauses the game, or uses other keys (the energy key again, or Q and E). Start the highlight on the active weapon, skip or allow dimmed weapons, decide whether it wraps, share one look with the mouse hover (last device wins), and decide whether Space loads only or also casts.
   - Escape closes the drawer. With a pause key later (`rm12`), Escape closes an open drawer first and pauses only when none is open.
   - Optional: a help overlay key (F1 to F3 are the dev tools), numpad 1 to 4.
   - When a cooldown comes (`rm9`), decide whether a held cast key casts again.
   - Add `enter` and `space` to the script runner when the drawer keys are built.
2. **One input layer in the kit.** No game reads `KeyCode` or mouse buttons directly. Today R-Town has its own `PlayerInput` (`games/r-town/src/input.rs`, `adr-x22`) and Long Way reads keys in `player.rs` and `hud.rs`. Considerations: named actions with default bindings a game can override, then rebinding and gamepads; keyboard and mouse writing the same state, last device wins; mouse aim for R-Town through the pixel camera (`pointer::cursor_to_world`) as an aim direction; the script runner writes into `ButtonInput` and must keep working; touch on the web (a first touch could send `WorldClick`); wasm differences (pointer lock, the browser scrolling on arrows and Space, Escape leaving full screen: the window's `prevent_default_event_handling`). Keep game actions out of the kit (`adr-x12`).

### rm9. Combat basics (priority 6)

Done: four weapons built and tuned (fire ring, ice ring, tornado, fireball, `games/long-way/docs/weapons.md`, `balance.md`), enemy HP and the ghost, player HP, energy cost per cast with a disabled button and cost overlay (`adr-lw39`), and weapons removed with their map. The fire wall and ice wall are planned in `PLAN-walls.md`.

Open (ask the user for the design before inventing mechanics):
- A cooldown per cast (the user's decision of 2026-11-11: a separate plan).
- Sound, impacts too.
- Enemy attacks other than the ghost's touch, and more enemy kinds (balance rules b2 and b13 wait for one).
- The fireball's later target rule (in a cluster, the enemy with the most HP).
- When several enemies attack at once, revisit the hit timing (the hit lands when the ghost's `Attack` ends).
- Exact blocked shapes for projectiles, if the 5 px early stop at tree sides is a problem (`adr-x21`).
- R-Town has no player HP.

The death and restart flow is in `rm12`.

### rm10. Web build and screen size (priority 7, absorbs rm13)

Done: the `wasm32-unknown-unknown` target is pinned (`adr-e5`), `./dev-scripts/rust.sh web` checks that the games compile for it, and `[profile.wasm-release]` is set for small builds (`adr-e6`). No game has run in a browser yet.

Open:
- **Build and host.** The tool that turns the `.wasm` into a page (`wasm-bindgen` CLI, Trunk or another, checked for the pinned versions); the HTML shell; asset loading over HTTP behind a loading state (`bevy_asset_loader` style), including `.aseprite` files through `bevy_aseprite_ultra`; the bundled `include_str!` maps (F1 reload and the script runner are native only); `wasm-opt` and the download size; one build per game; hosting (for example GitHub Pages for `wernerpyke/bevy-games`) without `dev-docs/`; a script that builds, serves and opens the page for testing. Record the method in `dev-docs/adr/ENGINE.md` and replace the "to define" lines in `CLAUDE.md` and `adr-e6`.
- **Screen size and shape.** Long Way has a fixed 576 by 756 window (`camera.rs`); a wider window shows more world, and nothing checks that the window fits the monitor (a 720 by 945 window did not fit this Mac). Considerations: read the monitor size and pick a window that fits; fullscreen and free resize; very wide or tall windows (more world, letterbox or a limited ratio, and what the camera bounds show); the integer scaling rule in `CLAUDE.md`, which the scale of 1.40625 already breaks (a whole-number scale, or accept a fractional one and say so); the web canvas and the browser window; whether the HUD is a fixed share of the view or adapts, for example to a phone shape; high-DPI screens; tests for the layout function with many window shapes. Ask the user for the target screens (desktop only, or phones and tablets on the web) first. Builds on `ScreenLayout` and `UiScale`, which already follow the window size.

### rm12. Death, restart and screens (priority 8)

Done in the kit: screen regions, the HUD bar, `AppState` with loading, title, playing and game over, and pause with a dim layer and an empty `PauseDialog` (`adr-x34`, `adr-x35`). Long Way's HUD has real content: weapon buttons, energy gauges and HP icons (`adr-lw34`). Long Way still starts in `Playing`, has no pause key, and at 0 HP the player stays alive and damage stops (`hp.rs`).

Open (ask the user what each screen shows before inventing it):
- **Death and restart:** the player plays `Die` (`rm1`), the game goes to `GameOver`, and a restart resets the game (maps, enemies, pickups, HP, energy, active weapons).
- **Title and game over screens** on the full window with their own background (`state_root`); the title may need the game camera off or a world behind it. Kit systems are not gated by state, so a game must not spawn the player in the title.
- **Pause:** a key, and content for the dialog, which sits in the game region and may need a full-window position. Escape must close an open weapon drawer first (`rm8`).
- **The overlay mode:** the HUD over a full-window game view. It is computed and tested but not used; its transparent root and picking need a try. A `ViewportNode` spike if one UI tree for all modes is wanted (not checked on wasm).
- Mouse to world must use the game viewport in every mode.

### rm90. Further disk savings for `target/` (priority 90)

`target/` is 2.9 GB after a clean build of all games and 4.3 GB with `lint`, `test` and `web` (5.2 GB after the first `run`; it was 25 GB before debug info was removed and 13 GB with stale and duplicate builds, `adr-e8`). `cargo-sweep` is still not installed. Only look at more savings if `target/` grows past the limit in `CLAUDE.md` ("Disk use") even with the routine. Options, each to be measured before it is kept: `split-debuginfo = "unpacked"` for `[profile.dev]` (macOS only), no incremental builds for dependencies, one shared `CARGO_TARGET_DIR` for all projects (saves space when they share a Bevy version, but all builds lock one folder), and `cargo-sweep` (`cargo install cargo-sweep`, then test that `--time` removes the right files on macOS; `rust.sh prune` already runs it when installed). Install `cargo-sweep` earlier if the project starts to use several Bevy versions or toolchains, or a shared target folder. Do not make `bevy/dynamic_linking` a default: it makes a second full set of outputs. Known learning: each game binary stays about 140 MB because Bevy is built at `opt-level = 3`. The savings came from `target/debug/deps`.
