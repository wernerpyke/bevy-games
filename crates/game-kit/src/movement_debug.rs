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
    movers: Query<(&Transform, &MoveGoal, &PathFollower), With<Mover>>,
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
            for (transform, goal, follower) in &movers {
                let mut from = transform.translation.truncate();
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

    #[test]
    fn a_line_sprite_covers_the_segment() {
        let (centre, length, angle) = line_between(Vec2::new(0.0, 0.0), Vec2::new(0.0, 10.0));
        assert_eq!(centre, Vec2::new(0.0, 5.0));
        assert_eq!(length, 10.0);
        assert!((angle - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
    }
}
