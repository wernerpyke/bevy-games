mod camera;

use bevy::prelude::*;
use bevy_aseprite_ultra::AsepriteUltraPlugin;

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
            game_kit::script::ScriptPlugin {
                env_prefix: "JET",
                game_commands: &[],
            },
        ))
        .run();
}
