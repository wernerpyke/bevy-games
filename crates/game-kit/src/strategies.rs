//! The movement strategies of the kit. A strategy only chooses a goal and writes it to `MoveGoal`: `movement.rs`
//! walks there. A game adds a strategy of its own the same way: a component and a system in `StrategySet` that writes
//! `MoveGoal`. See `dev-docs/extensions/enemy-movement.md`.
//!
//! - `RandomPatrol`: pick a random reachable place within a radius of the current place, walk there, pick again.
//! - `Pursue`: walk to the player (the `Controlled` entity), or to another entity.
//! - `Sentry`: patrol like `RandomPatrol` until the player is seen (`can_see`: in range and no edge between), then
//!   pursue for good.
//!
//! `StrategiesPlugin` needs `MovementPlugin`, `NavPlugin`, `RngPlugin` and `CollisionPlugin` (for the edges that
//! block sight).

use bevy::prelude::*;

use crate::Controlled;
use crate::camera::CameraBounds;
use crate::collision::CollisionWorld;
use crate::map::Params;
use crate::movement::{MoveGoal, MoveState, StrategySet};
use crate::nav::{NavGrid, segment_blocked};
use crate::outline::Edge;
use crate::rng::{GameRng, Rng};

/// Tries per pick of a patrol target.
const PICK_TRIES: usize = 8;
/// Seconds to wait when no reachable target was found, before trying again.
const NO_TARGET_WAIT: f32 = 1.0;

pub struct StrategiesPlugin;

impl Plugin for StrategiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (patrol, pursue, sentry).in_set(StrategySet));
    }
}

/// Walks to a random place within `radius` of where it is, then picks a new one.
#[derive(Component, Clone, Copy, Debug)]
#[require(MoveGoal)]
pub struct RandomPatrol {
    pub radius: f32,
    /// Seconds left to wait after a pick that found nothing.
    wait: f32,
}

impl RandomPatrol {
    pub fn new(radius: f32) -> Self {
        Self { radius, wait: 0.0 }
    }

    /// Reads `radius` (a number above 0) from the legend parameters of a map entry.
    pub fn from_params(params: &Params) -> Result<Self, String> {
        match params.f32("radius")? {
            Some(radius) if radius > 0.0 => Ok(Self::new(radius)),
            Some(radius) => Err(format!(
                "parameter `radius` must be above 0, found {radius}"
            )),
            None => Err("strategy `patrol` needs a parameter `radius=<pixels>`".to_string()),
        }
    }
}

/// Walks to the `Controlled` entity (the player), or to `target` when it is set. Stands still when the target does
/// not exist.
#[derive(Component, Clone, Copy, Debug, Default)]
#[require(MoveGoal)]
pub struct Pursue {
    pub target: Option<Entity>,
}

impl Pursue {
    /// Pursue takes no parameters. Others in the entry are not an error, so a game can use them for itself.
    pub fn from_params(_params: &Params) -> Result<Self, String> {
        Ok(Self::default())
    }
}

/// Walks like `RandomPatrol` (within `radius` of where it is) until it sees the `Controlled` entity: within `sight`
/// and with no blocked edge on the straight line between the feet. From then on it walks to that entity and never
/// goes back to patrolling. It stands still when that entity does not exist.
#[derive(Component, Clone, Copy, Debug)]
#[require(MoveGoal)]
pub struct Sentry {
    pub sight: f32,
    patrol: RandomPatrol,
    alerted: bool,
}

impl Sentry {
    pub fn new(radius: f32, sight: f32) -> Self {
        Self {
            sight,
            patrol: RandomPatrol::new(radius),
            alerted: false,
        }
    }

    /// True once the player has been seen.
    pub fn alerted(&self) -> bool {
        self.alerted
    }

    /// Reads `radius` and `sight` (numbers above 0) from the legend parameters of a map entry.
    pub fn from_params(params: &Params) -> Result<Self, String> {
        let positive = |key: &str| match params.f32(key)? {
            Some(value) if value > 0.0 => Ok(value),
            Some(value) => Err(format!("parameter `{key}` must be above 0, found {value}")),
            None => Err(format!(
                "strategy `sentry` needs a parameter `{key}=<pixels>`"
            )),
        };
        Ok(Self::new(positive("radius")?, positive("sight")?))
    }
}

/// Whether `from` sees `to`: it is within `range` and no blocked edge crosses the straight line between them.
pub fn can_see(from: Vec2, to: Vec2, range: f32, edges: &[Edge]) -> bool {
    from.distance(to) <= range && !segment_blocked(from, to, edges)
}

