//! Moves an entity to a goal. A strategy (patrol, pursue, or one a game writes) only sets `MoveGoal`. This module
//! finds a path with `NavGrid`, walks it with `collision::move_feet` (so the sliding rules of the player apply), and
//! reports in `MoveState` whether the entity is walking, which way it faces, and whether it arrived or the goal is
//! unreachable. See `dev-docs/extensions/enemy-movement.md`.
//!
//! The entity's `Transform` x and y are its feet, as for the player (`DepthSort { feet_offset: 0.0 }`).

use bevy::prelude::*;

use crate::camera::CameraBounds;
use crate::collision::{CollisionSettings, CollisionWorld, SlideAssist, move_feet};
use crate::nav::{NavGrid, NavSet};

/// Systems that choose goals run in this set, before the systems that follow them.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct StrategySet;

/// Systems that turn goals into steps.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct FollowSet;

/// How often a goal that moves may start a new path search, in seconds.
const REPATH_INTERVAL: f32 = 0.3;
/// A goal that moved less than this since the last search keeps its path, in world units.
const REPATH_DISTANCE: f32 = 4.0;
/// Seconds without real progress before the path is searched again.
const STALL_TIME: f32 = 0.5;

pub struct MovementPlugin {
    /// The most path searches per frame. An entity that has to wait keeps its old path or stands for a frame.
    pub searches_per_frame: usize,
}

impl Default for MovementPlugin {
    fn default() -> Self {
        Self {
            searches_per_frame: 4,
        }
    }
}

#[derive(Resource)]
struct SearchBudget(usize);

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SearchBudget(self.searches_per_frame))
            .configure_sets(
                Update,
                (StrategySet.after(NavSet), FollowSet.after(StrategySet)),
            )
            .add_systems(Update, follow_goals.in_set(FollowSet));
    }
}

/// The point the entity should walk to. `None` means stand still.
#[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
pub struct MoveGoal(pub Option<Vec2>);

/// Gives an entity the movement described above. It also adds `MoveGoal`, `MoveState`, `PathFollower` and
/// `SlideAssist`.
#[derive(Component, Clone, Copy, Debug)]
#[require(MoveGoal, MoveState, PathFollower, SlideAssist)]
pub struct Mover {
    /// World units per second.
    pub speed: f32,
    /// The feet stay this far inside the map rectangle on each side (half the width of the feet box).
    pub edge_margin: f32,
    /// The entity has arrived when its feet are this close to the goal.
    pub arrive_distance: f32,
}

impl Mover {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            edge_margin: 0.0,
            arrive_distance: 2.0,
        }
    }

    pub fn with_edge_margin(mut self, margin: f32) -> Self {
        self.edge_margin = margin;
        self
    }
}

/// What the mover did, for animation and for strategies.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct MoveState {
    /// True in a frame in which the feet moved.
    pub walking: bool,
    /// The last direction of movement as a unit vector. Right at the start.
    pub facing: Vec2,
    /// True when the feet are at the goal.
    pub arrived: bool,
    /// True when the last search found no way to the goal.
    pub unreachable: bool,
}

impl Default for MoveState {
    fn default() -> Self {
        Self {
            walking: false,
            facing: Vec2::X,
            arrived: false,
            unreachable: false,
        }
    }
}

/// The path to the goal and the bookkeeping for searching again.
#[derive(Component, Default, Debug)]
pub struct PathFollower {
    /// Points still to walk to, nearest first.
    path: std::collections::VecDeque<Vec2>,
    /// The goal the path was searched for.
    searched_for: Option<Vec2>,
    since_search: f32,
    stalled: f32,
}

