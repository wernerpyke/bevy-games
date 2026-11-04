//! Spawners: an entity that asks the game for a new entity every so many seconds. The kit keeps the timer and the
//! limit and sends `SpawnRequest`. The game reads it, spawns what it wants at the position and adds `SpawnedBy` to
//! it, so the spawner can count its live entities for `max`. See `dev-docs/PLAN-enemies.md`.

use bevy::prelude::*;

use crate::map::Params;

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

    /// Keeps a spawn due: used when a limit stops it, so it happens as soon as the limit allows.
    pub fn hold(&mut self) {
        self.elapsed = self.every;
    }
}

/// Spawns every `every` seconds at the entity's own position (`Transform` x and y), the first one at once.
#[derive(Component, Clone, Debug)]
pub struct Spawner {
    timer: SpawnTimer,
    /// The most live entities of this spawner. None means no limit.
    pub max: Option<usize>,
    /// The legend parameters of the spawner, passed to the game in each request.
    pub params: Params,
}

impl Spawner {
    pub fn new(every: f32, max: Option<usize>, params: Params) -> Self {
        Self {
            timer: SpawnTimer::new(every),
            max,
            params,
        }
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

/// Add this to every entity that a spawner asked for.
#[derive(Component, Clone, Copy, Debug)]
pub struct SpawnedBy(pub Entity);

/// A spawner is due. The game spawns the entity at `position`.
#[derive(Message, Clone, Debug)]
pub struct SpawnRequest {
    pub spawner: Entity,
    pub position: Vec2,
    pub params: Params,
}

fn run_spawners(
    time: Res<Time>,
    mut spawners: Query<(Entity, &Transform, &mut Spawner)>,
    spawned: Query<&SpawnedBy>,
    mut requests: MessageWriter<SpawnRequest>,
) {
    for (entity, transform, mut spawner) in &mut spawners {
        if !spawner.timer.tick(time.delta_secs()) {
            continue;
        }
        if let Some(max) = spawner.max {
            let alive = spawned.iter().filter(|by| by.0 == entity).count();
            if alive >= max {
                spawner.timer.hold();
                continue;
            }
        }
        requests.write(SpawnRequest {
            spawner: entity,
            position: transform.translation.truncate(),
            params: spawner.params.clone(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::time::TimeUpdateStrategy;
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

    #[derive(Resource, Default)]
    struct Requests(Vec<SpawnRequest>);

    fn collect(mut reader: MessageReader<SpawnRequest>, mut seen: ResMut<Requests>) {
        seen.0.extend(reader.read().cloned());
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                16,
            )))
            .init_resource::<Requests>()
            .add_plugins(SpawnerPlugin)
            .add_systems(Update, collect.after(run_spawners));
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
}
