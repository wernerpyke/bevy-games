//! The pixel art camera: a 2D camera that shows a fixed height of world units, so art scale on screen comes from
//! one number and not from the window size. See `dev-docs/extensions/pixel-art.md`.

use bevy::camera::ScalingMode;
use bevy::prelude::*;
use bevy::transform::TransformSystems;

use crate::Controlled;

/// Marks the camera that shows the world. Every kit system that needs the world camera asks for this marker and not
/// for `Camera2d`, because a second camera (the `HudCamera`, `screen.rs`) would make a `Single<.., With<Camera2d>>`
/// match two entities and Bevy skips such a system without a warning. See `dev-docs/adr/EXTENSIONS.md` adr-x34.
#[derive(Component)]
pub struct GameCamera;

/// Spawns the 2D camera (a `GameCamera`) at the world origin and sets the clear colour. A game that scrolls moves
/// that entity itself. The camera is also the default UI camera, so UI roots without a `UiTargetCamera` (the debug
/// info box, the map fade) are drawn in the game region.
pub struct PixelCameraPlugin {
    /// Height of the view in world units (art pixels). On-screen size of a sprite is its height divided by this,
    /// as a share of the screen height.
    pub view_height: f32,
    pub clear_color: Color,
}

impl Plugin for PixelCameraPlugin {
    fn build(&self, app: &mut App) {
        let view_height = self.view_height;
        app.insert_resource(ClearColor(self.clear_color))
            .add_systems(Startup, move |commands: Commands| {
                spawn_camera(commands, view_height)
            });
    }
}

fn spawn_camera(mut commands: Commands, view_height: f32) {
    commands.spawn((
        Camera2d,
        GameCamera,
        IsDefaultUiCamera,
        Msaa::Off,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: view_height,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}

/// The part of the world the camera shows. `None` for a camera that is not orthographic or has no area yet.
pub fn visible_rect(camera: &GlobalTransform, projection: &Projection) -> Option<Rect> {
    let Projection::Orthographic(orthographic) = projection else {
        return None;
    };
    let area = orthographic.area;
    if area.width() <= 0.0 || area.height() <= 0.0 {
        return None;
    }
    Some(Rect::from_center_size(
        camera.translation().truncate() + area.center(),
        area.size(),
    ))
}

/// Half the visible area in world units for a camera at the origin that shows `view_height` of height. The visible
/// width follows the window aspect ratio.
pub fn half_view(window_width: f32, window_height: f32, view_height: f32) -> Vec2 {
    Vec2::new(view_height * window_width / window_height, view_height) / 2.0
}

/// The world rectangle the camera must stay inside. A game inserts it once the map size is known. Without it the
/// camera follows without limits.
#[derive(Resource)]
pub struct CameraBounds(pub Rect);

/// Makes the camera glide when `CameraBounds` change, instead of jumping to where the new bounds put it. The camera
/// keeps following the player exactly. Only the part that the bounds change added is eased out: the camera starts
/// where it was and the difference decays with time constant `tau` seconds (about two thirds of it in `tau`). Without
/// this resource the camera always jumps. See `dev-docs/adr/EXTENSIONS.md` adr-x32.
#[derive(Resource)]
pub struct CameraSmoothing {
    pub tau: f32,
    /// Set to make the next change of the bounds jump (the screen is covered, as in a scene transition). Cleared by
    /// the camera after one frame.
    pub snap: bool,
}

/// The part of a camera move that is still to be eased out: the offset shrinks with time constant `tau`, and is
/// dropped when it is under a quarter of a unit.
pub fn smooth_offset(offset: Vec2, dt: f32, tau: f32) -> Vec2 {
    if tau <= 0.0 {
        return Vec2::ZERO;
    }
    let eased = offset * (-dt / tau).exp();
    if eased.length_squared() < 0.0625 {
        Vec2::ZERO
    } else {
        eased
    }
}

/// What `follow_controlled` remembers between frames when `CameraSmoothing` is present.
#[derive(Default)]
struct SmoothState {
    last_bounds: Option<Rect>,
    offset: Vec2,
}

/// Moves the `Camera2d` to follow the `Controlled` entity (`CameraBounds` limits it). The camera position is
/// rounded to a whole world unit, so pixel art does not shimmer when it scrolls.
pub struct CameraFollowPlugin;

impl Plugin for CameraFollowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            follow_controlled.before(TransformSystems::Propagate),
        );
    }
}

