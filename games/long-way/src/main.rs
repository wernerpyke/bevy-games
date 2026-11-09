mod background;
mod camera;
mod collision;
mod ghost;
mod hp;
mod hud;
mod inventory;
mod map;
mod markers;
mod pickups;
mod player;
mod rocks;
mod scenery;
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
                        resolution: (camera::WINDOW_WIDTH, camera::WINDOW_HEIGHT).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(AsepriteUltraPlugin)
        .add_plugins((
            game_kit::animation::AnimationHelpPlugin,
            game_kit::collectable::CollectablePlugin,
            inventory::InventoryPlugin,
            hp::HpPlugin,
            ghost::GhostPlugin,
        ))
        .add_plugins((
            game_kit::camera::PixelCameraPlugin {
                view_height: camera::VIEW_HEIGHT,
                clear_color: camera::CLEAR_COLOR,
            },
            game_kit::camera::CameraFollowPlugin,
            game_kit::screen::ScreenPlugin {
                mode: game_kit::screen::ScreenMode::BarTop {
                    ratio: camera::HUD_BAR_RATIO,
                },
            },
            // Long Way has no title, pause dialog or game over screen yet: it starts in the game and has no pause key.
            game_kit::app_state::AppFlowPlugin {
                pause_key: None,
                ..game_kit::app_state::AppFlowPlugin::new(game_kit::app_state::AppState::Playing)
            },
            hud::HudPlugin,
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
            game_kit::reload::ReloadPlugin,
            game_kit::script::ScriptPlugin {
                env_prefix: "LONG_WAY",
            },
        ))
        .add_plugins((
            game_kit::nav::NavPlugin { step: 4.0 },
            game_kit::movement::MovementPlugin::default(),
            game_kit::movement_debug::MovementDebugPlugin,
            game_kit::rng::RngPlugin {
                env_prefix: "LONG_WAY",
            },
            game_kit::strategies::StrategiesPlugin,
            game_kit::touch::TouchPlugin,
        ))
        .run();
}
