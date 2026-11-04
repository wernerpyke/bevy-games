//! Enemies: the skeleton and the goblin. A spawner in the map file (a legend entry such as
//! `"S": spawn_skeleton every=10 strategy=patrol radius=120`) asks for an enemy at its interval. The movement
//! strategy is named in the same entry. The kit does the spawning, path finding and walking
//! (`game_kit::spawner`, `nav`, `movement`, `strategies`). Here are the kinds, their values and their art.
//! A projectile that hits an enemy removes it.

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;
use game_kit::depth::DepthSort;
use game_kit::hitbox::HitBox;
use game_kit::map::{MapObject, Params};
use game_kit::movement::{FollowSet, MoveState, Mover};
use game_kit::projectile::{HitCause, ProjectileHit, ProjectileTarget};
use game_kit::spawner::{SpawnRequest, SpawnedBy, Spawner};
use game_kit::strategies::{Pursue, RandomPatrol};

use crate::camera::CELL_SIZE;
use crate::collision::FEET_HALF_WIDTH;
use crate::map::{MapEntity, MapSpawners};
use crate::player::WALK_SPEED;
use crate::walk_animation::{WalkAnimation, WalkSprite, WalkStateSet};

/// Height of the canvas of `monsters.aseprite` (13 wide, 11 high).
const CANVAS_HEIGHT: f32 = 11.0;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_spawners).add_systems(
            Update,
            (
                spawn_enemies,
                remove_hit_enemies,
                show_walking.in_set(WalkStateSet).after(FollowSet),
                log_positions,
            ),
        );
    }
}

#[derive(Component)]
pub struct Enemy;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyKind {
    Skeleton,
    Goblin,
}

impl EnemyKind {
    /// The legend name of the spawner of this kind.
    pub const SPAWNER_NAMES: [&'static str; 2] = ["spawn_skeleton", "spawn_goblin"];

    pub fn from_spawner_name(name: &str) -> Option<EnemyKind> {
        match name {
            "spawn_skeleton" => Some(EnemyKind::Skeleton),
            "spawn_goblin" => Some(EnemyKind::Goblin),
            _ => None,
        }
    }

    /// Walk speed in art pixels per second, as a share of the player's (`SPEC.md`).
    pub fn speed(self) -> f32 {
        WALK_SPEED
            * match self {
                EnemyKind::Skeleton => 0.8,
                EnemyKind::Goblin => 0.2,
            }
    }

    /// The tag in `art/characters/monsters.aseprite`.
    fn tag(self) -> &'static str {
        match self {
            EnemyKind::Skeleton => "mon_skeleton",
            EnemyKind::Goblin => "mon_goblin",
        }
    }

    /// The body: the sprite without its bottom 2 px. The art is bottom centre in the canvas.
    pub fn hit_box(self) -> HitBox {
        let (width, height) = match self {
            EnemyKind::Skeleton => (13.0, 11.0),
            EnemyKind::Goblin => (13.0, 10.0),
        };
        let body = height - 2.0;
        HitBox {
            offset: Vec2::new(0.0, 2.0 + body / 2.0),
            half_size: Vec2::new(width / 2.0, body / 2.0),
        }
    }
}

/// One spawner of the map, checked when the map is loaded.
#[derive(Clone, Debug)]
pub struct SpawnerDef {
    pub cell: IVec2,
    pub kind: EnemyKind,
    pub spawner: Spawner,
}

/// The movement strategy that a spawner names.
enum Strategy {
    Patrol(RandomPatrol),
    Pursue(Pursue),
}

fn strategy_from_params(params: &Params) -> Result<Strategy, String> {
    match params.get("strategy") {
        Some("patrol") => RandomPatrol::from_params(params).map(Strategy::Patrol),
        Some("pursue") => Pursue::from_params(params).map(Strategy::Pursue),
        Some(other) => Err(format!(
            "unknown strategy `{other}` (known: patrol, pursue)"
        )),
        None => {
            Err("a spawner needs a parameter `strategy=patrol` or `strategy=pursue`".to_string())
        }
    }
}

/// Checks the legend parameters of a spawner object. The error names the file and the legend line.
pub fn spawner_def(file: &str, object: &MapObject) -> Result<SpawnerDef, String> {
    let fail = |message: String| {
        format!(
            "{file}:{}: spawner `{}`: {message}",
            object.params.line(),
            object.kind
        )
    };
    let kind = EnemyKind::from_spawner_name(&object.kind)
        .ok_or_else(|| fail("not an enemy spawner".to_string()))?;
    let spawner = Spawner::from_params(&object.params).map_err(&fail)?;
    strategy_from_params(&object.params).map_err(&fail)?;
    Ok(SpawnerDef {
        cell: object.cell,
        kind,
        spawner,
    })
}

/// Creates the spawner entities of the current map. Also run again when the map is reloaded (F1). Each spawner
/// stands at the middle of its cell.
pub fn spawn_spawners(mut commands: Commands, spawners: Res<MapSpawners>) {
    for def in &spawners.0 {
        let centre = (def.cell.as_vec2() + Vec2::splat(0.5)) * CELL_SIZE;
        commands.spawn((
            MapEntity,
            def.kind,
            def.spawner.clone(),
            Transform::from_translation(centre.extend(0.0)),
        ));
    }
}

