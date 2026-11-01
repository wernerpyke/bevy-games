//! Merges blocked rectangles into clusters and finds the outline of each cluster. Plain Rust with no Bevy
//! types except `Rect`, so it is unit tested without a running app. See `dev-docs/PLAN-forest-collision.md`.
//!
//! All input rectangles are the areas Guy's feet may not enter, already grown by his half width, so the feet
//! are a point. Rectangles that touch or overlap form one cluster. The edges of a cluster are the boundary of
//! the union of its rectangles. Edges between two rectangles of the same cluster do not exist.

use bevy::math::Rect;

/// Coordinates closer than this are treated as equal, so rounding noise cannot open a hairline gap between
/// two rectangles that are meant to touch.
pub const EPSILON: f32 = 0.001;

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
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cluster {
    /// Indices into the input slice of the rectangles in this cluster, ascending.
    pub members: Vec<usize>,
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
    use crate::collision::GUY_HALF_WIDTH;

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
        }
    }

    fn horizontal(at: f32, from: f32, to: f32, blocked_positive: bool) -> Edge {
        Edge {
            orientation: Orientation::Horizontal,
            at,
            from,
            to,
            blocked_positive,
        }
    }

    fn sorted(mut edges: Vec<Edge>) -> Vec<Edge> {
        edges.sort_by(|a, b| {
            (a.orientation, a.at, a.from)
                .partial_cmp(&(b.orientation, b.at, b.from))
                .expect("no NaN")
        });
        edges
    }

    /// The blocked area of an obstacle `size` wide and high: the obstacle grown sideways by Guy's half width.
    fn grown(x: f32, y: f32, size: f32) -> Rect {
        rect(x - GUY_HALF_WIDTH, y, x + size + GUY_HALF_WIDTH, y + size)
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
            clusters[0].edges,
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
            clusters[0].edges,
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
            clusters[0].edges,
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
            clusters[0].edges,
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
        let edges = &clusters[0].edges;
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
        let edges = &clusters[0].edges;
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
    fn a_gap_wider_than_the_tolerance_stays_open() {
        let clusters = build(&[rect(0.0, 0.0, 10.0, 10.0), rect(10.01, 0.0, 20.0, 10.0)]);
        assert_eq!(clusters.len(), 2);
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
