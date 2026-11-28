//! The forest: a kit cell area (`adr-jt14`, `dev-docs/extensions/cell-areas.md`). The map draws it as outlines, and
//! this file places the tree sprites over its cells: a tree every 2 columns and every row, so the rows overlap and
//! the clusters look dense, as in the Pyxel game. Each tree has a random variant and a small random offset, from the
//! dice of its cell, so F1 gives the same forest.

use bevy::prelude::*;
use game_kit::animation::{StaticFrame, all_frames_sprite};
use game_kit::cell_area::{CellArea, CellAreaChanged, CellAreaSet, CellGrid};
use game_kit::depth::SortAtArea;
use game_kit::map::{Map, MapError};
use game_kit::map_background::cell_dice;

use crate::camera::{CELL_SIZE, MAP_ORIGIN};
use crate::map::{CurrentMap, FOREST};

const ART: &str = "art/trees.aseprite";
/// Number of tree variants: the frames of `trees.aseprite`, each picked as often.
const VARIANTS: u64 = 4;
/// Largest random shift of a tree sideways, in art pixels. Twice the Pyxel range at 4x, so the trees of one column
/// do not form neat vertical lines.
const JITTER_X: f32 = 16.0;
/// Largest random shift of a tree upwards, in art pixels: the size of the Pyxel range (0 to 2) at 4x. Upwards only,
/// so a trunk never stands below its cell, where it does not block.
const JITTER_UP: f32 = 8.0;
/// Height of the canvas of `trees.aseprite`. The trunk ends at its bottom row.
const TREE_HEIGHT: f32 = 64.0;
/// Share of the tree places left empty at random, to break up the rows and columns.
const THINNING: f32 = 0.15;
/// Dice salts, one per use (`cell_dice`).
const SALT_VARIANT: u64 = 1;
const SALT_JITTER: u64 = 2;
const SALT_THINNING: u64 = 3;

/// Marks the forest's cell area entity.
#[derive(Component)]
pub struct Forest;

/// Marks a tree sprite.
#[derive(Component)]
struct Tree;

pub struct ForestPlugin;

impl Plugin for ForestPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_trees.after(CellAreaSet));
    }
}

/// The forest cells of `map`: the outlines of `F` and their insides (`fill=enclosed`).
pub fn forest_grid(file: &str, map: &Map) -> Result<CellGrid, MapError> {
    CellGrid::from_map(file, map, FOREST)
}

pub fn spawn_forest(mut commands: Commands, map: Res<CurrentMap>) {
    let grid = match forest_grid(&map.file, &map.map) {
        Ok(grid) => grid,
        Err(error) => {
            error!("forest: {error}");
            return;
        }
    };
    // The trees are children of the area, so they go with the map.
    commands.spawn((
        Forest,
        CellArea::new(grid, CELL_SIZE, MAP_ORIGIN, Vec2::ZERO),
        Transform::default(),
        Visibility::default(),
    ));
}

/// The cells that get a tree: every filled cell in an even column, and a filled cell in an odd column whose left
/// neighbour is empty, so a forest that starts in an odd column still has trees there.
fn tree_cells(grid: &CellGrid) -> Vec<IVec2> {
    grid.filled()
        .filter(|cell| cell.x % 2 == 0 || !grid.get(*cell - IVec2::X))
        .collect()
}

/// The centre of the tree of `cell`, before the random shift, relative to the area origin. A tree is 2 cells wide:
/// it covers its cell and the one to the right when that is filled, else it is centred on its cell. It stands on the
/// bottom of its cell, so the trunk ends on the blocked area's line and its sort line, and the canopy reaches up
/// over the rows above, so the rows overlap.
fn tree_centre(grid: &CellGrid, cell: IVec2, cell_size: f32) -> Vec2 {
    let corner = cell.as_vec2() * cell_size;
    let x = if grid.get(cell + IVec2::X) {
        cell_size
    } else {
        cell_size / 2.0
    };
    corner + Vec2::new(x, TREE_HEIGHT / 2.0)
}

/// The variant (frame index) for a dice roll.
fn variant(roll: u64) -> u16 {
    (roll % VARIANTS) as u16
}

/// The right-hand cell that the tree of `cell` covers: the cell to its right when that is forest, else `cell`
/// (`tree_centre`).
fn last_covered(grid: &CellGrid, cell: IVec2) -> IVec2 {
    if grid.get(cell + IVec2::X) {
        cell + IVec2::X
    } else {
        cell
    }
}

/// The range of the random sideways shift of the tree of `cell`. A tree at the end of a row only shifts outwards,
/// so the thin art does not leave the end cell of the row bare: at the left end only left, at the right end only
/// right, and a tree that is both does not shift. Trees inside a row shift up to `JITTER_X` either way.
fn sideways_range(grid: &CellGrid, cell: IVec2) -> (f32, f32) {
    let left_end = !grid.get(cell - IVec2::X);
    let right_end = !grid.get(last_covered(grid, cell) + IVec2::X);
    let min = if right_end { 0.0 } else { -JITTER_X };
    let max = if left_end { 0.0 } else { JITTER_X };
    (min, max)
}

/// True when the tree of `cell` gets drawn, for the dice roll `roll` (0 to 1). `THINNING` of the places stay empty,
/// but only inside the forest: where the cells beside the tree and below it are forest, so the canopies of those
/// trees hide the gap. A tree at the edge is always drawn, so no blocked cell is left bare.
fn keeps_tree(grid: &CellGrid, cell: IVec2, roll: f32) -> bool {
    let last = last_covered(grid, cell);
    let hidden = grid.get(cell - IVec2::X)
        && grid.get(last + IVec2::X)
        && (cell.x..=last.x).all(|x| grid.get(IVec2::new(x, cell.y - 1)));
    !hidden || roll >= THINNING
}

