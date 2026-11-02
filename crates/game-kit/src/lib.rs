//! Code shared by the games in this workspace. See `dev-docs/adr/EXTENSIONS.md` adr-x12.

pub mod camera;
pub mod collision;
pub mod collision_debug;
pub mod debug;
pub mod depth;
pub mod layers;
pub mod map;
pub mod outline;
pub mod script;

use bevy::prelude::Component;

/// Marks the character the player controls. The debug info box reports on it, and the scripted run tool moves it.
#[derive(Component)]
pub struct Controlled;
