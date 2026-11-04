//! Path finding over the collision outlines. A grid of walk points covers the map. Two neighbouring points are linked
//! when a straight move between them crosses no outline edge from its free side, which is the same rule that
//! `collision::resolve` uses for feet. A* (`pathfinding` crate) searches the links, and the path is then smoothed
//! to the fewest straight legs that cross no edge. See `dev-docs/PLAN-enemies.md`.
//!
//! The grid is built from the edges alone, so it fits any actor whose blocked areas are in the `CollisionWorld`.
//! A point inside a blocked area has no way in: a goal there has no path. An actor that starts inside a blocked
//! area can walk out, because an edge only stops a move that comes from the free side.

use bevy::prelude::*;
use pathfinding::prelude::astar;

use crate::camera::CameraBounds;
use crate::collision::CollisionWorld;
use crate::outline::{Edge, Orientation, TOLERANCE};

/// Whether the segment from `a` to `b` crosses an edge from its free side. The same rule as `collision::resolve`
/// (an edge only stops a move that comes from the free side, and its ends at convex corners are not part of it),
/// but for the straight line instead of one axis at a time.
pub fn segment_blocked(a: Vec2, b: Vec2, edges: &[Edge]) -> bool {
    edges.iter().any(|edge| crosses_edge(a, b, edge))
}

fn crosses_edge(a: Vec2, b: Vec2, edge: &Edge) -> bool {
    let (from, to, along_from, along_to) = match edge.orientation {
        Orientation::Vertical => (a.x, b.x, a.y, b.y),
        Orientation::Horizontal => (a.y, b.y, a.x, b.x),
    };
    let reaches = if edge.blocked_positive {
        from <= edge.at + TOLERANCE && to > edge.at
    } else {
        from >= edge.at - TOLERANCE && to < edge.at
    };
    if !reaches {
        return false;
    }
    let t = ((edge.at - from) / (to - from)).clamp(0.0, 1.0);
    edge.covers(along_from + t * (along_to - along_from))
}

/// The 8 neighbours: east, north, west, south, then the four diagonals.
const DIRECTIONS: [(i32, i32); 8] = [
    (1, 0),
    (0, 1),
    (-1, 0),
    (0, -1),
    (1, 1),
    (-1, 1),
    (-1, -1),
    (1, -1),
];
/// Costs in tenths of a step: 10 for a straight link, 14 for a diagonal one.
const STRAIGHT: u32 = 10;
const DIAGONAL: u32 = 14;
/// Lowest cost per unit of distance (a diagonal link is 14 for 1.414 steps), for a heuristic that never
/// overestimates.
const MIN_COST_PER_STEP: f32 = 9.8;
/// A bucket of the edge lookup is this many steps wide.
const BUCKET_STEPS: f32 = 4.0;

#[derive(Resource, Default)]
pub struct NavGrid {
    min: Vec2,
    step: f32,
    cols: usize,
    rows: usize,
    /// One byte per point: bit `d` is set when the move in `DIRECTIONS[d]` is allowed.
    links: Vec<u8>,
    edges: Vec<Edge>,
    bucket_size: f32,
    bucket_cols: usize,
    bucket_rows: usize,
    buckets: Vec<Vec<u32>>,
}

impl NavGrid {
    /// A grid with a point every `step` units over `bounds`, linked by the rule above.
    pub fn build(edges: &[Edge], bounds: Rect, step: f32) -> Self {
        let size = bounds.size();
        let cols = (size.x / step).floor() as usize + 1;
        let rows = (size.y / step).floor() as usize + 1;
        let bucket_size = step * BUCKET_STEPS;
        let bucket_cols = (size.x / bucket_size).floor() as usize + 1;
        let bucket_rows = (size.y / bucket_size).floor() as usize + 1;
        let mut grid = NavGrid {
            min: bounds.min,
            step,
            cols,
            rows,
            links: vec![0; cols * rows],
            edges: edges.to_vec(),
            bucket_size,
            bucket_cols,
            bucket_rows,
            buckets: vec![Vec::new(); bucket_cols * bucket_rows],
        };
        grid.fill_buckets();
        grid.fill_links();
        grid
    }

