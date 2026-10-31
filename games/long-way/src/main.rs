mod camera;
mod collision;
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
            depth::DepthPlugin,
            player::PlayerPlugin,
            script::ScriptPlugin,
            trees::TreesPlugin,
        ))
        .run();
}