/// A random point within `radius` of `from` that is inside `bounds` (when there are some) and that `reachable`
/// accepts. Tries a few times, then gives up.
pub fn pick_target(
    rng: &mut Rng,
    from: Vec2,
    radius: f32,
    bounds: Option<Rect>,
    tries: usize,
    reachable: impl Fn(Vec2) -> bool,
) -> Option<Vec2> {
    for _ in 0..tries {
        let point = rng.point_in_disc(from, radius);
        if bounds.is_some_and(|area| !area.contains(point)) {
            continue;
        }
        if reachable(point) {
            return Some(point);
        }
    }
    None
}

fn patrol(
    time: Res<Time>,
    nav: Res<NavGrid>,
    bounds: Option<Res<CameraBounds>>,
    rng: Option<ResMut<GameRng>>,
    mut patrols: Query<(&Transform, &mut RandomPatrol, &mut MoveGoal, &MoveState)>,
) {
    let Some(mut rng) = rng else {
        return;
    };
    let dt = time.delta_secs();
    let bounds = bounds.map(|b| b.0);
    for (transform, mut patrol, mut goal, state) in &mut patrols {
        patrol_step(
            &mut patrol,
            dt,
            transform,
            &mut goal,
            state,
            &nav,
            bounds,
            &mut rng,
        );
    }
}

/// One frame of patrolling: keep walking to the goal, or wait, or pick a new place.
#[allow(clippy::too_many_arguments)]
fn patrol_step(
    patrol: &mut RandomPatrol,
    dt: f32,
    transform: &Transform,
    goal: &mut MoveGoal,
    state: &MoveState,
    nav: &NavGrid,
    bounds: Option<Rect>,
    rng: &mut Rng,
) {
    let walking_to_goal = goal.0.is_some() && !state.arrived && !state.unreachable;
    if walking_to_goal {
        return;
    }
    if patrol.wait > 0.0 {
        patrol.wait -= dt;
        goal.0 = None;
        return;
    }
    let feet = transform.translation.truncate();
    let target = pick_target(rng, feet, patrol.radius, bounds, PICK_TRIES, |point| {
        nav.find_path(feet, point).is_some()
    });
    match target {
        Some(point) => goal.0 = Some(point),
        None => {
            goal.0 = None;
            patrol.wait = NO_TARGET_WAIT;
        }
    }
}

fn pursue(
    positions: Query<&Transform, Without<Pursue>>,
    player: Query<&Transform, (With<Controlled>, Without<Pursue>)>,
    mut pursuers: Query<(&Pursue, &mut MoveGoal)>,
) {
    let player = player.single().ok();
    for (pursue, mut goal) in &mut pursuers {
        let target = match pursue.target {
            Some(entity) => positions.get(entity).ok(),
            None => player,
        };
        goal.0 = target.map(|transform| transform.translation.truncate());
    }
}

