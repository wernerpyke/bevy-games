//! Cell areas: a grid that says which cells of a map belong to an area (a forest, for example), with the blocked
//! areas made from it (`dev-docs/extensions/cell-areas.md`, adr-x62).
//!
//! - `CellGrid` is the plain grid. `fill_enclosed` fills the inside of outlines, `CellGrid::from_map` reads one
//!   legend kind of a map, with the `fill=enclosed` parameter.
//! - `CellArea` puts a grid in the world. The kit keeps one `Blocker` child per blocked rectangle, rebuilds the
//!   collision world when the grid changes (so the nav grid follows) and sends `CellAreaChanged`, so the game draws
//!   only the changed cells again.

use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;

use crate::collision::{Blocker, rebuild_blockers};
use crate::map::{Map, MapError};

/// Which cells of a `size` grid are filled. Cells are numbered from the bottom left, as in `MapObject::cell`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellGrid {
    size: IVec2,
    cells: Vec<bool>,
}

impl CellGrid {
    /// An empty grid.
    pub fn new(size: IVec2) -> Self {
        let size = size.max(IVec2::ZERO);
        Self {
            size,
            cells: vec![false; (size.x * size.y) as usize],
        }
    }

    pub fn size(&self) -> IVec2 {
        self.size
    }

    fn index(&self, cell: IVec2) -> Option<usize> {
        let inside = cell.cmpge(IVec2::ZERO).all() && cell.cmplt(self.size).all();
        inside.then(|| (cell.y * self.size.x + cell.x) as usize)
    }

    /// True when `cell` is filled. A cell outside the grid is not.
    pub fn get(&self, cell: IVec2) -> bool {
        self.index(cell).is_some_and(|i| self.cells[i])
    }

    /// Fills or clears `cell`. True when the cell changed. A cell outside the grid is ignored.
    pub fn set(&mut self, cell: IVec2, filled: bool) -> bool {
        match self.index(cell) {
            Some(i) if self.cells[i] != filled => {
                self.cells[i] = filled;
                true
            }
            _ => false,
        }
    }

    /// The filled cells, row by row from the bottom.
    pub fn filled(&self) -> impl Iterator<Item = IVec2> + '_ {
        (0..self.size.y)
            .flat_map(move |y| (0..self.size.x).map(move |x| IVec2::new(x, y)))
            .filter(|&cell| self.get(cell))
    }

    /// The filled cells as rectangles, in cells (`min` is the first cell, `max` one past the last). A run of filled
    /// cells in a row is joined with the same run (the same first and last column) in the rows above it.
    pub fn rects(&self) -> Vec<IRect> {
        let mut done = Vec::new();
        // Open rectangles by their columns `(first, end)`.
        let mut open: HashMap<(i32, i32), IRect> = HashMap::new();
        for y in 0..self.size.y {
            let mut row = HashMap::new();
            let mut x = 0;
            while x < self.size.x {
                if !self.get(IVec2::new(x, y)) {
                    x += 1;
                    continue;
                }
                let first = x;
                while x < self.size.x && self.get(IVec2::new(x, y)) {
                    x += 1;
                }
                let rect = match open.remove(&(first, x)) {
                    Some(below) => IRect::new(below.min.x, below.min.y, below.max.x, y + 1),
                    None => IRect::new(first, y, x, y + 1),
                };
                row.insert((first, x), rect);
            }
            done.extend(open.into_values());
            open = row;
        }
        done.extend(open.into_values());
        done.sort_by_key(|r| (r.min.y, r.min.x));
        done
    }

    /// The cells of the legend kind `kind` in `map`. With the legend parameter `fill=enclosed` the inside of their
    /// outlines is filled too (`fill_enclosed`). `file` names the map file in an error.
    pub fn from_map(file: &str, map: &Map, kind: &str) -> Result<Self, MapError> {
        let objects: Vec<_> = map.objects.iter().filter(|o| o.kind == kind).collect();
        let cells = objects.iter().map(|o| o.cell);
        let Some(first) = objects.first() else {
            return Ok(Self::new(map.size));
        };
        match first.params.get("fill") {
            None => {
                let mut grid = Self::new(map.size);
                for cell in cells {
                    grid.set(cell, true);
                }
                Ok(grid)
            }
            Some("enclosed") => Ok(fill_enclosed(map.size, cells)),
            Some(other) => Err(MapError {
                file: file.to_string(),
                line: first.params.line(),
                message: format!("`{kind}`: parameter `fill` must be `enclosed`, found `{other}`"),
            }),
        }
    }
}

