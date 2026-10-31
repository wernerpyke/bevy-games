use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;

use crate::collision::Trunk;
use crate::depth::DepthSort;

/// Both tree canvases are 128x128.
const TREE_SIZE: f32 = 128.0;
/// Horizontal extent of the solid trunk in the art, in pixels from the left edge of the canvas.
const TRUNK_LEFT_PX: f32 = 76.0;
const TRUNK_RIGHT_PX: f32 = 104.0;
/// World y of the centre of both trees. Puts the base level with Guy's feet.
const TREE_Y: f32 = 34.0;

pub struct TreesPlugin;

impl Plugin for TreesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_trees);
    }
}

fn spawn_trees(mut commands: Commands, server: Res<AssetServer>) {
    spawn_tree(
        &mut commands,
        &server,
        "art/features/Red tree.aseprite",
        220.0,
    );
    spawn_tree(
        &mut commands,
        &server,
        "art/features/Green tree.aseprite",
        -220.0,
    );
}

/// Spawns a tree with its canvas centred at `x`. Neither tree file has animation tags,
/// so `Animation::default()` plays every frame in a loop.
fn spawn_tree(commands: &mut Commands, server: &AssetServer, path: &'static str, x: f32) {
    let left_edge = x - TREE_SIZE / 2.0;
    commands.spawn((
        Trunk {
            left: left_edge + TRUNK_LEFT_PX,
            right: left_edge + TRUNK_RIGHT_PX,
            base_y: TREE_Y - TREE_SIZE / 2.0,
            height: TREE_SIZE,
        },
        DepthSort {
            feet_offset: TREE_SIZE / 2.0,
        },
        AseAnimation {
            aseprite: server.load(path),
            animation: Animation::default(),
        },
        Sprite::default(),
        Transform::from_xyz(x, TREE_Y, 0.0),
    ));
}
