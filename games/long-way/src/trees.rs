//! Spawns the trees of the current map (`map.rs`). Each map character of a tree kind is one tree whose 2x2 cell
//! footprint has its bottom left corner at the character's cell. The tree canvas fills the footprint.

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;

use crate::collision::Trunk;
use crate::map::{CurrentMap, Group, Kind, MapEntity, blocking_groups};
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

pub fn spawn_trees(mut commands: Commands, server: Res<AssetServer>, map: Res<CurrentMap>) {
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
        MapEntity,
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
