//! Mouse clicks in world units. `PointerPlugin` sends `WorldClick` when a mouse button goes down inside the game
//! region of the window, with the position converted through the `GameCamera`. A click on the HUD bar sends nothing.
//! A click on a UI node that carries `BlocksWorldClick` (or whose parent does), for example a drawer drawn over the
//! game region, sends nothing either. The kit decides nothing else: the game reads the message and chooses the
//! result. See `dev-docs/extensions/pointer.md`.
//!
//! The conversion runs in `PreUpdate`, so the camera is where it was drawn when the player clicked (the camera
//! follows in `Update`). The cursor position is the one of the picking pointer of the mouse, and the node under it
//! comes from the picking `HoverMap`, so the system runs after `PickingSystems::Hover`.

use bevy::input::InputSystems;
use bevy::picking::PickingSystems;
use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::{PointerId, PointerLocation};
use bevy::prelude::*;

use crate::camera::GameCamera;

/// Marks a UI node that takes the clicks on it: a mouse press on the node or on one of its children is not a
/// `WorldClick`. Put it on the root of a panel that is drawn over the game region.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct BlocksWorldClick;

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
            send_clicks
                .in_set(PointerSet)
                .after(InputSystems)
                .after(PickingSystems::Hover),
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

/// True when one of the hovered entities, or one of its parents, has the marker. `is_blocker` answers for one entity.
fn is_blocked(
    hovered: impl IntoIterator<Item = Entity>,
    parent_of: impl Fn(Entity) -> Option<Entity>,
    is_blocker: impl Fn(Entity) -> bool,
) -> bool {
    hovered.into_iter().any(|entity| {
        let mut current = Some(entity);
        while let Some(entity) = current {
            if is_blocker(entity) {
                return true;
            }
            current = parent_of(entity);
        }
        false
    })
}

fn send_clicks(
    input: Res<ButtonInput<MouseButton>>,
    pointers: Query<(&PointerId, &PointerLocation)>,
    hover: Option<Res<HoverMap>>,
    blockers: Query<(), With<BlocksWorldClick>>,
    parents: Query<&ChildOf>,
    camera: Query<(&Camera, &GlobalTransform), With<GameCamera>>,
    mut clicks: MessageWriter<WorldClick>,
) {
    if input.get_just_pressed().next().is_none() {
        return;
    }
    let Ok((camera, transform)) = camera.single() else {
        return;
    };
    let Some(cursor) = pointers
        .iter()
        .find(|(id, _)| **id == PointerId::Mouse)
        .and_then(|(_, location)| location.location.as_ref())
        .map(|location| location.position)
    else {
        return;
    };
    if let Some(hover) = hover
        && let Some(hits) = hover.get(&PointerId::Mouse)
        && is_blocked(
            hits.keys().copied(),
            |entity| parents.get(entity).ok().map(ChildOf::parent),
            |entity| blockers.contains(entity),
        )
    {
        return;
    }
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

    #[test]
    fn a_marked_node_or_a_child_of_it_blocks_the_click() {
        let panel = Entity::from_raw_u32(1).unwrap();
        let button = Entity::from_raw_u32(2).unwrap();
        let label = Entity::from_raw_u32(3).unwrap();
        let world_node = Entity::from_raw_u32(4).unwrap();
        let parent_of = |entity: Entity| {
            if entity == label {
                Some(button)
            } else if entity == button {
                Some(panel)
            } else {
                None
            }
        };
        let is_blocker = |entity: Entity| entity == panel;
        // The label is two levels below the marked panel.
        assert!(is_blocked([label], parent_of, is_blocker));
        assert!(is_blocked([panel], parent_of, is_blocker));
        // A node outside the panel, or nothing under the cursor, does not block.
        assert!(!is_blocked([world_node], parent_of, is_blocker));
        assert!(!is_blocked([], parent_of, is_blocker));
        // One blocking node among several hovered ones is enough.
        assert!(is_blocked([world_node, button], parent_of, is_blocker));
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
