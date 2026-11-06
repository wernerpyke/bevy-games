//! Long Way's maps: registers the map files and names the object kinds. The parser, the registry and the moves between
//! maps are in `game_kit::map` and `game_kit::map_transition`. See `games/long-way/docs/maps.md` for the kinds.

use crate::camera::MAP_ORIGIN;
use crate::rocks::FOOTPRINT_CELLS;
use bevy::prelude::*;
use game_kit::groups::{Adjacency, group_sizes, neighbour_groups};
use game_kit::map::{Map, MapObject, PLAYER_START};
use game_kit::map_transition::{
    Fade, MapHooks, MapLoad, MapRegistry, MapSource, MapTransitionPlugin, reload_current_map,
};
use game_kit::reload::ReloadMap;

/// Every map of the game. A native run reads the files from disk, so F1 reloads the current one. The web build has no
/// disk and uses the bundled copies.
const MAPS: &[MapSource] = &[
    MapSource {
        id: "1",
        file: "assets/maps/1.txt",
        bundled: include_str!("../assets/maps/1.txt"),
    },
    MapSource {
        id: "2",
        file: "assets/maps/2.txt",
        bundled: include_str!("../assets/maps/2.txt"),
    },
    MapSource {
        id: "3",
        file: "assets/maps/3.txt",
        bundled: include_str!("../assets/maps/3.txt"),
    },
];

/// The legend names the parser accepts.
const KIND_NAMES: &[&str] = &["rock_0", "rock_1", "green_tree", "red_tree", PLAYER_START];

/// The map the game starts in: the id from the file name in `LONG_WAY_MAP` (`rust.sh run long-way 2.txt` sets it), else
/// map 1. A map that is not the first needs a `player_start` marker to start in.
fn start_map() -> String {
    std::env::var("LONG_WAY_MAP")
        .ok()
        .and_then(|path| {
            std::path::Path::new(&path)
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "1".to_string())
}

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MapTransitionPlugin {
            sources: MAPS,
            kinds: KIND_NAMES,
            start: start_map(),
            origin: MAP_ORIGIN,
            fade: Fade::default(),
        });
        let hooks = MapHooks {
            load: app.world_mut().register_system(load_hook),
            rebuild: app
                .world_mut()
                .register_system(crate::rocks::rebuild_collision),
            arrive: app
                .world_mut()
                .register_system(crate::player::arrive_player),
        };
        app.insert_resource(hooks)
            .add_systems(Update, reload_map.run_if(on_message::<ReloadMap>));
    }
}

/// Spawns the rocks and trees of one map. Everything is tagged with the map id, so the kit removes it with the map.
fn load_hook(
    In(load): In<MapLoad>,
    mut commands: Commands,
    server: Res<AssetServer>,
    registry: Res<MapRegistry>,
) {
    let Some(registered) = registry.get(&load.id) else {
        error!("map: `{}` is not registered", load.id);
        return;
    };
    let map = &registered.map;
    info!(
        "map: {} cell {} size {} draw {:?} jitter {}, {} objects",
        map.name,
        map.cell_size,
        map.size,
        map.draw_size,
        map.jitter,
        map.objects.len()
    );
    for object in &map.objects {
        debug!("map: {} at cell {}", object.kind, object.cell);
    }
    crate::rocks::spawn_rocks(&mut commands, &server, map, &load);
    crate::trees::spawn_trees(&mut commands, &server, map, &load);
}

/// Runs on `ReloadMap` (F1, sent by the kit): reads the current map file again and loads it again. A file with an
/// error is reported and nothing changes.
fn reload_map(world: &mut World) {
    if let Err(error) = reload_current_map(world) {
        error!("map: reload failed, keeping the current map: {error}");
    }
}

/// How a blocking object relates to its neighbours (`adr-lw14`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Group {
    /// Number of blocking objects in its group. 1 is an object that stands alone.
    pub size: usize,
    /// Another object of the group has a footprint that overlaps or touches the top of this one. The blocked area
    /// then reaches the top of the footprint, so the two shapes merge.
    pub neighbour_above: bool,
}

impl Group {
    pub fn is_group(self) -> bool {
        self.size > 1
    }
}

/// The group of each object of the map, in the order of `map.objects`. Rocks and trees block, so they count. Markers
/// (the player start) block nothing and are not counted: they get a group of 1 that nobody reads. Footprints are 2x2
/// cells, so objects 2 cells apart touch and objects 3 cells apart do not. A new blocking kind must be counted here
/// too.
pub fn blocking_groups(map: &Map) -> Vec<Group> {
    let blocking: Vec<usize> = (0..map.objects.len())
        .filter(|&i| !Kind::of(&map.objects[i]).is_marker())
        .collect();
    let anchors: Vec<IVec2> = blocking.iter().map(|&i| map.objects[i].cell).collect();
    let footprint = IVec2::splat(FOOTPRINT_CELLS as i32);
    let sizes = group_sizes(&neighbour_groups(
        &anchors,
        footprint,
        Adjacency::EdgesAndCorners,
    ));
    let mut groups = vec![
        Group {
            size: 1,
            neighbour_above: false
        };
        map.objects.len()
    ];
    for ((&index, cell), size) in blocking.iter().zip(&anchors).zip(sizes) {
        groups[index] = Group {
            size,
            neighbour_above: anchors.iter().any(|other| {
                let d = *other - *cell;
                (1..=footprint.y).contains(&d.y) && d.x.abs() <= footprint.x
            }),
        };
    }
    groups
}

/// World position of the player's feet at the `player_start` marker: the centre of its cell. `None` when the map has no
/// marker. A second marker is reported and ignored.
pub fn player_start_feet(map: &Map, origin: Vec2) -> Option<Vec2> {
    let mut starts = map
        .objects
        .iter()
        .filter(|o| Kind::of(o) == Kind::PlayerStart);
    let start = starts.next()?;
    if starts.next().is_some() {
        warn!(
            "map {}: more than one player_start, using the first",
            map.name
        );
    }
    Some(origin + (start.cell.as_vec2() + Vec2::splat(0.5)) * map.cell_size)
}

/// What an object is. Art, footprint and collision rule for each kind are set in code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Rock0,
    Rock1,
    GreenTree,
    RedTree,
    /// Where the player starts. It has no art and blocks nothing.
    PlayerStart,
}

impl Kind {
    pub fn is_rock(self) -> bool {
        matches!(self, Kind::Rock0 | Kind::Rock1)
    }

    /// A marker is read by the game and never drawn or blocked.
    pub fn is_marker(self) -> bool {
        matches!(self, Kind::PlayerStart)
    }

    fn from_name(name: &str) -> Option<Kind> {
        match name {
            "rock_0" => Some(Kind::Rock0),
            "rock_1" => Some(Kind::Rock1),
            "green_tree" => Some(Kind::GreenTree),
            "red_tree" => Some(Kind::RedTree),
            PLAYER_START => Some(Kind::PlayerStart),
            _ => None,
        }
    }

    /// The kind of a parsed object. The parser only accepts the names in `KIND_NAMES`, so every object has one.
    pub fn of(object: &MapObject) -> Kind {
        Kind::from_name(&object.kind)
            .unwrap_or_else(|| panic!("map object of unknown kind `{}`", object.kind))
    }
}