fn follow_controlled(
    target: Single<&Transform, (With<Controlled>, Without<GameCamera>)>,
    mut camera: Single<(&mut Transform, &Projection), With<GameCamera>>,
    bounds: Option<Res<CameraBounds>>,
    smoothing: Option<ResMut<CameraSmoothing>>,
    time: Res<Time>,
    mut state: Local<SmoothState>,
) {
    let (transform, projection) = &mut *camera;
    let Projection::Orthographic(orthographic) = projection else {
        return;
    };
    let half = orthographic.area.size() / 2.0;
    if half.x <= 1.0 || half.y <= 1.0 {
        return;
    }
    let bounds = bounds.map(|bounds| bounds.0);
    let desired = follow_position(target.translation.truncate(), half, bounds);
    let mut position = desired;
    if let Some(mut smoothing) = smoothing {
        if state.last_bounds.is_some() && state.last_bounds != bounds {
            // The bounds changed: start from where the camera is, and ease out the difference.
            state.offset = if smoothing.snap {
                Vec2::ZERO
            } else {
                transform.translation.truncate() - desired
            };
        }
        smoothing.snap = false;
        state.last_bounds = bounds;
        state.offset = smooth_offset(state.offset, time.delta_secs(), smoothing.tau);
        position = (desired + state.offset).round();
    }
    transform.translation.x = position.x;
    transform.translation.y = position.y;
}

