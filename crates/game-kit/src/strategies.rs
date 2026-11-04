//! The movement strategies of the kit. A strategy only chooses a goal and writes it to `MoveGoal`: `movement.rs`
//! walks there. A game adds a strategy of its own the same way: a component and a system in `StrategySet` that writes
//! `MoveGoal`. See `dev-docs/extensions/enemy-movement.md`.
//!
//! - `RandomPatrol`: pick a random reachable place within a radius of the current place, walk there, pick again.
//! - `Pursue`: walk to the player (the `Controlled` entity), or to another entity.
//!
//! `StrategiesPlugin` needs `MovementPlugin`, `NavPlugin` and `RngPlugin`.

use bevy::prelude::*;

use crate::Controlled;
use crate::camera::CameraBounds;
use crate::map::Params;
use crate::movement::{MoveGoal, MoveState, StrategySet};
use crate::nav::NavGrid;
use crate::rng::{GameRng, Rng};

/// Tries per pick of a patrol target.
const PICK_TRIES: usize = 8;
/// Seconds to wait when no reachable target was found, before trying again.
const NO_TARGET_WAIT: f32 = 1.0;

pub struct StrategiesPlugin;

impl Plugin for StrategiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (patrol, pursue).in_set(StrategySet));
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
    for (transform, mut patrol, mut goal, state) in &mut patrols {
        let walking_to_goal = goal.0.is_some() && !state.arrived && !state.unreachable;
        if walking_to_goal {
            continue;
        }
        if patrol.wait > 0.0 {
            patrol.wait -= dt;
            goal.0 = None;
            continue;
        }
        let feet = transform.translation.truncate();
        let target = pick_target(
            &mut rng,
            feet,
            patrol.radius,
            bounds.as_ref().map(|b| b.0),
            PICK_TRIES,
            |point| nav.find_path(feet, point).is_some(),
        );
        match target {
            Some(point) => goal.0 = Some(point),
            None => {
                goal.0 = None;
                patrol.wait = NO_TARGET_WAIT;
            }
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
}
