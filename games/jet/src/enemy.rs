//! Enemies: they come in from the field edges and raid the house (`SPEC.md` section 6). One kind now, the Spinner.
//! The kit spawner places them (`spawning.rs`), the kit `Mover` walks them around trees, and the Jet strategy
//! `RaidHouse` picks their goal. A Spinner hits the house when it arrives at its target point next to the house
//! (`adr-jt17`).

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::{AnimationEvents, AnimationRepeat, AseAnimation};
use game_kit::animation::tag_sprite;
use game_kit::depth::DepthSort;
use game_kit::movement::{FollowSet, MoveGoal, MoveState, Mover, StrategySet, Stunned};
use game_kit::rng::GameRng;
use game_kit::spawner::{SpawnRequest, SpawnedBy};

use crate::camera::CELL_SIZE;
use crate::house::HouseHit;
use crate::map::{HouseCell, cell_corner};

const MOVE: &str = "Move";
const KILL: &str = "Kill";

/// The kinds of enemy. A new kind is a new row of the tables below and its own art.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyKind {
    Spinner,
}

impl EnemyKind {
    /// World units per second (`adr-jt4`).
    pub fn speed(self) -> f32 {
        match self {
            EnemyKind::Spinner => 3.75 * CELL_SIZE,
        }
    }

    pub fn art(self) -> &'static str {
        match self {
            EnemyKind::Spinner => "art/spinner.aseprite",
        }
    }
}

#[derive(Component)]
pub struct Enemy;

/// Set the moment an enemy hits the house (or, later, is killed). No check sees it again (`adr-jt11`), and it is
/// removed when its last animation ends.
#[derive(Component)]
pub struct Dead;

/// The filter for enemies that are still in play.
type Alive = (With<Enemy>, Without<Dead>);

/// The Jet strategy: walk to one of the target points next to the house. `target` is the index into `target_feet`.
#[derive(Component, Clone, Copy, Debug)]
pub struct RaidHouse {
    pub target: usize,
}

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_enemies)
            .add_systems(Update, raid_house.in_set(StrategySet))
            .add_systems(
                Update,
                (hit_house, remove_finished, report_stuck)
                    .chain()
                    .after(FollowSet),
            );
    }
}

/// The feet points an enemy walks to, one cell from the house whose footprint (4 x 4 cells) has its bottom left cell
/// at `anchor`. From `SPEC.md` section 6, relative to the house's top left cell (x right, y down): left (-1, +1),
/// right (+4, +1), below left (0, +4), below right (+3, +4). The feet are at the bottom middle of the cell, so the
/// enemy's sprite fills the cell and touches the house wall.
pub fn target_feet(anchor: IVec2, cell_size: f32) -> [Vec2; 4] {
    let top_left = anchor + IVec2::new(0, 3);
    [(-1, 1), (4, 1), (0, 4), (3, 4)].map(|(right, down)| {
        let cell = top_left + IVec2::new(right, -down);
        cell_corner(cell, cell_size) + Vec2::new(cell_size / 2.0, 0.0)
    })
}

/// A random target index other than `not`, for `count` targets.
fn other_target(roll: u64, not: usize, count: usize) -> usize {
    let pick = (roll % (count as u64 - 1)) as usize;
    if pick >= not { pick + 1 } else { pick }
}

fn spawn_enemies(
    mut commands: Commands,
    mut requests: MessageReader<SpawnRequest>,
    server: Res<AssetServer>,
    mut rng: ResMut<GameRng>,
) {
    for request in requests.read() {
        let kind = EnemyKind::Spinner;
        // A request may name its target (the script command `spinner`); else it is random.
        let target = request
            .params
            .get("target")
            .and_then(|t| t.parse::<usize>().ok())
            .filter(|t| *t < 4)
            .unwrap_or_else(|| (rng.next_u64() % 4) as usize);
        let half = CELL_SIZE / 2.0;
        commands.spawn((
            Enemy,
            kind,
            SpawnedBy(request.spawner),
            Mover::new(kind.speed()),
            RaidHouse { target },
            DepthSort { feet_offset: half },
            Transform::from_translation((request.position + Vec2::new(0.0, half)).extend(0.0)),
            tag_sprite(&server, kind.art(), MOVE),
        ));
        debug!(
            "enemy: {kind:?} at feet {} for target {target}",
            request.position
        );
    }
}

