use bevy::camera::ScalingMode;
use bevy::prelude::*;

/// Height of the game view in art pixels. Guy is 60 pixels tall, so 1200 makes him one twentieth of the screen height.
pub const VIEW_HEIGHT: f32 = 1200.0;

/// Size of one grid cell in art pixels.
pub const CELL_SIZE: f32 = 64.0;

/// World position of the bottom left corner of cell (0, 0) for map files and the F2 grid. Fixed, so the cells
/// do not move when the window is resized. It is the corner of a 16:9 view (33.3 by 18.75 cells) rounded to
/// whole cells, so in a 16:9 window the grid starts close to the bottom left of the screen.
pub const MAP_ORIGIN: Vec2 = Vec2::new(-16.0 * CELL_SIZE, -9.0 * CELL_SIZE);

/// Half the visible area in world units. The camera sits at the origin and shows a fixed height,
/// so the visible width follows the window aspect ratio.
pub fn half_view(window: &Window) -> Vec2 {
    Vec2::new(VIEW_HEIGHT * window.width() / window.height(), VIEW_HEIGHT) / 2.0
}

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
