//! The house in the centre of the field. Its lower three rows block; the top row (roof and chimney) is passable, as
//! in the Pyxel game (`SPEC.md` section 5).

use bevy::prelude::*;
use game_kit::animation::tag_sprite;
use game_kit::collision::Blocker;
use game_kit::depth::SortAtBlocker;

use crate::camera::CELL_SIZE;
use crate::map::{HouseCell, cell_corner};

const ART: &str = "art/house.aseprite";
/// The house footprint in cells.
const SIZE: i32 = 4;
/// Rows of the footprint that block, counted from the bottom.
const SOLID_ROWS: i32 = 3;

#[derive(Component)]
pub struct House;

/// An enemy hit the house. Damage comes with the combat (`SPEC.md` section 7); for now it is logged.
#[derive(Message, Clone, Copy, Debug)]
pub struct HouseHit {
    pub by: Entity,
}

pub struct HousePlugin;

impl Plugin for HousePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<HouseHit>()
            .add_systems(Update, log_hits.run_if(on_message::<HouseHit>));
    }
}

fn log_hits(mut hits: MessageReader<HouseHit>) {
    for hit in hits.read() {
        info!("house: hit by {:?}", hit.by);
    }
}

/// The blocked area of a house whose footprint has its bottom left cell at `anchor`.
fn blocked_area(anchor: IVec2, cell_size: f32) -> Rect {
    let min = cell_corner(anchor, cell_size);
    Rect::from_corners(
        min,
        min + Vec2::new(SIZE as f32, SOLID_ROWS as f32) * cell_size,
    )
}

pub fn spawn_house(mut commands: Commands, server: Res<AssetServer>, house: Res<HouseCell>) {
    let corner = cell_corner(house.0, CELL_SIZE);
    let centre = corner + Vec2::splat(SIZE as f32 * CELL_SIZE / 2.0);
    commands.spawn((
        House,
        Blocker(blocked_area(house.0, CELL_SIZE)),
        SortAtBlocker,
        Transform::from_translation(centre.extend(0.0)),
        tag_sprite(&server, ART, "Idle"),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::MAP_ORIGIN;

    #[test]
    fn the_house_blocks_its_lower_three_rows_only() {
        let area = blocked_area(IVec2::new(3, 2), 10.0);
        assert_eq!(
            area,
            Rect::from_corners(
                MAP_ORIGIN + Vec2::new(30.0, 20.0),
                MAP_ORIGIN + Vec2::new(70.0, 50.0)
            )
        );
    }
}
