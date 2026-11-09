//! Code shared by the games in this workspace. See `dev-docs/adr/EXTENSIONS.md` adr-x12.

pub mod animation;
pub mod app_state;
pub mod camera;
pub mod collectable;
pub mod collision;
pub mod collision_debug;
pub mod debug;
pub mod depth;
pub mod groups;
pub mod hitbox;
pub mod hud;
pub mod layers;
pub mod map;
pub mod map_areas;
pub mod map_background;
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
pub mod script;
pub mod spawner;
pub mod strategies;
pub mod touch;

use bevy::prelude::Component;

/// Marks the character the player controls. The debug info box reports on it, and the scripted run tool moves it.
#[derive(Component)]
pub struct Controlled;
