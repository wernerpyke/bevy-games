# Roadmap

## Items for Future Implementation

### 1. Art pipeline

Choose the palette and sprite size grid. Existing assets in `~/Dark/graveyard_shift/diodrama-assets/` are the starting point, copied one file at a time on request (`adr-a3`). `Guy.aseprite` is copied. The palette is chosen (`adr-a4`). Remaining: decide the size grid (64x64 cell accepted for the first test). Write `dev-scripts/aseprite/lib.lua`. Produce the first placeholder sprites.

### 2. Extract collision into shared code

Make `games/long-way/src/collision.rs` usable across games. Per `adr-w6` (`dev-docs/adr/WORKSPACE.md`), a shared crate is created only when a second game needs it, and only after asking the developer. Considerations: the file mixes general parts (`resolve`, the swept axis check) with Long Way specific parts (Guy's size and the three overlap rules for trees), so the extraction needs a clear split between engine-neutral code and per-game constants. A spec-first review on 2026-11-01 (`dev-docs/extensions/collision.md`) found what blocks the split: `outline.rs` is nearly standalone, but its tests import `GUY_HALF_WIDTH`. `collision.rs` depends on `Trunk` (a Long Way idea) and the `GUY_*` constants, and `move_feet` and `resolve` need an actor size parameter instead. `rocks.rs` and `trees.rs` depend on `camera::MAP_ORIGIN` and `map`. The outline, `resolve` and `move_feet` are the shared part, and `Trunk`, rocks and the overlay stay in the game. `depth.rs` is closely tied to collision (same feet line) and probably moves with it. Keep the unit tests with the code.

### 3. Evaluate avian2d

Spike `avian2d` 0.8.0 (declares `bevy ^0.20`) in a separate game under `games/` and compare it with the custom collision in `adr-lw1`. Questions to answer: whether the 50% overlap from below and the walk-behind-from-above rules can be expressed with sensors or collision filters, wasm build size and build time against the current baseline, behaviour with real-time top-down movement and later NPCs (`adr-lw7`), and how testable the result is without a running world. Check that it compiles for `wasm32-unknown-unknown`. Record the outcome as a decision in `dev-docs/adr/`. `bevy_rapier2d` 0.37.0 also declares Bevy 0.20 and can be included in the comparison. Not compiled yet.

### 4. Robust map and collision pipeline

The first rock map uses `include_str!` and an own parser for the frontmatter (`adr-lw8`, `adr-lw11`). For a real-time RPG with many areas, plan a sturdier approach. Considerations: a Bevy asset loader with hot reload (edit a map and see it without a rebuild, also on the web build); a real YAML parser or Tiled maps (`bevy_ecs_tiled`) instead of the own subset; validation with clear errors; one map file per area with chunked collision; dynamic obstacles (doors, destroyed objects) that rebuild one cluster; a broad phase for many moving entities; and a navigation mesh or occupancy grid for NPC pathfinding built from the same outline. Decide when the second map or the first NPC appears.

### 5. Alternative formats for map files

Maps are markdown files with frontmatter, a `# Features` section and a `# Map` section (`dev-docs/extensions/map-files.md` and `games/long-way/docs/maps.md`, `adr-lw8`). This suits small maps that a human reads and edits. Maps may become more complex than markdown handles well: many object kinds with their own settings, areas with triggers or dialogue, enemies and paths, several layers, and values that need real types (numbers, lists, nested data). Consider other formats, among them YAML and TOML. Questions to answer: how the painted `cells` grids fit in (a block string, or a separate file), how the features and the placements look in the new format, whether the human editing experience stays as good, comment support, error messages with line numbers, wasm support of the parser crate (check each crate for the pinned Rust and Bevy), and how it fits with roadmap item 4 (asset loader, hot reload, Tiled maps). Not worth doing now. Decide when a map needs data that markdown handles badly, and at the latest together with item 4. Keep the parser in `map.rs` behind one function (`parse`) so a format can be swapped without touching `rocks.rs`.

### 99. Debug grid: overlapping corner labels (low priority, cosmetic)

With the grid origin at the bottom left, the column "0" label (top edge) and the highest row label (left edge) overlap in the top left corner. Found in a screenshot on 2026-11-01. This is cosmetic and has low priority. Fix by moving the row labels, for example to the bottom of each cell, or by hiding the row label that sits on the top edge. The info box also covers the top left row labels, but F2 and F3 toggle each tool, so that needs no fix.

## Completed Items

### Tune sliding by playing

Done 2026-11-02. The developer played with rocks and trees and accepted the slide rules as they are (`adr-lw10`). The values stay: `SLIDE_RANGE_CAP` 64, `SLIDE_RANGE_FRACTION` 0.35, `SLIDE_CLEARANCE` 192, `SLIDE_DELAY` 0.2 s and `SLIDE_SPEED` 120. Options not built: a 96 px cap, or the nearest outline corner with a cap. Known issue: `SLIDE_CLEARANCE` assumes a 64 pixel cell, so write it in cells if the cell size changes. Not checked: a slide past a corner of the lower left patches, a tree touching a rock, and a flicker check of overlapping rocks over several frames.

### Rock clusters, trees and markdown maps

Done 2026-11-01. A markdown map file (`dev-docs/extensions/map-files.md` and `games/long-way/docs/maps.md`, `adr-lw8`, `adr-lw11`) places rocks and trees. Blocked areas of trunks and rocks merge into clusters with one outline (`outline.rs`), and Guy walks around the outline, sliding past edges and into notches (`adr-lw10`). Draw order uses layers (`adr-lw9`). The debug grid and collision overlay (F2) are sprites on the debug layers. A spec-first review of the collision code found and fixed several problems (the merge of trunks and rocks, corner-touching shapes, slide targets inside shapes). Research and geometry are in `dev-docs/extensions/collision-research.md`, the rules in `dev-docs/extensions/collision.md`, the values in `games/long-way/docs/collision.md`. Known issues and follow-ups: the player clamp to the view can move Guy's feet into a shape, Guy placed inside a shape can walk freely, `outline` builds one grid over all rectangles so large maps need chunking (item 4), and the maps are loaded with `include_str!` (item 4).

### Plugin compatibility check

Checked 2026-10-31. All candidate crates have releases that depend on Bevy 0.20. Versions and caveats are in `dev-docs/adr/ENGINE.md` adr-e4. WebAssembly behaviour of each crate is not yet verified and is checked when the crate is first added.

### Project scaffold

Done 2026-10-31. Bevy 0.20.1 (2D feature profile) with pinned Rust 1.97.1. Native build, `lint` and the `wasm32-unknown-unknown` check pass. `main.rs` opens a window with a nearest-sampling 2D camera only. The pixel art camera (fixed low resolution, integer scaling) and the web release pipeline (`wasm-bindgen`, HTML shell, hosting) are not done.

### First test: Guy walks with the arrow keys

Done 2026-10-31. Guy starts in the centre and walks with the arrow keys, using the `Idle`, `Walk`, `Up` and `Down` tags of `Guy.aseprite` through `bevy_aseprite_ultra`. Code is in `games/long-way/src/player.rs` and `camera.rs`. This is the real-time movement the game uses (`adr-lw7`). It builds, lints and compiles for the web, and starts without errors. Key presses were not tested by Claude. The web build has not been run in a browser.

### Keep Guy on screen

Done 2026-10-31. Guy's 60x60 canvas is clamped to the visible area. The visible width follows the window aspect ratio at the fixed 180-pixel view height. The sideways walk direction was corrected (`WALK_ART_FACES_RIGHT = false`). Key presses were not tested by Claude.

### Trees in Long Way

Done 2026-10-31. `Green tree.aseprite` and `Red tree.aseprite` are copied from the legacy library and drawn near Guy, each playing all 8 frames in a loop (`games/long-way/src/trees.rs`). Both carry a `Tree` marker component for the coming collision work. Both copies were resized to 128x128 with nearest-neighbour scaling (`dev-scripts/aseprite/resize.lua`); the library originals are unchanged (Green tree was 512x512, Red tree 64x64). Known issues: both trees use their own palettes (12 and 6 colours), not the game palette. Draw order is fixed (trees behind Guy), not depth-sorted. Not run in a browser.

### Tree collision

Done 2026-10-31. Guy cannot overlap a trunk from the left or right, can overlap it by 50% of his height when walking up from below, and can walk behind the tree from above but not into its bottom 10%. Draw order follows the feet so he passes behind and in front correctly. The rules are in `games/long-way/src/collision.rs` with 8 unit tests that pass. The blocked areas were first shown with F1 (now the F2 overlay). See `games/long-way/docs/adr.md`. Not tested by hand: the real feel at the screen edge, whether the debug gizmos draw on top of sprites, and the trunk edges (taken from art pixels) and Guy's body width (20 each side, an estimate) on screen.

### Forgiving collision (`allow_sliding`)

Done 2026-10-31. `CollisionPlugin { allow_sliding }` is set to `true` for Long Way. Walking straight into a trunk stops Guy. After 0.2 seconds of pushing (first 0.5, shortened after the developer tried it), if he is near an edge of the face (within 35% of its length), he slides past it and carries on. The middle of a face never slides him. With the flag off, any blocked move stops him completely. Fixed 2026-10-31 after the developer found it did not work: the push timer reset every few frames because directions were compared as floats (`adr-lw5`). Verified in the real game with scripted input: from feet x = 225 Guy stops at the trunk, slides left after the delay and walks on; from x = 240 (too far from an edge) he stays stopped. 16 unit tests pass. Tuning constants (`SLIDE_DELAY`, `SLIDE_SPEED`, `SLIDE_RANGE_FRACTION`) are guesses and not tested by hand. See `adr-lw4`.

### Visual debug tools and screenshots

Done 2026-11-01. `games/long-way/src/debug.rs` has a red grid (F2) with one line per 64 pixel cell, numbered along the top and left edges, drawn below the sprites, with a configurable origin (bottom left for Long Way). It also has an info box (F3) that any system can add values to, starting with Guy's position. The scripted input tool has a `screenshot:<name>` command that saves the window to `target/screenshots/` so Claude can check the result. See `adr-lw6` and `adr-lw5`. Known issues: the corner label overlap (roadmap item 99). With a bottom left origin the grid x numbers shift against the world when the window is resized, because the view width follows the window aspect ratio. Screenshot is native only.
