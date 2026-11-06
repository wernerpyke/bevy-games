//! Long Way's map: loads the map file and names the object kinds. The parser is `game_kit::map`. See
//! `games/long-way/docs/maps.md` for the kinds.

use crate::camera::map_rect;
use crate::rocks::FOOTPRINT_CELLS;
use bevy::prelude::*;
use game_kit::camera::CameraBounds;
use game_kit::collision::{CollisionWorld, register_blockers};
use game_kit::groups::{Adjacency, group_sizes, neighbour_groups};
use game_kit::map::{Map, MapObject, parse};
use game_kit::reload::ReloadMap;

/// The map file Long Way loads, relative to the game folder. A native run reads it from disk, so F1 reloads it. The
/// web build has no disk and uses the bundled copy.
const MAP_NAME: &str = "assets/maps/1.txt";
const MAP_SOURCE: &str = include_str!("../assets/maps/1.txt");

fn load_map() -> Result<Map, String> {
    let Some(dir) = std::env::var_os("CARGO_MANIFEST_DIR") else {
        return parse(MAP_NAME, MAP_SOURCE, KIND_NAMES).map_err(|error| error.to_string());
    };
    let path = std::path::Path::new(&dir).join(MAP_NAME);
    let name = path.display().to_string();
    let source = std::fs::read_to_string(&path).map_err(|error| format!("{name}: {error}"))?;
    parse(&name, &source, KIND_NAMES).map_err(|error| error.to_string())
}

/// The legend names the parser accepts.
const KIND_NAMES: &[&str] = &["rock_0", "rock_1", "green_tree", "red_tree"];

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        let map = load_map().unwrap_or_else(|error| panic!("{error}"));
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
        app.insert_resource(CameraBounds(map_rect(map.size)))
            .insert_resource(CurrentMap(map))
            .add_systems(Update, reload_map.run_if(on_message::<ReloadMap>));
    }
}

#[derive(Resource)]
pub struct CurrentMap(pub Map);

/// Marks everything a map spawns (rocks, trees and the player), so a reload removes it.
#[derive(Component)]
pub struct MapEntity;

/// Runs on `ReloadMap` (F1, sent by the kit): reads the map file again, removes everything marked `MapEntity`, spawns
/// the rocks, trees and player again and rebuilds the collision areas. A file with an error is reported and nothing
/// changes.
fn reload_map(world: &mut World) {
    let map = match load_map() {
        Ok(map) => map,
        Err(error) => {
            error!("map: reload failed, keeping the current map: {error}");
            return;
        }
    };
    info!("map: reloaded {}, {} objects", map.name, map.objects.len());
    let old: Vec<Entity> = world
        .query_filtered::<Entity, With<MapEntity>>()
        .iter(world)
        .collect();
    for entity in old {
        world.despawn(entity);
    }
    world.insert_resource(CameraBounds(map_rect(map.size)));
    world.insert_resource(CurrentMap(map));
    world.resource_mut::<CollisionWorld>().clear();
    let _ = world.run_system_cached(crate::rocks::spawn_rocks);
    let _ = world.run_system_cached(crate::trees::spawn_trees);
    let _ = world.run_system_cached(crate::player::spawn_player);
    let _ = world.run_system_cached(register_blockers);
    let _ = world.run_system_cached(crate::rocks::mark_interior);
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

/// The group of each object of the map, in the order of `map.objects`. Every kind of Long Way blocks, so every object
/// counts. Footprints are 2x2 cells, so objects 2 cells apart touch and objects 3 cells apart do not. A new blocking
/// kind must be counted here too.
pub fn blocking_groups(map: &Map) -> Vec<Group> {
    let anchors: Vec<IVec2> = map.objects.iter().map(|o| o.cell).collect();
    let footprint = IVec2::splat(FOOTPRINT_CELLS as i32);
    let sizes = group_sizes(&neighbour_groups(
        &anchors,
        footprint,
        Adjacency::EdgesAndCorners,
    ));
    anchors
        .iter()
        .zip(sizes)
        .map(|(cell, size)| Group {
            size,
            neighbour_above: anchors.iter().any(|other| {
                let d = *other - *cell;
                (1..=footprint.y).contains(&d.y) && d.x.abs() <= footprint.x
            }),
        })
        .collect()
}

/// What an object is. Art, footprint and collision rule for each kind are set in code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Rock0,
    Rock1,
    GreenTree,
    RedTree,
}

impl Kind {
    pub fn is_rock(self) -> bool {
        matches!(self, Kind::Rock0 | Kind::Rock1)
    }

    fn from_name(name: &str) -> Option<Kind> {
        match name {
            "rock_0" => Some(Kind::Rock0),
            "rock_1" => Some(Kind::Rock1),
            "green_tree" => Some(Kind::GreenTree),
            "red_tree" => Some(Kind::RedTree),
            _ => None,
        }
    }

    /// The kind of a parsed object. The parser only accepts the names in `KIND_NAMES`, so every object has one.
    pub fn of(object: &MapObject) -> Kind {
        Kind::from_name(&object.kind)
            .unwrap_or_else(|| panic!("map object of unknown kind `{}`", object.kind))
    }
}