/// Draws all trees again after the forest changes. The forest is a few hundred trees, so a full redraw is cheap.
fn draw_trees(
    mut commands: Commands,
    mut changed: MessageReader<CellAreaChanged>,
    server: Res<AssetServer>,
    map: Option<Res<CurrentMap>>,
    forests: Query<(Entity, &CellArea), With<Forest>>,
    trees: Query<Entity, With<Tree>>,
) {
    let changed_forests: Vec<Entity> = changed
        .read()
        .map(|message| message.area)
        .filter(|area| forests.contains(*area))
        .collect();
    let (Some(map), false) = (map, changed_forests.is_empty()) else {
        return;
    };
    for tree in &trees {
        commands.entity(tree).despawn();
    }
    for (entity, area) in &forests {
        let grid = area.grid();
        for cell in tree_cells(grid) {
            if !keeps_tree(
                grid,
                cell,
                cell_dice(&map.map.name, SALT_THINNING, cell).f32(),
            ) {
                continue;
            }
            let frame = variant(cell_dice(&map.map.name, SALT_VARIANT, cell).next_u64());
            let mut dice = cell_dice(&map.map.name, SALT_JITTER, cell);
            let (min_x, max_x) = sideways_range(grid, cell);
            let shift = Vec2::new(
                dice.range(min_x, max_x).round(),
                dice.range(0.0, JITTER_UP).round(),
            );
            let centre = area.origin + tree_centre(grid, cell, area.cell_size) + shift;
            commands.spawn((
                Tree,
                all_frames_sprite(&server, ART),
                StaticFrame { relative: frame },
                SortAtArea(area.cell_rect(cell)),
                Transform::from_translation(centre.extend(0.0)),
                ChildOf(entity),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(rows: &[&str]) -> CellGrid {
        let size = IVec2::new(rows[0].len() as i32, rows.len() as i32);
        let mut grid = CellGrid::new(size);
        for (i, row) in rows.iter().enumerate() {
            for (x, c) in row.chars().enumerate() {
                if c == '#' {
                    grid.set(IVec2::new(x as i32, size.y - 1 - i as i32), true);
                }
            }
        }
        grid
    }

    #[test]
    fn trees_stand_every_second_column_and_at_the_left_edge_of_each_run() {
        // Row 1: a run from column 1 (odd, so it gets a tree) to 4. Row 0: a run from column 2 to 5.
        let forest = grid(&[".####.", "..####"]);
        assert_eq!(
            tree_cells(&forest),
            vec![
                IVec2::new(2, 0),
                IVec2::new(4, 0),
                IVec2::new(1, 1),
                IVec2::new(2, 1),
                IVec2::new(4, 1)
            ]
        );
    }

    #[test]
    fn a_tree_stands_on_its_cell_bottom_and_covers_the_cell_to_its_right_only_when_that_is_forest()
    {
        let forest = grid(&["##.#", "...."]);
        // Cells of 10: the tree's bottom (centre y minus half its height) is the bottom of row 1, at y 10.
        assert_eq!(
            tree_centre(&forest, IVec2::new(0, 1), 10.0),
            Vec2::new(10.0, 10.0 + TREE_HEIGHT / 2.0)
        );
        assert_eq!(
            tree_centre(&forest, IVec2::new(3, 1), 10.0),
            Vec2::new(35.0, 10.0 + TREE_HEIGHT / 2.0)
        );
    }

    #[test]
    fn only_a_tree_inside_the_forest_is_thinned_and_only_below_the_thinning_share() {
        // The tree of (2, 1) covers columns 2 and 3, with forest beside it and below it.
        let forest = grid(&["######", "######", "######"]);
        let inside = IVec2::new(2, 1);
        assert!(!keeps_tree(&forest, inside, 0.0));
        assert!(!keeps_tree(&forest, inside, 0.149));
        assert!(keeps_tree(&forest, inside, 0.15));
        // At an edge the tree stays at any roll: the bottom row, the left end of a row, the right end of a row
        // (the tree of (3, 1) covers columns 3 and 4, and column 5 is empty), and a tree whose second column has no
        // forest below it.
        let right_end = grid(&["#####.", "#####.", "#####."]);
        let half_below = grid(&["######", "######", "###..."]);
        for (forest, cell) in [
            (&forest, IVec2::new(2, 0)),
            (&forest, IVec2::new(0, 1)),
            (&right_end, IVec2::new(3, 1)),
            (&half_below, IVec2::new(2, 1)),
        ] {
            assert!(keeps_tree(forest, cell, 0.0), "{cell}");
        }
    }

    #[test]
    fn a_tree_at_the_end_of_a_row_shifts_only_outwards() {
        // Trees at 0 (left end, covers 0 and 1), 2 (inside), 4 (right end, covers 4 and 5), and 7 alone.
        let forest = grid(&["######.#"]);
        assert_eq!(sideways_range(&forest, IVec2::new(0, 0)), (-JITTER_X, 0.0));
        assert_eq!(
            sideways_range(&forest, IVec2::new(2, 0)),
            (-JITTER_X, JITTER_X)
        );
        assert_eq!(sideways_range(&forest, IVec2::new(4, 0)), (0.0, JITTER_X));
        assert_eq!(sideways_range(&forest, IVec2::new(7, 0)), (0.0, 0.0));
    }

    #[test]
    fn the_variant_cycles_through_every_frame() {
        let picks: Vec<u16> = (0..=VARIANTS).map(variant).collect();
        assert_eq!(picks, vec![0, 1, 2, 3, 0]);
    }
}
