//! Spawning and the difficulty curve (`SPEC.md` section 8, `adr-jt18`). One kit `Spawner` puts groups of enemies on
//! free cells of the four field edges. Every 10 s of game time the level goes up: the group size and the gap range
//! change, and a new gap is drawn from the range.

use bevy::prelude::*;
use game_kit::cell_area::CellGrid;
use game_kit::map::Params;
use game_kit::rng::GameRng;
use game_kit::script::ScriptCommand;
use game_kit::spawner::{SpawnPlaces, SpawnRequest, Spawner};

use crate::camera::CELL_SIZE;
use crate::forest::forest_grid;
use crate::map::{CurrentMap, SIDE_FOREST_COLUMNS, cell_corner};

/// Seconds of game time between level-ups.
const LEVEL_SECONDS: f32 = 10.0;
/// The script command that jumps to a level: `level:<n>`.
pub const LEVEL_COMMAND: &str = "level";
/// The script command that spawns one Spinner on a cell: `spinner:<column>:<row>[:<target>]`.
pub const SPINNER_COMMAND: &str = "spinner";

/// The settings of one level. Worked out from the level number in whole hundredths and tenths, so float noise
/// cannot change the table of `SPEC.md` section 8.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Difficulty {
    /// Enemies per spawn: 1.0 at the start, +0.02 per level, rounded.
    pub group: u32,
    /// The gap between spawns is drawn from this range, in seconds.
    pub shortest_gap: f32,
    pub longest_gap: f32,
}

impl Difficulty {
    pub fn at(level: u32) -> Self {
        let level = level.min(100_000) as i64;
        let group_hundredths = 100 + 2 * level;
        Self {
            group: ((group_hundredths + 50) / 100) as u32,
            shortest_gap: (20 - 2 * level).max(8) as f32 / 10.0,
            longest_gap: (40 - 4 * level).max(12) as f32 / 10.0,
        }
    }
}

/// The level and the time towards the next level-up.
#[derive(Resource, Default)]
pub struct Director {
    pub level: u32,
    elapsed: f32,
}

/// Marks the spawner of the field edges.
#[derive(Component)]
pub struct FieldSpawner;

pub struct SpawningPlugin;

impl Plugin for SpawningPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Director>()
            .add_systems(Update, ((jump_level, level_up).chain(), spawn_spinner));
    }
}

/// The free cells of each field edge: the top row and the bottom row of the field columns, and the first and last
/// field column. The field is the columns `first` to `last` of `grid`.
fn edge_cells(grid: &CellGrid, first: i32, last: i32) -> [Vec<IVec2>; 4] {
    let top = grid.size().y - 1;
    let free = |cells: Vec<IVec2>| -> Vec<IVec2> {
        cells.into_iter().filter(|cell| !grid.get(*cell)).collect()
    };
    [
        free((first..=last).map(|x| IVec2::new(x, top)).collect()),
        free((first..=last).map(|x| IVec2::new(x, 0)).collect()),
        free((0..=top).map(|y| IVec2::new(first, y)).collect()),
        free((0..=top).map(|y| IVec2::new(last, y)).collect()),
    ]
}

/// Draws a gap from the range of `difficulty`.
fn draw_gap(difficulty: &Difficulty, rng: &mut GameRng) -> f32 {
    rng.range(difficulty.shortest_gap, difficulty.longest_gap)
}

/// Spawns the field spawner, from the map's `load` hook, and starts the difficulty again.
pub fn spawn_spawner(
    mut commands: Commands,
    map: Res<CurrentMap>,
    mut rng: ResMut<GameRng>,
    mut director: ResMut<Director>,
) {
    let grid = match forest_grid(&map.file, &map.map) {
        Ok(grid) => grid,
        Err(error) => {
            error!("spawning: {error}");
            return;
        }
    };
    let last = grid.size().x - 1 - SIDE_FOREST_COLUMNS;
    let groups = edge_cells(&grid, SIDE_FOREST_COLUMNS, last).map(|cells| {
        cells
            .into_iter()
            .map(|cell| cell_corner(cell, CELL_SIZE) + Vec2::new(CELL_SIZE / 2.0, 0.0))
            .collect::<Vec<_>>()
    });
    *director = Director::default();
    let difficulty = Difficulty::at(0);
    let gap = draw_gap(&difficulty, &mut rng);
    info!(
        "spawning: places per edge {:?}, first gap {gap:.2} s",
        groups.each_ref().map(Vec::len)
    );
    commands.spawn((
        FieldSpawner,
        Spawner::new(gap, None, Params::default())
            .first_after_gap()
            .with_group(difficulty.group)
            .with_places(SpawnPlaces {
                groups: groups.into(),
            }),
        Transform::default(),
    ));
}

fn apply_level(level: u32, spawner: &mut Spawner, rng: &mut GameRng) {
    let difficulty = Difficulty::at(level);
    let gap = draw_gap(&difficulty, rng);
    spawner.set_every(gap);
    spawner.group = difficulty.group;
    info!(
        "spawning: level {level}, group {}, gap {gap:.2} s (range {} to {})",
        difficulty.group, difficulty.shortest_gap, difficulty.longest_gap
    );
}

