mod camera;
mod collision;
mod map;
mod player;
mod rocks;
mod trees;

use bevy::prelude::*;
use bevy_aseprite_ultra::AsepriteUltraPlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: (camera::WINDOW_SIZE, camera::WINDOW_SIZE).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(AsepriteUltraPlugin)
        .add_plugins((
            game_kit::camera::PixelCameraPlugin {
                view_height: camera::VIEW_HEIGHT,
                clear_color: camera::CLEAR_COLOR,
            },
            game_kit::camera::CameraFollowPlugin,
            game_kit::collision::CollisionPlugin {
                allow_sliding: true,
                cell_size: camera::CELL_SIZE,
            },
            game_kit::collision_debug::CollisionDebugPlugin::default(),
            game_kit::debug::DebugPlugin {
                target_name: "player",
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
            game_kit::reload::ReloadPlugin,
            game_kit::script::ScriptPlugin {
                env_prefix: "LONG_WAY",
            },
            trees::TreesPlugin,
        ))
        .run();
}
