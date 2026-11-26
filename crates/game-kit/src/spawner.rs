//! Spawners: an entity that asks the game for new entities every so many seconds. The kit keeps the timer, the group
//! size, the limit and the places, and sends one `SpawnRequest` per entity. The game reads it, spawns what it wants at
//! the position and adds `SpawnedBy` to it, so the spawner can count its live entities for `max`. See
//! `dev-docs/extensions/spawners.md`.

use bevy::prelude::*;

use crate::collision::CollisionWorld;
use crate::map::Params;
use crate::map_transition::MapId;
use crate::rng::{GameRng, Rng};
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;

pub struct SpawnerPlugin;

impl Plugin for SpawnerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnRequest>()
            .add_systems(Update, run_spawners);
    }
}

/// The repeat timer of a spawner. It is full at the start, so the first spawn is at once.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpawnTimer {
    every: f32,
    elapsed: f32,
}

impl SpawnTimer {
    pub fn new(every: f32) -> Self {
        Self {
            every,
            elapsed: every,
        }
    }

    /// Adds `dt` and says whether a spawn is due. At most one spawn per call, however long the frame was.
    pub fn tick(&mut self, dt: f32) -> bool {
        self.elapsed += dt;
        if self.elapsed < self.every {
            return false;
        }
        self.elapsed %= self.every;
        true
    }

    /// A timer that is empty at the start, so the first spawn comes one interval after it.
    pub fn after_gap(every: f32) -> Self {
        Self {
            every,
            elapsed: 0.0,
        }
    }

    /// Changes the interval and starts it again, so the next spawn comes one new interval from now.
    pub fn restart(&mut self, every: f32) {
        self.every = every;
        self.elapsed = 0.0;
    }

    /// Keeps a spawn due: used when a limit stops it, so it happens as soon as the limit allows.
    pub fn hold(&mut self) {
        self.elapsed = self.every;
    }
}

/// Spawns `group` entities every `every` seconds, the first group at once (or one interval after the start, with
/// `first_after_gap`). Each entity is at the entity's own position (`Transform` x and y), or at a free point of
/// `places`.
#[derive(Component, Clone, Debug)]
pub struct Spawner {
    timer: SpawnTimer,
    /// Entities per spawn, sent as that many requests in one frame. `max` can cut a group short.
    pub group: u32,
    /// Where the entities start. None means the spawner's own position.
    pub places: Option<SpawnPlaces>,
    /// Set after the warning that `places` has no free point, so it is logged once.
    warned_no_place: bool,
    /// The most live entities of this spawner. None means no limit.
    pub max: Option<usize>,
    /// The legend parameters of the spawner, passed to the game in each request.
    pub params: Params,
}

impl Spawner {
    pub fn new(every: f32, max: Option<usize>, params: Params) -> Self {
        Self {
            timer: SpawnTimer::new(every),
            group: 1,
            places: None,
            warned_no_place: false,
            max,
            params,
        }
    }

    /// The first spawn comes one interval after the start, not at once.
    pub fn first_after_gap(mut self) -> Self {
        self.timer = SpawnTimer::after_gap(self.timer.every);
        self
    }

    pub fn with_group(mut self, group: u32) -> Self {
        self.group = group;
        self
    }

    pub fn with_places(mut self, places: SpawnPlaces) -> Self {
        self.places = Some(places);
        self
    }

    /// Changes the interval while the game runs and starts it again: the next spawn is one new interval from now.
    pub fn set_every(&mut self, every: f32) {
        self.timer.restart(every);
    }

    /// The interval in seconds.
    pub fn every(&self) -> f32 {
        self.timer.every
    }

    /// Reads `every` (seconds, above 0) and `max` (a whole number, 1 or more, optional) from the legend
    /// parameters. The parameters are kept for the game.
    pub fn from_params(params: &Params) -> Result<Self, String> {
        let every = match params.f32("every")? {
            Some(seconds) if seconds > 0.0 => seconds,
            Some(seconds) => {
                return Err(format!(
                    "parameter `every` must be above 0, found {seconds}"
                ));
            }
            None => return Err("a spawner needs a parameter `every=<seconds>`".to_string()),
        };
        let max = match params.f32("max")? {
            Some(count) if count >= 1.0 && count.fract() == 0.0 => Some(count as usize),
            Some(count) => {
                return Err(format!(
                    "parameter `max` must be a whole number from 1, found {count}"
                ));
            }
            None => None,
        };
        Ok(Self::new(every, max, params.clone()))
    }
}

/// Add this to every entity that a spawner asked for. It also takes the spawner's `MapId`, unless it is spawned with
/// one, so it goes with the spawner's map (adr-x60).
#[derive(Component, Clone, Copy, Debug)]
#[component(on_insert = take_spawner_map)]
pub struct SpawnedBy(pub Entity);

