# Roadmap

## Items for Future Implementation

### 1. Forest of overlapping trees (next session, 2026-11-01)

Start of the next session: build a forest out of overlapping trees, and have Guy slide around the edges of the forest. Today `allow_sliding` (`adr-lw4`) slides Guy to the nearest edge of one trunk's blocked area. With overlapping trees the blocked areas merge into one larger shape, so sliding to the nearest edge of a single tree could push Guy into a neighbouring tree. Things to settle: how the edge of the merged shape is found (union of the blocked rectangles, or walking along them), how far Guy may slide before giving up, and whether the 35% range rule still makes sense for a long forest edge. Also check that depth sorting still looks right where tree canopies overlap, that the walk-behind rule (`adr-lw1`) works when Guy is behind several trees, and how trees are placed (a hand-written list for now is probably enough). Reproduce and check each case with the scripted input tool (`adr-lw5`) and add unit tests for the merged edges. Expect this to change `collision.rs`, so it affects roadmap item 3 (extracting collision). Decide the approach with the developer before building.

### 2. Art pipeline

Choose the palette and sprite size grid. Existing assets in `~/Dark/graveyard_shift/diodrama-assets/` are the starting point, copied one file at a time on request (`adr-a3`). `Guy.aseprite` is copied. The palette is chosen (`adr-a4`). Remaining: decide the size grid (64x64 cell accepted for the first test). Write `dev-scripts/aseprite/lib.lua`. Produce the first placeholder sprites.

### 3. Extract collision into shared code

Make `games/long-way/src/collision.rs` usable across games. Per `adr-w6` (`dev-docs/adr/WORKSPACE.md`), a shared crate is created only when a second game needs it, and only after asking the developer. Considerations: the file mixes general parts (`resolve`, the swept axis check, the F1 debug overlay) with Long Way specific parts (Guy's size and the three overlap rules for trees), so the extraction needs a clear split between engine-neutral code and per-game constants. `depth.rs` is closely tied to collision (same feet line) and probably moves with it. Keep the unit tests with the code.

### 4. Evaluate avian2d

Spike `avian2d` 0.8.0 (declares `bevy ^0.20`) in a separate game under `games/` and compare it with the custom collision in `adr-lw1`. Questions to answer: whether the 50% overlap from below and the walk-behind-from-above rules can be expressed with sensors or collision filters, wasm build size and build time against the current baseline, behaviour with a turn-based and grid-based game, and how testable the result is without a running world. Check that it compiles for `wasm32-unknown-unknown`. Record the outcome as a decision in `dev-docs/adr/`. `bevy_rapier2d` 0.37.0 also declares Bevy 0.20 and can be included in the comparison. Not compiled yet.

### 5. Debug grid: overlapping corner labels

With the grid origin at the bottom left, the column "0" label (top edge) and the highest row label (left edge) overlap in the top left corner. Found in a screenshot on 2026-11-01. Fix by moving the row labels, for example to the bottom of each cell, or by hiding the row label that sits on the top edge. The info box also covers the top left row labels, but F2 and F3 toggle each tool, so that needs no fix.

## Completed Items

### Plugin compatibility check

Checked 2026-10-31. All candidate crates have releases that depend on Bevy 0.20. Versions and caveats are in `dev-docs/adr/ENGINE.md` adr-e4. WebAssembly behaviour of each crate is not yet verified and is checked when the crate is first added.

### Project scaffold

Done 2026-10-31. Bevy 0.20.1 (2D feature profile) with pinned Rust 1.97.1. Native build, `lint` and the `wasm32-unknown-unknown` check pass. `main.rs` opens a window with a nearest-sampling 2D camera only. The pixel art camera (fixed low resolution, integer scaling) and the web release pipeline (`wasm-bindgen`, HTML shell, hosting) are not done.

### First test: Guy walks with the arrow keys

Done 2026-10-31. Guy starts in the centre and walks with the arrow keys, using the `Idle`, `Walk`, `Up` and `Down` tags of `Guy.aseprite` through `bevy_aseprite_ultra`. Code is in `games/long-way/src/player.rs` and `camera.rs`. This is real-time movement to test the pipeline, not turn-based logic. It builds, lints and compiles for the web, and starts without errors. Key presses were not tested by Claude. The web build has not been run in a browser.

### Keep Guy on screen

Done 2026-10-31. Guy's 60x60 canvas is clamped to the visible area. The visible width follows the window aspect ratio at the fixed 180-pixel view height. The sideways walk direction was corrected (`WALK_ART_FACES_RIGHT = false`). Key presses were not tested by Claude. Build times are in `dev-docs/bevy-notes.md`.

### Trees in Long Way

Done 2026-10-31. `Green tree.aseprite` and `Red tree.aseprite` are copied from the legacy library and drawn near Guy, each playing all 8 frames in a loop (`games/long-way/src/trees.rs`). Both carry a `Tree` marker component for the coming collision work. Both copies were resized to 128x128 with nearest-neighbour scaling (`dev-scripts/aseprite/resize.lua`); the library originals are unchanged (Green tree was 512x512, Red tree 64x64). Known issues: both trees use their own palettes (12 and 6 colours), not the game palette. Draw order is fixed (trees behind Guy), not depth-sorted. Not run in a browser.

### Tree collision

Done 2026-10-31. Guy cannot overlap a trunk from the left or right, can overlap it by 50% of his height when walking up from below, and can walk behind the tree from above but not into its bottom 10%. Draw order follows the feet so he passes behind and in front correctly. The rules are in `games/long-way/src/collision.rs` with 8 unit tests that pass. F1 shows the blocked areas. See `dev-docs/adr/LONG-WAY.md`. Not tested by hand: the real feel at the screen edge, whether the debug gizmos draw on top of sprites, and the trunk edges (taken from art pixels) and Guy's body width (20 each side, an estimate) on screen.

### Forgiving collision (`allow_sliding`)

Done 2026-10-31. `CollisionPlugin { allow_sliding }` is set to `true` for Long Way. Walking straight into a trunk stops Guy. After 0.2 seconds of pushing (first 0.5, shortened after the developer tried it), if he is near an edge of the face (within 35% of its length), he slides past it and carries on. The middle of a face never slides him. With the flag off, any blocked move stops him completely. Fixed 2026-10-31 after the developer found it did not work: the push timer reset every few frames because directions were compared as floats (`adr-lw5`). Verified in the real game with scripted input: from feet x = 225 Guy stops at the trunk, slides left after the delay and walks on; from x = 240 (too far from an edge) he stays stopped. 16 unit tests pass. Tuning constants (`SLIDE_DELAY`, `SLIDE_SPEED`, `SLIDE_RANGE_FRACTION`) are guesses and not tested by hand. See `adr-lw4`.

### Visual debug tools and screenshots

Done 2026-11-01. `games/long-way/src/debug.rs` has a red grid (F2) with one line per 64 pixel cell, numbered along the top and left edges, drawn below the sprites, with a configurable origin (bottom left for Long Way). It also has an info box (F3) that any system can add values to, starting with Guy's position. The scripted input tool has a `screenshot:<name>` command that saves the window to `target/screenshots/` so Claude can check the result. See `adr-lw6` and `adr-lw5`. Known issues: the corner label overlap (roadmap item 5). With a bottom left origin the grid x numbers shift against the world when the window is resized, because the view width follows the window aspect ratio. Screenshot is native only.
