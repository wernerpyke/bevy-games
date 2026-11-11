//! Neighbour groups on a cell grid (`adr-x20`). Items whose cells touch form a group, and a game gives group
//! members a different blocked area than an item that stands alone (see `dev-docs/extensions/groups.md`).
//! Plain Rust, so it is unit tested without a running app.

use std::collections::HashMap;

use bevy::math::IVec2;

/// Which cells count as neighbours of a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Adjacency {
    /// The 4 cells that share an edge.
    Edges,
    /// The 8 cells around a cell: edges and corners.
    EdgesAndCorners,
}

impl Adjacency {
    fn offsets(self) -> &'static [IVec2] {
        const EDGES: [IVec2; 4] = [
            IVec2::new(1, 0),
            IVec2::new(-1, 0),
            IVec2::new(0, 1),
            IVec2::new(0, -1),
        ];
        const ALL: [IVec2; 8] = [
            IVec2::new(1, 0),
            IVec2::new(-1, 0),
            IVec2::new(0, 1),
            IVec2::new(0, -1),
            IVec2::new(1, 1),
            IVec2::new(1, -1),
            IVec2::new(-1, 1),
            IVec2::new(-1, -1),
        ];
        match self {
            Adjacency::Edges => &EDGES,
            Adjacency::EdgesAndCorners => &ALL,
        }
    }
}

/// The group of each item, in input order. An item at `anchors[i]` covers `size.x` by `size.y` cells, from the
/// anchor cell up and to the right (`IVec2::ONE` for an item of one cell). Two items are in the same group when a
/// chain of items links them, where two items are linked when their cells overlap or are neighbours.
/// A group id is the index of the first item of the group, so ids are stable for the same input.
pub fn neighbour_groups(anchors: &[IVec2], size: IVec2, adjacency: Adjacency) -> Vec<usize> {
    let mut parent: Vec<usize> = (0..anchors.len()).collect();
    // The smaller index always becomes the root, so the root is the first item of the group.
    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    fn union(parent: &mut [usize], a: usize, b: usize) {
        let (a, b) = (find(parent, a), find(parent, b));
        if a != b {
            parent[a.max(b)] = a.min(b);
        }
    }

    let mut owner: HashMap<IVec2, usize> = HashMap::new();
    let covered = |anchor: IVec2| {
        (0..size.y.max(1))
            .flat_map(move |dy| (0..size.x.max(1)).map(move |dx| anchor + IVec2::new(dx, dy)))
    };
    for (i, anchor) in anchors.iter().enumerate() {
        for cell in covered(*anchor) {
            if let Some(&other) = owner.get(&cell) {
                union(&mut parent, i, other);
            } else {
                owner.insert(cell, i);
            }
        }
    }
    for (i, anchor) in anchors.iter().enumerate() {
        for cell in covered(*anchor) {
            for offset in adjacency.offsets() {
                if let Some(&other) = owner.get(&(cell + *offset)) {
                    union(&mut parent, i, other);
                }
            }
        }
    }
    (0..anchors.len()).map(|i| find(&mut parent, i)).collect()
}

