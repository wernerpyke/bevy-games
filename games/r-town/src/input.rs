//! What the player asks for this frame, independent of the device. Systems read `PlayerInput` and never `KeyCode`,
//! so a mouse can be added as a second source later. Planned to move to the kit (roadmap item 9).

use bevy::prelude::*;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerInput>()
            .add_systems(Update, read_keyboard.in_set(InputSet));
    }
}

/// `PlayerInput` is up to date after this set. Systems that read it run `.after(InputSet)`.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct InputSet;

#[derive(Resource, Default)]
pub struct PlayerInput {
    /// The direction asked for, not normalised: each axis is -1, 0 or 1.
    pub move_dir: Vec2,
    /// True for the one frame in which the fire key went down.
    pub fire: bool,
}

fn read_keyboard(keys: Res<ButtonInput<KeyCode>>, mut input: ResMut<PlayerInput>) {
    input.move_dir = direction(&keys);
    input.fire = keys.just_pressed(KeyCode::KeyX);
}

/// The direction the arrow keys or WASD give.
fn direction(keys: &ButtonInput<KeyCode>) -> Vec2 {
    let mut direction = Vec2::ZERO;
    if keys.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) {
        direction.x -= 1.0;
    }
    if keys.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) {
        direction.x += 1.0;
    }
    if keys.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) {
        direction.y += 1.0;
    }
    if keys.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
        direction.y -= 1.0;
    }
    direction
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pressed(keys: &[KeyCode]) -> Vec2 {
        let mut input = ButtonInput::<KeyCode>::default();
        for key in keys {
            input.press(*key);
        }
        direction(&input)
    }

    #[test]
    fn arrows_and_wasd_give_the_same_direction() {
        assert_eq!(pressed(&[KeyCode::ArrowUp]), Vec2::new(0.0, 1.0));
        assert_eq!(pressed(&[KeyCode::KeyW]), Vec2::new(0.0, 1.0));
        assert_eq!(
            pressed(&[KeyCode::KeyD, KeyCode::KeyS]),
            Vec2::new(1.0, -1.0)
        );
    }

    #[test]
    fn opposite_keys_cancel() {
        assert_eq!(
            pressed(&[KeyCode::ArrowLeft, KeyCode::ArrowRight]),
            Vec2::ZERO
        );
    }
}