fn take_spawner_map(mut world: DeferredWorld, context: HookContext) {
    let Some(by) = world.get::<SpawnedBy>(context.entity).map(|by| by.0) else {
        return;
    };
    if world.get::<MapId>(context.entity).is_some() {
        return;
    }
    if let Some(map) = world.get::<MapId>(by).cloned() {
        world.commands().entity(context.entity).insert(map);
    }
}

/// A spawner is due. The game spawns the entity at `position`.
#[derive(Message, Clone, Debug)]
pub struct SpawnRequest {
    pub spawner: Entity,
    pub position: Vec2,
    pub params: Params,
}

/// Where a spawner's entities start: groups of points, for example one group per edge of a field. Each entity picks
/// a group at random, each group with the same chance however many points it has, then a free point of that group.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpawnPlaces {
    pub groups: Vec<Vec<Vec2>>,
}

impl SpawnPlaces {
    /// A random point for which `free` is true. Groups with no free point are left out. None when no point is free.
    pub fn pick(&self, rng: &mut Rng, free: impl Fn(Vec2) -> bool) -> Option<Vec2> {
        let open: Vec<Vec<Vec2>> = self
            .groups
            .iter()
            .map(|group| {
                group
                    .iter()
                    .copied()
                    .filter(|p| free(*p))
                    .collect::<Vec<_>>()
            })
            .filter(|group| !group.is_empty())
            .collect();
        if open.is_empty() {
            return None;
        }
        let group = &open[(rng.next_u64() % open.len() as u64) as usize];
        Some(group[(rng.next_u64() % group.len() as u64) as usize])
    }
}

