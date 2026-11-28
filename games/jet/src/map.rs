//! Jet's map: loads the map file and names the object kinds. The parser is `game_kit::map`, the forest is a kit
//! cell area (`forest.rs`, `adr-jt14`).

use bevy::prelude::*;
use game_kit::camera::CameraBounds;
use game_kit::collision::rebuild_blockers;
use game_kit::debug::DebugSettings;
use game_kit::map::{Map, Origin, PLAYER_START, parse, start_feet};
use game_kit::map_transition::{
    MapHooks, MapLifecyclePlugin, MapLoad, MapRegistry, MapSource, game_folder, reload_current_map,
};
use game_kit::reload::ReloadMap;

use crate::camera::{CELL_SIZE, MAP_ORIGIN};

/// The default map file, relative to the game folder. It is read from disk, so F1 reloads it.
const MAP_NAME: &str = "assets/maps/1.txt";
/// Environment variable with the path of a map file to load instead (`rust.sh run jet <map>`).
const MAP_VARIABLE: &str = "JET_MAP";

pub const HOUSE: &str = "house";
pub const FOREST: &str = "forest";

/// The map size in cells: the window shows exactly this (`adr-jt2`).
pub const MAP_CELLS: IVec2 = IVec2::new(60, 32);
/// Columns of side forest on each side of the field (`adr-jt3`).
pub const SIDE_FOREST_COLUMNS: i32 = 10;

/// The legend names the parser accepts.
const KIND_NAMES: &[&str] = &[PLAYER_START, HOUSE, FOREST];

/// The id of Jet's one map in the kit's registry.
const MAP_ID: &str = "field";

/// The kit's map lifecycle runs Jet's map (`MapLifecyclePlugin`): it loads the map in `PostStartup`, runs
/// `load_hook`, adopts what the hook spawns into the map root and rebuilds the collision world. F1 removes the map
/// and everything on it, and loads it again.
pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        // Check the file first, so a bad file stops the start with its line.
        let (name, source) = read_source().unwrap_or_else(|error| panic!("{error}"));
        check(&name, &source).unwrap_or_else(|error| panic!("{error}"));
        let file: &'static str = Box::leak(map_path().into_boxed_str());
        let sources: &'static [MapSource] = Box::leak(Box::new([MapSource { id: MAP_ID, file }]));
        app.add_plugins(MapLifecyclePlugin {
            sources,
            kinds: KIND_NAMES,
            cell_size: CELL_SIZE,
            start: MAP_ID.to_string(),
            origin: MAP_ORIGIN,
        });
        let hooks = MapHooks {
            load: app.world_mut().register_system(load_hook),
            rebuild: app.world_mut().register_system(rebuild_blockers),
            arrive: None,
        };
        app.insert_resource(hooks)
            .insert_resource(CameraBounds(map_rect()))
            .add_systems(Update, reload_map.run_if(on_message::<ReloadMap>));
    }
}

/// The map file: the path in `JET_MAP`, else the default file in the game folder.
fn map_path() -> String {
    std::env::var(MAP_VARIABLE).unwrap_or_else(|_| MAP_NAME.to_string())
}

fn read_source() -> Result<(String, String), String> {
    let path = game_folder().join(map_path());
    let name = path.display().to_string();
    std::fs::read_to_string(&path)
        .map(|source| (name.clone(), source))
        .map_err(|error| format!("{name}: {error}"))
}

/// Parses the map and checks what the game needs: one player start, one house, and a valid forest.
fn check(name: &str, source: &str) -> Result<(), String> {
    let map = parse(name, source, KIND_NAMES, CELL_SIZE).map_err(|error| error.to_string())?;
    if map.size != MAP_CELLS {
        return Err(format!(
            "{name}: the map must be {}x{} cells, the size of the window",
            MAP_CELLS.x, MAP_CELLS.y
        ));
    }
    player_start(name, &map)?;
    house_cell(name, &map)?;
    crate::forest::forest_grid(name, &map).map_err(|error| error.to_string())?;
    Ok(())
}

