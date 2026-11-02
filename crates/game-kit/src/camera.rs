//! The pixel art camera: a 2D camera that shows a fixed height of world units, so art scale on screen comes from
//! one number and not from the window size. See `dev-docs/extensions/pixel-art.md`.

use bevy::camera::ScalingMode;
use bevy::prelude::*;

/// Spawns the 2D camera at the world origin and sets the clear colour. A game that scrolls moves the `Camera2d`
/// entity itself.
pub struct PixelCameraPlugin {
    /// Height of the view in world units (art pixels). On-screen size of a sprite is its height divided by this,
    /// as a share of the screen height.
    pub view_height: f32,
    pub clear_color: Color,
}

impl Plugin for PixelCameraPlugin {
    fn build(&self, app: &mut App) {
        let view_height = self.view_height;
        app.insert_resource(ClearColor(self.clear_color))
            .add_systems(Startup, move |commands: Commands| {
                spawn_camera(commands, view_height)
            });
    }
}

fn spawn_camera(mut commands: Commands, view_height: f32) {
    commands.spawn((
        Camera2d,
        Msaa::Off,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: view_height,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}

/// Half the visible area in world units for a camera at the origin that shows `view_height` of height. The visible
/// width follows the window aspect ratio.
pub fn half_view(window_width: f32, window_height: f32, view_height: f32) -> Vec2 {
    Vec2::new(view_height * window_width / window_height, view_height) / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_view_width_follows_the_window_shape() {
        assert_eq!(
            half_view(1920.0, 1080.0, 1200.0),
            Vec2::new(1200.0 * 16.0 / 9.0 / 2.0, 600.0)
        );
        assert_eq!(half_view(1000.0, 1000.0, 240.0), Vec2::new(120.0, 120.0));
    }

    #[test]
    fn the_plugin_spawns_one_camera_with_a_fixed_view_height() {
        let mut app = App::new();
        app.add_plugins(PixelCameraPlugin {
            view_height: 240.0,
            clear_color: Color::BLACK,
        });
        app.update();
        let mut query = app.world_mut().query::<(&Camera2d, &Projection)>();
        let cameras: Vec<_> = query.iter(app.world()).collect();
        assert_eq!(cameras.len(), 1);
        let Projection::Orthographic(projection) = cameras[0].1 else {
            panic!("the camera is not orthographic");
        };
        assert!(matches!(
            projection.scaling_mode,
            ScalingMode::FixedVertical { viewport_height } if viewport_height == 240.0
        ));
        assert_eq!(app.world().resource::<ClearColor>().0, Color::BLACK);
    }

    #[test]
    fn the_height_does_not_depend_on_the_window_size() {
        assert_eq!(half_view(640.0, 360.0, 1200.0).y, 600.0);
        assert_eq!(half_view(3840.0, 2160.0, 1200.0).y, 600.0);
    }
}
