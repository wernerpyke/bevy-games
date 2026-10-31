use bevy::camera::ScalingMode;
use bevy::prelude::*;

/// Height of the game view in art pixels. Guy is 60 pixels tall, so 1200 makes him one twentieth of the screen height.
pub const VIEW_HEIGHT: f32 = 1200.0;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.16, 0.17, 0.2)))
            .add_systems(Startup, spawn_camera);
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Msaa::Off,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: VIEW_HEIGHT,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}