/// Sets the goal to the target point. When the kit finds no path to it, picks another target, so an enemy never
/// stands still for good.
fn raid_house(
    house: Option<Res<HouseCell>>,
    mut rng: ResMut<GameRng>,
    mut enemies: Query<(Entity, &mut RaidHouse, &mut MoveGoal, &MoveState), Without<Dead>>,
) {
    let Some(house) = house else {
        return;
    };
    let targets = target_feet(house.0, CELL_SIZE);
    for (entity, mut raid, mut goal, state) in &mut enemies {
        if state.unreachable && goal.0 == Some(targets[raid.target]) {
            raid.target = other_target(rng.next_u64(), raid.target, targets.len());
            debug!(
                "enemy: {entity:?} cannot reach its target, now target {}",
                raid.target
            );
        }
        let wanted = Some(targets[raid.target]);
        if goal.0 != wanted {
            goal.0 = wanted;
        }
    }
}

/// An enemy that arrived at its target hits the house: it stops, is marked dead, plays `Kill` once and sends
/// `HouseHit`.
fn hit_house(
    mut commands: Commands,
    mut enemies: Query<(Entity, &MoveState, &MoveGoal, &mut AseAnimation), Alive>,
    mut hits: MessageWriter<HouseHit>,
) {
    for (entity, state, goal, mut ase) in &mut enemies {
        if !state.arrived || goal.0.is_none() {
            continue;
        }
        commands.entity(entity).insert((Dead, Stunned));
        ase.animation.play(KILL, AnimationRepeat::Count(0));
        hits.write(HouseHit { by: entity });
    }
}

/// Seconds without moving after which a live enemy is reported as stuck.
const STUCK_SECONDS: f32 = 0.2;

/// What `report_stuck` reads of each live enemy.
type Watched = (
    Entity,
    &'static Transform,
    &'static DepthSort,
    &'static RaidHouse,
    &'static MoveState,
    &'static MoveGoal,
    &'static game_kit::movement::PathFollower,
);

/// Debug aid: logs a live enemy that has not moved for `STUCK_SECONDS`, once per stop, with its map cell (column,
/// row from the top), target, mover state and path.
fn report_stuck(
    time: Res<Time>,
    map: Option<Res<crate::map::CurrentMap>>,
    enemies: Query<Watched, Alive>,
    mut still: Local<std::collections::HashMap<Entity, (Vec2, f32, bool)>>,
) {
    let Some(map) = map else {
        return;
    };
    still.retain(|entity, _| enemies.contains(*entity));
    for (entity, transform, sort, raid, state, goal, follower) in &enemies {
        let feet = sort.feet(transform);
        let entry = still.entry(entity).or_insert((feet, 0.0, false));
        if entry.0.distance(feet) > 0.5 {
            *entry = (feet, 0.0, false);
            continue;
        }
        entry.1 += time.delta_secs();
        if entry.1 >= STUCK_SECONDS && !entry.2 {
            entry.2 = true;
            let cell = ((feet - crate::camera::MAP_ORIGIN) / CELL_SIZE)
                .floor()
                .as_ivec2();
            debug!(
                "enemy: {entity:?} stuck at feet {feet} cell ({}, {}) target {} goal {:?} walking {} arrived {} unreachable {} path {:?}",
                cell.x,
                map.map.size.y - 1 - cell.y,
                raid.target,
                goal.0,
                state.walking,
                state.arrived,
                state.unreachable,
                follower.points().collect::<Vec<_>>()
            );
        }
    }
}

/// Removes a dead enemy when its last animation ends.
fn remove_finished(
    mut commands: Commands,
    mut events: MessageReader<AnimationEvents>,
    dead: Query<(), (With<Enemy>, With<Dead>)>,
) {
    for event in events.read() {
        if let AnimationEvents::Finished(entity) = event
            && dead.contains(*entity)
        {
            debug!("enemy: {entity:?} removed after its last animation");
            commands.entity(*entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::MAP_ORIGIN;

    #[test]
    fn the_targets_are_the_cells_beside_and_below_the_house_at_their_bottom_middle() {
        // House footprint from cell (5, 7) to (8, 10); cells of 10. Top left cell is (5, 10).
        let feet = target_feet(IVec2::new(5, 7), 10.0);
        let at = |x: f32, y: f32| MAP_ORIGIN + Vec2::new(x, y);
        assert_eq!(
            feet,
            [
                // Left of the second row from the top: cell (4, 9).
                at(45.0, 90.0),
                // Right of it: cell (9, 9).
                at(95.0, 90.0),
                // Below the bottom left cell: cell (5, 6).
                at(55.0, 60.0),
                // Below the bottom right cell: cell (8, 6).
                at(85.0, 60.0),
            ]
        );
    }

    #[test]
    fn another_target_is_never_the_one_that_failed_and_each_other_one_can_come() {
        for not in 0..4 {
            let picks: Vec<usize> = (0..3).map(|roll| other_target(roll, not, 4)).collect();
            let expected: Vec<usize> = (0..4).filter(|t| *t != not).collect();
            let mut sorted = picks.clone();
            sorted.sort();
            assert_eq!(sorted, expected, "not {not}");
        }
    }
}
