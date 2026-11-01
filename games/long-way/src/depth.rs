use bevy::prelude::*;

/// Z distance between one world unit of y and the draw order. Small enough to stay inside the camera range.
const Z_PER_Y: f32 = 0.001;
/// Z distance between one world unit of x, used only to break ties. Two sprites with the same feet y and an
/// equal z have no guaranteed draw order, so overlapping neighbours in a row would flicker. This is far below
/// `Z_PER_Y` even at x = 5000, so it never overrides a difference in y.
const Z_PER_X: f32 = 0.000_000_01;

pub struct DepthPlugin;

impl Plugin for DepthPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sort_by_feet);
    }
}

/// Sorts an entity by its feet so it draws behind things whose feet are lower on the screen.
/// `feet_offset` is the distance from the entity's centre down to its feet.
#[derive(Component)]
pub struct DepthSort {
    pub feet_offset: f32,
}

impl DepthSort {
    pub fn feet_y(&self, centre_y: f32) -> f32 {
        centre_y - self.feet_offset
    }
}

fn sort_by_feet(mut query: Query<(&mut Transform, &DepthSort)>) {
    for (mut transform, sort) in &mut query {
        transform.translation.z = depth_z(
            transform.translation.x,
            sort.feet_y(transform.translation.y),
        );
    }
}

/// Draw order from the feet. Lower feet are in front. At the same feet y, the sprite further right is in front,
/// so the order is fixed and never flickers.
fn depth_z(x: f32, feet_y: f32) -> f32 {
    -feet_y * Z_PER_Y + x * Z_PER_X
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_feet_are_in_front() {
        assert!(depth_z(0.0, -100.0) > depth_z(0.0, 100.0));
    }

    #[test]
    fn same_feet_y_gives_different_z_by_x() {
        assert!(depth_z(64.0, 50.0) > depth_z(0.0, 50.0));
        assert_eq!(depth_z(64.0, 50.0), depth_z(64.0, 50.0));
        // Neighbouring rocks 64 apart must differ by more than f32 rounding at this z.
        let z = depth_z(0.0, 50.0);
        assert!(depth_z(64.0, 50.0) - z > z.abs() * f32::EPSILON * 4.0);
    }

    #[test]
    fn x_never_beats_a_difference_in_y() {
        // One unit of y lower is in front, even with 5000 units of x against it.
        assert!(depth_z(-5000.0, 49.0) > depth_z(5000.0, 50.0));
    }
}
