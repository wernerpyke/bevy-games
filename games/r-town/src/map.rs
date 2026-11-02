//! R-Town's map: loads the map file, names the object kinds and draws the ground. The parser is `game_kit::map`.

use bevy::prelude::*;
use game_kit::camera::CameraBounds;
use game_kit::debug::DebugSettings;
use game_kit::layers::BACKGROUND_Z;
use game_kit::map::{Map, MapObject, Origin, parse};

use crate::camera::CELL_SIZE;

/// The map file R-Town loads. Edit it and rebuild.
const MAP_NAME: &str = "assets/maps/test.txt";
const MAP_SOURCE: &str = include_str!("../assets/maps/test.txt");

/// The legend name of the marker that sets where the hero starts. It has no art.
const PLAYER_START: &str = "player_start";

/// The legend names the parser accepts.
const KIND_NAMES: &[&str] = &[PLAYER_START, "tree_big", "tree_dead"];

/// Placeholder ground colour, `#548058`, until tiled backgrounds exist (roadmap item 5).
const GROUND_COLOR: Color = Color::srgb_u8(0x54, 0x80, 0x58);

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        let (map, start) = load_map();
        info!(
            "map: {} cell {} size {} {} objects, player start {}",
            map.name,
            map.cell_size,
            map.size,
            map.objects.len(),
            start
        );
        let current = CurrentMap(map);
        app.insert_resource(CameraBounds(current.rect()))
            .insert_resource(current)
            .insert_resource(PlayerStart(start))
            .add_systems(Startup, (spawn_ground, number_rows_like_the_file));
    }
}

/// Parses the bundled map. Returns the map without the start marker, and the cell of that marker.
fn load_map() -> (Map, IVec2) {
    let mut map = parse(MAP_NAME, MAP_SOURCE, KIND_NAMES).unwrap_or_else(|error| panic!("{error}"));
    let mut starts = map.objects.iter().filter(|o| o.kind == PLAYER_START);
    let (Some(start), None) = (starts.next().map(|o| o.cell), starts.next()) else {
        panic!("{MAP_NAME}: the grid needs exactly one `{PLAYER_START}` cell");
    };
    map.objects.retain(|o| o.kind != PLAYER_START);
    (map, start)
}

#[derive(Resource)]
pub struct CurrentMap(pub Map);

impl CurrentMap {
    /// The map area in world units. Cell (0, 0) has its bottom left corner at the world origin.
    pub fn rect(&self) -> Rect {
        Rect::new(
            0.0,
            0.0,
            self.0.size.x as f32 * CELL_SIZE,
            self.0.size.y as f32 * CELL_SIZE,
        )
    }
}

/// The cell where the hero starts, from the `player_start` marker in the map file.
#[derive(Resource)]
pub struct PlayerStart(pub IVec2);

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

    /// The kind of a parsed object. The parser only accepts the names in `KIND_NAMES` and `load_map` removes the
    /// start marker, so every object has one.
    pub fn of(object: &MapObject) -> Kind {
        Kind::from_name(&object.kind)
            .unwrap_or_else(|| panic!("map object of unknown kind `{}`", object.kind))
    }
}

/// The F2 grid numbers rows like the map file does: from the top when the file says `origin: top-left`.
fn number_rows_like_the_file(map: Res<CurrentMap>, debug: Option<ResMut<DebugSettings>>) {
    if let Some(mut debug) = debug
        && map.0.origin == Origin::TopLeft
    {
        debug.rows_from_top = Some(map.0.size.y);
    }
}

fn spawn_ground(mut commands: Commands, map: Res<CurrentMap>) {
    let area = map.rect();
    commands.spawn((
        Sprite::from_color(GROUND_COLOR, area.size()),
        Transform::from_translation(area.center().extend(BACKGROUND_Z)),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(map: &Map, kind: Kind) -> usize {
        map.objects.iter().filter(|o| Kind::of(o) == kind).count()
    }

    #[test]
    fn the_bundled_map_parses_and_stays_inside_the_map() {
        let (map, _) = load_map();
        assert_eq!(map.cell_size, CELL_SIZE);
        assert_eq!(map.size, IVec2::new(80, 60));
        assert_eq!(count(&map, Kind::TreeBig), 68);
        assert_eq!(count(&map, Kind::TreeDead), 22);
        for object in &map.objects {
            assert!(
                object.cell.x >= 0
                    && object.cell.y >= 0
                    && object.cell.x < map.size.x
                    && object.cell.y < map.size.y,
                "{:?} is outside the map",
                object
            );
        }
    }

    #[test]
    fn the_player_starts_at_the_marker_and_the_marker_has_no_art() {
        let (map, start) = load_map();
        assert_eq!(start, IVec2::new(40, 30));
        assert!(map.objects.iter().all(|o| o.kind != PLAYER_START));
    }

    #[test]
    fn every_kind_name_has_a_kind() {
        for name in KIND_NAMES.iter().filter(|name| **name != PLAYER_START) {
            assert!(Kind::from_name(name).is_some(), "{name}");
        }
    }

    #[test]
    fn the_map_is_80_by_60_cells() {
        let (map, _) = load_map();
        assert_eq!(CurrentMap(map).rect().size(), Vec2::new(1280.0, 960.0));
    }
}
