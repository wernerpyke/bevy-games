use bevy::prelude::*;

/// Z distance between one world unit of y and the draw order. Small enough to stay inside the camera range.
const Z_PER_Y: f32 = 0.001;

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
        transform.translation.z = -sort.feet_y(transform.translation.y) * Z_PER_Y;
    }
}
