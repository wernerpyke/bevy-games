mod camera;
mod forest;
mod house;
mod map;

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
            game_kit::script::ScriptPlugin {
                env_prefix: "JET",
                game_commands: &[],
            },
        ))
        .add_plugins((map::MapPlugin, forest::ForestPlugin))
        .run();
}