    pub fn point_count(&self) -> usize {
        self.cols * self.rows
    }

    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    fn bucket_range(&self, low: f32, high: f32, min: f32, count: usize) -> (usize, usize) {
        let last = count as i32 - 1;
        let first = (((low - min) / self.bucket_size).floor() as i32).clamp(0, last);
        let end = (((high - min) / self.bucket_size).floor() as i32).clamp(0, last);
        (first as usize, end as usize)
    }

    fn fill_buckets(&mut self) {
        const PAD: f32 = 0.05;
        for (index, edge) in self.edges.iter().enumerate() {
            let ((x0, x1), (y0, y1)) = match edge.orientation {
                Orientation::Vertical => (
                    (edge.at - PAD, edge.at + PAD),
                    (edge.from - PAD, edge.to + PAD),
                ),
                Orientation::Horizontal => (
                    (edge.from - PAD, edge.to + PAD),
                    (edge.at - PAD, edge.at + PAD),
                ),
            };
            let (bx0, bx1) = self.bucket_range(x0, x1, self.min.x, self.bucket_cols);
            let (by0, by1) = self.bucket_range(y0, y1, self.min.y, self.bucket_rows);
            for by in by0..=by1 {
                for bx in bx0..=bx1 {
                    self.buckets[by * self.bucket_cols + bx].push(index as u32);
                }
            }
        }
    }

    /// The edges that may cross the segment from `a` to `b`.
    fn edges_near(&self, a: Vec2, b: Vec2) -> Vec<Edge> {
        let low = a.min(b);
        let high = a.max(b);
        let (bx0, bx1) = self.bucket_range(low.x, high.x, self.min.x, self.bucket_cols);
        let (by0, by1) = self.bucket_range(low.y, high.y, self.min.y, self.bucket_rows);
        let mut indices: Vec<u32> = Vec::new();
        for by in by0..=by1 {
            for bx in bx0..=bx1 {
                indices.extend_from_slice(&self.buckets[by * self.bucket_cols + bx]);
            }
        }
        indices.sort_unstable();
        indices.dedup();
        indices
            .into_iter()
            .map(|i| self.edges[i as usize])
            .collect()
    }

    /// True when a straight move from `a` to `b` crosses no edge from its free side.
    pub fn line_free(&self, a: Vec2, b: Vec2) -> bool {
        !segment_blocked(a, b, &self.edges_near(a, b))
    }

    fn position(&self, col: usize, row: usize) -> Vec2 {
        self.min + Vec2::new(col as f32, row as f32) * self.step
    }

    fn fill_links(&mut self) {
        // Straight links first, then diagonal ones that need both straight links around the corner.
        for row in 0..self.rows {
            for col in 0..self.cols {
                let here = self.position(col, row);
                let mut bits = 0u8;
                for (d, (dx, dy)) in DIRECTIONS.iter().enumerate().take(4) {
                    let (nc, nr) = (col as i32 + dx, row as i32 + dy);
                    if nc < 0 || nr < 0 || nc >= self.cols as i32 || nr >= self.rows as i32 {
                        continue;
                    }
                    let there = self.position(nc as usize, nr as usize);
                    if self.line_free(here, there) {
                        bits |= 1 << d;
                    }
                }
                self.links[row * self.cols + col] = bits;
            }
        }
        for row in 0..self.rows {
            for col in 0..self.cols {
                let mut bits = self.links[row * self.cols + col];
                for (d, (dx, dy)) in DIRECTIONS.iter().enumerate().skip(4) {
                    if self.diagonal_open(col, row, *dx, *dy) {
                        bits |= 1 << d;
                    }
                }
                self.links[row * self.cols + col] = bits;
            }
        }
    }

