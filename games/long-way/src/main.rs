mod camera;
mod collision;
mod debug;
mod depth;
mod player;
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
            collision::CollisionPlugin {
                allow_sliding: true,
            },
            debug::DebugPlugin {
                cell_size: camera::CELL_SIZE,
                grid_color: Color::srgb(1.0, 0.1, 0.1),
                grid_origin: debug::GridOrigin::BottomLeft,
            },
            depth::DepthPlugin,
            player::PlayerPlugin,
            script::ScriptPlugin,
            trees::TreesPlugin,
        ))
        .run();
}
