//! The movement debug view, drawn as thin sprites on `MOVEMENT_DEBUG_Z` (`layers.rs`). It follows the F3 grid switch
//! (`DebugSettings::show_grid`). It shows:
//! - magenta: the path of every `Mover`, from its feet through the points still to walk to, and a square at its goal
//! - orange: a square at every `Spawner`

use bevy::prelude::*;

use crate::debug::DebugSettings;
use crate::layers::MOVEMENT_DEBUG_Z;
use crate::movement::{MoveGoal, Mover, PathFollower};
use crate::spawner::Spawner;

const PATH_COLOR: Color = Color::srgb(1.0, 0.2, 1.0);
const SPAWNER_COLOR: Color = Color::srgb(1.0, 0.55, 0.1);
// Sizes in world units, tuned for a cell size of 64. `DebugSettings::scale` scales them to the game.
const LINE_THICKNESS: f32 = 2.0;
const MARKER_SIZE: f32 = 12.0;

pub struct MovementDebugPlugin;

impl Plugin for MovementDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_root)
            .add_systems(Update, draw);
    }
}

#[derive(Component)]
struct DebugRoot;

fn spawn_root(mut commands: Commands) {
    commands.spawn((DebugRoot, Transform::default(), Visibility::default()));
}

/// The centre, the length and the angle of a line sprite from `a` to `b`.
fn line_between(a: Vec2, b: Vec2) -> (Vec2, f32, f32) {
    let delta = b - a;
    ((a + b) / 2.0, delta.length(), delta.to_angle())
}

fn draw(
    mut commands: Commands,
    settings: Res<DebugSettings>,
    root: Single<(Entity, &mut Visibility), With<DebugRoot>>,
    movers: Query<(&Transform, &Mover, &MoveGoal, &PathFollower)>,
    spawners: Query<&Transform, With<Spawner>>,
) {
    let (root, mut visibility) = root.into_inner();
    if !settings.show_grid {
        *visibility = Visibility::Hidden;
        return;
    }
    *visibility = Visibility::Inherited;
    let scale = settings.scale();
    let marker = Vec2::splat(MARKER_SIZE * scale);
    commands
        .entity(root)
        .despawn_children()
        .with_children(|parent| {
            for (transform, mover, goal, follower) in &movers {
                let mut from = Vec2::new(
                    transform.translation.x,
                    transform.translation.y - mover.feet_offset,
                );
                for to in follower.points() {
                    let (centre, length, angle) = line_between(from, to);
                    parent.spawn((
                        Sprite::from_color(PATH_COLOR, Vec2::new(length, LINE_THICKNESS * scale)),
                        Transform::from_translation(centre.extend(MOVEMENT_DEBUG_Z))
                            .with_rotation(Quat::from_rotation_z(angle)),
                    ));
                    from = to;
                }
                if let Some(goal) = goal.0 {
                    parent.spawn((
                        Sprite::from_color(PATH_COLOR, marker),
                        Transform::from_translation(goal.extend(MOVEMENT_DEBUG_Z)),
                    ));
                }
            }
            for transform in &spawners {
                parent.spawn((
                    Sprite::from_color(SPAWNER_COLOR, marker * 1.5),
                    Transform::from_translation(
                        transform.translation.truncate().extend(MOVEMENT_DEBUG_Z),
                    ),
                ));
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::map::Params;

    fn debug_settings(show_grid: bool) -> DebugSettings {
        DebugSettings {
            cell_size: 64.0,
            grid_color: Color::WHITE,
            grid_origin: crate::debug::GridOrigin::Center,
            target_name: "t",
            grid_z: 0.0,
            hud_z_index: 0,
            rows_from_top: None,
            show_grid,
            show_info: true,
        }
    }

    /// Colour, size, centre and angle of each sprite under the debug root.
    fn sprites(app: &mut App) -> Vec<(Color, Vec2, Vec2, f32)> {
        let world = app.world_mut();
        world
            .query_filtered::<(&Sprite, &Transform), With<ChildOf>>()
            .iter(world)
            .map(|(sprite, t)| {
                let (_, angle) = t.rotation.to_axis_angle();
                (
                    sprite.color,
                    sprite.custom_size.unwrap(),
                    t.translation.truncate(),
                    angle,
                )
            })
            .collect()
    }

    #[test]
    fn each_path_is_drawn_from_the_feet_through_its_points_to_a_goal_square() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(debug_settings(true))
            .add_plugins(MovementDebugPlugin);
        // The feet are 10 below the transform, at (0, 0).
        app.world_mut().spawn((
            Transform::from_xyz(0.0, 10.0, 0.0),
            Mover::new(10.0).with_feet_offset(10.0),
            MoveGoal(Some(Vec2::new(30.0, 100.0))),
            PathFollower::along(&[Vec2::new(30.0, 40.0), Vec2::new(30.0, 100.0)]),
        ));
        app.world_mut().spawn((
            Transform::from_xyz(5.0, 6.0, 7.0),
            Spawner::new(1.0, None, Params::default()),
        ));
        app.update();
        let found = sprites(&mut app);
        assert_eq!(found.len(), 4, "{found:?}");
        let angle = |a: f32, b: f32| (a - b).abs() < 1e-5;
        // From (0, 0) to (30, 40), then up to (30, 100).
        let first = (40.0f32).atan2(30.0);
        assert!(found.iter().any(|s| s.0 == PATH_COLOR
            && s.1 == Vec2::new(50.0, LINE_THICKNESS)
            && s.2 == Vec2::new(15.0, 20.0)
            && angle(s.3, first)));
        assert!(found.iter().any(|s| s.0 == PATH_COLOR
            && s.1 == Vec2::new(60.0, LINE_THICKNESS)
            && s.2 == Vec2::new(30.0, 70.0)
            && angle(s.3, std::f32::consts::FRAC_PI_2)));
        assert!(found.contains(&(
            PATH_COLOR,
            Vec2::splat(MARKER_SIZE),
            Vec2::new(30.0, 100.0),
            0.0
        )));
        assert!(found.contains(&(
            SPAWNER_COLOR,
            Vec2::splat(MARKER_SIZE * 1.5),
            Vec2::new(5.0, 6.0),
            0.0
        )));
        // Drawn again each frame, not added to.
        app.update();
        assert_eq!(sprites(&mut app).len(), 4);
    }

    #[test]
    fn the_view_is_hidden_with_the_grid() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(debug_settings(false))
            .add_plugins(MovementDebugPlugin);
        app.update();
        let world = app.world_mut();
        let visibility = world
            .query_filtered::<&Visibility, With<DebugRoot>>()
            .single(world)
            .unwrap();
        assert_eq!(*visibility, Visibility::Hidden);
    }
}
