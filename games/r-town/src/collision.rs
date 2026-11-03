//! The hero's collision values. The engine is `game_kit::collision` (`adr-rt5`).

use bevy::prelude::*;

use crate::camera::CELL_SIZE;

/// Half the width of the hero's feet box: the 10 px wide box. Blocked areas are grown sideways by this.
pub const FEET_HALF_WIDTH: f32 = 5.0;
/// Depth of the blocked band at the base of a tree or house, the height of the hero's 4 px feet box. Above it the
/// hero walks behind the object. The kit collision works on the feet point, so the box height is not otherwise used.
pub const BASE_BAND: f32 = 4.0;

/// The area the hero's feet may not enter for an object that blocks its whole cell: the cell grown sideways by the
/// feet half width (`adr-rt9`).
pub fn cell_area(cell: IVec2) -> Rect {
    let corner = cell.as_vec2() * CELL_SIZE;
    Rect::new(
        corner.x - FEET_HALF_WIDTH,
        corner.y,
        corner.x + CELL_SIZE + FEET_HALF_WIDTH,
        corner.y + CELL_SIZE,
    )
}