fn level_up(
    time: Res<Time>,
    mut director: ResMut<Director>,
    mut rng: ResMut<GameRng>,
    mut spawners: Query<&mut Spawner, With<FieldSpawner>>,
) {
    let Ok(mut spawner) = spawners.single_mut() else {
        return;
    };
    director.elapsed += time.delta_secs();
    while director.elapsed >= LEVEL_SECONDS {
        director.elapsed -= LEVEL_SECONDS;
        director.level += 1;
        apply_level(director.level, &mut spawner, &mut rng);
    }
}

/// The script command `spinner:<column>:<row>[:<target>]`: one Spinner on a map cell (rows from the top, as in the
/// map file and the F3 grid), walking to target 0 to 3 of `enemy::target_feet`, or a random one.
fn spawn_spinner(
    mut commands: MessageReader<ScriptCommand>,
    map: Option<Res<CurrentMap>>,
    spawners: Query<Entity, With<FieldSpawner>>,
    mut requests: MessageWriter<SpawnRequest>,
) {
    for command in commands.read().filter(|c| c.name == SPINNER_COMMAND) {
        let numbers: Vec<Option<i32>> = command.args.iter().map(|a| a.parse().ok()).collect();
        let (cell, target) = match numbers.as_slice() {
            [Some(x), Some(row)] => (IVec2::new(*x, *row), None),
            [Some(x), Some(row), Some(target)] => (IVec2::new(*x, *row), Some(*target)),
            _ => (IVec2::NEG_ONE, None),
        };
        let (Some(map), Ok(spawner), true) = (map.as_ref(), spawners.single(), cell.x >= 0) else {
            warn!(
                "spawning: `spinner:<column>:<row>[:<target>]` got {:?}",
                command.args
            );
            continue;
        };
        let cell = IVec2::new(cell.x, map.map.size.y - 1 - cell.y);
        let position = cell_corner(cell, CELL_SIZE) + Vec2::new(CELL_SIZE / 2.0, 0.0);
        let target = target.map(|t| t.to_string());
        let params = match &target {
            Some(t) => Params::from_pairs(&[("target", t)]),
            None => Params::default(),
        };
        info!(
            "spawning: script Spinner at cell {:?} (feet {position}), target {target:?}",
            command.args
        );
        requests.write(SpawnRequest {
            spawner,
            position,
            params,
        });
    }
}

/// The script command `level:<n>`: jumps to level n at once.
fn jump_level(
    mut commands: MessageReader<ScriptCommand>,
    mut director: ResMut<Director>,
    mut rng: ResMut<GameRng>,
    mut spawners: Query<&mut Spawner, With<FieldSpawner>>,
) {
    for command in commands.read().filter(|c| c.name == LEVEL_COMMAND) {
        let level = match command.args.as_slice() {
            [n] => n.parse::<u32>().ok(),
            _ => None,
        };
        let (Some(level), Ok(mut spawner)) = (level, spawners.single_mut()) else {
            warn!(
                "spawning: `level:<n>` needs a whole number and a spawner, got {:?}",
                command.args
            );
            continue;
        };
        director.level = level;
        director.elapsed = 0.0;
        apply_level(level, &mut spawner, &mut rng);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_levels_follow_the_table_of_the_spec() {
        // Level n is n x 10 s of play.
        for (level, group, shortest, longest) in [
            (0, 1, 2.0, 4.0),
            (3, 1, 1.4, 2.8),
            (6, 1, 0.8, 1.6),
            (7, 1, 0.8, 1.2),
            (24, 1, 0.8, 1.2),
            (25, 2, 0.8, 1.2),
            (74, 2, 0.8, 1.2),
            (75, 3, 0.8, 1.2),
        ] {
            assert_eq!(
                Difficulty::at(level),
                Difficulty {
                    group,
                    shortest_gap: shortest,
                    longest_gap: longest
                },
                "level {level}"
            );
        }
    }

    #[test]
    fn the_gaps_stop_at_their_floors_and_a_huge_level_does_not_overflow() {
        assert_eq!(Difficulty::at(5).shortest_gap, 1.0);
        assert_eq!(Difficulty::at(5).longest_gap, 2.0);
        let late = Difficulty::at(u32::MAX);
        assert_eq!((late.shortest_gap, late.longest_gap), (0.8, 1.2));
        assert_eq!(late.group, 2001);
    }

    #[test]
    fn edge_cells_are_the_free_cells_of_each_field_edge() {
        // 8 x 5 cells, field columns 2 to 5. Forest at (3, 4) on the top row, (2, 1) in the left column and (5, 0)
        // on the bottom row, which is also the right column.
        let mut grid = CellGrid::new(IVec2::new(8, 5));
        for cell in [IVec2::new(3, 4), IVec2::new(2, 1), IVec2::new(5, 0)] {
            grid.set(cell, true);
        }
        let [top, bottom, left, right] = edge_cells(&grid, 2, 5);
        let cells = |list: &[(i32, i32)]| {
            list.iter()
                .map(|&(x, y)| IVec2::new(x, y))
                .collect::<Vec<_>>()
        };
        assert_eq!(top, cells(&[(2, 4), (4, 4), (5, 4)]));
        assert_eq!(bottom, cells(&[(2, 0), (3, 0), (4, 0)]));
        assert_eq!(left, cells(&[(2, 0), (2, 2), (2, 3), (2, 4)]));
        assert_eq!(right, cells(&[(5, 1), (5, 2), (5, 3), (5, 4)]));
    }
}