/// The `outline` cells, and every other cell that cannot reach an empty cell on the border of the grid through
/// cells that are not in the outline, stepping up, down, left or right. So the inside of a closed outline is
/// filled. An area at the map edge draws its outline along the border cells; an inside that touches the border
/// itself is outside. A gap of one cell in an outline lets the outside in, but a corner contact does not. A clearing
/// closed in on all sides by an outline is filled too.
pub fn fill_enclosed(size: IVec2, outline: impl IntoIterator<Item = IVec2>) -> CellGrid {
    let mut grid = CellGrid::new(size);
    for cell in outline {
        grid.set(cell, true);
    }
    let size = grid.size;
    let mut outside = vec![false; grid.cells.len()];
    let mut queue = VecDeque::new();
    for y in 0..size.y {
        for x in 0..size.x {
            let on_border = x == 0 || y == 0 || x == size.x - 1 || y == size.y - 1;
            let cell = IVec2::new(x, y);
            if on_border && !grid.get(cell) {
                outside[grid.index(cell).unwrap()] = true;
                queue.push_back(cell);
            }
        }
    }
    while let Some(cell) = queue.pop_front() {
        for step in [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y] {
            let next = cell + step;
            if let Some(i) = grid.index(next)
                && !grid.cells[i]
                && !outside[i]
            {
                outside[i] = true;
                queue.push_back(next);
            }
        }
    }
    for (filled, out) in grid.cells.iter_mut().zip(outside) {
        *filled = !out;
    }
    grid
}

/// An area of cells in the world. The kit keeps its blocked areas (one `Blocker` child per rectangle of
/// `CellGrid::rects`) and sends `CellAreaChanged`. Change cells with `set`, so the kit sees the change.
#[derive(Component, Clone, Debug)]
pub struct CellArea {
    grid: CellGrid,
    /// The size of one cell in world units.
    pub cell_size: f32,
    /// The world position of the bottom left corner of cell (0, 0).
    pub origin: Vec2,
    /// How far each blocked rectangle is grown: `x` on the left and the right, `y` at the top and the bottom. The
    /// game's actor size: the area a feet point may not enter (`collision::grow_sideways` is `x` only).
    pub grow: Vec2,
    /// Cells changed since the last `CellAreaChanged`. All filled cells when the area is made.
    changed: Vec<IVec2>,
    /// True when a cell changed since the blockers were made.
    stale: bool,
}

impl CellArea {
    pub fn new(grid: CellGrid, cell_size: f32, origin: Vec2, grow: Vec2) -> Self {
        let changed = grid.filled().collect();
        Self {
            grid,
            cell_size,
            origin,
            grow,
            changed,
            stale: false,
        }
    }

    pub fn grid(&self) -> &CellGrid {
        &self.grid
    }

    /// Fills or clears `cell`. True when the cell changed. The kit then makes the blockers again, rebuilds the
    /// collision world and sends `CellAreaChanged` in its next run.
    pub fn set(&mut self, cell: IVec2, filled: bool) -> bool {
        let changed = self.grid.set(cell, filled);
        if changed {
            self.changed.push(cell);
            self.stale = true;
        }
        changed
    }

    /// The world rectangle of `cell`.
    pub fn cell_rect(&self, cell: IVec2) -> Rect {
        let min = self.origin + cell.as_vec2() * self.cell_size;
        Rect::from_corners(min, min + Vec2::splat(self.cell_size))
    }

    /// The cell that contains the world point `point`.
    pub fn cell_at(&self, point: Vec2) -> IVec2 {
        ((point - self.origin) / self.cell_size).floor().as_ivec2()
    }

    /// The blocked areas in world units: each rectangle of filled cells, grown by `grow`.
    pub fn blocked_rects(&self) -> Vec<Rect> {
        self.grid
            .rects()
            .iter()
            .map(|r| {
                let min = self.origin + r.min.as_vec2() * self.cell_size - self.grow;
                let max = self.origin + r.max.as_vec2() * self.cell_size + self.grow;
                Rect::from_corners(min, max)
            })
            .collect()
    }
}

/// Marks a `Blocker` child that the kit made for a `CellArea`.
#[derive(Component)]
pub struct CellAreaBlocker;