/// For each entry of `groups` (from `neighbour_groups`), the number of items in its group. A size of 1 is an item
/// that stands alone.
pub fn group_sizes(groups: &[usize]) -> Vec<usize> {
    let mut counts: HashMap<usize, usize> = HashMap::new();
    for group in groups {
        *counts.entry(*group).or_insert(0) += 1;
    }
    groups.iter().map(|group| counts[group]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: Adjacency = Adjacency::EdgesAndCorners;

    fn cells(list: &[(i32, i32)]) -> Vec<IVec2> {
        list.iter().map(|&(x, y)| IVec2::new(x, y)).collect()
    }

    fn sizes_of(list: &[(i32, i32)], size: IVec2, adjacency: Adjacency) -> Vec<usize> {
        group_sizes(&neighbour_groups(&cells(list), size, adjacency))
    }

    fn sizes(list: &[(i32, i32)], adjacency: Adjacency) -> Vec<usize> {
        sizes_of(list, IVec2::ONE, adjacency)
    }

    #[test]
    fn no_items_give_no_groups() {
        assert!(neighbour_groups(&[], IVec2::ONE, ALL).is_empty());
        assert!(group_sizes(&[]).is_empty());
    }

    #[test]
    fn one_item_stands_alone() {
        assert_eq!(sizes(&[(3, 4)], ALL), vec![1]);
    }

    #[test]
    fn items_side_by_side_form_a_group() {
        assert_eq!(sizes(&[(0, 0), (1, 0)], Adjacency::Edges), vec![2, 2]);
        assert_eq!(sizes(&[(0, 0), (0, 1)], Adjacency::Edges), vec![2, 2]);
    }

    #[test]
    fn a_corner_link_counts_only_with_corner_adjacency() {
        assert_eq!(sizes(&[(0, 0), (1, 1)], ALL), vec![2, 2]);
        assert_eq!(sizes(&[(0, 0), (1, 1)], Adjacency::Edges), vec![1, 1]);
        assert_eq!(sizes(&[(0, 1), (1, 0)], ALL), vec![2, 2]);
    }

    #[test]
    fn items_two_apart_do_not_touch() {
        assert_eq!(sizes(&[(0, 0), (2, 0), (0, 2)], ALL), vec![1, 1, 1]);
    }

    #[test]
    fn a_chain_is_one_group_whatever_the_input_order() {
        let chain = [(5, 0), (0, 0), (3, 0), (1, 0), (4, 0), (2, 0)];
        assert_eq!(sizes(&chain, Adjacency::Edges), vec![6; 6]);
    }

    #[test]
    fn separate_groups_have_their_own_ids_and_sizes() {
        let list = cells(&[(0, 0), (1, 0), (10, 10), (0, 1)]);
        let groups = neighbour_groups(&list, IVec2::ONE, ALL);
        assert_eq!(groups, vec![0, 0, 2, 0]);
        assert_eq!(group_sizes(&groups), vec![3, 3, 1, 3]);
    }

    #[test]
    fn items_on_the_same_cell_join_the_same_group() {
        let groups = neighbour_groups(&cells(&[(0, 0), (5, 5), (0, 0)]), IVec2::ONE, ALL);
        assert_eq!(groups, vec![0, 1, 0]);
    }

    #[test]
    fn negative_cells_work() {
        assert_eq!(sizes(&[(-1, -1), (0, 0), (-2, -2)], ALL), vec![3, 3, 3]);
    }

    #[test]
    fn a_two_by_two_item_touches_a_neighbour_two_cells_away() {
        let two = IVec2::splat(2);
        assert_eq!(
            sizes_of(&[(0, 0), (2, 0)], two, Adjacency::Edges),
            vec![2, 2]
        );
        assert_eq!(sizes_of(&[(0, 0), (3, 0)], two, ALL), vec![1, 1]);
        // Diagonal: the footprints share only the corner at (2,2).
        assert_eq!(sizes_of(&[(0, 0), (2, 2)], two, ALL), vec![2, 2]);
        assert_eq!(
            sizes_of(&[(0, 0), (2, 2)], two, Adjacency::Edges),
            vec![1, 1]
        );
    }

    #[test]
    fn overlapping_footprints_are_one_group() {
        assert_eq!(
            sizes_of(&[(0, 0), (1, 1)], IVec2::splat(2), Adjacency::Edges),
            vec![2, 2]
        );
    }

    #[test]
    fn a_solid_block_with_ragged_rows_is_one_group() {
        let block = [
            (0, 0),
            (1, 0),
            (2, 0),
            (1, 1),
            (2, 1),
            (3, 1),
            (2, 2),
            (3, 2),
        ];
        assert_eq!(sizes(&block, ALL), vec![8; 8]);
    }

    #[test]
    fn a_shape_joined_only_through_a_corner_is_one_group_only_with_corner_adjacency() {
        // (2,2) touches (1,1) only at a corner.
        let joined = [(0, 0), (1, 0), (1, 1), (2, 2), (3, 2)];
        assert_eq!(sizes(&joined, ALL), vec![5; 5]);
        assert_eq!(sizes(&joined, Adjacency::Edges), vec![3, 3, 3, 2, 2]);
    }
}
