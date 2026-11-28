//! R-Town's map: loads the map file, names the object kinds and draws the ground. The parser is `game_kit::map`.

use bevy::prelude::*;
use game_kit::collision::rebuild_blockers;
use game_kit::debug::DebugSettings;
use game_kit::groups::{Adjacency, group_sizes, neighbour_groups};
use game_kit::layers::BACKGROUND_Z;
use game_kit::map::{Map, MapObject, Origin, PLAYER_START, parse, start_feet};
use game_kit::map_transition::{
    MapHooks, MapLifecyclePlugin, MapLoad, MapRegistry, MapSource, game_folder, reload_current_map,
};
use game_kit::reload::ReloadMap;

use crate::camera::CELL_SIZE;
use crate::enemy::{EnemyKind, SpawnerDef, spawner_def};
use crate::water::Part;

/// The default map file, relative to the game folder. It is read from disk, so F1 reloads it and an edit needs no
/// rebuild (the web build is paused, `adr-e11`).
const MAP_NAME: &str = "assets/maps/test.txt";
/// Environment variable with the path of a map file to load instead (`rust.sh run r-town <map>`).
const MAP_VARIABLE: &str = "R_TOWN_MAP";

/// The legend names of the enemy spawners (`enemy.rs`). They are markers: the objects are removed from the map
/// and become `MapSpawners`.
fn is_spawner(kind: &str) -> bool {
    EnemyKind::SPAWNER_NAMES.contains(&kind)
}

/// The legend names the parser accepts.
const KIND_NAMES: &[&str] = &[
    PLAYER_START,
    "spawn_skeleton",
    "spawn_goblin",
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

/// Placeholder ground colour, `#548058`, until tiled backgrounds exist (`rm5`).
const GROUND_COLOR: Color = Color::srgb_u8(0x54, 0x80, 0x58);

/// The id of R-Town's one map in the kit's registry.
const MAP_ID: &str = "town";

/// The kit's map lifecycle runs R-Town's map (`adr-x58`, `MapLifecyclePlugin`): it loads the map in `PostStartup`,
/// runs `load_hook`, adopts everything the hook spawns into the map root, rebuilds the collision world and sets the camera
/// limits. F1 removes the map, with the player, the enemies and the projectiles in flight (they carry its `MapId`), and
/// loads it again, so the game starts over.
pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        // The spawners are checked here too, so a bad file stops the start with its legend line.
        load_map().unwrap_or_else(|error| panic!("{error}"));
        let file: &'static str = Box::leak(map_path().into_boxed_str());
        let sources: &'static [MapSource] = Box::leak(Box::new([MapSource { id: MAP_ID, file }]));
        app.add_plugins(MapLifecyclePlugin {
            sources,
            kinds: KIND_NAMES,
            cell_size: CELL_SIZE,
            start: MAP_ID.to_string(),
            origin: Vec2::ZERO,
        });
        let hooks = MapHooks {
            load: app.world_mut().register_system(load_hook),
            rebuild: app.world_mut().register_system(rebuild_blockers),
            arrive: None,
        };
        app.insert_resource(hooks)
            .add_systems(Update, reload_map.run_if(on_message::<ReloadMap>));
    }
}

/// The map file: the path in `R_TOWN_MAP`, else the default file in the game folder. An absolute path replaces the game
/// folder when the kit joins them.
fn map_path() -> String {
    std::env::var(MAP_VARIABLE).unwrap_or_else(|_| MAP_NAME.to_string())
}

/// Reads and checks the map file, as the kit will load it.
fn read_source() -> Result<(String, String), String> {
    let path = game_folder().join(map_path());
    let name = path.display().to_string();
    std::fs::read_to_string(&path)
        .map(|source| (name.clone(), source))
        .map_err(|error| format!("{name}: {error}"))
}

/// The kit's `load` hook: takes the markers out of the registered map, keeps the map and its start and spawners as
/// resources, and spawns the ground, the player, the spawners, the trees and the water. The kit adopts them all into
/// the map root.
fn load_hook(In(load): In<MapLoad>, world: &mut World) {
    let Some(registered) = world.resource::<MapRegistry>().get(&load.id).cloned() else {
        error!("map: `{}` is not registered", load.id);
        return;
    };
    let Loaded {
        map,
        start,
        spawners,
    } = match split_markers(&registered.file, registered.map) {
        Ok(loaded) => loaded,
        Err(error) => {
            error!("map: {error}");
            return;
        }
    };
    info!(
        "map: {} cell {} size {} {} objects, player start {}",
        map.name,
        map.cell_size,
        map.size,
        map.objects.len(),
        start
    );
    world.insert_resource(CurrentMap(map));
    world.insert_resource(PlayerStart(start));
    world.insert_resource(MapSpawners(spawners));
    let _ = world.run_system_cached(spawn_ground);
    let _ = world.run_system_cached(number_rows_like_the_file);
    let _ = world.run_system_cached(crate::player::spawn_player);
    let _ = world.run_system_cached(crate::enemy::spawn_spawners);
    let _ = world.run_system_cached(crate::trees::spawn_trees);
    let _ = world.run_system_cached(crate::water::spawn_water);
}

/// A parsed map: the objects without the markers, the player's start feet and the checked spawners.
pub struct Loaded {
    map: Map,
    start: Vec2,
    spawners: Vec<SpawnerDef>,
}

