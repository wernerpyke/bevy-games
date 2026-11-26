//! Code shared by the games in this workspace. See `dev-docs/adr/EXTENSIONS.md` adr-x12.
//!
//! Before writing game code for movement, draw order, collision, maps, projectiles, particles or the HUD, look here
//! and in the symptom index in `dev-docs/extensions/README.md`. A kit feature derives, requires or warns about what a
//! spawn site needs (adr-x56).
//!
//! | Area | Modules | Guide in `dev-docs/extensions/` |
//! |---|---|---|
//! | Draw order | `depth` (sort by feet, `SortAtBlocker`, `TieBreakX`), `layers` (every z) | `draw-order.md` |
//! | Collision | `collision` (`Blocker`, `CollisionWorld`, sliding), `outline` (merged edges), `groups` (neighbour groups), `area` (ring, band, push geometry) | `collision.md`, `groups.md`, `area-effects.md` |
//! | Enemies | `movement` (`Mover`, goals), `strategies` (patrol, pursue, sentry), `nav` (path finding), `touch`, `hitbox`, `spawner`, `rng`, `movement_debug` | `enemy-movement.md`, `nav.md`, `spawners.md` |
//! | Shots and effects | `projectile` (straight and thrown), `fx` (Enoki emitters, follow an owner), `fx_strip` (particle sheets from `.aseprite`) | `projectiles.md`, `particles.md` |
//! | Pickups | `collectable` | `collectables.md` |
//! | Maps | `map` (parser), `cell_area` (areas of cells drawn as outlines, can change during play), `map_areas`, `map_links`, `map_edge`, `map_background`, `map_transition` (`MapLifecyclePlugin`, `MapTransitionPlugin`, `MapId`, `MapRoot`, `OnCurrentMap`, `MapHooks`, fades, seamless joins), `reload` (F1) | `map-files.md`, `map-transitions.md`, `cell-areas.md` |
//! | Screen | `camera`, `screen` (regions), `screen_fx` (camera shake), `hud`, `app_state` (`AppState`, `PlayState`, pause), `pointer` (clicks, `BlocksWorldClick`), `animation` | `screen-layout.md`, `screen-fx.md`, `pointer.md`, `pixel-art.md` |
//! | Debug | `debug` (F2 info, F3 grid), `collision_debug`, `script` (scripted runs) | `debug-tools.md`, `dev-feedback.md` |

pub mod animation;
pub mod app_state;
pub mod area;
pub mod camera;
pub mod cell_area;
pub mod collectable;
pub mod collision;
pub mod collision_debug;
pub mod debug;
pub mod depth;
pub mod fx;
pub mod fx_strip;
pub mod groups;
pub mod hitbox;
pub mod hud;
pub mod layers;
pub mod map;
pub mod map_areas;
pub mod map_background;
pub mod map_edge;
pub mod map_links;
pub mod map_transition;
pub mod movement;
pub mod movement_debug;
pub mod nav;
pub mod outline;
pub mod pointer;
pub mod projectile;
pub mod reload;
pub mod rng;
pub mod screen;
pub mod screen_fx;
pub mod script;
pub mod spawner;
pub mod strategies;
#[cfg(test)]
pub(crate) mod test_util;
pub mod touch;

use bevy::prelude::Component;

/// Marks the character the player controls. The debug info box reports on it, and the scripted run tool moves it.
#[derive(Component)]
pub struct Controlled;