/// The cells of `area` that changed: all filled cells after the area is added, then each cell `CellArea::set`
/// changed. The game draws those cells and their neighbours again.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct CellAreaChanged {
    pub area: Entity,
    pub cells: Vec<IVec2>,
}

/// The kit's cell area systems run here. Order systems that read `CellAreaChanged` after it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CellAreaSet;

/// Set when blockers of an area were made again, so the collision world is rebuilt once.
#[derive(Resource, Default)]
struct CollisionStale(bool);

/// Cell areas: the blockers, the collision rebuild after a change and `CellAreaChanged`. Needs `CollisionPlugin`.
pub struct CellAreaPlugin;

impl Plugin for CellAreaPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CellAreaChanged>()
            .init_resource::<CollisionStale>()
            .add_observer(make_blockers_on_insert)
            .add_systems(
                Update,
                (
                    sync_areas,
                    rebuild_blockers.run_if(|stale: Res<CollisionStale>| stale.0),
                    |mut stale: ResMut<CollisionStale>| stale.0 = false,
                )
                    .chain()
                    .in_set(CellAreaSet),
            );
    }
}

/// Replaces the `CellAreaBlocker` children of `area` with one per blocked rectangle of `cells`.
fn make_blockers(
    commands: &mut Commands,
    area: Entity,
    cells: &CellArea,
    children: Option<&Children>,
    old: &Query<(), With<CellAreaBlocker>>,
) {
    for child in children.into_iter().flatten() {
        if old.contains(*child) {
            commands.entity(*child).despawn();
        }
    }
    for rect in cells.blocked_rects() {
        commands.spawn((CellAreaBlocker, Blocker(rect), ChildOf(area)));
    }
}

/// Makes the blockers as soon as an area is inserted, so a map's `rebuild` hook, which runs right after its `load`
/// hook, already sees them. A later rebuild covers an area inserted during play.
fn make_blockers_on_insert(
    insert: On<Insert<CellArea>>,
    mut commands: Commands,
    areas: Query<(&CellArea, Option<&Children>)>,
    old: Query<(), With<CellAreaBlocker>>,
    mut stale: ResMut<CollisionStale>,
) {
    let Ok((cells, children)) = areas.get(insert.entity) else {
        return;
    };
    make_blockers(&mut commands, insert.entity, cells, children, &old);
    stale.0 = true;
}