    /// The straight link of a point in a direction, false outside the grid.
    fn straight_open(&self, col: i32, row: i32, dx: i32, dy: i32) -> bool {
        if col < 0 || row < 0 || col >= self.cols as i32 || row >= self.rows as i32 {
            return false;
        }
        let d = DIRECTIONS
            .iter()
            .position(|&dir| dir == (dx, dy))
            .expect("a straight direction");
        self.links[row as usize * self.cols + col as usize] & (1 << d) != 0
    }

    /// A diagonal move is allowed when both ways around the corner are open.
    fn diagonal_open(&self, col: usize, row: usize, dx: i32, dy: i32) -> bool {
        let (col, row) = (col as i32, row as i32);
        let x_then_y =
            self.straight_open(col, row, dx, 0) && self.straight_open(col + dx, row, 0, dy);
        let y_then_x =
            self.straight_open(col, row, 0, dy) && self.straight_open(col, row + dy, dx, 0);
        x_then_y && y_then_x
    }

    fn index(&self, col: usize, row: usize) -> usize {
        row * self.cols + col
    }

    fn col_row(&self, index: usize) -> (usize, usize) {
        (index % self.cols, index / self.cols)
    }

    /// The up to four grid points around `p`, nearest first, that a straight move from `p` reaches.
    fn reachable_points(&self, p: Vec2, towards_p: bool) -> Vec<usize> {
        let local = (p - self.min) / self.step;
        let (c0, r0) = (local.x.floor() as i32, local.y.floor() as i32);
        let mut found: Vec<(f32, usize)> = Vec::new();
        for (c, r) in [(c0, r0), (c0 + 1, r0), (c0, r0 + 1), (c0 + 1, r0 + 1)] {
            if c < 0 || r < 0 || c >= self.cols as i32 || r >= self.rows as i32 {
                continue;
            }
            let point = self.position(c as usize, r as usize);
            let free = if towards_p {
                self.line_free(point, p)
            } else {
                self.line_free(p, point)
            };
            if free {
                found.push((point.distance(p), self.index(c as usize, r as usize)));
            }
        }
        found.sort_by(|a, b| a.0.total_cmp(&b.0));
        found.into_iter().map(|(_, index)| index).collect()
    }

    /// A path from `from` to `to`: the points to walk to, in order, without `from`. The last point is `to` when it
    /// can be reached, so a goal inside a blocked area gives `None`. A straight line gives just `[to]`.
    pub fn find_path(&self, from: Vec2, to: Vec2) -> Option<Vec<Vec2>> {
        if self.is_empty() {
            return None;
        }
        if self.line_free(from, to) {
            return Some(vec![to]);
        }
        let starts = self.reachable_points(from, false);
        let goals = self.reachable_points(to, true);
        if starts.is_empty() || goals.is_empty() {
            return None;
        }
        let virtual_start = self.point_count();
        let cost_of = |a: Vec2, b: Vec2| (a.distance(b) / self.step * STRAIGHT as f32) as u32;
        let (nodes, _) = astar(
            &virtual_start,
            |&node| {
                if node == virtual_start {
                    return starts
                        .iter()
                        .map(|&s| {
                            let (c, r) = self.col_row(s);
                            (s, cost_of(from, self.position(c, r)))
                        })
                        .collect::<Vec<_>>();
                }
                let (col, row) = self.col_row(node);
                let bits = self.links[node];
                let mut next = Vec::with_capacity(8);
                for (d, (dx, dy)) in DIRECTIONS.iter().enumerate() {
                    if bits & (1 << d) != 0 {
                        let target =
                            self.index((col as i32 + dx) as usize, (row as i32 + dy) as usize);
                        next.push((target, if d < 4 { STRAIGHT } else { DIAGONAL }));
                    }
                }
                next
            },
            |&node| {
                if node == virtual_start {
                    return 0;
                }
                let (c, r) = self.col_row(node);
                (self.position(c, r).distance(to) / self.step * MIN_COST_PER_STEP) as u32
            },
            |&node| goals.contains(&node),
        )?;
        let mut points = vec![from];
        for &node in nodes.iter().skip(1) {
            let (c, r) = self.col_row(node);
            points.push(self.position(c, r));
        }
        points.push(to);
        Some(self.smooth(points))
    }

