//! The player: four directions, full speed at once, full stop at once (`SPEC.md` section 4).
//!
//! Collision follows the Pyxel game: one point on the player's centre line, at its leading edge, is tested against
//! the blocked areas, which are not grown (`adr-jt16`). So the player slides past a tree corner that only its side
//! overlaps, and fits through a gap of one cell. The test starts from the centre, which never stands in a blocked
//! area, so a tree that already overlaps the front half after a turn still stops the player. Blocked means stop,
//! forget the held directions, and send `PlayerBlocked`.

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::{AnimationRepeat, AseAnimation};
use game_kit::Controlled;
use game_kit::animation::{tag_animation, tag_sprite};
use game_kit::collision::{Blocker, CollisionWorld, resolve};
use game_kit::depth::DepthSort;
use game_kit::outline::Edge;
use game_kit::screen_fx::CameraShake;

use crate::camera::CELL_SIZE;
use crate::house::House;
use crate::input::{Dir, Steering};
use crate::map::map_rect;

/// 30 cells per second, the real Pyxel speed (`adr-jt4`).
pub const SPEED: f32 = 30.0 * CELL_SIZE;
/// From the centre to the leading edge: the player is one cell.
const HALF: f32 = CELL_SIZE / 2.0;
/// How far past the stop the blocked point is looked up, to find what blocked the player.
const PROBE_PAST: f32 = 0.5;
/// A move that ends this much short of where it was going was blocked.
const BLOCK_TOLERANCE: f32 = 0.001;
/// The shake on every block except the house (`adr-jt10`).
const SHAKE_SECONDS: f32 = 0.1;
const SHAKE_DISTANCE: f32 = 2.0;

const ART: &str = "art/player.aseprite";
const SHOCKWAVE_ART: &str = "art/shockwave.aseprite";
const IDLE: &str = "Idle";
const MOVE: &str = "Move";
const TRAIL: &str = "Trail";
const WAVE: &str = "Wave";

#[derive(Component)]
pub struct Player;

/// The trail one cell behind the moving player and the shockwave one cell ahead. Children of the player, so they
/// move and turn with it.
#[derive(Component)]
struct Attached;

/// The direction the player's sprite shows, so the animation changes only when the direction does.
#[derive(Component, Default)]
struct Shown(Option<Dir>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockedBy {
    /// Any tree, in the field or the side forest, or the map edge.
    Wall,
    House,
}

/// The player ran into something and stopped.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct PlayerBlocked {
    pub by: BlockedBy,
    pub direction: Dir,
    /// A point just past where the player's leading edge stopped, inside what blocked it.
    pub at: Vec2,
}

/// The order of the player's systems in `Update`.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct PlayerSet;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlayerBlocked>().add_systems(
            Update,
            (steer, move_player, shake_at_wall, show_motion)
                .chain()
                .in_set(PlayerSet),
        );
    }
}

pub fn spawn_player(In(start): In<Vec2>, mut commands: Commands, server: Res<AssetServer>) {
    commands.spawn((
        Player,
        Controlled,
        Steering::default(),
        Shown::default(),
        DepthSort { feet_offset: HALF },
        Transform::from_translation(start.extend(0.0)),
        tag_sprite(&server, ART, IDLE),
    ));
}

/// Where one frame of movement takes the player.
#[derive(Debug, PartialEq)]
struct Step {
    centre: Vec2,
    /// A point just past the stop, inside what blocked the player. None when it was not blocked.
    blocked_at: Option<Vec2>,
}

