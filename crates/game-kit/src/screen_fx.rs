//! Effects on the whole game view (`adr-x61`). A game asks for an effect with a message and gives every value; the
//! kit moves the `GameCamera` and puts it back exactly when the effect ends. See `dev-docs/extensions/screen-fx.md`.
//!
//! Camera shake: the camera is pushed `distance` along `direction`, then to the other side, flipping each frame, for
//! `duration` seconds of game time. The offset is taken off in `First` and added again in `PostUpdate` after the
//! camera follow (`CameraFollowSet`), so game systems in `Update` always see the camera where it really is, and the
//! follow and its whole-pixel rounding never see the offset.

use bevy::prelude::*;
use bevy::transform::TransformSystems;

use crate::camera::{CameraFollowSet, GameCamera};

/// Asks for a camera shake. A new shake replaces one that is running.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct CameraShake {
    /// Seconds of game time. Pause stops the shake, and it goes on after the pause.
    pub duration: f32,
    /// How far the camera is pushed, in world units. The offset is rounded to whole units.
    pub distance: f32,
    /// The direction of the first push. Need not be a unit vector.
    pub direction: Vec2,
}

/// A shake ends when less than this much of its time is left, so float noise in the frame times does not add a frame.
const END_TOLERANCE: f32 = 0.001;

pub struct ScreenFxPlugin;

impl Plugin for ScreenFxPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CameraShake>()
            .init_resource::<ShakeState>()
            .add_systems(First, take_off_offset)
            .add_systems(
                PostUpdate,
                apply_shake
                    .after(CameraFollowSet)
                    .before(TransformSystems::Propagate),
            );
    }
}

#[derive(Resource, Default)]
struct ShakeState {
    running: Option<Shake>,
    /// The offset added to the camera this frame, taken off again in the next `First`.
    applied: Vec2,
}

struct Shake {
    request: CameraShake,
    elapsed: f32,
    /// True on the frames that push along `direction`, false on the frames that push the other way.
    forward: bool,
}

/// The camera offset of `shake` on a frame that pushes forward or back: `distance` along the direction, rounded to
/// whole world units, so pixel art stays on the pixel grid.
pub fn shake_offset(shake: &CameraShake, forward: bool) -> Vec2 {
    let sign = if forward { 1.0 } else { -1.0 };
    (shake.direction.normalize_or_zero() * shake.distance * sign).round()
}

fn take_off_offset(
    mut state: ResMut<ShakeState>,
    camera: Option<Single<&mut Transform, With<GameCamera>>>,
) {
    if let Some(mut camera) = camera {
        camera.translation -= state.applied.extend(0.0);
    }
    state.applied = Vec2::ZERO;
}

