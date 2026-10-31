# Roadmap

## Items for Future Implementation

### 1. Art pipeline

Choose the palette and sprite size grid. Existing assets in `~/Dark/graveyard_shift/diodrama-assets/` are the starting point, copied one file at a time on request (`adr-a3`). `Guy.aseprite` is copied. The palette is chosen (`adr-a4`). Remaining: decide the size grid (64x64 cell accepted for the first test). Write `dev-scripts/aseprite/lib.lua`. Produce the first placeholder sprites.

## Completed Items

### Plugin compatibility check

Checked 2026-10-31. All candidate crates have releases that depend on Bevy 0.20. Versions and caveats are in `dev-docs/adr/ENGINE.md` adr-e4. WebAssembly behaviour of each crate is not yet verified and is checked when the crate is first added.

### Project scaffold

Done 2026-10-31. Bevy 0.20.1 (2D feature profile) with pinned Rust 1.97.1. Native build, `lint` and the `wasm32-unknown-unknown` check pass. `main.rs` opens a window with a nearest-sampling 2D camera only. The pixel art camera (fixed low resolution, integer scaling) and the web release pipeline (`wasm-bindgen`, HTML shell, hosting) are not done.

### First test: Guy walks with the arrow keys

Done 2026-10-31. Guy starts in the centre and walks with the arrow keys, using the `Idle`, `Walk`, `Up` and `Down` tags of `Guy.aseprite` through `bevy_aseprite_ultra`. Code is in `games/long-way/src/player.rs` and `camera.rs`. This is real-time movement to test the pipeline, not turn-based logic. It builds, lints and compiles for the web, and starts without errors. Key presses were not tested by Claude. The web build has not been run in a browser.

### Keep Guy on screen

Done 2026-10-31. Guy's 60x60 canvas is clamped to the visible area. The visible width follows the window aspect ratio at the fixed 180-pixel view height. The sideways walk direction was corrected (`WALK_ART_FACES_RIGHT = false`). Key presses were not tested by Claude. Build times are in `dev-docs/bevy-notes.md`.