    /// Keeps the fewest points of a path such that each straight leg between them crosses no edge. The first point
    /// is dropped from the result.
    fn smooth(&self, points: Vec<Vec2>) -> Vec<Vec2> {
        let mut result = Vec::new();
        let mut at = 0;
        let last = points.len() - 1;
        while at < last {
            let mut next = last;
            while next > at + 1 && !self.line_free(points[at], points[next]) {
                next -= 1;
            }
            result.push(points[next]);
            at = next;
        }
        result
    }
}

/// Builds the grid from the `CollisionWorld` and the `CameraBounds` (the map rectangle) and builds it again when
/// either changes, for example when a map is reloaded. Systems that use the grid run `.after(NavSet)`.
pub struct NavPlugin {
    /// Distance between grid points in world units. A smaller step finds narrower gaps and costs more memory.
    pub step: f32,
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct NavSet;

#[derive(Resource)]
struct NavStep(f32);

impl Plugin for NavPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavGrid>()
            .insert_resource(NavStep(self.step))
            .add_systems(Update, rebuild_grid.run_if(needs_rebuild).in_set(NavSet));
    }
}

/// True when the map rectangle exists and it or the blocked areas changed (also on the first frame).
fn needs_rebuild(world: Option<Res<CollisionWorld>>, bounds: Option<Res<CameraBounds>>) -> bool {
    match (world, bounds) {
        (Some(world), Some(bounds)) => world.is_changed() || bounds.is_changed(),
        _ => false,
    }
}