impl PathFollower {
    /// The points still to walk to, nearest first, for the debug view.
    pub fn points(&self) -> impl Iterator<Item = Vec2> + '_ {
        self.path.iter().copied()
    }

    fn clear(&mut self) {
        self.path.clear();
        self.searched_for = None;
        self.stalled = 0.0;
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn follow_goals(
    time: Res<Time>,
    world: Res<CollisionWorld>,
    settings: Res<CollisionSettings>,
    nav: Res<NavGrid>,
    bounds: Option<Res<CameraBounds>>,
    budget: Res<SearchBudget>,
    mut movers: Query<(
        &mut Transform,
        &Mover,
        &MoveGoal,
        &mut MoveState,
        &mut PathFollower,
        &mut SlideAssist,
    )>,
    mut searches_left: Local<usize>,
) {
    let dt = time.delta_secs();
    *searches_left = budget.0;
    for (mut transform, mover, goal, mut state, mut follower, mut assist) in &mut movers {
        follower.since_search += dt;
        let feet = transform.translation.truncate();
        state.walking = false;
        let Some(goal) = goal.0 else {
            follower.clear();
            assist.reset();
            state.arrived = false;
            state.unreachable = false;
            continue;
        };
        if feet.distance(goal) <= mover.arrive_distance {
            follower.clear();
            assist.reset();
            state.arrived = true;
            state.unreachable = false;
            continue;
        }
        state.arrived = false;

        let goal_moved = follower
            .searched_for
            .is_none_or(|old| old.distance(goal) > REPATH_DISTANCE);
        let due = follower.path.is_empty() && !state.unreachable
            || goal_moved && follower.since_search >= REPATH_INTERVAL
            || follower.searched_for.is_none()
            || follower.stalled >= STALL_TIME
            || state.unreachable && follower.since_search >= REPATH_INTERVAL;
        if due && *searches_left > 0 {
            *searches_left -= 1;
            follower.since_search = 0.0;
            follower.stalled = 0.0;
            follower.searched_for = Some(goal);
            match nav.find_path(feet, goal) {
                Some(path) => {
                    follower.path = path.into();
                    state.unreachable = false;
                }
                None => {
                    follower.path.clear();
                    state.unreachable = true;
                }
            }
        }

        let Some(&next) = follower.path.front() else {
            assist.reset();
            continue;
        };
        let to_next = next - feet;
        let distance = to_next.length();
        let step_length = (mover.speed * dt).min(distance);
        let direction = to_next / distance.max(f32::EPSILON);
        let old = feet;
        let mut new = move_feet(
            &settings,
            &mut assist,
            old,
            direction * step_length,
            dt,
            world.edges(),
        );
        if let Some(bounds) = &bounds {
            let margin = Vec2::splat(mover.edge_margin);
            new = new.clamp(bounds.0.min + margin, bounds.0.max - margin);
        }
        if distance <= step_length + 1e-3 && new.distance(next) < 0.5 {
            follower.path.pop_front();
        }
        let moved = new.distance(old);
        if moved > 0.0 {
            state.walking = true;
            state.facing = (new - old) / moved;
        }
        if moved < step_length * 0.25 {
            follower.stalled += dt;
        } else {
            follower.stalled = 0.0;
        }
        transform.translation.x = new.x;
        transform.translation.y = new.y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::SlideTuning;
    use crate::nav::NavPlugin;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    fn app_with(blocked: &[Rect]) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                16,
            )));
        let mut world = CollisionWorld::default();
        world.add_blocked(blocked);
        app.insert_resource(world)
            .insert_resource(CollisionSettings {
                allow_sliding: true,
                slide: SlideTuning::for_cell_size(16.0),
            })
            .insert_resource(CameraBounds(Rect::new(0.0, 0.0, 200.0, 200.0)))
            .add_plugins((NavPlugin { step: 4.0 }, MovementPlugin::default()));
        app
    }

    fn spawn(app: &mut App, at: Vec2, goal: Option<Vec2>) -> Entity {
        app.world_mut()
            .spawn((
                Transform::from_translation(at.extend(0.0)),
                Mover::new(80.0),
                MoveGoal(goal),
            ))
            .id()
    }

    fn feet(app: &App, entity: Entity) -> Vec2 {
        app.world()
            .get::<Transform>(entity)
            .unwrap()
            .translation
            .truncate()
    }

    #[test]
    fn an_entity_walks_around_a_band_to_its_goal() {
        let band = Rect::new(100.0, 90.0, 140.0, 94.0);
        let mut app = app_with(&[band]);
        let goal = Vec2::new(120.0, 120.0);
        let entity = spawn(&mut app, Vec2::new(120.0, 60.0), Some(goal));
        for _ in 0..600 {
            app.update();
            let at = feet(&app, entity);
            let inside = at.x > 100.0 && at.x < 140.0 && at.y > 90.0 && at.y < 94.0;
            assert!(!inside, "inside the band at {at:?}");
        }
        assert!(
            feet(&app, entity).distance(goal) <= 2.5,
            "{:?}",
            feet(&app, entity)
        );
        let state = app.world().get::<MoveState>(entity).unwrap();
        assert!(state.arrived && !state.unreachable);
    }

    #[test]
    fn no_goal_means_standing_still() {
        let mut app = app_with(&[]);
        let entity = spawn(&mut app, Vec2::new(50.0, 50.0), None);
        for _ in 0..30 {
            app.update();
        }
        assert_eq!(feet(&app, entity), Vec2::new(50.0, 50.0));
        assert!(!app.world().get::<MoveState>(entity).unwrap().walking);
    }

    #[test]
    fn a_goal_inside_a_blocked_area_is_reported_unreachable() {
        let block = Rect::new(80.0, 80.0, 120.0, 120.0);
        let mut app = app_with(&[block]);
        let entity = spawn(
            &mut app,
            Vec2::new(20.0, 30.0),
            Some(Vec2::new(100.0, 100.0)),
        );
        for _ in 0..60 {
            app.update();
        }
        assert_eq!(feet(&app, entity), Vec2::new(20.0, 30.0));
        assert!(app.world().get::<MoveState>(entity).unwrap().unreachable);
    }

    #[test]
    fn the_entity_walks_at_its_speed_and_faces_its_direction() {
        let mut app = app_with(&[]);
        let entity = spawn(
            &mut app,
            Vec2::new(20.0, 100.0),
            Some(Vec2::new(180.0, 100.0)),
        );
        for _ in 0..61 {
            app.update();
        }
        // The first update only sets the start time, so 60 steps of 16 ms at 80 px/s are about 77 px.
        let travelled = feet(&app, entity).x - 20.0;
        assert!((travelled - 76.8).abs() < 3.0, "{travelled}");
        let state = app.world().get::<MoveState>(entity).unwrap();
        assert!(state.walking);
        assert!((state.facing - Vec2::X).length() < 1e-4);
    }

    #[test]
    fn a_custom_strategy_can_drive_the_mover() {
        // A game strategy is a component and a system that writes `MoveGoal`. This one always walks to a spot.
        #[derive(Component)]
        struct GoHome(Vec2);
        fn go_home(mut query: Query<(&GoHome, &mut MoveGoal)>) {
            for (home, mut goal) in &mut query {
                goal.0 = Some(home.0);
            }
        }
        let mut app = app_with(&[]);
        app.add_systems(Update, go_home.in_set(StrategySet));
        let home = Vec2::new(150.0, 40.0);
        let entity = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::new(30.0, 40.0, 0.0)),
                Mover::new(80.0),
                GoHome(home),
            ))
            .id();
        for _ in 0..300 {
            app.update();
        }
        assert!(feet(&app, entity).distance(home) <= 2.5);
    }
}
