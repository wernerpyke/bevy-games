//! Merges blocked rectangles into clusters and finds the outline of each cluster. Plain Rust with no Bevy
//! types except `Rect`, so it is unit tested without a running app. See `dev-docs/extensions/collision.md`.
//!
//! All input rectangles are the areas Guy's feet may not enter, already grown by his half width, so the feet
//! are a point. Rectangles that touch or overlap form one cluster. The edges of a cluster are the boundary of
//! the union of its rectangles. Edges between two rectangles of the same cluster do not exist.

use bevy::math::Rect;

/// Coordinates closer than this are treated as equal, so rounding noise cannot open a hairline gap between
/// two rectangles that are meant to touch.
pub const EPSILON: f32 = 0.001;

/// Slack for positions that sit on an edge line. Guy's feet are recomputed from his sprite centre every frame
/// (add 30, subtract 30), which can move a coordinate by a few millionths. Far above that and far below a pixel.
pub const TOLERANCE: f32 = 0.01;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Orientation {
    /// The edge has a fixed x and runs along y.
    Vertical,
    /// The edge has a fixed y and runs along x.
    Horizontal,
}

/// One maximal straight piece of the outline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Edge {
    pub orientation: Orientation,
    /// x of a vertical edge, y of a horizontal edge.
    pub at: f32,
    /// Start and end along the edge (y for vertical, x for horizontal), `from < to`.
    pub from: f32,
    pub to: f32,
    /// True when the blocked side is the side with larger x (vertical) or larger y (horizontal).
    pub blocked_positive: bool,
    /// True when the outline turns inward at this end (the blocked region fills three of the four cells around
    /// the corner, or two cells that touch only at the corner). The end point itself then counts as part of the
    /// edge, see `covers`.
    pub from_concave: bool,
    pub to_concave: bool,
}