fn rebuild_grid(
    world: Res<CollisionWorld>,
    bounds: Res<CameraBounds>,
    step: Res<NavStep>,
    mut grid: ResMut<NavGrid>,
) {
    let started = bevy::platform::time::Instant::now();
    *grid = NavGrid::build(world.edges(), bounds.0, step.0);
    debug!(
        "nav: grid of {} points (step {}) built in {:?}",
        grid.point_count(),
        step.0,
        started.elapsed()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::outline;
    use crate::rng::Rng;

    fn edges_of(rects: &[Rect]) -> Vec<Edge> {
        outline::build(rects)
            .iter()
            .flat_map(|c| c.edges.iter().copied())
            .collect()
    }

    fn map() -> Rect {
        Rect::new(0.0, 0.0, 200.0, 200.0)
    }

    fn grid(rects: &[Rect]) -> NavGrid {
        NavGrid::build(&edges_of(rects), map(), 4.0)
    }

    /// Every leg of the path, from `from`, crosses no edge.
    fn legal(path: &[Vec2], from: Vec2, edges: &[Edge]) -> bool {
        let mut at = from;
        path.iter().all(|&p| {
            let ok = !segment_blocked(at, p, edges);
            at = p;
            ok
        })
    }

    #[test]
    fn open_ground_is_one_straight_leg() {
        let nav = grid(&[]);
        let path = nav.find_path(Vec2::new(10.0, 10.0), Vec2::new(150.0, 90.0));
        assert_eq!(path, Some(vec![Vec2::new(150.0, 90.0)]));
    }

    #[test]
    fn a_thin_band_is_walked_around() {
        let band = Rect::new(100.0, 90.0, 140.0, 94.0);
        let nav = grid(&[band]);
        let (from, to) = (Vec2::new(120.0, 80.0), Vec2::new(120.0, 110.0));
        let path = nav.find_path(from, to).expect("a path");
        assert!(path.len() >= 2, "{path:?}");
        assert_eq!(*path.last().unwrap(), to);
        assert!(legal(&path, from, &edges_of(&[band])));
    }

    #[test]
    fn a_grove_is_walked_around() {
        let grove = [
            Rect::new(60.0, 60.0, 100.0, 100.0),
            Rect::new(100.0, 60.0, 140.0, 100.0),
            Rect::new(80.0, 100.0, 120.0, 140.0),
        ];
        let nav = grid(&grove);
        let (from, to) = (Vec2::new(70.0, 50.0), Vec2::new(110.0, 150.0));
        let path = nav.find_path(from, to).expect("a path");
        assert!(legal(&path, from, &edges_of(&grove)));
        assert_eq!(*path.last().unwrap(), to);
    }

    #[test]
    fn a_six_unit_gap_can_be_used() {
        // A wall at x 90 to 100 with a gap between y 97 and 103.
        let wall = [
            Rect::new(90.0, 0.0, 100.0, 97.0),
            Rect::new(90.0, 103.0, 100.0, 200.0),
        ];
        let nav = grid(&wall);
        let (from, to) = (Vec2::new(80.0, 100.0), Vec2::new(110.0, 100.0));
        let path = nav.find_path(from, to).expect("a path through the gap");
        assert!(legal(&path, from, &edges_of(&wall)));
    }

    #[test]
    fn a_goal_inside_a_blocked_area_has_no_path() {
        let block = Rect::new(80.0, 80.0, 120.0, 120.0);
        let nav = grid(&[block]);
        assert_eq!(
            nav.find_path(Vec2::new(10.0, 20.0), Vec2::new(100.0, 100.0)),
            None
        );
    }

    #[test]
    fn a_start_inside_a_blocked_area_can_walk_out() {
        let block = Rect::new(80.0, 80.0, 120.0, 120.0);
        let nav = grid(&[block]);
        let to = Vec2::new(150.0, 100.0);
        let path = nav
            .find_path(Vec2::new(100.0, 100.0), to)
            .expect("a way out");
        assert_eq!(*path.last().unwrap(), to);
    }

    #[test]
    fn an_enclosed_goal_has_no_path() {
        let ring = [
            Rect::new(90.0, 90.0, 130.0, 100.0),
            Rect::new(90.0, 120.0, 130.0, 130.0),
            Rect::new(90.0, 100.0, 100.0, 120.0),
            Rect::new(120.0, 100.0, 130.0, 120.0),
        ];
        let nav = grid(&ring);
        assert_eq!(
            nav.find_path(Vec2::new(10.0, 20.0), Vec2::new(110.0, 110.0)),
            None
        );
    }

    #[test]
    fn paths_are_legal_for_many_random_pairs() {
        let rects = [
            Rect::new(40.0, 40.0, 70.0, 44.0),
            Rect::new(100.0, 20.0, 130.0, 80.0),
            Rect::new(60.0, 100.0, 90.0, 104.0),
            Rect::new(140.0, 120.0, 190.0, 160.0),
            Rect::new(20.0, 150.0, 60.0, 154.0),
        ];
        let edges = edges_of(&rects);
        let nav = NavGrid::build(&edges, map(), 4.0);
        let mut rng = Rng::new(5);
        let mut found = 0;
        for _ in 0..300 {
            let from = Vec2::new(rng.range(1.0, 199.0), rng.range(1.0, 199.0));
            let to = Vec2::new(rng.range(1.0, 199.0), rng.range(1.0, 199.0));
            if let Some(path) = nav.find_path(from, to) {
                found += 1;
                assert!(legal(&path, from, &edges), "{from:?} -> {to:?}: {path:?}");
            }
        }
        assert!(found > 100, "{found}");
    }

    #[test]
    fn an_empty_grid_finds_nothing() {
        assert_eq!(NavGrid::default().find_path(Vec2::ZERO, Vec2::ONE), None);
    }

    #[test]
    fn a_segment_crosses_only_from_the_free_side() {
        let edges = edges_of(&[Rect::new(10.0, 10.0, 20.0, 20.0)]);
        assert!(segment_blocked(
            Vec2::new(0.0, 15.0),
            Vec2::new(15.0, 15.0),
            &edges
        ));
        assert!(!segment_blocked(
            Vec2::new(15.0, 15.0),
            Vec2::new(0.0, 15.0),
            &edges
        ));
        assert!(!segment_blocked(
            Vec2::new(0.0, 5.0),
            Vec2::new(30.0, 5.0),
            &edges
        ));
        assert!(segment_blocked(
            Vec2::new(0.0, 5.0),
            Vec2::new(30.0, 35.0),
            &edges
        ));
    }
}
