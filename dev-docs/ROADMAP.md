# Roadmap

## Items for Future Implementation

### 1. Art pipeline

Choose the palette and sprite size grid. Existing assets in `~/Dark/graveyard_shift/diodrama-assets/` are the starting point, copied one file at a time on request (`adr-a3`). `Guy.aseprite` is copied. The palette is chosen (`adr-a4`). Remaining: decide the size grid (64x64 cell accepted for the first test). Write `dev-scripts/aseprite/lib.lua`. Produce the first placeholder sprites.

### 2. Evaluate avian2d

Spike `avian2d` 0.8.0 (declares `bevy ^0.20`) in a separate game under `games/` and compare it with the custom collision in `adr-lw1`. Questions to answer: whether the 50% overlap from below and the walk-behind-from-above rules can be expressed with sensors or collision filters, wasm build size and build time against the current baseline, behaviour with real-time top-down movement and later NPCs (`adr-lw7`), and how testable the result is without a running world. Check that it compiles for `wasm32-unknown-unknown`. Record the outcome as a decision in `dev-docs/adr/`. `bevy_rapier2d` 0.37.0 also declares Bevy 0.20 and can be included in the comparison. Not compiled yet.

### 3. Robust map and collision pipeline

R-Town already reloads its map from disk on F1 on native (`adr-rt8`); Long Way and the web build do not. The maps use `include_str!` and an own parser for the front matter and the drawn grid (`adr-lw8`, `adr-x18`). For a real-time RPG with many areas, plan a sturdier approach. Considerations: a Bevy asset loader with hot reload (edit a map and see it without a rebuild, also on the web build); a real YAML parser or Tiled maps (`bevy_ecs_tiled`) instead of the own subset; validation with clear errors; one map file per area with chunked collision; dynamic obstacles (doors, destroyed objects) that rebuild one cluster; a broad phase for many moving entities; and a navigation mesh or occupancy grid for NPC pathfinding built from the same outline. Decide when the second map or the first NPC appears.

### 4. Map data beyond one grid

Maps are text files with front matter and one drawn grid, one character per cell (`dev-docs/extensions/map-files.md`, `games/long-way/docs/maps.md`, `adr-x18`, research in `dev-docs/RESEARCH-map-layouts.md`). This suits maps that a human reads and edits as a picture. Maps may become more complex than one object per cell handles well: several layers (ground and objects), objects with their own settings, areas with triggers or dialogue, enemies and paths, and values that need real types (numbers, lists, nested data). Options: more grids in the same file (a `layer:` name per grid), a companion YAML or TOML file for data, chunk files for large maps, and generated maps (a seeded generator, with a per-chunk seed from the master seed and the chunk position, can return the same `Map` as the parser). Questions to answer: how layers and larger objects look in the grid, whether the human editing experience stays as good, error messages with line numbers, wasm support of any parser crate (check each crate for the pinned Rust and Bevy), and how it fits with roadmap item 3 (asset loader, hot reload, Tiled maps). Not worth doing now. Decide when a map needs data that one grid handles badly. Keep the parser in `map.rs` behind one function (`parse`) so the format can change without touching the code that spawns objects.

### 5. Tiled backgrounds

Give maps a ground made of tiles, so R-Town can draw grass, paths, stone and water instead of one flat colour (the placeholder `GROUND_COLOR` in `games/r-town/src/map.rs`). The pack's `0-tileset.png` (320x192, a 16 px grid of 20x12 tiles) is the first source. Considerations: how a map file describes a tile layer (a new section in the markdown format, or an external editor format such as Tiled, which ties in with item 5); whether a maintained crate fits the pinned Bevy version and wasm (for example `bevy_ecs_tilemap`, to be checked before any decision) or custom code, with the chunked drawing that a 60x60 map needs; where tiles sit in the draw order (`BACKGROUND_Z` and its sublayers in the kit layers); how blocking tiles such as water feed `CollisionWorld` (`adr-rt5`: water blocks the player), merged into the same clusters as objects; packing the terrain tiles into an `.aseprite` file with tags (`adr-rt1`, `adr-rt6`); and whether the kit or the game owns the tile layer.

### 6. Visual map editor

Goal: edit a map by clicking cells and save the result in the text map format (`dev-docs/extensions/map-files.md`, `adr-x18`). The text file stays the source of truth, so editing by hand keeps working. Build it later, after the map format settles (items 3 and 4).