/// Where the camera goes to show `target` with a view of half size `half`: centred on the target, kept inside
/// `bounds` when there are some (centred on the bounds on an axis where the view is larger), and rounded to whole
/// world units.
pub fn follow_position(target: Vec2, half: Vec2, bounds: Option<Rect>) -> Vec2 {
    let Some(bounds) = bounds else {
        return target.round();
    };
    let axis = |target: f32, half: f32, low: f32, high: f32| {
        if high - low <= 2.0 * half {
            (low + high) / 2.0
        } else {
            target.clamp(low + half, high - half)
        }
    };
    Vec2::new(
        axis(target.x, half.x, bounds.min.x, bounds.max.x),
        axis(target.y, half.y, bounds.min.y, bounds.max.y),
    )
    .round()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::timed_app;

    #[test]
    fn a_camera_offset_decays_without_stopping_and_ends() {
        let mut offset = Vec2::new(0.0, -200.0);
        let mut last = offset.length();
        for _ in 0..600 {
            offset = smooth_offset(offset, 1.0 / 60.0, 0.9);
            assert!(
                offset.length() < last,
                "it shrinks every frame, never stops"
            );
            // No frame moves the camera by more than it needs: a smooth glide, not a jump.
            assert!(last - offset.length() < 4.0);
            last = offset.length();
            if offset == Vec2::ZERO {
                break;
            }
        }
        assert_eq!(offset, Vec2::ZERO);
        assert_eq!(smooth_offset(Vec2::new(5.0, 5.0), 0.016, 0.0), Vec2::ZERO);
    }

    #[test]
    fn the_view_width_follows_the_window_shape() {
        assert_eq!(
            half_view(1920.0, 1080.0, 1200.0),
            Vec2::new(1200.0 * 16.0 / 9.0 / 2.0, 600.0)
        );
        assert_eq!(half_view(1000.0, 1000.0, 240.0), Vec2::new(120.0, 120.0));
        assert_eq!(
            half_view(640.0, 360.0, 1200.0),
            half_view(1920.0, 1080.0, 1200.0),
            "the window size does not change the view, only its shape does"
        );
    }

    const DT: f32 = 0.1;
    const TAU: f32 = 0.5;

    /// A game camera with a view of 200 by 200 following a target at (500, 400), bounds and smoothing on.
    fn smoothed_follow() -> (App, Entity) {
        let mut app = timed_app(std::time::Duration::from_secs_f32(DT));
        app.add_plugins(CameraFollowPlugin)
            .insert_resource(CameraBounds(Rect::new(0.0, 0.0, 1000.0, 800.0)))
            .insert_resource(CameraSmoothing {
                tau: TAU,
                snap: false,
            });
        let projection = Projection::Orthographic(OrthographicProjection {
            area: Rect::new(-100.0, -100.0, 100.0, 100.0),
            ..OrthographicProjection::default_2d()
        });
        let camera = app
            .world_mut()
            .spawn((GameCamera, projection, Transform::default()))
            .id();
        app.world_mut()
            .spawn((Controlled, Transform::from_xyz(500.0, 400.0, 0.0)));
        app.update();
        app.update();
        (app, camera)
    }

    fn camera_at(app: &App, camera: Entity) -> Vec2 {
        app.world()
            .get::<Transform>(camera)
            .unwrap()
            .translation
            .truncate()
    }

    #[test]
    fn a_change_of_bounds_glides_the_camera_from_where_it_was() {
        let (mut app, camera) = smoothed_follow();
        assert_eq!(camera_at(&app, camera), Vec2::new(500.0, 400.0));
        // The new bounds put the camera at x 200, 300 units to the left.
        app.insert_resource(CameraBounds(Rect::new(0.0, 0.0, 300.0, 800.0)));
        app.update();
        let expected =
            (Vec2::new(200.0, 400.0) + smooth_offset(Vec2::new(300.0, 0.0), DT, TAU)).round();
        assert_eq!(camera_at(&app, camera), expected);
        assert!(expected.x > 200.0 && expected.x < 500.0);
        for _ in 0..50 {
            app.update();
        }
        assert_eq!(camera_at(&app, camera), Vec2::new(200.0, 400.0));
    }

    #[test]
    fn a_snap_makes_the_next_change_of_bounds_jump_once() {
        let (mut app, camera) = smoothed_follow();
        app.insert_resource(CameraBounds(Rect::new(0.0, 0.0, 300.0, 800.0)));
        app.world_mut().resource_mut::<CameraSmoothing>().snap = true;
        app.update();
        assert_eq!(camera_at(&app, camera), Vec2::new(200.0, 400.0));
        assert!(!app.world().resource::<CameraSmoothing>().snap);
        app.insert_resource(CameraBounds(Rect::new(0.0, 0.0, 1000.0, 800.0)));
        app.update();
        let glide =
            (Vec2::new(500.0, 400.0) + smooth_offset(Vec2::new(-300.0, 0.0), DT, TAU)).round();
        assert_eq!(camera_at(&app, camera), glide, "the next change glides");
    }

    #[test]
    fn a_second_camera_does_not_stop_the_game_camera_following() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, CameraFollowPlugin));
        let projection = Projection::Orthographic(OrthographicProjection {
            area: Rect::new(-100.0, -100.0, 100.0, 100.0),
            ..OrthographicProjection::default_2d()
        });
        let game = app
            .world_mut()
            .spawn((Camera2d, GameCamera, projection, Transform::default()))
            .id();
        app.world_mut().spawn((Camera2d, Transform::default()));
        app.world_mut()
            .spawn((Controlled, Transform::from_xyz(40.0, 25.0, 0.0)));
        app.update();
        let moved = app.world().get::<Transform>(game).unwrap().translation;
        assert_eq!(moved.truncate(), Vec2::new(40.0, 25.0));
    }

    #[test]
    fn the_visible_rect_follows_the_camera_position() {
        let projection = Projection::Orthographic(OrthographicProjection {
            area: Rect::new(-100.0, -50.0, 100.0, 50.0),
            ..OrthographicProjection::default_2d()
        });
        let camera = GlobalTransform::from_xyz(30.0, -20.0, 0.0);
        assert_eq!(
            visible_rect(&camera, &projection),
            Some(Rect::new(-70.0, -70.0, 130.0, 30.0))
        );
    }

    #[test]
    fn a_camera_with_no_area_or_no_orthographic_view_has_no_visible_rect() {
        let camera = GlobalTransform::from_xyz(30.0, -20.0, 0.0);
        let perspective = Projection::Perspective(PerspectiveProjection::default());
        assert_eq!(visible_rect(&camera, &perspective), None);
        for area in [
            Rect::new(-100.0, 0.0, 100.0, 0.0),
            Rect::new(0.0, -50.0, 0.0, 50.0),
        ] {
            let flat = Projection::Orthographic(OrthographicProjection {
                area,
                ..OrthographicProjection::default_2d()
            });
            assert_eq!(visible_rect(&camera, &flat), None, "{area:?}");
        }
    }

    const BOUNDS: Rect = Rect {
        min: Vec2::new(0.0, 0.0),
        max: Vec2::new(1000.0, 800.0),
    };
    const HALF: Vec2 = Vec2::new(200.0, 100.0);

    #[test]
    fn the_camera_centres_on_the_target_inside_the_bounds() {
        assert_eq!(
            follow_position(Vec2::new(500.0, 400.0), HALF, Some(BOUNDS)),
            Vec2::new(500.0, 400.0)
        );
    }

    #[test]
    fn the_camera_stops_at_the_edges_of_the_bounds() {
        assert_eq!(
            follow_position(Vec2::new(10.0, 790.0), HALF, Some(BOUNDS)),
            Vec2::new(200.0, 700.0)
        );
        assert_eq!(
            follow_position(Vec2::new(990.0, 5.0), HALF, Some(BOUNDS)),
            Vec2::new(800.0, 100.0)
        );
    }

    #[test]
    fn a_view_larger_than_the_bounds_is_centred_on_them() {
        let small = Rect::new(0.0, 0.0, 300.0, 100.0);
        assert_eq!(
            follow_position(Vec2::new(10.0, 10.0), HALF, Some(small)),
            Vec2::new(150.0, 50.0)
        );
    }

    #[test]
    fn without_bounds_the_camera_follows_anywhere_and_rounds() {
        assert_eq!(
            follow_position(Vec2::new(-5000.4, 12.6), HALF, None),
            Vec2::new(-5000.0, 13.0)
        );
    }
}