/// The player's start feet: the centre of the `@` cell. Exactly one is needed.
fn player_start(name: &str, map: &Map) -> Result<Vec2, String> {
    let starts = map
        .objects
        .iter()
        .filter(|o| o.kind == PLAYER_START)
        .count();
    match (starts, start_feet(map, MAP_ORIGIN)) {
        (1, Some(start)) => Ok(start),
        _ => Err(format!(
            "{name}: the grid needs exactly one `{PLAYER_START}` cell"
        )),
    }
}

/// The anchor cell of the house (the bottom left of its footprint). Exactly one is needed.
fn house_cell(name: &str, map: &Map) -> Result<IVec2, String> {
    let mut houses = map.objects.iter().filter(|o| o.kind == HOUSE);
    match (houses.next(), houses.next()) {
        (Some(house), None) => Ok(house.cell),
        _ => Err(format!("{name}: the grid needs exactly one `{HOUSE}` cell")),
    }
}

/// The kit's `load` hook: keeps the map and the house cell as resources, and spawns the house and the forest. The kit
/// adopts them into the map root.
fn load_hook(In(load): In<MapLoad>, world: &mut World) {
    let Some(registered) = world.resource::<MapRegistry>().get(&load.id).cloned() else {
        error!("map: `{}` is not registered", load.id);
        return;
    };
    let name = registered.file.clone();
    let (start, house) = match player_start(&name, &registered.map)
        .and_then(|start| house_cell(&name, &registered.map).map(|house| (start, house)))
    {
        Ok(found) => found,
        Err(error) => {
            error!("map: {error}");
            return;
        }
    };
    info!(
        "map: {} cell {} size {} {} objects, player start {start}, house {house}",
        registered.map.name,
        registered.map.cell_size,
        registered.map.size,
        registered.map.objects.len(),
    );
    world.insert_resource(CurrentMap {
        file: name,
        map: registered.map,
    });
    world.insert_resource(HouseCell(house));
    let _ = world.run_system_cached(number_rows_like_the_file);
    let _ = world.run_system_cached(crate::house::spawn_house);
    let _ = world.run_system_cached(crate::forest::spawn_forest);
    let _ = world.run_system_cached_with(crate::player::spawn_player, start);
    let _ = world.run_system_cached(crate::spawning::spawn_spawner);
}

/// Runs on `ReloadMap` (F1, sent by the kit): checks the file first, then lets the kit load the map again. A file
/// with an error is reported and nothing changes.
fn reload_map(world: &mut World) {
    let checked = read_source()
        .and_then(|(name, source)| check(&name, &source))
        .and_then(|_| reload_current_map(world));
    if let Err(error) = checked {
        error!("map: reload failed, keeping the current map: {error}");
    }
}

/// The F3 grid numbers its rows from the top, like the map file.
fn number_rows_like_the_file(map: Res<CurrentMap>, debug: Option<ResMut<DebugSettings>>) {
    if let Some(mut debug) = debug
        && map.map.origin == Origin::TopLeft
    {
        debug.rows_from_top = Some(map.map.size.y);
    }
}

#[derive(Resource)]
pub struct CurrentMap {
    /// The file the map was read from, for error messages.
    pub file: String,
    pub map: Map,
}

/// The anchor cell of the house: the bottom left cell of its 4 x 4 footprint.
#[derive(Resource)]
pub struct HouseCell(pub IVec2);

/// The world position of the bottom left corner of `cell`.
pub fn cell_corner(cell: IVec2, cell_size: f32) -> Vec2 {
    MAP_ORIGIN + cell.as_vec2() * cell_size
}

/// The whole map in world units.
pub fn map_rect() -> Rect {
    Rect::from_corners(MAP_ORIGIN, MAP_ORIGIN + MAP_CELLS.as_vec2() * CELL_SIZE)
}
