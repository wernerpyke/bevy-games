//! Spawns the trees of the current map (`map.rs`). A tree stands on its cell: the bottom centre of its art is the
//! bottom centre of the cell. Only the base blocks the hero (`adr-rt5`), so the hero walks behind the rest.

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;
use game_kit::collision::Blocker;
use game_kit::depth::DepthSort;
use game_kit::map::MapObject;

use crate::camera::CELL_SIZE;
use crate::collision::{BASE_BAND, FEET_HALF_WIDTH};
use crate::map::{CurrentMap, Kind, MapEntity, blocking_group_sizes};

/// Both art files are packed with the same canvas: 30 wide and 39 high, each tree at the bottom centre
/// (`dev-scripts/aseprite/pack_pngs.lua`).
const CANVAS_HEIGHT: f32 = 39.0;
/// A tree sorts this far above its base, so the hero standing on the base line is drawn in front (as `adr-lw13`).
const SORT_BIAS: f32 = 1.0;

pub struct TreesPlugin;

impl Plugin for TreesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_trees);
    }
}

/// The art and the base of a tree kind.
struct TreeArt {
    /// Aseprite tag in `art/features/background-elements.aseprite`.
    tag: &'static str,
    /// The blocking base: its left and right edge in pixels from the centre of the tree, measured in the art.
    base: (f32, f32),
}

fn art(kind: Kind) -> TreeArt {
    match kind {
        // `bg_18`: the base of the big tree is columns 6 to 22 of 30.
        Kind::TreeBig => TreeArt {
            tag: "bg_18",
            base: (-9.0, 8.0),
        },
        // `bg_19`: the trunk of the dead tree is columns 3 to 9 of 14.
        Kind::TreeDead => TreeArt {
            tag: "bg_19",
            base: (-4.0, 3.0),
        },
    }
}

/// The world position of the bottom centre of the cell an object stands on.
fn base_of(object: &MapObject) -> Vec2 {
    object.cell.as_vec2() * CELL_SIZE + Vec2::new(CELL_SIZE / 2.0, 0.0)
}

/// The area the hero's feet may not enter for a tree that stands alone: the base of the tree grown sideways by the
/// feet half width, 4 px deep. The hero walks behind the rest (`adr-rt5`).
fn blocked_area(base: Vec2, edges: (f32, f32)) -> Rect {
    Rect::new(
        base.x + edges.0 - FEET_HALF_WIDTH,
        base.y,
        base.x + edges.1 + FEET_HALF_WIDTH,
        base.y + BASE_BAND,
    )
}

/// The area the hero's feet may not enter for a tree in a group: its whole cell grown sideways by the feet half
/// width, so the group is one solid shape (`adr-rt9`).
fn group_area(cell: IVec2) -> Rect {
    let corner = cell.as_vec2() * CELL_SIZE;
    Rect::new(
        corner.x - FEET_HALF_WIDTH,
        corner.y,
        corner.x + CELL_SIZE + FEET_HALF_WIDTH,
        corner.y + CELL_SIZE,
    )
}

/// The blocked area of an object: the base band when it stands alone, the whole cell when it is in a group.
fn area_of(object: &MapObject, art: &TreeArt, group_size: usize) -> Rect {
    if group_size > 1 {
        group_area(object.cell)
    } else {
        blocked_area(base_of(object), art.base)
    }
}

pub fn spawn_trees(mut commands: Commands, server: Res<AssetServer>, map: Res<CurrentMap>) {
    let sizes = blocking_group_sizes(&map.0);
    for (object, group_size) in map.0.objects.iter().zip(sizes) {
        let art = art(Kind::of(object));
        let base = base_of(object);
        commands
            .spawn((
                MapEntity,
                Blocker(area_of(object, &art, group_size)),
                DepthSort {
                    feet_offset: -SORT_BIAS,
                },
                Transform::from_translation(base.extend(0.0)),
                Visibility::default(),
            ))
            .with_child((
                AseAnimation {
                    aseprite: server.load("art/features/background-elements.aseprite"),
                    animation: Animation::tag(art.tag),
                },
                Sprite::default(),
                Transform::from_xyz(0.0, CANVAS_HEIGHT / 2.0, 0.0),
            ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_kit::groups::{Adjacency, group_sizes, neighbour_groups};

    fn tree(kind: &str, x: i32, y: i32) -> MapObject {
        MapObject {
            kind: kind.to_string(),
            cell: IVec2::new(x, y),
        }
    }

    /// The number of collision clusters the objects make, with the area each one gets.
    fn clusters(objects: &[MapObject]) -> usize {
        let cells: Vec<IVec2> = objects.iter().map(|o| o.cell).collect();
        let sizes = group_sizes(&neighbour_groups(
            &cells,
            IVec2::ONE,
            Adjacency::EdgesAndCorners,
        ));
        let areas: Vec<Rect> = objects
            .iter()
            .zip(sizes)
            .map(|(o, size)| area_of(o, &art(Kind::of(o)), size))
            .collect();
        game_kit::outline::build(&areas).len()
    }

    #[test]
    fn a_tree_alone_blocks_its_base_band_and_a_grouped_tree_its_cell() {
        let alone = tree("tree_big", 2, 3);
        assert_eq!(
            area_of(&alone, &art(Kind::TreeBig), 1),
            Rect::new(26.0, 48.0, 53.0, 52.0)
        );
        assert_eq!(
            area_of(&alone, &art(Kind::TreeBig), 2),
            Rect::new(27.0, 48.0, 53.0, 64.0)
        );
    }

    #[test]
    fn a_solid_group_is_one_cluster_and_lone_trees_stay_apart() {
        let group: Vec<MapObject> = [(10, 1), (11, 1), (12, 1), (12, 2), (13, 3), (14, 4)]
            .iter()
            .map(|&(x, y)| tree("tree_big", x, y))
            .collect();
        assert_eq!(clusters(&group), 1);
        let lone = [tree("tree_dead", 21, 1), tree("tree_dead", 24, 4)];
        assert_eq!(clusters(&lone), 2);
    }

    #[test]
    fn a_mixture_with_notches_is_one_cluster_through_a_corner() {
        let mixed = [
            tree("tree_big", 33, 1),
            tree("tree_big", 34, 1),
            tree("tree_big", 35, 1),
            tree("tree_dead", 34, 2),
            tree("tree_big", 33, 3),
            tree("tree_dead", 35, 3),
            tree("tree_big", 36, 3),
        ];
        assert_eq!(clusters(&mixed), 1);
    }

    #[test]
    fn a_tree_stands_on_the_bottom_centre_of_its_cell() {
        let object = MapObject {
            kind: "tree_big".to_string(),
            cell: IVec2::new(2, 3),
        };
        assert_eq!(base_of(&object), Vec2::new(40.0, 48.0));
    }

    #[test]
    fn the_blocked_area_is_the_base_grown_by_the_feet_half_width() {
        let area = blocked_area(Vec2::new(40.0, 48.0), (-9.0, 8.0));
        assert_eq!(area, Rect::new(26.0, 48.0, 53.0, 52.0));
    }
}
