mod camera;
mod collision;
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
            game_kit::collision_debug::CollisionDebugPlugin::default(),
            game_kit::debug::DebugPlugin {
                target_name: "guy",
                ..game_kit::debug::DebugPlugin::new(
                    camera::CELL_SIZE,
                    Color::srgb(1.0, 0.1, 0.1),
                    game_kit::debug::GridOrigin::World(camera::MAP_ORIGIN),
                )
            },
            game_kit::depth::DepthPlugin::default(),
            map::MapPlugin,
            player::PlayerPlugin,
            rocks::RocksPlugin,
            script::ScriptPlugin,
            trees::TreesPlugin,
        ))
        .run();
}
