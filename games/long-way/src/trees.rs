//! Spawns the trees of the current map (`map.rs`). Each map character of a tree kind is one tree whose 2x2 cell
//! footprint has its bottom left corner at the character's cell. The tree canvas fills the footprint.

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;

use crate::collision::Trunk;
use crate::map::{CurrentMap, Kind};
use crate::rocks::{SORT_BIAS, footprint_corner};
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
    for object in &map.objects {
        let Some(path) = art_path(Kind::of(object)) else {
            continue;
        };
        spawn_tree(&mut commands, &server, path, object, map);
    }
}

fn spawn_tree(
    commands: &mut Commands,
    server: &AssetServer,
    path: &'static str,
    object: &MapObject,
    map: &Map,
) {
    let (centre, trunk) = placement(footprint_corner(object, map));
    commands.spawn((
        trunk.blocker(),
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

    #[test]
    fn the_trunk_sits_at_its_pixels_in_the_art() {
        let (_, trunk) = placement(Vec2::new(100.0, 200.0));
        assert_eq!((trunk.left, trunk.right), (176.0, 204.0));
    }
}