/// Makes the blockers of each changed area again and sends `CellAreaChanged`.
fn sync_areas(
    mut commands: Commands,
    mut areas: Query<(Entity, &mut CellArea, Option<&Children>), Changed<CellArea>>,
    old: Query<(), With<CellAreaBlocker>>,
    mut stale: ResMut<CollisionStale>,
    mut changed: MessageWriter<CellAreaChanged>,
) {
    for (entity, mut area, children) in &mut areas {
        let area = area.bypass_change_detection();
        if area.stale {
            make_blockers(&mut commands, entity, area, children, &old);
            area.stale = false;
            stale.0 = true;
        }
        if !area.changed.is_empty() {
            changed.write(CellAreaChanged {
                area: entity,
                cells: std::mem::take(&mut area.changed),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::{CollisionPlugin, CollisionWorld};
    use crate::map::parse;
    use crate::test_util::{Collected, collect};
    use bevy::ecs::system::RunSystemOnce;

    /// A grid of `size` from rows drawn top row first, `#` filled.
    fn grid(rows: &[&str]) -> CellGrid {
        let size = IVec2::new(rows[0].len() as i32, rows.len() as i32);
        let mut grid = CellGrid::new(size);
        for (i, row) in rows.iter().enumerate() {
            let y = size.y - 1 - i as i32;
            for (x, c) in row.chars().enumerate() {
                if c == '#' {
                    grid.set(IVec2::new(x as i32, y), true);
                }
            }
        }
        grid
    }

    #[test]
    fn a_closed_outline_is_filled_inside_and_the_outside_stays_empty() {
        let outline = grid(&[
            "........", ".#####..", ".#...#..", ".#...#..", ".#####..", "........",
        ]);
        let filled = fill_enclosed(outline.size(), outline.filled());
        assert_eq!(
            filled,
            grid(&[
                "........", ".#####..", ".#####..", ".#####..", ".#####..", "........",
            ])
        );
    }

    #[test]
    fn an_outline_along_the_border_closes_and_an_inside_on_the_border_is_outside() {
        // Left: an outline drawn on the border cells of the left, top and bottom, as an area at the map edge.
        // Right: the same shape with its border side left out, so its inside touches the border.
        let outline = grid(&[
            "####....###.",
            "#..#......#.",
            "#..#......#.",
            "####....###.",
        ]);
        assert_eq!(
            fill_enclosed(outline.size(), outline.filled()),
            grid(&[
                "####....###.",
                "####......#.",
                "####......#.",
                "####....###.",
            ])
        );
    }

    #[test]
    fn a_gap_of_one_cell_lets_the_outside_in_and_a_corner_contact_does_not() {
        let gap = grid(&[".......", ".##.##.", ".#...#.", ".#####.", "......."]);
        assert_eq!(fill_enclosed(gap.size(), gap.filled()), gap);
        let corner = grid(&[".......", "..#....", ".#.#...", "..#....", "......."]);
        assert_eq!(
            fill_enclosed(corner.size(), corner.filled()),
            grid(&[".......", "..#....", ".###...", "..#....", ".......",])
        );
    }

    #[test]
    fn a_band_with_no_inside_stays_as_drawn() {
        let band = grid(&["......", "####..", "####..", "......"]);
        assert_eq!(fill_enclosed(band.size(), band.filled()), band);
    }

    #[test]
    fn rects_join_equal_runs_of_neighbouring_rows_only() {
        let cells = grid(&["##...", "###..", "###.#", "....#"]);
        // Bottom up: the single cell column at x 4 (rows 0 and 1), the run x 0..3 of rows 1 and 2, the run x 0..2
        // of row 3, which is narrower and so a new rectangle.
        assert_eq!(
            cells.rects(),
            vec![
                IRect::new(4, 0, 5, 2),
                IRect::new(0, 1, 3, 3),
                IRect::new(0, 3, 2, 4),
            ]
        );
        assert_eq!(CellGrid::new(IVec2::new(3, 2)).rects(), vec![]);
    }

    #[test]
    fn set_reports_a_change_and_ignores_cells_outside() {
        let mut grid = CellGrid::new(IVec2::new(3, 2));
        assert!(grid.set(IVec2::new(2, 1), true));
        assert!(!grid.set(IVec2::new(2, 1), true));
        assert!(grid.get(IVec2::new(2, 1)));
        for outside in [
            IVec2::new(3, 0),
            IVec2::new(0, 2),
            IVec2::new(-1, 0),
            IVec2::new(0, -1),
        ] {
            assert!(!grid.set(outside, true), "{outside}");
            assert!(!grid.get(outside), "{outside}");
        }
        assert_eq!(grid.filled().collect::<Vec<_>>(), vec![IVec2::new(2, 1)]);
    }

    /// A 7 by 5 map (cells of 10, top-left origin) with a box drawn with `F`, and `legend` as the legend line of `F`.
    fn map_with(legend: &str) -> Result<Map, MapError> {
        let text = format!(
            "---\nmap: t\norigin: top-left\nsize: 7x5\nlegend:\n  \".\": empty\n{legend}\n---\n\
             0 |. . . . . . .|\n\
             1 |. F F F F . .|\n\
             2 |. F     F . .|\n\
             3 |. F F F F . .|\n\
             4 |. . . . . . .|\n"
        );
        parse("t.txt", &text, &["forest"], 10.0)
    }

    #[test]
    fn from_map_fills_with_fill_enclosed_and_keeps_the_drawing_without_it() {
        let drawn = grid(&[".......", ".####..", ".#..#..", ".####..", "......."]);
        let map = map_with("  \"F\": forest").unwrap();
        assert_eq!(CellGrid::from_map("t.txt", &map, "forest").unwrap(), drawn);
        let map = map_with("  \"F\": forest fill=enclosed").unwrap();
        assert_eq!(
            CellGrid::from_map("t.txt", &map, "forest").unwrap(),
            fill_enclosed(drawn.size(), drawn.filled())
        );
        assert_eq!(
            CellGrid::from_map("t.txt", &map, "water").unwrap(),
            CellGrid::new(IVec2::new(7, 5))
        );
    }

    #[test]
    fn from_map_reports_a_bad_fill_with_its_legend_line() {
        let map = map_with("  \"F\": forest fill=all").unwrap();
        let error = CellGrid::from_map("t.txt", &map, "forest").unwrap_err();
        assert_eq!(
            error.to_string(),
            "t.txt:7: `forest`: parameter `fill` must be `enclosed`, found `all`"
        );
    }

    #[test]
    fn blocked_rects_are_placed_from_the_origin_and_grown() {
        let area = CellArea::new(
            grid(&["#.", "##"]),
            12.0,
            Vec2::new(-30.0, 40.0),
            Vec2::new(3.0, 1.0),
        );
        assert_eq!(
            area.blocked_rects(),
            vec![
                Rect::new(-33.0, 39.0, -3.0, 53.0),
                Rect::new(-33.0, 51.0, -15.0, 65.0)
            ]
        );
        assert_eq!(
            area.cell_rect(IVec2::new(1, 0)),
            Rect::new(-18.0, 40.0, -6.0, 52.0)
        );
        assert_eq!(area.cell_at(Vec2::new(-18.0, 51.9)), IVec2::new(1, 0));
        assert_eq!(area.cell_at(Vec2::new(-18.1, 52.0)), IVec2::new(0, 1));
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins((
                CollisionPlugin {
                    allow_sliding: false,
                    cell_size: 10.0,
                },
                CellAreaPlugin,
            ))
            .init_resource::<Collected<CellAreaChanged>>()
            .add_systems(Update, collect::<CellAreaChanged>.after(CellAreaSet));
        app
    }

    fn blockers(app: &mut App) -> Vec<Rect> {
        let mut rects: Vec<Rect> = app
            .world_mut()
            .query_filtered::<&Blocker, With<CellAreaBlocker>>()
            .iter(app.world())
            .map(|b| b.0)
            .collect();
        rects.sort_by(|a, b| (a.min.y, a.min.x).partial_cmp(&(b.min.y, b.min.x)).unwrap());
        rects
    }

    fn cluster_bounds(app: &App) -> Vec<Rect> {
        app.world()
            .resource::<CollisionWorld>()
            .clusters()
            .iter()
            .map(|c| c.bounds)
            .collect()
    }

    #[test]
    fn blockers_exist_when_the_spawning_system_ends_so_a_load_rebuild_sees_them() {
        let mut app = app();
        // As a map's `load` hook: a system spawns the area, then the `rebuild` hook runs, before any update.
        app.world_mut()
            .run_system_once(|mut commands: Commands| {
                commands.spawn(CellArea::new(
                    grid(&["##"]),
                    10.0,
                    Vec2::new(5.0, 0.0),
                    Vec2::ZERO,
                ));
            })
            .unwrap();
        app.world_mut().run_system_once(rebuild_blockers).unwrap();
        assert_eq!(blockers(&mut app), vec![Rect::new(5.0, 0.0, 25.0, 10.0)]);
        assert_eq!(cluster_bounds(&app), vec![Rect::new(5.0, 0.0, 25.0, 10.0)]);
    }

    #[test]
    fn a_change_replaces_the_blockers_rebuilds_collision_and_names_the_cells() {
        let mut app = app();
        let area = app
            .world_mut()
            .spawn(CellArea::new(
                grid(&["#.", "##"]),
                10.0,
                Vec2::ZERO,
                Vec2::ZERO,
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().resource::<Collected<CellAreaChanged>>().0,
            vec![CellAreaChanged {
                area,
                cells: vec![IVec2::new(0, 0), IVec2::new(1, 0), IVec2::new(0, 1)]
            }]
        );
        assert_eq!(cluster_bounds(&app), vec![Rect::new(0.0, 0.0, 20.0, 20.0)]);

        // In a later frame, clear the left column and fill the gap: one message with the three cells, in the order
        // they were set. A cell outside the grid is ignored.
        let mut cells = app.world_mut().get_mut::<CellArea>(area).unwrap();
        cells.set(IVec2::new(0, 1), false);
        cells.set(IVec2::new(1, 1), true);
        cells.set(IVec2::new(0, 0), false);
        cells.set(IVec2::new(5, 5), true);
        app.update();
        assert_eq!(
            app.world().resource::<Collected<CellAreaChanged>>().0[1..],
            [CellAreaChanged {
                area,
                cells: vec![IVec2::new(0, 1), IVec2::new(1, 1), IVec2::new(0, 0)]
            }]
        );
        assert_eq!(blockers(&mut app), vec![Rect::new(10.0, 0.0, 20.0, 20.0)]);
        assert_eq!(cluster_bounds(&app), vec![Rect::new(10.0, 0.0, 20.0, 20.0)]);

        // A frame with no change sends nothing.
        app.update();
        assert_eq!(
            app.world().resource::<Collected<CellAreaChanged>>().0.len(),
            2
        );
    }
}
