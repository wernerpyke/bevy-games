//! The player dies when an enemy touches it: the two hit boxes overlap. For now death removes the player entity and
//! its sprite (hit points and a death screen come later). F1 resets the game state and creates the player again
//! (`map.rs`).

use bevy::prelude::*;
use game_kit::hitbox::{HitBox, overlaps};

use crate::enemy::Enemy;
use crate::player::Player;

pub struct DeathPlugin;

impl Plugin for DeathPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, kill_player_on_touch);
    }
}

/// Runs only while there is a player.
fn kill_player_on_touch(
    mut commands: Commands,
    player: Single<(Entity, &Transform, &HitBox), With<Player>>,
    enemies: Query<(&Transform, &HitBox), With<Enemy>>,
) {
    let (entity, transform, hit_box) = *player;
    let body = hit_box.rect_at(transform.translation.truncate());
    let touched = enemies
        .iter()
        .any(|(enemy, enemy_box)| overlaps(body, enemy_box.rect_at(enemy.translation.truncate())));
    if touched {
        info!(
            "player: killed by an enemy at {:?}",
            transform.translation.truncate()
        );
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(DeathPlugin);
        app
    }

    fn body() -> HitBox {
        HitBox {
            offset: Vec2::new(0.0, 6.0),
            half_size: Vec2::new(4.0, 6.0),
        }
    }

    #[test]
    fn an_enemy_that_touches_the_player_kills_it() {
        let mut app = app();
        let player = app
            .world_mut()
            .spawn((
                Player::default(),
                body(),
                Transform::from_xyz(100.0, 100.0, 0.0),
            ))
            .id();
        app.world_mut()
            .spawn((Enemy, body(), Transform::from_xyz(106.0, 100.0, 0.0)));
        app.update();
        assert!(app.world().get_entity(player).is_err());
    }

    #[test]
    fn an_enemy_that_only_stands_near_does_not() {
        let mut app = app();
        let player = app
            .world_mut()
            .spawn((
                Player::default(),
                body(),
                Transform::from_xyz(100.0, 100.0, 0.0),
            ))
            .id();
        app.world_mut()
            .spawn((Enemy, body(), Transform::from_xyz(108.0, 100.0, 0.0)));
        app.update();
        assert!(app.world().get_entity(player).is_ok());
    }

    #[test]
    fn without_a_player_nothing_happens() {
        let mut app = app();
        app.world_mut()
            .spawn((Enemy, body(), Transform::from_xyz(0.0, 0.0, 0.0)));
        app.update();
    }
}
