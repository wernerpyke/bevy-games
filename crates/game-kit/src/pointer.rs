//! Mouse clicks in world units. `PointerPlugin` sends `WorldClick` when a mouse button goes down inside the game
//! region of the window, with the position converted through the `GameCamera`. A click on the HUD bar sends nothing.
//! The kit decides nothing else: the game reads the message and chooses the result. See
//! `dev-docs/extensions/pointer.md`.
//!
//! The conversion runs in `PreUpdate`, so the camera is where it was drawn when the player clicked (the camera
//! follows in `Update`).

use bevy::input::InputSystems;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::camera::GameCamera;

/// A mouse button went down at `position` (world units). The scripted run tool sends it too (`click:<x>:<y>`).
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct WorldClick {
    pub position: Vec2,
    pub button: MouseButton,
}

/// The system that sends `WorldClick` runs in this set, in `PreUpdate`. A system that reads the message in `Update`
/// sees the click in the frame it happens.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct PointerSet;

pub struct PointerPlugin;

impl Plugin for PointerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<WorldClick>().add_systems(
            PreUpdate,
            send_clicks.in_set(PointerSet).after(InputSystems),
        );
    }
}

/// True when `cursor` (logical window pixels, top left origin) is inside `viewport`.
fn inside(viewport: Rect, cursor: Vec2) -> bool {
    cursor.cmpge(viewport.min).all() && cursor.cmplt(viewport.max).all()
}

/// The world position under a window cursor, or `None` when the cursor is outside the camera's viewport.
pub fn cursor_to_world(camera: &Camera, transform: &GlobalTransform, cursor: Vec2) -> Option<Vec2> {
    if !inside(camera.logical_viewport_rect()?, cursor) {
        return None;
    }
    camera.viewport_to_world_2d(transform, cursor).ok()
}

fn send_clicks(
    input: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<GameCamera>>,
    mut clicks: MessageWriter<WorldClick>,
) {
    if !input.get_just_pressed().next().is_some() {
        return;
    }
    let (Ok(window), Ok((camera, transform))) = (window.single(), camera.single()) else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Some(position) = cursor_to_world(camera, transform, cursor) else {
        return;
    };
    for &button in input.get_just_pressed() {
        clicks.write(WorldClick { position, button });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cursor_must_be_inside_the_viewport() {
        let game = Rect::new(0.0, 40.0, 800.0, 640.0);
        assert!(inside(game, Vec2::new(0.0, 40.0)));
        assert!(inside(game, Vec2::new(400.0, 300.0)));
        // The HUD bar above the game region.
        assert!(!inside(game, Vec2::new(400.0, 20.0)));
        assert!(!inside(game, Vec2::new(800.0, 300.0)));
        assert!(!inside(game, Vec2::new(400.0, 640.0)));
    }

    #[derive(Resource, Default)]
    struct Seen(Vec<WorldClick>);

    fn collect(mut messages: MessageReader<WorldClick>, mut seen: ResMut<Seen>) {
        seen.0.extend(messages.read().copied());
    }

    #[test]
    fn without_a_window_a_press_sends_nothing() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(PointerPlugin)
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<Seen>()
            .add_systems(Update, collect);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert!(app.world().resource::<Seen>().0.is_empty());
    }
}
