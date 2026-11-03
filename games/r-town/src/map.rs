//! R-Town's map: loads the map file, names the object kinds and draws the ground. The parser is `game_kit::map`.

use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;
use game_kit::camera::CameraBounds;
use game_kit::collision::{Blocker, CollisionWorld};
use game_kit::debug::DebugSettings;
use game_kit::groups::{Adjacency, group_sizes, neighbour_groups};
use game_kit::layers::BACKGROUND_Z;
use game_kit::map::{Map, MapObject, Origin, parse};

use crate::camera::CELL_SIZE;
use crate::water::Part;

/// The default map file, relative to the game folder. A native run reads it from disk, so F1 reloads it. The web
/// build has no disk and uses the bundled copy.
const MAP_NAME: &str = "assets/maps/test.txt";
const BUNDLED_SOURCE: &str = include_str!("../assets/maps/test.txt");
/// Environment variable with the path of a map file to load instead (`rust.sh run r-town <map>`).
const MAP_VARIABLE: &str = "R_TOWN_MAP";

/// The legend name of the marker that sets where the hero starts. It has no art.
const PLAYER_START: &str = "player_start";

/// The legend names the parser accepts.
const KIND_NAMES: &[&str] = &[
    PLAYER_START,
    "tree_giant",
    "tree_dead",
    "tree_green",
    "tree_dark",
    "water_nw",
    "water_n",
    "water_ne",
    "water_w",
    "water_c",
    "water_e",
    "water_sw",
    "water_s",
    "water_se",
];

/// Placeholder ground colour, `#548058`, until tiled backgrounds exist (roadmap item 5).
const GROUND_COLOR: Color = Color::srgb_u8(0x54, 0x80, 0x58);

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        let (map, start) = load_map().unwrap_or_else(|error| panic!("{error}"));
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
            .add_systems(Startup, (spawn_ground, number_rows_like_the_file))
            .add_systems(Update, reload_map.run_if(input_just_pressed(KeyCode::F1)));
    }
}

/// Marks everything a map spawns, so a reload can remove it.
#[derive(Component)]
pub struct MapEntity;

/// Reads the map file: the path in `R_TOWN_MAP`, else the default file in the game folder, else the bundled copy.
fn read_source() -> Result<(String, String), String> {
    let path = std::env::var_os(MAP_VARIABLE)
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("CARGO_MANIFEST_DIR")
                .map(|dir| std::path::Path::new(&dir).join(MAP_NAME))
        });
    let Some(path) = path else {
        return Ok((MAP_NAME.to_string(), BUNDLED_SOURCE.to_string()));
    };
    let name = path.display().to_string();
    std::fs::read_to_string(&path)
        .map(|source| (name.clone(), source))
        .map_err(|error| format!("{name}: {error}"))
}

/// Parses a map. Returns the map without the start marker, and the cell of that marker.
fn parse_map(name: &str, source: &str) -> Result<(Map, IVec2), String> {
    let mut map = parse(name, source, KIND_NAMES).map_err(|error| error.to_string())?;
    let mut starts = map.objects.iter().filter(|o| o.kind == PLAYER_START);
    let (Some(start), None) = (starts.next().map(|o| o.cell), starts.next()) else {
        return Err(format!(
            "{name}: the grid needs exactly one `{PLAYER_START}` cell"
        ));
    };
    map.objects.retain(|o| o.kind != PLAYER_START);
    Ok((map, start))
}

fn load_map() -> Result<(Map, IVec2), String> {
    let (name, source) = read_source()?;
    parse_map(&name, &source)
}

/// For each object of the map, the number of trees in its group (`adr-rt9`). A size of 1 is a tree that stands alone.
/// Water is not counted and gets 1: it always blocks its whole cell. A new kind with tall art must be counted here.
pub fn blocking_group_sizes(map: &Map) -> Vec<usize> {
    let cells: Vec<IVec2> = map
        .objects
        .iter()
        .filter(|o| Kind::of(o).is_tree())
        .map(|o| o.cell)
        .collect();
    let mut tree_sizes = group_sizes(&neighbour_groups(
        &cells,
        IVec2::ONE,
        Adjacency::EdgesAndCorners,
    ))
    .into_iter();
    map.objects
        .iter()
        .map(|o| {
            if Kind::of(o).is_tree() {
                tree_sizes.next().unwrap_or(1)
            } else {
                1
            }
        })
        .collect()
}