Considerations. The editor reads and writes the same files as the game, and a round trip must not change a file the user did not edit. It keeps comments, rulers, row labels, blank footprint cells and the front matter as written. The parser in `game-kit` (`parse`) does not keep these, so the editor needs a writer that edits the text in place, or a parser that returns the layout with the objects. Options: an in-game editor on top of the F3 grid and the debug tools (Bevy, native and web, no new language, and it draws the real art and footprints); or a separate tool. The language rule applies: Rust, Lua and shell only, and any other language needs approval. Features to consider: pick a kind from the legend and paint or erase cells, undo, a live footprint outline, the player start marker, a size change that adds or removes rows and columns, errors from `parse` shown in the editor, and a save that keeps `origin` (`bottom-left` or `top-left`) and the row labels right. Research on existing editors (Tiled, LDtk, Ogmo and the plain text editors for ASCII maps) is in `dev-docs/RESEARCH-map-layouts.md`. Those need their own UI and formats, so they are not a fit for now.

Known issues. The file has no footprint data, so the editor needs the sizes from the game (`Kind` and its art) to draw footprints. Layers and objects bigger than one cell were left out of the format on purpose, and an editor may bring them back (item 4). Seeded generated maps (item 4) would need a way to save a generated result and then edit it as a normal map.

### 7. Re-assess the group footprint rule

An item that stands alone blocks a thin band and a member of a group blocks its whole footprint (`adr-x20`, `adr-rt9`, `adr-lw14`). Check by playing whether this feels right and whether the area per member should change. Considerations: a thinner footprint for tall trees so the player can walk closer behind a grove; a footprint per kind; corner contact linking items (8 neighbours) so that a single diagonal neighbour turns a passable tree into a solid one; a deliberate gap size; and the Long Way top inset that is dropped when a member sits above (`Group::neighbour_above`). Known issue: a lone rock in Long Way now has a 12.8 px band, which the developer has not yet tried in play.

### 8. Keyboard and mouse input in the game-kit

Input handling starts as a game-local module in R-Town: a `PlayerInput` resource (move direction, fire) filled from the keyboard, read by the player and weapon code (`dev-docs/PLAN-projectiles.md`, D2). Move it to the kit when the mouse is added or a second game needs it. Considerations: one resource that games read, so no game code reads `KeyCode` or mouse buttons directly; keyboard and mouse as two sources that write the same resource, with the last used device winning; mouse aim converted from window position to world position through the pixel camera (integer scaling and the letterbox), and an aim direction that can replace the facing direction; named actions with default key bindings that a game can override, and later rebinding and gamepads; the scripted run tool (`script.rs`) must keep working by writing key presses into `ButtonInput`, and may need a way to place the mouse; wasm differences (pointer lock, touch). Keep game specific actions out of the kit, as for the other modules (`adr-x12`).

### 99. Debug grid: overlapping corner labels (low priority, cosmetic)

With the grid origin at the bottom left, the column "0" label (top edge) and the highest row label (left edge) overlap in the top left corner. Found in a screenshot on 2026-11-01. This is cosmetic and has low priority. Fix by moving the row labels, for example to the bottom of each cell, or by hiding the row label that sits on the top edge. The info box also covers the top left row labels, but F2 and F3 toggle each tool, so that needs no fix.

## Completed Items

### Text map files

Done 2026-11-02. A map is one text file with front matter and a drawn grid that shows every cell (`dev-docs/extensions/map-files.md`, `adr-x18`, `adr-lw8`, `adr-rt7`). The parser in `game-kit` reads blanks and `.` as empty, checks row labels, rulers and sizes, supports `origin: bottom-left` and `top-left`, and treats legend markers such as `player_start` as ordinary objects. Long Way (`rocks-test.txt`, 32x18) and R-Town (`test.txt`, 60x60, top-left) load their maps this way. The F3 grid can number rows from the top to match a top-left file. Research on other formats and on seeded generation is in `dev-docs/RESEARCH-map-layouts.md`. Known issues and follow-ups: no layers and one object per cell (roadmap item 4), footprint drawings are not checked against the real sizes, and the maps are loaded with `include_str!` (item 3).

### Shared code in the game-kit crate

