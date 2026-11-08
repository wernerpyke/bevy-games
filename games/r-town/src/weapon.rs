//! The player's projectile weapon: the fire key spawns a projectile that flies the way the player faces. The flight,
//! the hits and the removal are `game_kit::projectile`.

use bevy::prelude::*;
use game_kit::animation::tag_sprite;
use game_kit::depth::DepthSort;
use game_kit::projectile::{Projectile, ProjectileSprite};

use crate::input::{InputSet, PlayerInput};
use crate::map::MapEntity;
use crate::player::{Player, WALK_SPEED};

/// Projectile speed in art pixels per second: twice the walk speed.
const PROJECTILE_SPEED: f32 = 2.0 * WALK_SPEED;
/// Height of the projectile above the ground, about the player's hands.
const PROJECTILE_HEIGHT: f32 = 8.0;

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, fire.after(InputSet));
    }
}

/// The projectile starts at the player's feet, which are never inside a blocked area, so it cannot start inside a
/// tree band. Its sprite is a child lifted to hand height.
fn fire(
    mut commands: Commands,
    server: Res<AssetServer>,
    input: Res<PlayerInput>,
    player: Single<(&Transform, &Player)>,
) {
    if !input.fire {
        return;
    }
    let (transform, player) = *player;
    commands
        .spawn((
            Projectile::new(player.facing, PROJECTILE_SPEED).at_height(PROJECTILE_HEIGHT),
            MapEntity,
            DepthSort { feet_offset: 0.0 },
            Transform::from_translation(transform.translation.truncate().extend(0.0)),
            Visibility::default(),
        ))
        .with_child((
            ProjectileSprite,
            tag_sprite(&server, "art/features/projectiles.aseprite", "proj_8"),
            Transform::from_xyz(0.0, PROJECTILE_HEIGHT, 0.0),
        ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_projectile_is_twice_as_fast_as_the_player() {
        assert_eq!(PROJECTILE_SPEED, 160.0);
    }
}
