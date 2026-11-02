//! The hero's collision values. The engine is `game_kit::collision` (`adr-rt5`).

/// Half the width of the hero's feet box: the 10 px wide box. Blocked areas are grown sideways by this.
pub const FEET_HALF_WIDTH: f32 = 5.0;
/// Depth of the blocked band at the base of a tree or house, the height of the hero's 4 px feet box. Above it the
/// hero walks behind the object. The kit collision works on the feet point, so the box height is not otherwise used.
pub const BASE_BAND: f32 = 4.0;