/// F1: reads the map file again and rebuilds the ground, trees, collision, camera limits and row numbers. The hero
/// stays where it is. A file with an error is reported and the current map stays.
fn reload_map(world: &mut World) {
    let (map, start) = match load_map() {
        Ok(loaded) => loaded,
        Err(error) => {
            error!("map: reload failed, keeping the current map: {error}");
            return;
        }
    };
    info!(
        "map: reloaded {} {} objects, player start {}",
        map.name,
        map.objects.len(),
        start
    );
    let old: Vec<Entity> = world
        .query_filtered::<Entity, With<MapEntity>>()
        .iter(world)
        .collect();
    for entity in old {
        world.despawn(entity);
    }
    let current = CurrentMap(map);
    world.insert_resource(CameraBounds(current.rect()));
    world.insert_resource(current);
    world.insert_resource(PlayerStart(start));
    let _ = world.run_system_cached(spawn_ground);
    let _ = world.run_system_cached(number_rows_like_the_file);
    let _ = world.run_system_cached(crate::trees::spawn_trees);
    let _ = world.run_system_cached(crate::water::spawn_water);
    let _ = world.run_system_cached(replace_collision_areas);
}

/// Replaces the collision areas with the blockers of the new map. Runs after the spawn commands are applied.
fn replace_collision_areas(blockers: Query<&Blocker>, mut world: ResMut<CollisionWorld>) {
    world.clear();
    let areas: Vec<Rect> = blockers.iter().map(|blocker| blocker.0).collect();
    world.add_blocked(&areas);
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
    TreeGiant,
    TreeDead,
    TreeGreen,
    TreeDark,
    Water(Part),
}

impl Kind {
    fn from_name(name: &str) -> Option<Kind> {
        match name {
            "tree_giant" => Some(Kind::TreeGiant),
            "tree_dead" => Some(Kind::TreeDead),
            "tree_green" => Some(Kind::TreeGreen),
            "tree_dark" => Some(Kind::TreeDark),
            _ => Part::ALL
                .iter()
                .find(|part| part.name() == name)
                .map(|&part| Kind::Water(part)),
        }
    }

    /// Trees have tall art, so the hero walks behind a lone one. Water fills its cell.
    pub fn is_tree(self) -> bool {
        !matches!(self, Kind::Water(_))
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
        MapEntity,
        Sprite::from_color(GROUND_COLOR, area.size()),
        Transform::from_translation(area.center().extend(BACKGROUND_Z)),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inside(map: &Map, cell: IVec2) -> bool {
        cell.x >= 0 && cell.y >= 0 && cell.x < map.size.x && cell.y < map.size.y
    }

    #[test]
    fn the_bundled_map_parses_and_stays_inside_the_map() {
        let (map, _) = parse_map(MAP_NAME, BUNDLED_SOURCE).unwrap();
        assert_eq!(map.cell_size, CELL_SIZE);
        for object in &map.objects {
            assert!(inside(&map, object.cell), "{object:?} is outside the map");
        }
    }

    #[test]
    fn the_player_starts_inside_the_map_and_the_marker_has_no_art() {
        let (map, start) = parse_map(MAP_NAME, BUNDLED_SOURCE).unwrap();
        assert!(
            inside(&map, start),
            "player start {start} is outside the map"
        );
        assert!(map.objects.iter().all(|o| o.kind != PLAYER_START));
    }

    #[test]
    fn every_kind_name_has_a_kind() {
        for name in KIND_NAMES.iter().filter(|name| **name != PLAYER_START) {
            assert!(Kind::from_name(name).is_some(), "{name}");
        }
    }

    #[test]
    fn the_map_area_is_the_size_in_cells_times_the_cell_size() {
        let (map, _) = parse_map(MAP_NAME, BUNDLED_SOURCE).unwrap();
        let expected = map.size.as_vec2() * CELL_SIZE;
        assert_eq!(CurrentMap(map).rect().size(), expected);
    }
}