/// Parses a map and takes out its markers (`split_markers`).
fn parse_map(name: &str, source: &str) -> Result<Loaded, String> {
    let map = parse(name, source, KIND_NAMES, CELL_SIZE).map_err(|error| error.to_string())?;
    split_markers(name, map)
}

/// Takes the markers (the start and the spawners) out of the object list of a parsed map, and checks them.
fn split_markers(name: &str, mut map: Map) -> Result<Loaded, String> {
    let starts = map
        .objects
        .iter()
        .filter(|o| o.kind == PLAYER_START)
        .count();
    // Cell (0, 0) has its bottom left corner at the world origin (`CurrentMap::rect`).
    let (1, Some(start)) = (starts, start_feet(&map, Vec2::ZERO)) else {
        return Err(format!(
            "{name}: the grid needs exactly one `{PLAYER_START}` cell"
        ));
    };
    let spawners = map
        .objects
        .iter()
        .filter(|o| is_spawner(&o.kind))
        .map(|o| spawner_def(name, o))
        .collect::<Result<Vec<_>, _>>()?;
    map.objects
        .retain(|o| o.kind != PLAYER_START && !is_spawner(&o.kind));
    Ok(Loaded {
        map,
        start,
        spawners,
    })
}

fn load_map() -> Result<Loaded, String> {
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

/// Runs on `ReloadMap` (F1, sent by the kit): checks the file first, then lets the kit load the map again
/// (`reload_current_map`), which starts the game over. A file with an error is reported and nothing changes.
fn reload_map(world: &mut World) {
    let checked = load_map().and_then(|_| reload_current_map(world));
    if let Err(error) = checked {
        error!("map: reload failed, keeping the current map: {error}");
    }
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

/// The enemy spawners of the map, from the legend entries with a `spawn_` kind.
#[derive(Resource)]
pub struct MapSpawners(pub Vec<SpawnerDef>);

/// Where the player's feet start: the centre of the `player_start` cell in the map file (`map::start_feet`).
#[derive(Resource)]
pub struct PlayerStart(pub Vec2);

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

    /// Trees have tall art, so the player walks behind a lone one. Water fills its cell.
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
        Sprite::from_color(GROUND_COLOR, area.size()),
        Transform::from_translation(area.center().extend(BACKGROUND_Z)),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default map file, read from the game folder.
    fn source() -> String {
        std::fs::read_to_string(game_folder().join(MAP_NAME)).unwrap()
    }

    fn inside(map: &Map, cell: IVec2) -> bool {
        cell.x >= 0 && cell.y >= 0 && cell.x < map.size.x && cell.y < map.size.y
    }

    #[test]
    fn the_default_map_parses_and_stays_inside_the_map() {
        let Loaded { map, .. } = parse_map(MAP_NAME, &source()).unwrap();
        for object in &map.objects {
            assert!(inside(&map, object.cell), "{object:?} is outside the map");
        }
    }

    #[test]
    fn the_player_starts_in_the_middle_of_a_cell_inside_the_map_and_the_marker_has_no_art() {
        let Loaded { map, start, .. } = parse_map(MAP_NAME, &source()).unwrap();
        assert!(map.objects.iter().all(|o| o.kind != PLAYER_START));
        let area = CurrentMap(map).rect();
        assert!(
            area.contains(start),
            "player start {start} is outside {area:?}"
        );
        // Not on a cell corner, where a blocker in the cell to the left reaches into the start.
        assert_eq!(start % CELL_SIZE, Vec2::splat(CELL_SIZE / 2.0));
    }

    #[test]
    fn the_test_map_has_a_skeleton_and_a_goblin_spawner() {
        let Loaded { map, spawners, .. } = parse_map(MAP_NAME, &source()).unwrap();
        assert!(map.objects.iter().all(|o| !is_spawner(&o.kind)));
        let kinds: Vec<EnemyKind> = spawners.iter().map(|s| s.kind).collect();
        assert_eq!(kinds, [EnemyKind::Goblin, EnemyKind::Skeleton]);
    }

    #[test]
    fn a_bad_spawner_entry_stops_the_load_with_its_legend_line() {
        let source = source();
        let wrong = source.replace("strategy=pursue", "strategy=evade");
        let error = parse_map("test.txt", &wrong).err().unwrap();
        assert!(error.contains("unknown strategy `evade`"), "{error}");
        let line = source
            .lines()
            .position(|l| l.contains("spawn_goblin"))
            .unwrap()
            + 1;
        assert!(error.starts_with(&format!("test.txt:{line}:")), "{error}");
        let no_radius = source.replace(" radius=120", "");
        let error = parse_map("test.txt", &no_radius).err().unwrap();
        assert!(error.contains("radius"), "{error}");
    }

    #[test]
    fn every_kind_name_has_a_kind() {
        for name in KIND_NAMES
            .iter()
            .filter(|name| **name != PLAYER_START && !is_spawner(name))
        {
            assert!(Kind::from_name(name).is_some(), "{name}");
        }
    }

    #[test]
    fn the_map_area_is_the_size_in_cells_times_the_cell_size() {
        let Loaded { map, .. } = parse_map(MAP_NAME, &source()).unwrap();
        let expected = map.size.as_vec2() * CELL_SIZE;
        assert_eq!(CurrentMap(map).rect().size(), expected);
    }
}