impl Edge {
    /// True when `along` (y for a vertical edge, x for a horizontal one) is within the edge. The ends are
    /// excluded, so feet exactly on a corner line can walk along the outside of a convex corner. An end at an
    /// inward (concave) corner is included: past it the feet would be inside the shape. Positions within
    /// `TOLERANCE` of an end count as at the end, so rounding noise cannot change the answer.
    pub fn covers(&self, along: f32) -> bool {
        let above_start = if self.from_concave {
            along >= self.from - TOLERANCE
        } else {
            along > self.from + TOLERANCE
        };
        let below_end = if self.to_concave {
            along <= self.to + TOLERANCE
        } else {
            along < self.to - TOLERANCE
        };
        above_start && below_end
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cluster {
    /// Indices into the input slice of the rectangles in this cluster, ascending.
    pub members: Vec<usize>,
    /// Members that touch no empty space: every side of every grid cell they cover has a blocked cell next to it,
    /// so no outline edge lies on them. Ascending input indices. Collision uses only the edges, so these cost
    /// nothing there. Do not drop them from the input of `build`: an interior rectangle can be what fills a hole.
    pub interior: Vec<usize>,
    /// Outline edges, sorted by orientation, then `at`, then `from`.
    pub edges: Vec<Edge>,
    /// Smallest rectangle around all members.
    pub bounds: Rect,
}

/// Groups `rects` into clusters and finds the outline of each. Clusters are ordered by their lowest member
/// index. Rectangles with no area are ignored.
pub fn build(rects: &[Rect]) -> Vec<Cluster> {
    let valid: Vec<usize> = (0..rects.len())
        .filter(|&i| {
            let r = rects[i];
            r.max.x - r.min.x > EPSILON && r.max.y - r.min.y > EPSILON
        })
        .collect();
    if valid.is_empty() {
        return Vec::new();
    }

    // Coordinate compression: every rectangle edge becomes an index into a sorted list, so the union is
    // computed on a small grid whose cells are either fully inside or fully outside each rectangle.
    let xs = snap_axis(valid.iter().flat_map(|&i| [rects[i].min.x, rects[i].max.x]));
    let ys = snap_axis(valid.iter().flat_map(|&i| [rects[i].min.y, rects[i].max.y]));
    let spans: Vec<Span> = valid
        .iter()
        .map(|&i| Span {
            x0: index_of(&xs, rects[i].min.x),
            x1: index_of(&xs, rects[i].max.x),
            y0: index_of(&ys, rects[i].min.y),
            y1: index_of(&ys, rects[i].max.y),
        })
        .collect();

    // Rectangles that touch or overlap (also at a corner) are one cluster.
    let mut parent: Vec<usize> = (0..valid.len()).collect();
    for a in 0..valid.len() {
        for b in (a + 1)..valid.len() {
            if spans[a].touches(&spans[b]) {
                let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
                parent[rb.max(ra)] = rb.min(ra);
            }
        }
    }
    let mut cluster_of_root: Vec<Option<usize>> = vec![None; valid.len()];
    let mut clusters: Vec<Cluster> = Vec::new();
    let mut cluster_of_rect = vec![0; valid.len()];
    for k in 0..valid.len() {
        let root = find(&mut parent, k);
        let cluster = *cluster_of_root[root].get_or_insert_with(|| {
            clusters.push(Cluster {
                members: Vec::new(),
                interior: Vec::new(),
                edges: Vec::new(),
                bounds: rects[valid[k]],
            });
            clusters.len() - 1
        });
        cluster_of_rect[k] = cluster;
        let rect = rects[valid[k]];
        let c = &mut clusters[cluster];
        c.members.push(valid[k]);
        c.bounds = Rect::new(
            c.bounds.min.x.min(rect.min.x),
            c.bounds.min.y.min(rect.min.y),
            c.bounds.max.x.max(rect.max.x),
            c.bounds.max.y.max(rect.max.y),
        );
    }

    // Which cluster fills each grid cell.
    let (cols, rows) = (xs.len() - 1, ys.len() - 1);
    let mut owner: Vec<Option<usize>> = vec![None; cols * rows];
    for (k, span) in spans.iter().enumerate() {
        for j in span.y0..span.y1 {
            for i in span.x0..span.x1 {
                owner[j * cols + i] = Some(cluster_of_rect[k]);
            }
        }
    }
    // A rectangle is interior when none of the cells it covers has an empty cell beside it. The edge of the grid
    // counts as empty. Indices stay ascending because `valid` is ascending.
    for (k, span) in spans.iter().enumerate() {
        let touches_empty = (span.y0..span.y1).any(|j| {
            (span.x0..span.x1).any(|i| {
                [(0_isize, -1_isize), (0, 1), (-1, 0), (1, 0)]
                    .iter()
                    .any(|(di, dj)| {
                        let (ni, nj) = (i as isize + di, j as isize + dj);
                        ni < 0
                            || nj < 0
                            || ni >= cols as isize
                            || nj >= rows as isize
                            || owner[nj as usize * cols + ni as usize].is_none()
                    })
            })
        });
        if !touches_empty {
            clusters[cluster_of_rect[k]].interior.push(valid[k]);
        }
    }

    // An inward corner has exactly three filled cells around its vertex (k, l). Two filled cells that only touch
    // at the vertex (a checkerboard) count too: there is no gap to walk through, so both edges include the vertex.
    let concave = |k: usize, l: usize| -> bool {
        let filled = |i: Option<usize>, j: Option<usize>| match (i, j) {
            (Some(i), Some(j)) if i < cols && j < rows => owner[j * cols + i].is_some(),
            _ => false,
        };
        let [bl, br, tl, tr] = [
            filled(k.checked_sub(1), l.checked_sub(1)),
            filled(Some(k), l.checked_sub(1)),
            filled(k.checked_sub(1), Some(l)),
            filled(Some(k), Some(l)),
        ];
        [bl, br, tl, tr].iter().filter(|f| **f).count() == 3 || (bl == tr && br == tl && bl != br)
    };
    let cell = |i: Option<usize>, j: Option<usize>| -> Option<usize> {
        match (i, j) {
            (Some(i), Some(j)) if i < cols && j < rows => owner[j * cols + i],
            _ => None,
        }
    };

    // A boundary piece lies between a filled and an empty cell. Pieces in a row with the same cluster and
    // blocked side are merged into one edge.
    for (k, &x) in xs.iter().enumerate() {
        let mut run: Option<(usize, bool, usize)> = None;
        for j in 0..=rows {
            let piece = if j < rows {
                let left = cell(k.checked_sub(1), Some(j));
                let right = cell(Some(k), Some(j));
                (left.is_some() != right.is_some())
                    .then(|| (left.or(right).unwrap_or(0), right.is_some()))
            } else {
                None
            };
            flush(&mut run, piece, j, |cluster, positive, from, to| {
                clusters[cluster].edges.push(Edge {
                    orientation: Orientation::Vertical,
                    at: x,
                    from: ys[from],
                    to: ys[to],
                    blocked_positive: positive,
                    from_concave: concave(k, from),
                    to_concave: concave(k, to),
                });
            });
        }
    }
    for (l, &y) in ys.iter().enumerate() {
        let mut run: Option<(usize, bool, usize)> = None;
        for i in 0..=cols {
            let piece = if i < cols {
                let below = cell(Some(i), l.checked_sub(1));
                let above = cell(Some(i), Some(l));
                (below.is_some() != above.is_some())
                    .then(|| (below.or(above).unwrap_or(0), above.is_some()))
            } else {
                None
            };
            flush(&mut run, piece, i, |cluster, positive, from, to| {
                clusters[cluster].edges.push(Edge {
                    orientation: Orientation::Horizontal,
                    at: y,
                    from: xs[from],
                    to: xs[to],
                    blocked_positive: positive,
                    from_concave: concave(from, l),
                    to_concave: concave(to, l),
                });
            });
        }
    }

    for cluster in &mut clusters {
        cluster.edges.sort_by(|a, b| {
            (a.orientation, a.at, a.from)
                .partial_cmp(&(b.orientation, b.at, b.from))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    clusters
}

/// A rectangle as index ranges into the compressed coordinate lists.
struct Span {
    x0: usize,
    x1: usize,
    y0: usize,
    y1: usize,
}

impl Span {
    /// True when the closed ranges meet on both axes, so touching at an edge or a corner counts.
    fn touches(&self, other: &Span) -> bool {
        self.x0 <= other.x1 && other.x0 <= self.x1 && self.y0 <= other.y1 && other.y0 <= self.y1
    }
}

/// Sorted coordinates with values within `EPSILON` of each other merged into the lowest one.
fn snap_axis(values: impl Iterator<Item = f32>) -> Vec<f32> {
    let mut sorted: Vec<f32> = values.collect();
    sorted.sort_by(f32::total_cmp);
    let mut out: Vec<f32> = Vec::new();
    for value in sorted {
        if out.last().is_none_or(|last| value - last > EPSILON) {
            out.push(value);
        }
    }
    out
}

fn index_of(axis: &[f32], value: f32) -> usize {
    axis.partition_point(|a| *a < value - EPSILON)
}

fn find(parent: &mut [usize], mut node: usize) -> usize {
    while parent[node] != node {
        parent[node] = parent[parent[node]];
        node = parent[node];
    }
    node
}

/// Extends or ends a run of boundary pieces. `position` is the index of the piece being looked at; a run
/// from `start` to `position` ends when the piece changes or disappears.
fn flush(
    run: &mut Option<(usize, bool, usize)>,
    piece: Option<(usize, bool)>,
    position: usize,
    mut emit: impl FnMut(usize, bool, usize, usize),
) {
    match (*run, piece) {
        (Some((cluster, positive, _)), Some((c, p))) if cluster == c && positive == p => {}
        (current, next) => {
            if let Some((cluster, positive, start)) = current {
                emit(cluster, positive, start, position);
            }
            *run = next.map(|(c, p)| (c, p, position));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Half the width of an actor, used to grow obstacles in the tests. Same value as Long Way's Guy.
    const ACTOR_HALF_WIDTH: f32 = 20.0;

    fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::new(x0, y0, x1, y1)
    }

    fn vertical(at: f32, from: f32, to: f32, blocked_positive: bool) -> Edge {
        Edge {
            orientation: Orientation::Vertical,
            at,
            from,
            to,
            blocked_positive,
            from_concave: false,
            to_concave: false,
        }
    }

    fn horizontal(at: f32, from: f32, to: f32, blocked_positive: bool) -> Edge {
        Edge {
            orientation: Orientation::Horizontal,
            at,
            from,
            to,
            blocked_positive,
            from_concave: false,
            to_concave: false,
        }
    }

    /// The edges without the concave flags, so shape tests can compare positions and sides only.
    fn plain(edges: &[Edge]) -> Vec<Edge> {
        edges
            .iter()
            .map(|e| Edge {
                from_concave: false,
                to_concave: false,
                ..*e
            })
            .collect()
    }

    fn sorted(mut edges: Vec<Edge>) -> Vec<Edge> {
        edges.sort_by(|a, b| {
            (a.orientation, a.at, a.from)
                .partial_cmp(&(b.orientation, b.at, b.from))
                .expect("no NaN")
        });
        edges
    }

    /// The blocked area of an obstacle `size` wide and high: the obstacle grown sideways by the actor's half width.
    fn grown(x: f32, y: f32, size: f32) -> Rect {
        rect(
            x - ACTOR_HALF_WIDTH,
            y,
            x + size + ACTOR_HALF_WIDTH,
            y + size,
        )
    }

    #[test]
    fn no_rectangles_give_no_clusters() {
        assert!(build(&[]).is_empty());
    }

    #[test]
    fn a_rectangle_with_no_area_is_ignored() {
        assert!(build(&[rect(0.0, 0.0, 0.0, 10.0), rect(5.0, 5.0, 9.0, 5.0)]).is_empty());
    }

    #[test]
    fn one_rectangle_has_four_edges_with_the_inside_blocked() {
        let clusters = build(&[rect(0.0, 0.0, 10.0, 20.0)]);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].members, vec![0]);
        assert_eq!(clusters[0].bounds, rect(0.0, 0.0, 10.0, 20.0));
        assert_eq!(
            plain(&clusters[0].edges),
            sorted(vec![
                vertical(0.0, 0.0, 20.0, true),
                vertical(10.0, 0.0, 20.0, false),
                horizontal(0.0, 0.0, 10.0, true),
                horizontal(20.0, 0.0, 10.0, false),
            ])
        );
    }

    #[test]
    fn two_rectangles_side_by_side_merge_and_the_shared_edge_disappears() {
        let clusters = build(&[rect(0.0, 0.0, 10.0, 10.0), rect(10.0, 0.0, 25.0, 10.0)]);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].members, vec![0, 1]);
        assert_eq!(
            plain(&clusters[0].edges),
            sorted(vec![
                vertical(0.0, 0.0, 10.0, true),
                vertical(25.0, 0.0, 10.0, false),
                horizontal(0.0, 0.0, 25.0, true),
                horizontal(10.0, 0.0, 25.0, false),
            ])
        );
    }

    #[test]
    fn overlapping_rectangles_merge_into_one_outline() {
        let clusters = build(&[rect(0.0, 0.0, 10.0, 10.0), rect(5.0, 0.0, 20.0, 10.0)]);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].edges.len(), 4);
        assert_eq!(clusters[0].bounds, rect(0.0, 0.0, 20.0, 10.0));
    }

