//! Spawns the water of the current map (`map.rs`). Water is a 16 px tile that fills its cell and blocks the player on
//! the whole cell. It lies on the ground, so it has no depth sort and no walking behind it.

use bevy::prelude::*;
use game_kit::animation::tag_sprite;
use game_kit::collision::Blocker;
use game_kit::layers::BACKGROUND_Z;

use crate::camera::CELL_SIZE;
use crate::collision::cell_area;
use crate::map::{CurrentMap, Kind, MapEntity};

/// Water draws just above the ground colour. It is R-Town's own sublayer of the background (`adr-x14`).
const WATER_Z: f32 = BACKGROUND_Z + 1.0;

pub struct WaterPlugin;

impl Plugin for WaterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_water);
    }
}

/// The nine tiles of a pond: the outer ring is the edge, the middle is open water.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    NorthWest,
    North,
    NorthEast,
    West,
    Centre,
    East,
    SouthWest,
    South,
    SouthEast,
}

impl Part {
    pub const ALL: [Part; 9] = [
        Part::NorthWest,
        Part::North,
        Part::NorthEast,
        Part::West,
        Part::Centre,
        Part::East,
        Part::SouthWest,
        Part::South,
        Part::SouthEast,
    ];

    /// The legend name, which is also the Aseprite tag in `art/features/water.aseprite`.
    pub fn name(self) -> &'static str {
        match self {
            Part::NorthWest => "water_nw",
            Part::North => "water_n",
            Part::NorthEast => "water_ne",
            Part::West => "water_w",
            Part::Centre => "water_c",
            Part::East => "water_e",
            Part::SouthWest => "water_sw",
            Part::South => "water_s",
            Part::SouthEast => "water_se",
        }
    }
}

pub fn spawn_water(mut commands: Commands, server: Res<AssetServer>, map: Res<CurrentMap>) {
    for object in &map.0.objects {
        let Kind::Water(part) = Kind::of(object) else {
            continue;
        };
        let centre = (object.cell.as_vec2() + Vec2::splat(0.5)) * CELL_SIZE;
        commands.spawn((
            MapEntity,
            Blocker(cell_area(object.cell)),
            tag_sprite(&server, "art/features/water.aseprite", part.name()),
            Transform::from_translation(centre.extend(WATER_Z)),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_part_has_its_own_name() {
        let mut names: Vec<&str> = Part::ALL.iter().map(|p| p.name()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 9);
    }
}
