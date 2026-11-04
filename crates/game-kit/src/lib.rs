//! Code shared by the games in this workspace. See `dev-docs/adr/EXTENSIONS.md` adr-x12.

pub mod camera;
pub mod collision;
pub mod collision_debug;
pub mod debug;
pub mod depth;
pub mod groups;
pub mod hitbox;
pub mod layers;
pub mod map;
pub mod movement;
pub mod movement_debug;
pub mod nav;
pub mod outline;
pub mod projectile;
pub mod rng;
pub mod script;
pub mod spawner;
pub mod strategies;

use bevy::prelude::Component;

/// Marks the character the player controls. The debug info box reports on it, and the scripted run tool moves it.
#[derive(Component)]
pub struct Controlled;