    #[test]
    fn duplicate_rectangles_are_one_shape() {
        let clusters = build(&[rect(0.0, 0.0, 10.0, 10.0), rect(0.0, 0.0, 10.0, 10.0)]);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].members, vec![0, 1]);
        assert_eq!(clusters[0].edges.len(), 4);
    }

    #[test]
    fn staggered_rectangles_make_a_step() {
        // Left one is 0..10 high, right one is 5..15 high, they overlap in x from 8 to 10.
        let clusters = build(&[rect(0.0, 0.0, 10.0, 10.0), rect(8.0, 5.0, 20.0, 15.0)]);
        assert_eq!(clusters.len(), 1);
        assert_eq!(
            plain(&clusters[0].edges),
            sorted(vec![
                vertical(0.0, 0.0, 10.0, true),
                vertical(8.0, 10.0, 15.0, true),
                vertical(10.0, 0.0, 5.0, false),
                vertical(20.0, 5.0, 15.0, false),
                horizontal(0.0, 0.0, 10.0, true),
                horizontal(5.0, 10.0, 20.0, true),
                horizontal(10.0, 0.0, 8.0, false),
                horizontal(15.0, 8.0, 20.0, false),
            ])
        );
    }

    #[test]
    fn an_l_shape_has_six_edges() {
        // A bar 0..30 by 0..10 with a tower 0..10 by 10..30 on its left end.
        let clusters = build(&[rect(0.0, 0.0, 30.0, 10.0), rect(0.0, 10.0, 10.0, 30.0)]);
        assert_eq!(clusters.len(), 1);
        assert_eq!(
            plain(&clusters[0].edges),
            sorted(vec![
                vertical(0.0, 0.0, 30.0, true),
                vertical(10.0, 10.0, 30.0, false),
                vertical(30.0, 0.0, 10.0, false),
                horizontal(0.0, 0.0, 30.0, true),
                horizontal(10.0, 10.0, 30.0, false),
                horizontal(30.0, 0.0, 10.0, false),
            ])
        );
    }

    #[test]
    fn a_notch_has_walls_that_face_each_other() {
        // A U shape: left pillar 0..10, right pillar 30..40, base 0..40 by 0..10, notch 10..30 above the base.
        let clusters = build(&[
            rect(0.0, 0.0, 40.0, 10.0),
            rect(0.0, 10.0, 10.0, 40.0),
            rect(30.0, 10.0, 40.0, 40.0),
        ]);
        assert_eq!(clusters.len(), 1);
        let edges = plain(&clusters[0].edges);
        assert_eq!(edges.len(), 8);
        // The notch floor is blocked below it, and the two notch walls are blocked outside the notch.
        assert!(edges.contains(&horizontal(10.0, 10.0, 30.0, false)));
        assert!(edges.contains(&vertical(10.0, 10.0, 40.0, false)));
        assert!(edges.contains(&vertical(30.0, 10.0, 40.0, true)));
    }

    #[test]
    fn a_ring_has_an_inner_outline_with_the_blocked_side_flipped() {
        // Eight rectangles around a 10 by 10 hole.
        let clusters = build(&[
            rect(0.0, 0.0, 10.0, 10.0),
            rect(10.0, 0.0, 20.0, 10.0),
            rect(20.0, 0.0, 30.0, 10.0),
            rect(0.0, 10.0, 10.0, 20.0),
            rect(20.0, 10.0, 30.0, 20.0),
            rect(0.0, 20.0, 10.0, 30.0),
            rect(10.0, 20.0, 20.0, 30.0),
            rect(20.0, 20.0, 30.0, 30.0),
        ]);
        assert_eq!(clusters.len(), 1);
        let edges = plain(&clusters[0].edges);
        assert_eq!(edges.len(), 8);
        assert!(edges.contains(&vertical(10.0, 10.0, 20.0, false)));
        assert!(edges.contains(&vertical(20.0, 10.0, 20.0, true)));
        assert!(edges.contains(&horizontal(10.0, 10.0, 20.0, false)));
        assert!(edges.contains(&horizontal(20.0, 10.0, 20.0, true)));
    }

    #[test]
    fn rectangles_touching_only_at_a_corner_are_one_cluster() {
        let clusters = build(&[rect(0.0, 0.0, 10.0, 10.0), rect(10.0, 10.0, 20.0, 20.0)]);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].members, vec![0, 1]);
    }

    #[test]
    fn the_vertex_where_two_rectangles_touch_at_a_corner_counts_as_an_inward_corner() {
        let clusters = build(&[rect(0.0, 0.0, 10.0, 10.0), rect(10.0, 10.0, 20.0, 20.0)]);
        let at_vertex = |e: &&Edge| match e.orientation {
            Orientation::Horizontal => e.from == 10.0 || e.to == 10.0,
            Orientation::Vertical => e.from == 10.0 || e.to == 10.0,
        };
        let ends: Vec<bool> = clusters[0]
            .edges
            .iter()
            .filter(|e| (e.at == 10.0) && at_vertex(e))
            .flat_map(|e| {
                [
                    (e.from == 10.0).then_some(e.from_concave),
                    (e.to == 10.0).then_some(e.to_concave),
                ]
            })
            .flatten()
            .collect();
        assert_eq!(ends.len(), 4, "four edge ends meet at (10, 10)");
        assert!(ends.iter().all(|concave| *concave));
    }

    #[test]
    fn rectangles_far_apart_are_separate_clusters_ordered_by_lowest_member() {
        let clusters = build(&[
            rect(0.0, 0.0, 10.0, 10.0),
            rect(100.0, 0.0, 110.0, 10.0),
            rect(5.0, 5.0, 12.0, 12.0),
        ]);
        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].members, vec![0, 2]);
        assert_eq!(clusters[1].members, vec![1]);
        assert_eq!(clusters[1].edges.len(), 4);
    }

    #[test]
    fn a_gap_just_wide_enough_for_guy_keeps_two_clusters() {
        // Obstacles 41 apart: after growing by 20 each side, 1 unit is left for the feet.
        let clusters = build(&[grown(0.0, 0.0, 100.0), grown(141.0, 0.0, 100.0)]);
        assert_eq!(clusters.len(), 2);
    }

    #[test]
    fn a_gap_of_exactly_guys_width_is_closed() {
        // Obstacles 40 apart: the grown areas touch, so there is no room for the feet.
        let clusters = build(&[grown(0.0, 0.0, 100.0), grown(140.0, 0.0, 100.0)]);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].edges.len(), 4);
    }

    #[test]
    fn a_gap_too_narrow_for_guy_is_closed() {
        let clusters = build(&[grown(0.0, 0.0, 100.0), grown(139.0, 0.0, 100.0)]);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].edges.len(), 4);
    }

    #[test]
    fn rounding_noise_does_not_open_a_hairline_gap() {
        let clusters = build(&[
            rect(0.0, 0.0, 10.0, 10.0),
            rect(10.000_01, 0.0, 20.0, 10.000_01),
        ]);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].edges.len(), 4, "{:?}", clusters[0].edges);
    }

    #[test]
    fn a_gap_wider_than_epsilon_stays_open() {
        let clusters = build(&[rect(0.0, 0.0, 10.0, 10.0), rect(10.01, 0.0, 20.0, 10.0)]);
        assert_eq!(clusters.len(), 2);
    }

    #[test]
    fn a_single_rectangle_has_no_concave_ends() {
        let clusters = build(&[rect(0.0, 0.0, 10.0, 10.0)]);
        assert!(
            clusters[0]
                .edges
                .iter()
                .all(|e| !e.from_concave && !e.to_concave)
        );
    }

    #[test]
    fn an_l_shape_has_one_inward_corner_at_its_inner_vertex() {
        let clusters = build(&[rect(0.0, 0.0, 30.0, 10.0), rect(0.0, 10.0, 10.0, 30.0)]);
        let edges = &clusters[0].edges;
        let concave_ends: Vec<_> = edges
            .iter()
            .flat_map(|e| [(e, e.from_concave, e.from), (e, e.to_concave, e.to)])
            .filter(|(_, concave, _)| *concave)
            .map(|(e, _, along)| match e.orientation {
                Orientation::Vertical => (e.at, along),
                Orientation::Horizontal => (along, e.at),
            })
            .collect();
        // Two edges meet at the inner vertex (10, 10), and both mark that end.
        assert_eq!(concave_ends, vec![(10.0, 10.0), (10.0, 10.0)]);
    }

    #[test]
    fn the_ends_of_a_notch_floor_are_both_inward_corners() {
        let clusters = build(&[
            rect(0.0, 0.0, 40.0, 10.0),
            rect(0.0, 10.0, 10.0, 40.0),
            rect(30.0, 10.0, 40.0, 40.0),
        ]);
        let floor = clusters[0]
            .edges
            .iter()
            .find(|e| e.orientation == Orientation::Horizontal && e.at == 10.0)
            .copied()
            .expect("notch floor");
        assert!(floor.from_concave && floor.to_concave);
        // The notch walls meet the floor at their lower end, which is inward, and end in the open at the top.
        let wall = clusters[0]
            .edges
            .iter()
            .find(|e| e.orientation == Orientation::Vertical && e.at == 10.0)
            .copied()
            .expect("notch wall");
        assert!(wall.from_concave && !wall.to_concave);
    }

    #[test]
    fn the_inner_corners_of_a_ring_are_inward() {
        let rects: Vec<Rect> = [
            (0.0, 0.0),
            (10.0, 0.0),
            (20.0, 0.0),
            (0.0, 10.0),
            (20.0, 10.0),
            (0.0, 20.0),
            (10.0, 20.0),
            (20.0, 20.0),
        ]
        .iter()
        .map(|(x, y)| rect(*x, *y, x + 10.0, y + 10.0))
        .collect();
        let edges = build(&rects)[0].edges.clone();
        let inner: Vec<_> = edges
            .iter()
            .filter(|e| e.at == 10.0 || e.at == 20.0)
            .filter(|e| e.from == 10.0 && e.to == 20.0)
            .collect();
        assert_eq!(inner.len(), 4);
        assert!(inner.iter().all(|e| e.from_concave && e.to_concave));
        let outer = edges.iter().filter(|e| e.at == 0.0 || e.at == 30.0);
        assert!(outer.into_iter().all(|e| !e.from_concave && !e.to_concave));
    }

    #[test]
    fn covers_excludes_convex_ends_and_includes_inward_ends() {
        let edge = |from_concave, to_concave| Edge {
            orientation: Orientation::Vertical,
            at: 0.0,
            from: 10.0,
            to: 20.0,
            blocked_positive: true,
            from_concave,
            to_concave,
        };
        let convex = edge(false, false);
        assert!(convex.covers(15.0));
        assert!(!convex.covers(10.0) && !convex.covers(20.0));
        let inward = edge(true, true);
        assert!(inward.covers(10.0) && inward.covers(20.0));
        assert!(!inward.covers(9.9) && !inward.covers(20.1));
        let mixed = edge(true, false);
        assert!(mixed.covers(10.0) && !mixed.covers(20.0));
        // Noise within the tolerance counts as at the end.
        assert!(!convex.covers(10.005) && !convex.covers(19.995));
        assert!(convex.covers(10.02) && convex.covers(19.98));
        assert!(inward.covers(9.995) && inward.covers(20.005));
        assert!(!inward.covers(9.98) && !inward.covers(20.02));
    }

    fn interior_of(rects: &[Rect]) -> Vec<usize> {
        let mut found: Vec<usize> = build(rects)
            .iter()
            .flat_map(|c| c.interior.iter().copied())
            .collect();
        found.sort();
        found
    }

    fn grid_of_rects(columns: usize, rows: usize) -> Vec<Rect> {
        (0..rows)
            .flat_map(|j| (0..columns).map(move |i| (i as f32 * 10.0, j as f32 * 10.0)))
            .map(|(x, y)| rect(x, y, x + 10.0, y + 10.0))
            .collect()
    }

    #[test]
    fn a_single_rectangle_is_not_interior() {
        assert!(interior_of(&[rect(0.0, 0.0, 10.0, 10.0)]).is_empty());
    }

    #[test]
    fn only_the_middle_of_a_three_by_three_block_is_interior() {
        assert_eq!(interior_of(&grid_of_rects(3, 3)), vec![4]);
    }

    #[test]
    fn a_five_by_five_block_has_nine_interior_rectangles() {
        let interior = interior_of(&grid_of_rects(5, 5));
        let expected: Vec<usize> = (1..4)
            .flat_map(|j| (1..4).map(move |i| j * 5 + i))
            .collect();
        assert_eq!(interior, expected);
    }

    #[test]
    fn the_middle_of_a_plus_is_interior_though_its_corners_are_empty() {
        let interior = interior_of(&[
            rect(10.0, 10.0, 20.0, 20.0),
            rect(0.0, 10.0, 10.0, 20.0),
            rect(20.0, 10.0, 30.0, 20.0),
            rect(10.0, 0.0, 20.0, 10.0),
            rect(10.0, 20.0, 20.0, 30.0),
        ]);
        assert_eq!(interior, vec![0]);
    }

    #[test]
    fn dropping_an_interior_rectangle_from_the_input_can_open_a_hole() {
        // A ring of eight with a ninth rectangle filling the hole. The ninth is interior, and it is the only
        // thing covering that area, so the flag must not be used to remove rectangles from `build`.
        let full = grid_of_rects(3, 3);
        assert_eq!(interior_of(&full), vec![4]);
        assert_eq!(build(&full)[0].edges.len(), 4);
        let mut ring = full.clone();
        ring.remove(4);
        assert_eq!(build(&ring)[0].edges.len(), 8);
    }

    /// A small repeatable random number source, so the random layouts below are the same on every run.
    struct Lcg(u64);

    impl Lcg {
        fn below(&mut self, n: u64) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (self.0 >> 33) % n
        }
    }

    /// True when an outline edge has some length lying on the closed rectangle.
    fn touches_an_edge(r: &Rect, edges: &[Edge]) -> bool {
        edges.iter().any(|e| {
            let (across, along_min, along_max, rect_across, rect_min, rect_max) = match e
                .orientation
            {
                Orientation::Vertical => (e.at, r.min.y, r.max.y, (r.min.x, r.max.x), e.from, e.to),
                Orientation::Horizontal => {
                    (e.at, r.min.x, r.max.x, (r.min.y, r.max.y), e.from, e.to)
                }
            };
            across >= rect_across.0 - EPSILON
                && across <= rect_across.1 + EPSILON
                && rect_max.min(along_max) - rect_min.max(along_min) > EPSILON
        })
    }

    #[test]
    fn interior_means_no_edge_lies_on_the_rectangle_in_random_layouts() {
        let mut random = Lcg(7);
        let mut interior_seen = 0;
        for _ in 0..300 {
            let count = 3 + random.below(10) as usize;
            let rects: Vec<Rect> = (0..count)
                .map(|_| {
                    let (x, y) = (random.below(6) as f32 * 10.0, random.below(6) as f32 * 10.0);
                    let (w, h) = (
                        10.0 + random.below(3) as f32 * 10.0,
                        10.0 + random.below(3) as f32 * 10.0,
                    );
                    rect(x, y, x + w, y + h)
                })
                .collect();
            for cluster in build(&rects) {
                for &member in &cluster.members {
                    let on_outline = touches_an_edge(&rects[member], &cluster.edges);
                    let flagged = cluster.interior.contains(&member);
                    assert_eq!(flagged, !on_outline, "member {member} of {rects:?}");
                    interior_seen += flagged as usize;
                }
            }
        }
        assert!(
            interior_seen > 20,
            "the layouts must contain interior rectangles, got {interior_seen}"
        );
    }

    #[test]
    fn input_order_does_not_change_the_edges() {
        let rects = [
            rect(0.0, 0.0, 40.0, 10.0),
            rect(0.0, 10.0, 10.0, 40.0),
            rect(30.0, 10.0, 40.0, 40.0),
        ];
        let reversed = [rects[2], rects[1], rects[0]];
        assert_eq!(build(&rects)[0].edges, build(&reversed)[0].edges);
    }
}
