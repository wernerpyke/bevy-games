//! The player dies when an enemy touches it: the kit reports the overlap of the two hit boxes (`game_kit::touch`,
//! the enemies have `Touchable`). For now death removes the player entity and its sprite (hit points and a death
//! screen come later). F1 resets the game state and creates the player again (`map.rs`).

use bevy::prelude::*;
use game_kit::touch::{ControlledTouched, TouchSet};

use crate::player::Player;

pub struct DeathPlugin;

impl Plugin for DeathPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, kill_player_on_touch.after(TouchSet));
    }
}

/// Runs in every frame in which an enemy touches the player. Does nothing while there is no player.
fn kill_player_on_touch(
    mut commands: Commands,
    mut touched: MessageReader<ControlledTouched>,
    player: Query<(Entity, &Transform), With<Player>>,
) {
    if touched.read().count() == 0 {
        return;
    }
    let Ok((entity, transform)) = player.single() else {
        return;
    };
    info!(
        "player: killed by an enemy at {:?}",
        transform.translation.truncate()
    );
    commands.entity(entity).despawn();
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_kit::Controlled;
    use game_kit::hitbox::HitBox;
    use game_kit::touch::{TouchPlugin, Touchable};

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins((TouchPlugin, DeathPlugin));
        app
    }

    fn body() -> HitBox {
        HitBox {
            offset: Vec2::new(0.0, 6.0),
            half_size: Vec2::new(4.0, 6.0),
        }
    }

    fn spawn_player(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((
                Player::default(),
                Controlled,
                body(),
                Transform::from_xyz(100.0, 100.0, 0.0),
            ))
            .id()
    }

    #[test]
    fn an_enemy_that_touches_the_player_kills_it() {
        let mut app = app();
        let player = spawn_player(&mut app);
        app.world_mut()
            .spawn((Touchable, body(), Transform::from_xyz(106.0, 100.0, 0.0)));
        app.update();
        assert!(app.world().get_entity(player).is_err());
    }

    #[test]
    fn an_enemy_that_only_stands_near_does_not() {
        let mut app = app();
        let player = spawn_player(&mut app);
        app.world_mut()
            .spawn((Touchable, body(), Transform::from_xyz(108.0, 100.0, 0.0)));
        app.update();
        assert!(app.world().get_entity(player).is_ok());
    }

    #[test]
    fn without_a_player_nothing_happens() {
        let mut app = app();
        app.world_mut()
            .spawn((Touchable, body(), Transform::from_xyz(0.0, 0.0, 0.0)));
        app.update();
    }
}