#[allow(clippy::too_many_arguments)]
fn sentry(
    time: Res<Time>,
    nav: Res<NavGrid>,
    world: Res<CollisionWorld>,
    bounds: Option<Res<CameraBounds>>,
    rng: Option<ResMut<GameRng>>,
    player: Query<&Transform, (With<Controlled>, Without<Sentry>)>,
    mut sentries: Query<(&Transform, &mut Sentry, &mut MoveGoal, &MoveState)>,
) {
    let Some(mut rng) = rng else {
        return;
    };
    let dt = time.delta_secs();
    let bounds = bounds.map(|b| b.0);
    let player = player.single().ok().map(|t| t.translation.truncate());
    for (transform, mut sentry, mut goal, state) in &mut sentries {
        let feet = transform.translation.truncate();
        if !sentry.alerted
            && player.is_some_and(|at| can_see(feet, at, sentry.sight, world.edges()))
        {
            sentry.alerted = true;
        }
        if sentry.alerted {
            goal.0 = player;
        } else {
            patrol_step(
                &mut sentry.patrol,
                dt,
                transform,
                &mut goal,
                state,
                &nav,
                bounds,
                &mut rng,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::{CollisionSettings, CollisionWorld, SlideTuning};
    use crate::movement::{MovementPlugin, Mover};
    use crate::nav::NavPlugin;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    #[test]
    fn radius_is_read_from_the_parameters() {
        let ok = Params::from_pairs(&[("strategy", "patrol"), ("radius", "120")]);
        assert_eq!(RandomPatrol::from_params(&ok).unwrap().radius, 120.0);
        let missing = Params::from_pairs(&[("strategy", "patrol")]);
        assert!(
            RandomPatrol::from_params(&missing)
                .unwrap_err()
                .contains("needs")
        );
        let bad = Params::from_pairs(&[("radius", "far")]);
        assert!(
            RandomPatrol::from_params(&bad)
                .unwrap_err()
                .contains("number")
        );
        let zero = Params::from_pairs(&[("radius", "0")]);
        assert!(
            RandomPatrol::from_params(&zero)
                .unwrap_err()
                .contains("above 0")
        );
        assert!(Pursue::from_params(&Params::default()).is_ok());
    }

    #[test]
    fn a_patrol_target_is_inside_the_radius_and_the_bounds() {
        let mut rng = Rng::new(1);
        let from = Vec2::new(20.0, 20.0);
        let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        for _ in 0..200 {
            let point = pick_target(&mut rng, from, 50.0, Some(bounds), 8, |_| true);
            if let Some(point) = point {
                assert!(point.distance(from) <= 50.0 + 1e-3);
                assert!(bounds.contains(point));
            }
        }
    }

    #[test]
    fn only_reachable_targets_are_picked() {
        let mut rng = Rng::new(2);
        let from = Vec2::new(100.0, 100.0);
        for _ in 0..100 {
            let point = pick_target(&mut rng, from, 40.0, None, 64, |p| p.x > from.x).unwrap();
            assert!(point.x > from.x);
        }
        assert_eq!(pick_target(&mut rng, from, 40.0, None, 8, |_| false), None);
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                16,
            )))
            .insert_resource(CollisionWorld::default())
            .insert_resource(CollisionSettings {
                allow_sliding: true,
                slide: SlideTuning::for_cell_size(16.0),
            })
            .insert_resource(CameraBounds(Rect::new(0.0, 0.0, 400.0, 400.0)))
            .insert_resource(GameRng::seeded(7))
            .add_plugins((
                NavPlugin { step: 4.0 },
                MovementPlugin::default(),
                StrategiesPlugin,
            ));
        app
    }

    #[test]
    fn a_patrolling_entity_keeps_choosing_new_places() {
        let mut app = app();
        let start = Vec2::new(200.0, 200.0);
        let entity = app
            .world_mut()
            .spawn((
                Transform::from_translation(start.extend(0.0)),
                Mover::new(80.0),
                RandomPatrol::new(40.0),
            ))
            .id();
        let mut goals: Vec<Vec2> = Vec::new();
        for _ in 0..900 {
            app.update();
            if let Some(goal) = app.world().get::<MoveGoal>(entity).unwrap().0
                && goals.last() != Some(&goal)
            {
                goals.push(goal);
            }
        }
        assert!(goals.len() >= 5, "{} goals", goals.len());
        for pair in goals.windows(2) {
            // Each new place is within the radius of the place where the entity was, which is the last goal.
            assert!(pair[0].distance(pair[1]) <= 40.0 + 6.0, "{pair:?}");
        }
    }

    #[test]
    fn a_pursuer_follows_a_moving_player_and_stops_without_one() {
        let mut app = app();
        let pursuer = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::new(50.0, 50.0, 0.0)),
                Mover::new(80.0),
                Pursue::default(),
            ))
            .id();
        for _ in 0..30 {
            app.update();
        }
        assert_eq!(
            app.world()
                .get::<Transform>(pursuer)
                .unwrap()
                .translation
                .truncate(),
            Vec2::new(50.0, 50.0)
        );
        let player = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::new(300.0, 300.0, 0.0)),
                Controlled,
            ))
            .id();
        for frame in 0..600 {
            if frame == 150 {
                app.world_mut()
                    .get_mut::<Transform>(player)
                    .unwrap()
                    .translation = Vec3::new(300.0, 60.0, 0.0);
            }
            app.update();
        }
        let at = app
            .world()
            .get::<Transform>(pursuer)
            .unwrap()
            .translation
            .truncate();
        assert!(at.distance(Vec2::new(300.0, 60.0)) <= 3.0, "{at:?}");
        app.world_mut().despawn(player);
        app.update();
        assert_eq!(app.world().get::<MoveGoal>(pursuer).unwrap().0, None);
    }

    fn wall_edges() -> Vec<Edge> {
        crate::outline::build(&[Rect::new(90.0, 0.0, 110.0, 100.0)])
            .iter()
            .flat_map(|c| c.edges.iter().copied())
            .collect()
    }

    #[test]
    fn sight_needs_range_and_a_clear_line() {
        let edges = wall_edges();
        let (here, beside, behind) = (
            Vec2::new(50.0, 50.0),
            Vec2::new(50.0, 150.0),
            Vec2::new(150.0, 50.0),
        );
        assert!(can_see(here, beside, 200.0, &edges));
        assert!(!can_see(here, beside, 99.0, &edges));
        assert!(!can_see(here, behind, 200.0, &edges));
        assert!(can_see(behind, here, 200.0, &[]));
    }

    #[test]
    fn sentry_parameters_are_read_and_checked() {
        let ok = Params::from_pairs(&[("radius", "40"), ("sight", "90")]);
        let sentry = Sentry::from_params(&ok).unwrap();
        assert_eq!(sentry.sight, 90.0);
        assert!(!sentry.alerted());
        let no_sight = Params::from_pairs(&[("radius", "40")]);
        assert!(
            Sentry::from_params(&no_sight)
                .unwrap_err()
                .contains("`sight=")
        );
        let no_radius = Params::from_pairs(&[("sight", "90")]);
        assert!(
            Sentry::from_params(&no_radius)
                .unwrap_err()
                .contains("`radius=")
        );
        let zero = Params::from_pairs(&[("radius", "40"), ("sight", "0")]);
        assert!(Sentry::from_params(&zero).unwrap_err().contains("above 0"));
    }

    fn spawn_sentry(app: &mut App, at: Vec2, radius: f32, sight: f32) -> Entity {
        app.world_mut()
            .spawn((
                Transform::from_translation(at.extend(0.0)),
                Mover::new(10.0),
                Sentry::new(radius, sight),
            ))
            .id()
    }

    fn spawn_player(app: &mut App, at: Vec2) -> Entity {
        app.world_mut()
            .spawn((Transform::from_translation(at.extend(0.0)), Controlled))
            .id()
    }

    fn run(app: &mut App, frames: usize) {
        for _ in 0..frames {
            app.update();
        }
    }

    #[test]
    fn a_sentry_patrols_while_the_player_is_out_of_range_or_hidden() {
        let mut far = app();
        let sentry = spawn_sentry(&mut far, Vec2::new(100.0, 200.0), 10.0, 50.0);
        spawn_player(&mut far, Vec2::new(350.0, 200.0));
        run(&mut far, 300);
        assert!(!far.world().get::<Sentry>(sentry).unwrap().alerted());

        let mut hidden = app();
        hidden
            .world_mut()
            .resource_mut::<CollisionWorld>()
            .add_blocked(&[Rect::new(190.0, 0.0, 210.0, 400.0)]);
        let sentry = spawn_sentry(&mut hidden, Vec2::new(100.0, 200.0), 10.0, 400.0);
        spawn_player(&mut hidden, Vec2::new(300.0, 200.0));
        run(&mut hidden, 300);
        assert!(!hidden.world().get::<Sentry>(sentry).unwrap().alerted());
    }

    #[test]
    fn a_sentry_that_sees_the_player_pursues_for_good() {
        let mut app = app();
        let sentry = spawn_sentry(&mut app, Vec2::new(100.0, 200.0), 10.0, 100.0);
        let player = spawn_player(&mut app, Vec2::new(150.0, 200.0));
        run(&mut app, 2);
        assert!(app.world().get::<Sentry>(sentry).unwrap().alerted());
        assert_eq!(
            app.world().get::<MoveGoal>(sentry).unwrap().0,
            Some(Vec2::new(150.0, 200.0))
        );
        // Out of sight now, and still followed.
        app.world_mut()
            .get_mut::<Transform>(player)
            .unwrap()
            .translation = Vec3::new(380.0, 380.0, 0.0);
        run(&mut app, 5);
        assert!(app.world().get::<Sentry>(sentry).unwrap().alerted());
        assert_eq!(
            app.world().get::<MoveGoal>(sentry).unwrap().0,
            Some(Vec2::new(380.0, 380.0))
        );
        // The player is gone: it stands still but stays alerted.
        app.world_mut().despawn(player);
        run(&mut app, 2);
        assert_eq!(app.world().get::<MoveGoal>(sentry).unwrap().0, None);
        assert!(app.world().get::<Sentry>(sentry).unwrap().alerted());
    }
}