fn apply_shake(
    mut requests: MessageReader<CameraShake>,
    mut state: ResMut<ShakeState>,
    time: Res<Time>,
    camera: Option<Single<&mut Transform, With<GameCamera>>>,
) {
    if let Some(request) = requests.read().last() {
        state.running = Some(Shake {
            request: *request,
            elapsed: 0.0,
            forward: true,
        });
    }
    let dt = time.delta_secs();
    let mut offset = Vec2::ZERO;
    if let Some(shake) = &mut state.running {
        if shake.request.duration - shake.elapsed > END_TOLERANCE {
            offset = shake_offset(&shake.request, shake.forward);
            // A paused frame keeps the offset: no flip and no time used.
            if dt > 0.0 {
                shake.forward = !shake.forward;
                shake.elapsed += dt;
            }
        } else {
            state.running = None;
        }
    }
    if let Some(mut camera) = camera {
        camera.translation += offset.extend(0.0);
        state.applied = offset;
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::Controlled;
    use crate::camera::CameraFollowPlugin;
    use crate::test_util::timed_app;

    /// A 60 fps frame cut to whole nanoseconds, a little under 1/60 s, so six frames add up to just under 0.1 s.
    const FRAME: Duration = Duration::from_nanos(16_666_666);

    fn app_with_camera(at: Vec2) -> (App, Entity) {
        let mut app = timed_app(FRAME);
        app.add_plugins(ScreenFxPlugin);
        let camera = app
            .world_mut()
            .spawn((GameCamera, Transform::from_translation(at.extend(3.0))))
            .id();
        app.update();
        (app, camera)
    }

    fn camera_at(app: &App, camera: Entity) -> Vec3 {
        app.world().get::<Transform>(camera).unwrap().translation
    }

    /// The camera position after each of `frames` updates, x and y only.
    fn run(app: &mut App, camera: Entity, frames: usize) -> Vec<Vec2> {
        (0..frames)
            .map(|_| {
                app.update();
                camera_at(app, camera).truncate()
            })
            .collect()
    }

    fn shake(duration: f32, distance: f32, direction: Vec2) -> CameraShake {
        CameraShake {
            duration,
            distance,
            direction,
        }
    }

    #[test]
    fn the_offset_is_the_distance_along_the_direction_in_whole_units() {
        let right = shake(0.1, 2.0, Vec2::new(5.0, 0.0));
        assert_eq!(shake_offset(&right, true), Vec2::new(2.0, 0.0));
        assert_eq!(shake_offset(&right, false), Vec2::new(-2.0, 0.0));
        // 3 units diagonally is 2.12 on each axis.
        let diagonal = shake(0.1, 3.0, Vec2::new(-1.0, 1.0));
        assert_eq!(shake_offset(&diagonal, true), Vec2::new(-2.0, 2.0));
        assert_eq!(shake_offset(&shake(0.1, 2.0, Vec2::ZERO), true), Vec2::ZERO);
    }

    #[test]
    fn a_shake_flips_sides_each_frame_and_ends_exactly_where_the_camera_was() {
        let (mut app, camera) = app_with_camera(Vec2::new(10.0, 20.0));
        // 0.1 s at 60 frames per second: six frames.
        app.world_mut()
            .write_message(shake(0.1, 2.0, Vec2::new(1.0, 0.0)));
        let x: Vec<f32> = run(&mut app, camera, 9).iter().map(|p| p.x).collect();
        assert_eq!(x, vec![12.0, 8.0, 12.0, 8.0, 12.0, 8.0, 10.0, 10.0, 10.0]);
        assert_eq!(camera_at(&app, camera), Vec3::new(10.0, 20.0, 3.0));
    }

    #[test]
    fn a_new_shake_replaces_the_running_one() {
        let (mut app, camera) = app_with_camera(Vec2::new(10.0, 20.0));
        app.world_mut()
            .write_message(shake(0.1, 2.0, Vec2::new(1.0, 0.0)));
        run(&mut app, camera, 2);
        app.world_mut()
            .write_message(shake(0.05, 4.0, Vec2::new(0.0, -1.0)));
        assert_eq!(
            run(&mut app, camera, 5),
            vec![
                Vec2::new(10.0, 16.0),
                Vec2::new(10.0, 24.0),
                Vec2::new(10.0, 16.0),
                Vec2::new(10.0, 20.0),
                Vec2::new(10.0, 20.0),
            ]
        );
    }

    #[test]
    fn a_pause_holds_the_shake_and_it_goes_on_after() {
        let (mut app, camera) = app_with_camera(Vec2::new(10.0, 20.0));
        app.world_mut()
            .write_message(shake(0.1, 2.0, Vec2::new(1.0, 0.0)));
        run(&mut app, camera, 2);
        app.world_mut().resource_mut::<Time<Virtual>>().pause();
        let paused: Vec<f32> = run(&mut app, camera, 4).iter().map(|p| p.x).collect();
        assert_eq!(paused, vec![12.0, 12.0, 12.0, 12.0]);
        app.world_mut().resource_mut::<Time<Virtual>>().unpause();
        // Two frames were used before the pause; the paused frames used no time, so four are left.
        let after: Vec<f32> = run(&mut app, camera, 6).iter().map(|p| p.x).collect();
        assert_eq!(after, vec![12.0, 8.0, 12.0, 8.0, 10.0, 10.0]);
    }

    #[test]
    fn a_shake_is_added_to_the_followed_position_and_ends_on_it() {
        let (mut app, camera) = app_with_camera(Vec2::ZERO);
        app.add_plugins(CameraFollowPlugin);
        app.world_mut()
            .entity_mut(camera)
            .insert(Projection::Orthographic(OrthographicProjection {
                area: Rect::new(-100.0, -100.0, 100.0, 100.0),
                ..OrthographicProjection::default_2d()
            }));
        let target = app
            .world_mut()
            .spawn((Controlled, Transform::from_xyz(40.0, 25.0, 0.0)))
            .id();
        app.update();
        app.world_mut()
            .write_message(shake(0.05, 3.0, Vec2::new(0.0, 1.0)));
        let mut seen = run(&mut app, camera, 1);
        // The target moves during the shake: the camera follows it and the shake stays on top.
        app.world_mut()
            .get_mut::<Transform>(target)
            .unwrap()
            .translation
            .x = 60.0;
        seen.extend(run(&mut app, camera, 4));
        assert_eq!(
            seen,
            vec![
                Vec2::new(40.0, 28.0),
                Vec2::new(60.0, 22.0),
                Vec2::new(60.0, 28.0),
                Vec2::new(60.0, 25.0),
                Vec2::new(60.0, 25.0),
            ]
        );
    }
}
