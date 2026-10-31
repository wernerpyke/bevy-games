mod camera;
mod player;

use bevy::prelude::*;
use bevy_aseprite_ultra::AsepriteUltraPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(AsepriteUltraPlugin)
        .add_plugins((camera::CameraPlugin, player::PlayerPlugin))
        .run();
}
