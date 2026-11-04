//! Spawns the trees of the current map (`map.rs`). Each map character of a tree kind is one tree whose 2x2 cell
//! footprint has its bottom left corner at the character's cell. The tree canvas fills the footprint.

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;

use crate::collision::Trunk;
use crate::map::{CurrentMap, Group, Kind, blocking_groups};
use crate::rocks::{SORT_BIAS, footprint_corner, group_area};
use game_kit::collision::Blocker;
use game_kit::depth::DepthSort;
use game_kit::map::{Map, MapObject};

/// Both tree canvases are 128x128, the size of a 2x2 cell footprint at a cell size of 64.
const TREE_SIZE: f32 = 128.0;
/// Horizontal extent of the solid trunk in the art, in pixels from the left edge of the canvas.
const TRUNK_LEFT_PX: f32 = 76.0;
const TRUNK_RIGHT_PX: f32 = 104.0;

pub struct TreesPlugin;

impl Plugin for TreesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_trees);
    }
}

/// The art file of a tree kind. Neither tree file has animation tags, so all frames play in a loop.
fn art_path(kind: Kind) -> Option<&'static str> {
    match kind {
        Kind::RedTree => Some("art/features/Red tree.aseprite"),
        Kind::GreenTree => Some("art/features/Green tree.aseprite"),
        _ => None,
    }
}

/// Where the canvas is centred and which trunk blocks Guy, for a tree whose footprint corner is `corner`.
/// The base of the tree is the bottom of the footprint.
fn placement(corner: Vec2) -> (Vec2, Trunk) {
    (
        corner + Vec2::splat(TREE_SIZE / 2.0),
        Trunk {
            left: corner.x + TRUNK_LEFT_PX,
            right: corner.x + TRUNK_RIGHT_PX,
            base_y: corner.y,
            height: TREE_SIZE,
        },
    )
}

fn spawn_trees(mut commands: Commands, server: Res<AssetServer>, map: Res<CurrentMap>) {
    let map = &map.0;
    for (object, group) in map.objects.iter().zip(blocking_groups(map)) {
        let Some(path) = art_path(Kind::of(object)) else {
            continue;
        };
        spawn_tree(&mut commands, &server, path, object, map, group);
    }
}

/// A tree that stands alone blocks its trunk. A tree in a group blocks like a rock in a group, so trees and rocks
/// that touch merge into one shape (`adr-lw14`).
fn tree_blocker(trunk: &Trunk, object: &MapObject, map: &Map, group: Group) -> Blocker {
    if group.is_group() {
        Blocker(group_area(object, map, group.neighbour_above))
    } else {
        trunk.blocker()
    }
}

fn spawn_tree(
    commands: &mut Commands,
    server: &AssetServer,
    path: &'static str,
    object: &MapObject,
    map: &Map,
    group: Group,
) {
    let (centre, trunk) = placement(footprint_corner(object, map));
    commands.spawn((
        tree_blocker(&trunk, object, map, group),
        DepthSort {
            feet_offset: TREE_SIZE / 2.0 - SORT_BIAS,
        },
        AseAnimation {
            aseprite: server.load(path),
            animation: Animation::default(),
        },
        Sprite::default(),
        Transform::from_translation(centre.extend(0.0)),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_kit::map::Map;

    #[test]
    fn only_tree_kinds_have_tree_art() {
        assert!(art_path(Kind::RedTree).is_some());
        assert!(art_path(Kind::GreenTree).is_some());
        assert!(art_path(Kind::Rock0).is_none());
        assert!(art_path(Kind::Rock1).is_none());
    }

    #[test]
    fn the_canvas_fills_the_footprint_and_the_base_is_its_bottom() {
        let (centre, trunk) = placement(Vec2::new(100.0, 200.0));
        assert_eq!(centre, Vec2::new(164.0, 264.0));
        assert_eq!(trunk.base_y, 200.0);
        assert_eq!(trunk.height, 128.0);
    }

    fn object(kind: &str, x: i32, y: i32) -> MapObject {
        MapObject {
            kind: kind.to_string(),
            cell: IVec2::new(x, y),
            params: Default::default(),
        }
    }

    fn test_map(objects: Vec<MapObject>) -> Map {
        Map {
            name: "t".into(),
            cell_size: 64.0,
            origin: game_kit::map::Origin::BottomLeft,
            size: IVec2::ZERO,
            draw_size: Some(128.0),
            jitter: 0.0,
            objects,
        }
    }

    #[test]
    fn a_tree_alone_blocks_its_trunk_and_a_tree_in_a_group_blocks_like_a_rock() {
        let m = test_map(vec![object("green_tree", 0, 0)]);
        let (_, trunk) = placement(footprint_corner(&m.objects[0], &m));
        let alone = tree_blocker(&trunk, &m.objects[0], &m, Group::ALONE);
        assert_eq!(alone.0, trunk.blocker().0);
        let group = Group {
            size: 2,
            neighbour_above: false,
        };
        let grouped = tree_blocker(&trunk, &m.objects[0], &m, group);
        assert_eq!(grouped.0, group_area(&m.objects[0], &m, false));
    }

    #[test]
    fn a_grove_of_trees_and_rocks_two_cells_apart_is_one_cluster() {
        let m = test_map(vec![
            object("green_tree", 12, 2),
            object("red_tree", 14, 2),
            object("rock_0", 12, 4),
            object("green_tree", 14, 4),
        ]);
        let areas: Vec<Rect> = m
            .objects
            .iter()
            .zip(blocking_groups(&m))
            .map(|(o, g)| {
                let (_, trunk) = placement(footprint_corner(o, &m));
                tree_blocker(&trunk, o, &m, g).0
            })
            .collect();
        assert_eq!(game_kit::outline::build(&areas).len(), 1);
    }

    #[test]
    fn the_trunk_sits_at_its_pixels_in_the_art() {
        let (_, trunk) = placement(Vec2::new(100.0, 200.0));
        assert_eq!((trunk.left, trunk.right), (176.0, 204.0));
    }
}
