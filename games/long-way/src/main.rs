mod camera;
mod collision;
mod collision_debug;
mod debug;
mod depth;
mod layers;
mod map;
mod player;
mod rocks;
mod script;
mod trees;

use bevy::prelude::*;
use bevy_aseprite_ultra::AsepriteUltraPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(AsepriteUltraPlugin)
        .add_plugins((
            camera::CameraPlugin,
            game_kit::collision::CollisionPlugin {
                allow_sliding: true,
                cell_size: camera::CELL_SIZE,
            },
            collision_debug::CollisionDebugPlugin,
            debug::DebugPlugin {
                cell_size: camera::CELL_SIZE,
                grid_color: Color::srgb(1.0, 0.1, 0.1),
                grid_origin: debug::GridOrigin::World(camera::MAP_ORIGIN),
            },
            depth::DepthPlugin,
            map::MapPlugin,
            player::PlayerPlugin,
            rocks::RocksPlugin,
            script::ScriptPlugin,
            trees::TreesPlugin,
        ))
        .run();
}
