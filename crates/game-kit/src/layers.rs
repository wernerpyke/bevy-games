//! Draw order of everything on screen, from the back to the front. This is the only place with z numbers: other
//! files use these names. See `dev-docs/adr/EXTENSIONS.md` adr-x9.
//!
//! Sprites are ordered by their z (`Transform`), UI nodes by `GlobalZIndex`. Bevy has no layer system of its own
//! for this. Two sprites with the same z have no fixed order, so give overlapping sprites different z.
//!
//! | Layer | Kind | Value |
//! |---|---|---|
//! | Background | sprite z | `BACKGROUND_Z` |
//! | Background tile images | sprite z | `BACKGROUND_TILES_Z` plus `BACKGROUND_TILE_Z_STEP` per image |
//! | Grid debug | sprite z | `GRID_DEBUG_Z` |
//! | World | sprite z | within `WORLD_HALF_BAND` of 0, set by `depth.rs` from the feet |
//! | Feature debug | sprite z | `FEATURE_DEBUG_Z` |
//! | Movement debug | sprite z | `MOVEMENT_DEBUG_Z` |
//! | HUD | UI `GlobalZIndex` | `HUD_Z_INDEX` |
//! | Pause dim | UI `GlobalZIndex` | `DIM_Z_INDEX` |
//! | UI (dialogs) | UI `GlobalZIndex` | `UI_Z_INDEX` |
//! | Map fade | UI `GlobalZIndex` | `FADE_Z_INDEX` |
//!
//! Features (rocks, trees) and characters (the player, NPCs) are one layer, the world, sorted together by their
//! feet. Separate layers would stop a character walking behind a tree. The HUD and UI are drawn by `bevy_ui`
//! after all sprites and gizmos, so they are always above the feature debug layer.

/// The ground colour of a map: a rectangle the size of the map. The tile images are above it, see `BACKGROUND_TILES_Z`.
pub const BACKGROUND_Z: f32 = -500.0;
/// The tile images of a map background (`map_background.rs`) start here. Images overlap when they are jittered or
/// scaled, and sprites with the same z have no fixed order, so image number i is at `BACKGROUND_TILES_Z + i x
/// BACKGROUND_TILE_Z_STEP`. Room for `BACKGROUND_TILES_MAX` images below the grid.
pub const BACKGROUND_TILES_Z: f32 = -490.0;
pub const BACKGROUND_TILE_Z_STEP: f32 = 0.001;
pub const BACKGROUND_TILES_MAX: usize = 100_000;
/// The F3 grid lines and their numbers.
pub const GRID_DEBUG_Z: f32 = -200.0;
/// The world layer spans z from `-WORLD_HALF_BAND` to `WORLD_HALF_BAND`. With `depth.rs` at 0.001 per unit of
/// y, that holds feet from y -99000 to 99000 (checked in `depth.rs`).
pub const WORLD_HALF_BAND: f32 = 100.0;
/// Debug outlines of features, above everything in the world.
pub const FEATURE_DEBUG_Z: f32 = 200.0;
/// The movement debug view (goals, paths and spawners), above the collision overlay.
pub const MOVEMENT_DEBUG_Z: f32 = 203.0;
/// Sublayers of the feature debug layer, so overlapping debug sprites never share a z: the outline of interior
/// areas, then the feet markers.
pub const FEATURE_DEBUG_INTERIOR_Z: f32 = 201.0;
pub const FEATURE_DEBUG_FEET_Z: f32 = 202.0;
/// Z index of the HUD: the info box and other always-on-screen readouts.
pub const HUD_Z_INDEX: i32 = 0;
/// Z index of the dim layer over the HUD and the game while paused (`app_state.rs`), below the pause dialog.
pub const DIM_Z_INDEX: i32 = 50;
/// Z index of dialogs and menus, above the HUD.
pub const UI_Z_INDEX: i32 = 100;
/// Z index of the full-screen fade of a map transition (`map_transition.rs`), above the UI.
pub const FADE_Z_INDEX: i32 = 1000;

// Checked when the game is compiled.
const _: () = {
    assert!(FEATURE_DEBUG_FEET_Z < MOVEMENT_DEBUG_Z);
    assert!(BACKGROUND_Z < GRID_DEBUG_Z);
    assert!(BACKGROUND_Z < BACKGROUND_TILES_Z);
    assert!(
        BACKGROUND_TILES_Z + BACKGROUND_TILES_MAX as f32 * BACKGROUND_TILE_Z_STEP < GRID_DEBUG_Z
    );
    assert!(GRID_DEBUG_Z < -WORLD_HALF_BAND);
    assert!(WORLD_HALF_BAND < FEATURE_DEBUG_Z);
    assert!(
        FEATURE_DEBUG_Z < FEATURE_DEBUG_INTERIOR_Z
            && FEATURE_DEBUG_INTERIOR_Z < FEATURE_DEBUG_FEET_Z
    );
    // `OrthographicProjection::default_2d` shows z from -1000 to 1000.
    assert!(BACKGROUND_Z > -1000.0 && MOVEMENT_DEBUG_Z < 1000.0);
    assert!(HUD_Z_INDEX < DIM_Z_INDEX && DIM_Z_INDEX < UI_Z_INDEX && UI_Z_INDEX < FADE_Z_INDEX);
};
