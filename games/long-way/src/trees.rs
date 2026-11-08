//! Spawns the trees of the current map (`map.rs`). Each map character of a tree kind is one tree whose 2x2 cell
//! footprint has its bottom left corner at the character's cell. The tree canvas fills the footprint.

use bevy::prelude::*;
use game_kit::animation::RandomStartFrame;
use game_kit::animation::all_frames_animation;

use crate::collision::Trunk;
use crate::map::{Group, Kind, blocking_groups};
use crate::rocks::{SORT_BIAS, footprint_corner};
use game_kit::collision::Blocker;
use game_kit::depth::DepthSort;
use game_kit::map::{Map, MapObject};
use game_kit::map_transition::{MapId, MapLoad};

/// Both tree canvases are 128x128, the size of a 2x2 cell footprint at a cell size of 64.
const TREE_SIZE: f32 = 128.0;
/// Horizontal extent of the solid trunk in the art, in pixels from the left edge of the canvas.
const TRUNK_LEFT_PX: f32 = 76.0;
const TRUNK_RIGHT_PX: f32 = 104.0;

/// The art file of a tree kind. Neither tree file has animation tags, so all frames play in a loop.
fn art_path(kind: Kind) -> Option<&'static str> {
    match kind {
        Kind::RedTree => Some("art/features/Red tree.aseprite"),
        Kind::GreenTree => Some("art/features/Green tree.aseprite"),
        _ => None,
    }
}

/// Where the canvas is centred and which trunk blocks the player, for a tree whose footprint corner is `corner`.
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

/// Spawns the trees of one map. Their trunks are blockers, which `rebuild_collision` adds to the collision world.
pub fn spawn_trees(commands: &mut Commands, server: &AssetServer, map: &Map, load: &MapLoad) {
    for (object, group) in map.objects.iter().zip(blocking_groups(map)) {
        let Some(path) = art_path(Kind::of(object)) else {
            continue;
        };
        spawn_tree(commands, server, path, object, map, load, group);
    }
}

/// A tree that stands alone blocks the bottom band of its trunk. A tree in a group blocks exactly one map cell: the
/// bottom right cell of its footprint, which holds the trunk (`adr-lw18`).
fn tree_blocker(trunk: &Trunk, corner: Vec2, cell: f32, group: Group) -> Blocker {
    if group.is_group() {
        Blocker(Rect::new(
            corner.x + cell,
            corner.y,
            corner.x + 2.0 * cell,
            corner.y + cell,
        ))
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
    load: &MapLoad,
    group: Group,
) {
    let corner = footprint_corner(object, map, load.origin);
    let (centre, trunk) = placement(corner);
    commands.spawn((
        MapId(load.id.clone()),
        tree_blocker(&trunk, corner, map.cell_size, group),
        DepthSort {
            feet_offset: TREE_SIZE / 2.0 - SORT_BIAS,
        },
        all_frames_animation(server, path),
        RandomStartFrame::for_cell(&map.name, object.cell),
        Sprite::default(),
        Transform::from_translation(centre.extend(0.0)),
    ));
}