fn spawn_enemies(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut requests: MessageReader<SpawnRequest>,
    kinds: Query<&EnemyKind, Without<Enemy>>,
) {
    for request in requests.read() {
        let Ok(&kind) = kinds.get(request.spawner) else {
            continue;
        };
        let strategy = match strategy_from_params(&request.params) {
            Ok(strategy) => strategy,
            Err(message) => {
                error!("enemy: {message}");
                continue;
            }
        };
        let mut enemy = commands.spawn((
            Enemy,
            kind,
            MapEntity,
            SpawnedBy(request.spawner),
            Mover::new(kind.speed()).with_edge_margin(FEET_HALF_WIDTH),
            WalkAnimation::default(),
            kind.hit_box(),
            ProjectileTarget,
            DepthSort { feet_offset: 0.0 },
            Transform::from_translation(request.position.extend(0.0)),
            Visibility::default(),
        ));
        match strategy {
            Strategy::Patrol(patrol) => enemy.insert(patrol),
            Strategy::Pursue(pursue) => enemy.insert(pursue),
        };
        enemy.with_child((
            WalkSprite {
                canvas_height: CANVAS_HEIGHT,
            },
            AseAnimation {
                aseprite: server.load("art/characters/monsters.aseprite"),
                animation: Animation::tag(kind.tag()),
            },
            Sprite::default(),
            Transform::from_xyz(0.0, CANVAS_HEIGHT / 2.0, 0.0),
        ));
        debug!(
            "enemy: {kind:?} spawned at {:?} by {:?}",
            request.position, request.spawner
        );
    }
}

/// A projectile that hit an enemy removes it. A dead enemy leaves nothing behind.
fn remove_hit_enemies(
    mut commands: Commands,
    mut hits: MessageReader<ProjectileHit>,
    enemies: Query<(), With<Enemy>>,
) {
    for hit in hits.read() {
        if let HitCause::Target(entity) = hit.cause
            && enemies.contains(entity)
        {
            commands.entity(entity).despawn();
            debug!("enemy: {entity:?} killed at {:?}", hit.position);
        }
    }
}

fn show_walking(mut enemies: Query<(&MoveState, &mut WalkAnimation), With<Enemy>>) {
    for (state, mut walk) in &mut enemies {
        walk.walking = state.walking;
        if state.walking {
            walk.face(state.facing);
        }
    }
}

/// Once a second, at debug level, where each enemy stands. Scripted runs read it (`LOG_FILTER="enemy:"`).
fn log_positions(
    time: Res<Time>,
    mut since_log: Local<f32>,
    enemies: Query<(Entity, &EnemyKind, &Transform, &MoveState), With<Enemy>>,
) {
    *since_log += time.delta_secs();
    if *since_log < 1.0 {
        return;
    }
    *since_log = 0.0;
    for (entity, kind, transform, state) in &enemies {
        debug!(
            "enemy: {entity:?} {kind:?} at ({:.0}, {:.0}) walking {} unreachable {}",
            transform.translation.x, transform.translation.y, state.walking, state.unreachable
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object(kind: &str, pairs: &[(&str, &str)]) -> MapObject {
        MapObject {
            kind: kind.to_string(),
            cell: IVec2::new(3, 4),
            params: Params::from_pairs(pairs),
        }
    }

    #[test]
    fn speeds_are_shares_of_the_player_speed() {
        assert_eq!(EnemyKind::Skeleton.speed(), 0.8 * WALK_SPEED);
        assert_eq!(EnemyKind::Goblin.speed(), 0.2 * WALK_SPEED);
    }

    #[test]
    fn a_hit_box_leaves_out_the_bottom_two_pixels() {
        let skeleton = EnemyKind::Skeleton.hit_box().rect_at(Vec2::ZERO);
        assert_eq!(skeleton, Rect::new(-6.5, 2.0, 6.5, 11.0));
        let goblin = EnemyKind::Goblin.hit_box().rect_at(Vec2::ZERO);
        assert_eq!(goblin, Rect::new(-6.5, 2.0, 6.5, 10.0));
    }

    #[test]
    fn the_map_entries_of_the_test_map_are_valid() {
        let skeleton = object(
            "spawn_skeleton",
            &[("every", "10"), ("strategy", "patrol"), ("radius", "120")],
        );
        let def = spawner_def("t.txt", &skeleton).unwrap();
        assert_eq!(def.kind, EnemyKind::Skeleton);
        assert_eq!(def.cell, IVec2::new(3, 4));
        let goblin = object("spawn_goblin", &[("every", "15"), ("strategy", "pursue")]);
        assert_eq!(
            spawner_def("t.txt", &goblin).unwrap().kind,
            EnemyKind::Goblin
        );
    }

    #[test]
    fn bad_spawner_entries_are_errors_that_name_the_file() {
        for (pairs, text) in [
            (vec![("every", "10")], "needs a parameter `strategy"),
            (
                vec![("every", "10"), ("strategy", "evade")],
                "unknown strategy `evade`",
            ),
            (vec![("every", "10"), ("strategy", "patrol")], "radius"),
            (vec![("strategy", "pursue")], "every"),
        ] {
            let error = spawner_def("map.txt", &object("spawn_goblin", &pairs)).unwrap_err();
            assert!(error.starts_with("map.txt:0:"), "{error}");
            assert!(error.contains(text), "{pairs:?}: {error}");
        }
    }
}