/// Moves the player's `centre` `distance` towards `dir`. The leading edge stops at the first edge of a blocked area
/// that the centre line crosses, and at the map edge. A step of any length is tested along its whole path, so a long
/// frame cannot carry the player through a tree.
fn step(centre: Vec2, dir: Dir, distance: f32, edges: &[Edge], map: Rect) -> Step {
    let d = dir.vec();
    let to_map_edge = match dir {
        Dir::Up => map.max.y - centre.y,
        Dir::Down => centre.y - map.min.y,
        Dir::Left => centre.x - map.min.x,
        Dir::Right => map.max.x - centre.x,
    };
    let wanted = HALF + distance;
    let reach = wanted.min(to_map_edge);
    let ahead = resolve(centre, centre + d * reach, edges).feet;
    let reached = (ahead - centre).dot(d);
    let moved = (reached - HALF).max(0.0);
    Step {
        centre: centre + d * moved,
        blocked_at: (reached < wanted - BLOCK_TOLERANCE)
            .then(|| centre + d * (reached + PROBE_PAST)),
    }
}

/// What is at `point`, a point just past the player's stop: the house, or else a wall (a tree or the map edge).
fn blocked_by(point: Vec2, house: Option<Rect>) -> BlockedBy {
    if house.is_some_and(|house| house.contains(point)) {
        BlockedBy::House
    } else {
        BlockedBy::Wall
    }
}

fn steer(keys: Res<ButtonInput<KeyCode>>, mut players: Query<&mut Steering, With<Player>>) {
    let pressed: Vec<Dir> = Dir::ALL
        .into_iter()
        .filter(|dir| keys.just_pressed(dir.key()))
        .collect();
    let released: Vec<Dir> = Dir::ALL
        .into_iter()
        .filter(|dir| keys.just_released(dir.key()))
        .collect();
    if pressed.is_empty() && released.is_empty() {
        return;
    }
    for mut steering in &mut players {
        steering.update(&pressed, &released, |dir| keys.pressed(dir.key()));
    }
}

fn move_player(
    time: Res<Time>,
    world: Res<CollisionWorld>,
    houses: Query<&Blocker, With<House>>,
    mut players: Query<(&mut Transform, &mut Steering), With<Player>>,
    mut blocked: MessageWriter<PlayerBlocked>,
) {
    let map = map_rect();
    let house = houses.iter().next().map(|blocker| blocker.0);
    for (mut transform, mut steering) in &mut players {
        let Some(dir) = steering.current else {
            continue;
        };
        let moved = step(
            transform.translation.truncate(),
            dir,
            SPEED * time.delta_secs(),
            world.edges(),
            map,
        );
        transform.translation.x = moved.centre.x;
        transform.translation.y = moved.centre.y;
        if let Some(at) = moved.blocked_at {
            steering.stop();
            let by = blocked_by(at, house);
            debug!("player: blocked by {by:?} going {dir:?} at {at}");
            blocked.write(PlayerBlocked {
                by,
                direction: dir,
                at,
            });
        }
    }
}

fn shake_at_wall(mut blocked: MessageReader<PlayerBlocked>, mut shake: MessageWriter<CameraShake>) {
    for message in blocked.read() {
        if message.by == BlockedBy::Wall {
            debug!("player: wall, camera shake {:?}", message.direction);
            shake.write(CameraShake {
                duration: SHAKE_SECONDS,
                distance: SHAKE_DISTANCE,
                direction: message.direction.vec(),
            });
        }
    }
}