fn run_spawners(
    time: Res<Time>,
    mut spawners: Query<(Entity, &Transform, &mut Spawner)>,
    spawned: Query<&SpawnedBy>,
    world: Option<Res<CollisionWorld>>,
    mut rng: Option<ResMut<GameRng>>,
    mut requests: MessageWriter<SpawnRequest>,
) {
    for (entity, transform, mut spawner) in &mut spawners {
        if !spawner.timer.tick(time.delta_secs()) {
            continue;
        }
        let mut room = spawner.group as usize;
        if let Some(max) = spawner.max {
            let alive = spawned.iter().filter(|by| by.0 == entity).count();
            if alive >= max {
                spawner.timer.hold();
                continue;
            }
            room = room.min(max - alive);
        }
        for _ in 0..room {
            let position = match (&spawner.places, rng.as_deref_mut()) {
                (None, _) => Some(transform.translation.truncate()),
                (Some(places), Some(rng)) => places.pick(rng, |point| {
                    world.as_ref().is_none_or(|world| !world.is_blocked(point))
                }),
                (Some(_), None) => {
                    if !spawner.warned_no_place {
                        warn!(
                            "spawner: {entity:?} has places but there is no `GameRng` (add `RngPlugin`)"
                        );
                    }
                    None
                }
            };
            let Some(position) = position else {
                if !spawner.warned_no_place {
                    warn!("spawner: {entity:?} has no free place, the spawn is skipped");
                    spawner.warned_no_place = true;
                }
                break;
            };
            requests.write(SpawnRequest {
                spawner: entity,
                position,
                params: spawner.params.clone(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{Collected, collect, timed_app};
    use std::time::Duration;

    #[test]
    fn the_first_spawn_is_at_once_then_every_interval() {
        let mut timer = SpawnTimer::new(10.0);
        assert!(timer.tick(0.0));
        assert!(!timer.tick(9.9));
        assert!(timer.tick(0.2));
        assert!(!timer.tick(5.0));
    }

    #[test]
    fn a_long_frame_gives_one_spawn_not_a_burst() {
        let mut timer = SpawnTimer::new(1.0);
        assert!(timer.tick(0.0));
        assert!(timer.tick(7.5));
        assert!(!timer.tick(0.1));
    }

    #[test]
    fn a_timer_after_a_gap_fires_first_one_interval_after_the_start() {
        let mut timer = SpawnTimer::after_gap(3.0);
        assert!(!timer.tick(0.0));
        assert!(!timer.tick(2.9));
        assert!(timer.tick(0.2));
        assert!(!timer.tick(2.5));
    }

    #[test]
    fn a_restart_changes_the_interval_and_counts_it_from_now() {
        let mut timer = SpawnTimer::new(4.0);
        assert!(timer.tick(0.0));
        assert!(!timer.tick(3.5));
        // 3.5 s of the old 4 s interval had passed: the restart drops them.
        timer.restart(2.0);
        assert!(!timer.tick(1.9));
        assert!(timer.tick(0.2));
        assert!(!timer.tick(1.7));
        assert!(timer.tick(0.2));
    }

    #[test]
    fn each_group_of_places_has_the_same_chance_however_many_points_it_has() {
        let alone = Vec2::new(-5.0, 7.0);
        let crowd: Vec<Vec2> = (0..9).map(|i| Vec2::new(i as f32 * 3.0, 40.0)).collect();
        let places = SpawnPlaces {
            groups: vec![vec![alone], crowd.clone()],
        };
        let mut rng = Rng::new(11);
        let picks: Vec<Vec2> = (0..2000)
            .map(|_| places.pick(&mut rng, |_| true).unwrap())
            .collect();
        let lone = picks.iter().filter(|p| **p == alone).count();
        assert!((900..1100).contains(&lone), "{lone} of 2000");
        // Every point of the large group is picked.
        for point in &crowd {
            assert!(picks.contains(point), "{point}");
        }
    }

    #[test]
    fn a_place_that_is_not_free_is_never_picked_and_a_group_with_none_free_is_left_out() {
        let blocked = Vec2::new(1.0, 1.0);
        let open = Vec2::new(2.0, 2.0);
        let places = SpawnPlaces {
            groups: vec![vec![blocked, open], vec![Vec2::new(9.0, 9.0)]],
        };
        let free = |p: Vec2| p != blocked && p.x < 5.0;
        let mut rng = Rng::new(3);
        for _ in 0..200 {
            assert_eq!(places.pick(&mut rng, free), Some(open));
        }
        assert_eq!(places.pick(&mut rng, |_| false), None);
        assert_eq!(SpawnPlaces::default().pick(&mut rng, |_| true), None);
    }

    #[test]
    fn a_held_timer_fires_at_the_next_tick() {
        let mut timer = SpawnTimer::new(10.0);
        assert!(timer.tick(0.0));
        timer.hold();
        assert!(timer.tick(0.0));
    }

    #[test]
    fn settings_are_read_from_the_parameters() {
        let ok = Params::from_pairs(&[("every", "15"), ("strategy", "pursue")]);
        let spawner = Spawner::from_params(&ok).unwrap();
        assert_eq!(spawner.max, None);
        assert_eq!(spawner.params.get("strategy"), Some("pursue"));
        let limited = Params::from_pairs(&[("every", "10"), ("max", "3")]);
        assert_eq!(Spawner::from_params(&limited).unwrap().max, Some(3));
        for (pairs, text) in [
            (vec![("max", "3")], "needs"),
            (vec![("every", "0")], "above 0"),
            (vec![("every", "soon")], "number"),
            (vec![("every", "5"), ("max", "0")], "whole number"),
            (vec![("every", "5"), ("max", "1.5")], "whole number"),
        ] {
            let error = Spawner::from_params(&Params::from_pairs(&pairs)).unwrap_err();
            assert!(error.contains(text), "{pairs:?}: {error}");
        }
    }

    type Requests = Collected<SpawnRequest>;

    fn app() -> App {
        let mut app = timed_app(Duration::from_millis(16));
        app.init_resource::<Requests>()
            .add_plugins(SpawnerPlugin)
            .add_systems(Update, collect::<SpawnRequest>.after(run_spawners));
        app
    }

    #[test]
    fn a_spawner_sends_a_request_at_the_start_and_then_every_interval() {
        let mut app = app();
        app.world_mut().spawn((
            Transform::from_xyz(40.0, 80.0, 0.0),
            Spawner::new(1.0, None, Params::default()),
        ));
        for _ in 0..225 {
            app.update();
        }
        let seen = &app.world().resource::<Requests>().0;
        // 225 frames of 16 ms are 3.58 s: spawns at 0, 1, 2 and 3 seconds.
        assert_eq!(seen.len(), 4);
        assert_eq!(seen[0].position, Vec2::new(40.0, 80.0));
    }

    fn drain(app: &mut App) -> Vec<SpawnRequest> {
        app.world_mut()
            .resource_mut::<Requests>()
            .0
            .drain(..)
            .collect()
    }

    #[test]
    fn a_group_sends_its_requests_in_one_frame_and_max_cuts_it_short() {
        let mut app = app();
        let spawner = app
            .world_mut()
            .spawn((
                Transform::from_xyz(-12.0, 30.0, 0.0),
                Spawner::new(1.0, Some(5), Params::default()).with_group(3),
            ))
            .id();
        app.update();
        let first = drain(&mut app);
        assert_eq!(first.len(), 3);
        assert!(
            first
                .iter()
                .all(|r| r.spawner == spawner && r.position == Vec2::new(-12.0, 30.0))
        );
        for _ in 0..3 {
            app.world_mut().spawn(SpawnedBy(spawner));
        }
        // 1 s later: 3 alive of at most 5, so only 2 of the group of 3.
        for _ in 0..63 {
            app.update();
        }
        assert_eq!(drain(&mut app).len(), 2);
    }

    #[test]
    fn a_spawner_that_starts_after_a_gap_sends_nothing_at_the_start() {
        let mut app = app();
        app.world_mut().spawn((
            Transform::default(),
            Spawner::new(0.5, None, Params::default()).first_after_gap(),
        ));
        // The first update has no time step, so 32 updates are 31 frames of 16 ms, 0.496 s: not yet.
        for _ in 0..32 {
            app.update();
        }
        assert_eq!(drain(&mut app).len(), 0);
        app.update();
        assert_eq!(drain(&mut app).len(), 1);
    }

    #[test]
    fn requests_use_a_place_that_the_collision_world_leaves_free() {
        let mut app = app();
        let inside = Vec2::new(50.0, 50.0);
        let open = Vec2::new(-70.0, 10.0);
        let mut world = CollisionWorld::default();
        world.add_blocked(&[Rect::new(40.0, 40.0, 60.0, 60.0)]);
        app.insert_resource(world)
            .insert_resource(GameRng::seeded(5));
        app.world_mut().spawn((
            Transform::default(),
            Spawner::new(0.1, None, Params::default())
                .with_group(4)
                .with_places(SpawnPlaces {
                    groups: vec![vec![inside], vec![open]],
                }),
        ));
        app.update();
        let positions: Vec<Vec2> = drain(&mut app).iter().map(|r| r.position).collect();
        assert_eq!(positions, vec![open; 4]);
        // The open place is blocked now (the forest grew): no place is free, so there is no request.
        app.world_mut()
            .resource_mut::<CollisionWorld>()
            .add_temporary(Rect::new(-80.0, 0.0, -60.0, 20.0));
        for _ in 0..20 {
            app.update();
        }
        assert_eq!(drain(&mut app).len(), 0);
    }

    #[test]
    fn a_spawned_entity_takes_the_map_of_its_spawner_unless_it_has_one() {
        let mut app = App::new();
        let spawner = app.world_mut().spawn(MapId("east".into())).id();
        let unmapped = app.world_mut().spawn_empty().id();
        let plain = app.world_mut().spawn(SpawnedBy(spawner)).id();
        let own = app
            .world_mut()
            .spawn((MapId("west".into()), SpawnedBy(spawner)))
            .id();
        let orphan = app.world_mut().spawn(SpawnedBy(unmapped)).id();
        app.world_mut().flush();
        let map = |entity| app.world().get::<MapId>(entity).map(|id| id.0.as_str());
        assert_eq!(map(plain), Some("east"));
        assert_eq!(map(own), Some("west"));
        assert_eq!(map(orphan), None);
    }

    #[test]
    fn the_limit_counts_live_entities_of_the_spawner() {
        let mut app = app();
        let spawner = app
            .world_mut()
            .spawn((
                Transform::default(),
                Spawner::new(0.5, Some(2), Params::default()),
            ))
            .id();
        for _ in 0..400 {
            app.update();
            let requests: Vec<Entity> = app
                .world_mut()
                .resource_mut::<Requests>()
                .0
                .drain(..)
                .map(|r| r.spawner)
                .collect();
            for by in requests {
                app.world_mut().spawn(SpawnedBy(by));
            }
        }
        let alive = app
            .world_mut()
            .query::<&SpawnedBy>()
            .iter(app.world())
            .filter(|by| by.0 == spawner)
            .count();
        assert_eq!(alive, 2);
        // One dies: the next request comes at the next frame.
        let victim = app
            .world_mut()
            .query_filtered::<Entity, With<SpawnedBy>>()
            .iter(app.world())
            .next()
            .unwrap();
        app.world_mut().despawn(victim);
        app.update();
        app.update();
        assert_eq!(app.world().resource::<Requests>().0.len(), 1);
    }

    #[test]
    fn the_entities_of_another_spawner_do_not_count_towards_the_limit() {
        let mut app = app();
        let spawn = |app: &mut App| {
            app.world_mut()
                .spawn((
                    Transform::default(),
                    Spawner::new(1.0, Some(1), Params::default()),
                ))
                .id()
        };
        let full = spawn(&mut app);
        let empty = spawn(&mut app);
        app.world_mut().spawn(SpawnedBy(full));
        app.update();
        let requests: Vec<Entity> = app
            .world()
            .resource::<Requests>()
            .0
            .iter()
            .map(|r| r.spawner)
            .collect();
        assert_eq!(requests, [empty]);
    }
}
