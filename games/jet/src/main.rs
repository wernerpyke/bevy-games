mod camera;
mod enemy;
mod forest;
mod house;
mod input;
mod map;
mod player;
mod spawning;

use bevy::prelude::*;
use bevy_aseprite_ultra::AsepriteUltraPlugin;
use game_kit::screen::ScreenMode;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Jet".into(),
                        resolution: (camera::WINDOW_WIDTH, camera::WINDOW_HEIGHT).into(),
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
            game_kit::screen::ScreenPlugin {
                mode: ScreenMode::BarTop {
                    ratio: camera::HUD_BAR_HEIGHT / camera::VIEW_HEIGHT,
                },
            },
            game_kit::collision::CollisionPlugin {
                allow_sliding: true,
                cell_size: camera::CELL_SIZE,
            },
            game_kit::collision_debug::CollisionDebugPlugin::default(),
            game_kit::debug::DebugPlugin {
                target_name: "player",
                ..game_kit::debug::DebugPlugin::new(
                    camera::CELL_SIZE,
                    camera::GRID_COLOR,
                    game_kit::debug::GridOrigin::World(camera::MAP_ORIGIN),
                )
            },
            game_kit::depth::DepthPlugin::default(),
            game_kit::animation::AnimationHelpPlugin,
            game_kit::cell_area::CellAreaPlugin,
            game_kit::reload::ReloadPlugin,
            game_kit::screen_fx::ScreenFxPlugin,
            game_kit::script::ScriptPlugin {
                env_prefix: "JET",
                game_commands: &[spawning::LEVEL_COMMAND, spawning::SPINNER_COMMAND],
            },
        ))
        .add_plugins((
            game_kit::nav::NavPlugin { step: 4.0 },
            game_kit::movement::MovementPlugin::default(),
            game_kit::movement_debug::MovementDebugPlugin,
            game_kit::rng::RngPlugin { env_prefix: "JET" },
            game_kit::spawner::SpawnerPlugin,
        ))
        .add_plugins((
            map::MapPlugin,
            forest::ForestPlugin,
            house::HousePlugin,
            player::PlayerPlugin,
            enemy::EnemyPlugin,
            spawning::SpawningPlugin,
        ))
        .run();
}