/// On a start or a turn: `Move` plays once, turned to the direction, and holds its last frame, and the trail and the
/// shockwave are added. On a stop: the idle frame, not turned, and the trail and the shockwave go.
fn show_motion(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut players: Query<(
        Entity,
        &Steering,
        &mut Shown,
        &mut Transform,
        &mut AseAnimation,
    )>,
    attached: Query<(Entity, &ChildOf), With<Attached>>,
) {
    for (player, steering, mut shown, mut transform, mut ase) in &mut players {
        if shown.0 == steering.current {
            continue;
        }
        match steering.current {
            Some(dir) => {
                transform.rotation = dir.rotation();
                ase.animation.play(MOVE, AnimationRepeat::Count(0));
                if shown.0.is_none() {
                    commands.entity(player).with_children(|parent| {
                        parent.spawn((
                            Attached,
                            tag_sprite(&server, ART, TRAIL),
                            Transform::from_xyz(0.0, -CELL_SIZE, 0.0),
                        ));
                        parent.spawn((
                            Attached,
                            Sprite::default(),
                            tag_animation(&server, SHOCKWAVE_ART, WAVE),
                            Transform::from_xyz(0.0, CELL_SIZE, 0.0),
                        ));
                    });
                }
            }
            None => {
                transform.rotation = Quat::IDENTITY;
                ase.animation.play_loop(IDLE);
                for (entity, parent) in &attached {
                    if parent.parent() == player {
                        commands.entity(entity).despawn();
                    }
                }
            }
        }
        shown.0 = steering.current;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_kit::outline;

    fn edges_of(rects: &[Rect]) -> Vec<Edge> {
        outline::build(rects)
            .iter()
            .flat_map(|cluster| cluster.edges.iter().copied())
            .collect()
    }

    const MAP: Rect = Rect {
        min: Vec2::new(0.0, 0.0),
        max: Vec2::new(320.0, 160.0),
    };

    /// A tree cell: 16 x 16 from (160, 64).
    fn tree() -> Rect {
        Rect::new(160.0, 64.0, 176.0, 80.0)
    }

    #[test]
    fn a_free_step_moves_the_whole_distance_in_each_direction() {
        let edges = edges_of(&[tree()]);
        let from = Vec2::new(100.0, 100.0);
        for (dir, to) in [
            (Dir::Up, Vec2::new(100.0, 108.0)),
            (Dir::Down, Vec2::new(100.0, 92.0)),
            (Dir::Left, Vec2::new(92.0, 100.0)),
            (Dir::Right, Vec2::new(108.0, 100.0)),
        ] {
            assert_eq!(
                step(from, dir, 8.0, &edges, MAP),
                Step {
                    centre: to,
                    blocked_at: None
                },
                "{dir:?}"
            );
        }
    }

    #[test]
    fn the_leading_edge_stops_at_a_tree_from_each_side() {
        let edges = edges_of(&[tree()]);
        // Centre on the tree's middle line, 20 px from each face; a step of 30 would pass the face.
        for (dir, from, stop, at) in [
            (
                Dir::Right,
                Vec2::new(140.0, 72.0),
                Vec2::new(152.0, 72.0),
                Vec2::new(160.5, 72.0),
            ),
            (
                Dir::Left,
                Vec2::new(196.0, 72.0),
                Vec2::new(184.0, 72.0),
                Vec2::new(175.5, 72.0),
            ),
            (
                Dir::Up,
                Vec2::new(168.0, 44.0),
                Vec2::new(168.0, 56.0),
                Vec2::new(168.0, 64.5),
            ),
            (
                Dir::Down,
                Vec2::new(168.0, 100.0),
                Vec2::new(168.0, 88.0),
                Vec2::new(168.0, 79.5),
            ),
        ] {
            assert_eq!(
                step(from, dir, 30.0, &edges, MAP),
                Step {
                    centre: stop,
                    blocked_at: Some(at)
                },
                "{dir:?}"
            );
        }
    }

    #[test]
    fn the_player_passes_a_tree_that_only_its_side_overlaps() {
        let edges = edges_of(&[tree()]);
        // Centre 1 px left of the tree's left face: the right half of the player overlaps the tree by 7 px.
        let beside = step(Vec2::new(159.0, 40.0), Dir::Up, 60.0, &edges, MAP);
        assert_eq!(beside.centre, Vec2::new(159.0, 100.0));
        assert_eq!(beside.blocked_at, None);
        // 1 px further right, the centre line meets the tree.
        let onto = step(Vec2::new(161.0, 40.0), Dir::Up, 60.0, &edges, MAP);
        assert_eq!(onto.centre, Vec2::new(161.0, 56.0));
    }

    #[test]
    fn the_player_fits_a_gap_of_one_cell() {
        // Trees at x 144 to 160 and 176 to 192, a gap from 160 to 176.
        let edges = edges_of(&[
            Rect::new(144.0, 64.0, 160.0, 80.0),
            Rect::new(176.0, 64.0, 192.0, 80.0),
        ]);
        let through = step(Vec2::new(168.0, 40.0), Dir::Up, 60.0, &edges, MAP);
        assert_eq!(through.centre, Vec2::new(168.0, 100.0));
        assert_eq!(through.blocked_at, None);
    }

    #[test]
    fn a_tree_already_in_front_after_a_turn_stops_the_player_where_it_is() {
        let edges = edges_of(&[tree()]);
        // Centre 3 px left of the tree's face, so the front half already overlaps it when the player turns right.
        let turned = step(Vec2::new(157.0, 72.0), Dir::Right, 8.0, &edges, MAP);
        assert_eq!(
            turned,
            Step {
                centre: Vec2::new(157.0, 72.0),
                blocked_at: Some(Vec2::new(160.5, 72.0))
            }
        );
    }

    #[test]
    fn a_long_frame_cannot_carry_the_player_through_a_thin_tree_band() {
        // A band one cell thick across the map, and a step of 200 px (a frame of 0.4 s at full speed).
        let edges = edges_of(&[Rect::new(0.0, 64.0, 320.0, 80.0)]);
        let moved = step(Vec2::new(100.0, 20.0), Dir::Up, 200.0, &edges, MAP);
        assert_eq!(moved.centre, Vec2::new(100.0, 56.0));
        assert_eq!(moved.blocked_at, Some(Vec2::new(100.0, 64.5)));
    }

    #[test]
    fn the_leading_edge_stops_at_the_map_edge() {
        for (dir, from, stop, at) in [
            (
                Dir::Up,
                Vec2::new(50.0, 150.0),
                Vec2::new(50.0, 152.0),
                Vec2::new(50.0, 160.5),
            ),
            (
                Dir::Down,
                Vec2::new(50.0, 10.0),
                Vec2::new(50.0, 8.0),
                Vec2::new(50.0, -0.5),
            ),
            (
                Dir::Left,
                Vec2::new(10.0, 50.0),
                Vec2::new(8.0, 50.0),
                Vec2::new(-0.5, 50.0),
            ),
            (
                Dir::Right,
                Vec2::new(310.0, 50.0),
                Vec2::new(312.0, 50.0),
                Vec2::new(320.5, 50.0),
            ),
        ] {
            assert_eq!(
                step(from, dir, 8.0, &[], MAP),
                Step {
                    centre: stop,
                    blocked_at: Some(at)
                },
                "{dir:?}"
            );
        }
    }

    #[test]
    fn a_step_that_ends_exactly_on_a_face_is_not_blocked() {
        let edges = edges_of(&[tree()]);
        let moved = step(Vec2::new(144.0, 72.0), Dir::Right, 8.0, &edges, MAP);
        assert_eq!(moved.centre, Vec2::new(152.0, 72.0));
        assert_eq!(moved.blocked_at, None);
    }

    #[test]
    fn the_blocked_point_tells_the_house_from_a_wall() {
        let house = Some(Rect::new(144.0, 64.0, 176.0, 96.0));
        for (point, by) in [
            // The map edge (top, bottom, left, right), and trees next to the house.
            (Vec2::new(100.0, 160.5), BlockedBy::Wall),
            (Vec2::new(100.0, -0.5), BlockedBy::Wall),
            (Vec2::new(-0.5, 50.0), BlockedBy::Wall),
            (Vec2::new(320.5, 50.0), BlockedBy::Wall),
            (Vec2::new(150.0, 63.5), BlockedBy::Wall),
            (Vec2::new(143.5, 80.0), BlockedBy::Wall),
            // The house from below, from the left and from the right.
            (Vec2::new(150.0, 64.5), BlockedBy::House),
            (Vec2::new(144.5, 80.0), BlockedBy::House),
            (Vec2::new(175.5, 80.0), BlockedBy::House),
        ] {
            assert_eq!(blocked_by(point, house), by, "{point}");
        }
        assert_eq!(blocked_by(Vec2::new(150.0, 64.5), None), BlockedBy::Wall);
    }
}
