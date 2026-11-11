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
    use crate::test_util::{Collected, collect};

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

    type Seen = Collected<WorldClick>;

    /// A game camera at (1000, 2000) that shows 800 by 600 world units in a viewport of 800 by 600 logical pixels,
    /// 40 pixels below the top of a window 640 high (a HUD bar above it).
    fn game_camera() -> (Camera, GlobalTransform) {
        use bevy::camera::{CameraProjection, RenderTargetInfo, Viewport};
        let mut projection = OrthographicProjection::default_2d();
        projection.update(800.0, 600.0);
        let mut camera = Camera {
            viewport: Some(Viewport {
                physical_position: UVec2::new(0, 40),
                physical_size: UVec2::new(800, 600),
                ..default()
            }),
            ..default()
        };
        camera.computed.clip_from_view = projection.get_clip_from_view();
        camera.computed.target_info = Some(RenderTargetInfo {
            physical_size: UVec2::new(800, 640),
            scale_factor: 1.0,
        });
        (camera, GlobalTransform::from_xyz(1000.0, 2000.0, 0.0))
    }

    /// Close enough for a point that went through a matrix and its inverse.
    fn near(a: Option<Vec2>, b: Vec2) -> bool {
        a.is_some_and(|a| a.distance(b) < 1e-3)
    }

    #[test]
    fn a_cursor_in_the_viewport_is_converted_through_the_camera() {
        let (camera, transform) = game_camera();
        let world = |cursor| cursor_to_world(&camera, &transform, cursor);
        assert!(near(
            world(Vec2::new(400.0, 340.0)),
            Vec2::new(1000.0, 2000.0)
        ));
        // The top left corner of the viewport is the top left of the view.
        assert!(near(world(Vec2::new(0.0, 40.0)), Vec2::new(600.0, 2300.0)));
        // On the HUD bar.
        assert_eq!(world(Vec2::new(400.0, 20.0)), None);
        // A camera that has not been given a render target yet.
        let mut unready = camera.clone();
        unready.computed.target_info = None;
        assert_eq!(
            cursor_to_world(&unready, &transform, Vec2::new(400.0, 340.0)),
            None
        );
    }

    fn pointer_app(cursor: Option<Vec2>) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(PointerPlugin)
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<bevy::picking::pointer::PointerMap>()
            .init_resource::<Seen>()
            .add_systems(Update, collect::<WorldClick>);
        let (camera, transform) = game_camera();
        app.world_mut().spawn((GameCamera, camera, transform));
        let location = cursor.map(|position| bevy::picking::pointer::Location {
            target: bevy::camera::NormalizedRenderTarget::None {
                width: 800,
                height: 640,
            },
            position,
        });
        app.world_mut()
            .spawn((PointerId::Mouse, PointerLocation { location }));
        app
    }

    #[test]
    fn a_press_in_the_game_region_sends_a_click_for_each_button() {
        let mut app = pointer_app(Some(Vec2::new(400.0, 340.0)));
        let mut input = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        input.press(MouseButton::Left);
        input.press(MouseButton::Right);
        app.update();
        let mut seen = app.world().resource::<Seen>().0.clone();
        seen.sort_by_key(|click| click.button != MouseButton::Left);
        assert_eq!(seen.len(), 2);
        for (click, button) in seen.iter().zip([MouseButton::Left, MouseButton::Right]) {
            assert_eq!(click.button, button);
            assert!(
                near(Some(click.position), Vec2::new(1000.0, 2000.0)),
                "{click:?}"
            );
        }
    }

    #[test]
    fn without_a_window_a_press_sends_nothing() {
        // The camera is there, but the mouse pointer has no place on a window.
        let mut app = pointer_app(None);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert!(app.world().resource::<Seen>().0.is_empty());
    }
}
