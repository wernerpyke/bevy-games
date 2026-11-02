//! R-Town's map: loads the map file, names the object kinds and draws the ground. The parser is `game_kit::map`.

use bevy::prelude::*;
use game_kit::camera::CameraBounds;
use game_kit::layers::BACKGROUND_Z;
use game_kit::map::{Map, MapObject, parse};

use crate::camera::CELL_SIZE;

/// The map file R-Town loads. Edit it and rebuild.
const MAP_NAME: &str = "assets/maps/test.md";
const MAP_SOURCE: &str = include_str!("../assets/maps/test.md");

/// The legend names the parser accepts.
const KIND_NAMES: &[&str] = &["tree_big", "tree_dead"];

/// Size of the map in cells. The map file lists objects only, so the size is set here.
pub const MAP_CELLS: UVec2 = UVec2::new(80, 60);

/// Placeholder ground colour, `#548058`, until tiled backgrounds exist (roadmap item 5).
const GROUND_COLOR: Color = Color::srgb_u8(0x54, 0x80, 0x58);

/// The map area in world units. Cell (0, 0) has its bottom left corner at the world origin.
pub fn map_rect() -> Rect {
    Rect::new(
        0.0,
        0.0,
        MAP_CELLS.x as f32 * CELL_SIZE,
        MAP_CELLS.y as f32 * CELL_SIZE,
    )
}

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        let map = parse(MAP_NAME, MAP_SOURCE, KIND_NAMES).unwrap_or_else(|error| panic!("{error}"));
        info!(
            "map: {} cell {} {} objects",
            map.name,
            map.cell_size,
            map.objects.len()
        );
        app.insert_resource(CurrentMap(map))
            .insert_resource(CameraBounds(map_rect()))
            .add_systems(Startup, spawn_ground);
    }
}

#[derive(Resource)]
pub struct CurrentMap(pub Map);

/// What an object is. Art, footprint and collision rule for each kind are set in code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    TreeBig,
    TreeDead,
}

impl Kind {
    fn from_name(name: &str) -> Option<Kind> {
        match name {
            "tree_big" => Some(Kind::TreeBig),
            "tree_dead" => Some(Kind::TreeDead),
            _ => None,
        }
    }

    /// The kind of a parsed object. The parser only accepts the names in `KIND_NAMES`, so every object has one.
    pub fn of(object: &MapObject) -> Kind {
        Kind::from_name(&object.kind)
            .unwrap_or_else(|| panic!("map object of unknown kind `{}`", object.kind))
    }
}

fn spawn_ground(mut commands: Commands) {
    let area = map_rect();
    commands.spawn((
        Sprite::from_color(GROUND_COLOR, area.size()),
        Transform::from_translation(area.center().extend(BACKGROUND_Z)),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_map_parses_and_stays_inside_the_map() {
        let map = parse(MAP_NAME, MAP_SOURCE, KIND_NAMES).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(map.cell_size, CELL_SIZE);
        assert!(!map.objects.is_empty());
        for object in &map.objects {
            assert!(
                object.cell.x >= 0
                    && object.cell.y >= 0
                    && (object.cell.x as u32) < MAP_CELLS.x
                    && (object.cell.y as u32) < MAP_CELLS.y,
                "{:?} is outside the map",
                object
            );
        }
    }

    #[test]
    fn every_kind_name_has_a_kind() {
        for name in KIND_NAMES {
            assert!(Kind::from_name(name).is_some(), "{name}");
        }
    }

    #[test]
    fn the_map_is_80_by_60_cells() {
        assert_eq!(map_rect().size(), Vec2::new(1280.0, 960.0));
    }
}