Done 2026-11-02. A second game, R-Town, needed the code of Long Way, so the shared crate `crates/game-kit` was created (`adr-w6`, `adr-x12`). It holds the collision engine with merged outlines and sliding (`Blocker`, `SlideTuning`, `adr-x16`), the depth sort and default layers (`adr-x14`), the markdown map parser with string kinds (`adr-x13`), the pixel camera with a follow system (`adr-x17`), the debug grid, info box and collision overlay, and the scripted run tool (`adr-x15`). Games pass their values through plugin fields and the `Controlled` marker, with no generics. Long Way keeps `Trunk`, the Guy constants, its kinds and its art. The tests moved with the code, and the kit builds and tests on its own. Learnings: build and test the kit alone, because a workspace build unifies Bevy features and hid a missing `ui_api` feature. Debug sizes must scale with the cell size. Known issues and follow-ups: the scripted run tool has no way to register game specific commands yet (`adr-x15`), the kit has no camera shake or smoothing, and the scrolling debug grid was checked in R-Town only.

### Tune sliding by playing

Done 2026-11-02. The developer played with rocks and trees and accepted the slide rules as they are (`adr-x10`). The values stay: `SLIDE_RANGE_CAP` 64, `SLIDE_RANGE_FRACTION` 0.35, `SLIDE_CLEARANCE` 192, `SLIDE_DELAY` 0.2 s and `SLIDE_SPEED` 120. Options not built: a 96 px cap, or the nearest outline corner with a cap. Known issue: `SLIDE_CLEARANCE` assumes a 64 pixel cell, so write it in cells if the cell size changes. Not checked: a slide past a corner of the lower left patches, a tree touching a rock, and a flicker check of overlapping rocks over several frames.

### Rock clusters, trees and markdown maps

Done 2026-11-01. A markdown map file (replaced on 2026-11-02 by a text grid, `adr-x18`; `dev-docs/extensions/map-files.md` and `games/long-way/docs/maps.md`, `adr-lw8`, `adr-x11`) places rocks and trees. Blocked areas of trunks and rocks merge into clusters with one outline (`outline.rs`), and Guy walks around the outline, sliding past edges and into notches (`adr-x10`). Draw order uses layers (`adr-x9`). The debug grid and collision overlay (F3) are sprites on the debug layers. A spec-first review of the collision code found and fixed several problems (the merge of trunks and rocks, corner-touching shapes, slide targets inside shapes). Research and geometry are in `dev-docs/extensions/collision-research.md`, the rules in `dev-docs/extensions/collision.md`, the values in `games/long-way/docs/collision.md`. Known issues and follow-ups: the player clamp to the view can move Guy's feet into a shape, Guy placed inside a shape can walk freely, `outline` builds one grid over all rectangles so large maps need chunking (item 3), and the maps are loaded with `include_str!` (item 3).

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

Done 2026-10-31. Guy cannot overlap a trunk from the left or right, can overlap it by 50% of his height when walking up from below, and can walk behind the tree from above but not into its bottom 10%. Draw order follows the feet so he passes behind and in front correctly. The rules are in `games/long-way/src/collision.rs` with 8 unit tests that pass. The blocked areas were first shown with F1 (now the F3 overlay). See `games/long-way/docs/adr.md`. Not tested by hand: the real feel at the screen edge, whether the debug gizmos draw on top of sprites, and the trunk edges (taken from art pixels) and Guy's body width (20 each side, an estimate) on screen.

### Forgiving collision (`allow_sliding`)

Done 2026-10-31. `CollisionPlugin { allow_sliding }` is set to `true` for Long Way. Walking straight into a trunk stops Guy. After 0.2 seconds of pushing (first 0.5, shortened after the developer tried it), if he is near an edge of the face (within 35% of its length), he slides past it and carries on. The middle of a face never slides him. With the flag off, any blocked move stops him completely. Fixed 2026-10-31 after the developer found it did not work: the push timer reset every few frames because directions were compared as floats (`adr-x5`). Verified in the real game with scripted input: from feet x = 225 Guy stops at the trunk, slides left after the delay and walks on; from x = 240 (too far from an edge) he stays stopped. 16 unit tests pass. Tuning constants (`SLIDE_DELAY`, `SLIDE_SPEED`, `SLIDE_RANGE_FRACTION`) are guesses and not tested by hand. See `adr-x4`.

### Visual debug tools and screenshots

Done 2026-11-01. `debug.rs` (now in `crates/game-kit`) has a red grid (F3) with one line per 64 pixel cell, numbered along the top and left edges, drawn below the sprites, with a configurable origin (bottom left for Long Way). It also has an info box (F2) that any system can add values to, starting with Guy's position. The scripted input tool has a `screenshot:<name>` command that saves the window to `target/screenshots/` so Claude can check the result. See `adr-x6` and `adr-x5`. Known issues: the corner label overlap (roadmap item 99). With a bottom left origin the grid x numbers shift against the world when the window is resized, because the view width follows the window aspect ratio. Screenshot is native only.
