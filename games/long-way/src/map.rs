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
const KIND_NAMES: &[&str] = &[
    "rock_0",
    "rock_1",
    "green_tree",
    "red_tree",
    "lantern",
    "lantern_red",
    "marker",
    "marker_red",
    "green_tree_medium",
    "red_tree_medium",
    "red_tree_small",
    "green_shrub",
    "green_shrub_small",
    "energy_blue",
    "energy_red",
    "energy_violet",
    "energy_black",
    "spawn_ghost",
    PLAYER_START,
];

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
            camera_ease_secs: crate::camera::CAMERA_EASE_SECS,
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
            .add_systems(Startup, crate::background::check_tiles)
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
        "map: {} cell {} size {} draw {:?} jitter {}, {} objects, background {}",
        map.name,
        map.cell_size,
        map.size,
        map.draw_size,
        map.jitter,
        map.objects.len(),
        map.background
            .as_ref()
            .map_or("none".to_string(), |b| format!(
                "{} fill {} jitter {} scale {}",
                b.color.to_hex(),
                b.fill,
                b.jitter,
                b.scale
            ))
    );
    for object in &map.objects {
        debug!("map: {} at cell {}", object.kind, object.cell);
    }
    crate::background::spawn_background(&mut commands, &server, map, &load);
    crate::rocks::spawn_rocks(&mut commands, &server, map, &load);
    crate::trees::spawn_trees(&mut commands, &server, map, &load);
    crate::markers::spawn_markers(&mut commands, &server, map, &load);
    crate::scenery::spawn_scenery(&mut commands, &server, map, &load);
    crate::pickups::spawn_pickups(&mut commands, &server, map, &load);
    crate::ghost::spawn_ghosts(&mut commands, &server, map, &load);
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

/// The group of each object of the map, in the order of `map.objects`. Rocks, trees and the Zen Garden scenery block, so they
/// count. The player start, lanterns and markers are not counted: they get a group of 1 that nobody reads. Each kind has its
/// own footprint (`Kind::footprint`). Every object is expanded into the cells of its footprint and the kit groups the cells
/// (size 1), so two objects are linked when their footprints overlap or touch, whatever their sizes. Two 2x2 objects 2 cells
/// apart touch and 3 cells apart do not. A new blocking kind must be counted here too.
pub fn blocking_groups(map: &Map) -> Vec<Group> {
    let blocking: Vec<usize> = (0..map.objects.len())
        .filter(|&i| {
            let kind = Kind::of(&map.objects[i]);
            !kind.is_marker() && !kind.is_small() && !kind.is_pickup()
        })
        .collect();
    let boxes: Vec<(IVec2, IVec2)> = blocking
        .iter()
        .map(|&i| (map.objects[i].cell, Kind::of(&map.objects[i]).footprint()))
        .collect();
    let mut cells = Vec::new();
    let mut first_cell = Vec::with_capacity(boxes.len());
    for &(anchor, size) in &boxes {
        first_cell.push(cells.len());
        for dy in 0..size.y {
            for dx in 0..size.x {
                cells.push(anchor + IVec2::new(dx, dy));
            }
        }
    }
    // The cells of one object touch, so they share a group id. That id is the group of the object.
    let cell_groups = neighbour_groups(&cells, IVec2::ONE, Adjacency::EdgesAndCorners);
    let item_groups: Vec<usize> = first_cell.iter().map(|&c| cell_groups[c]).collect();
    let sizes = group_sizes(&item_groups);
    let mut groups = vec![
        Group {
            size: 1,
            neighbour_above: false
        };
        map.objects.len()
    ];
    for ((&index, &(anchor, size)), group_size) in blocking.iter().zip(&boxes).zip(sizes) {
        groups[index] = Group {
            size: group_size,
            neighbour_above: boxes
                .iter()
                .any(|&(other, other_size)| touches_top(anchor, size, other, other_size)),
        };
    }
    groups
}

/// Whether the footprint of `other` overlaps or touches the top row of the footprint at `anchor` (corners count): its lowest
/// row is the top row of the footprint or the row above it, and its columns reach the footprint or touch it.
fn touches_top(anchor: IVec2, size: IVec2, other: IVec2, other_size: IVec2) -> bool {
    let d = other - anchor;
    (size.y - 1..=size.y).contains(&d.y) && d.x <= size.x && d.x + other_size.x >= 0
}

/// What an object is. Art, footprint and collision rule for each kind are set in code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Rock0,
    Rock1,
    GreenTree,
    RedTree,
    Lantern,
    LanternRed,
    Marker,
    MarkerRed,
    /// Zen Garden trees and shrubs (`scenery.rs`). They block like a tree trunk and join the groups (`adr-lw23`).
    GreenTreeMedium,
    RedTreeMedium,
    RedTreeSmall,
    GreenShrub,
    GreenShrubSmall,
    /// Energy pickups (`pickups.rs`). They do not block and are not part of a group.
    EnergyBlue,
    EnergyRed,
    EnergyViolet,
    EnergyBlack,
    /// Where a ghost is placed when the map loads (`ghost.rs`). It has no art of its own and blocks nothing.
    SpawnGhost,
    /// Where the player starts. It has no art and blocks nothing.
    PlayerStart,
}

impl Kind {
    pub fn is_rock(self) -> bool {
        matches!(self, Kind::Rock0 | Kind::Rock1)
    }

    /// Lanterns and markers are small, block on their own and are not part of a group of 2x2 footprints.
    pub fn is_small(self) -> bool {
        matches!(
            self,
            Kind::Lantern | Kind::LanternRed | Kind::Marker | Kind::MarkerRed
        )
    }

    /// The cells a kind covers, in x and y, from its anchor cell up and to the right. Grouping uses it (`blocking_groups`).
    pub fn footprint(self) -> IVec2 {
        match self {
            Kind::RedTreeMedium => IVec2::new(1, 2),
            Kind::RedTreeSmall | Kind::GreenShrub | Kind::GreenShrubSmall => IVec2::ONE,
            _ => IVec2::splat(FOOTPRINT_CELLS as i32),
        }
    }

    /// A pickup is collected by touch and never blocks.
    pub fn is_pickup(self) -> bool {
        matches!(
            self,
            Kind::EnergyBlue | Kind::EnergyRed | Kind::EnergyViolet | Kind::EnergyBlack
        )
    }

    /// A marker is read by the game and never drawn or blocked.
    pub fn is_marker(self) -> bool {
        matches!(self, Kind::PlayerStart | Kind::SpawnGhost)
    }

    fn from_name(name: &str) -> Option<Kind> {
        match name {
            "rock_0" => Some(Kind::Rock0),
            "rock_1" => Some(Kind::Rock1),
            "green_tree" => Some(Kind::GreenTree),
            "red_tree" => Some(Kind::RedTree),
            "lantern" => Some(Kind::Lantern),
            "lantern_red" => Some(Kind::LanternRed),
            "marker" => Some(Kind::Marker),
            "marker_red" => Some(Kind::MarkerRed),
            "green_tree_medium" => Some(Kind::GreenTreeMedium),
            "red_tree_medium" => Some(Kind::RedTreeMedium),
            "red_tree_small" => Some(Kind::RedTreeSmall),
            "green_shrub" => Some(Kind::GreenShrub),
            "green_shrub_small" => Some(Kind::GreenShrubSmall),
            "energy_blue" => Some(Kind::EnergyBlue),
            "energy_red" => Some(Kind::EnergyRed),
            "energy_violet" => Some(Kind::EnergyViolet),
            "energy_black" => Some(Kind::EnergyBlack),
            "spawn_ghost" => Some(Kind::SpawnGhost),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A map of 8 by 6 cells with `objects` as (legend character, x, y), cell (0, 0) at the bottom left.
    fn map_with(objects: &[(char, i32, i32)]) -> Map {
        let mut text = String::from(
            "---\nmap: t\ncell: 64\norigin: bottom-left\nsize: 8x6\nlegend:\n  \".\": empty\n  \"R\": rock_0\n  \"G\": green_tree\n  \"s\": green_shrub\n  \"t\": red_tree_medium\n---\n",
        );
        for y in (0..6).rev() {
            let row: Vec<String> = (0..8)
                .map(|x| {
                    objects
                        .iter()
                        .find(|o| o.1 == x && o.2 == y)
                        .map_or('.', |o| o.0)
                        .to_string()
                })
                .collect();
            text.push_str(&format!(" {y} |{}|\n", row.join(" ")));
        }
        parse("test.txt", &text, KIND_NAMES).unwrap()
    }

    fn group_at(map: &Map, groups: &[Group], x: i32, y: i32) -> Group {
        let i = map
            .objects
            .iter()
            .position(|o| o.cell == IVec2::new(x, y))
            .unwrap();
        groups[i]
    }

    fn size_at(map: &Map, groups: &[Group], x: i32, y: i32) -> usize {
        group_at(map, groups, x, y).size
    }

    use game_kit::map::parse;

    #[test]
    fn big_objects_group_when_two_cells_apart_and_not_when_three() {
        let map = map_with(&[('G', 0, 0), ('R', 2, 0), ('G', 5, 0)]);
        let groups = blocking_groups(&map);
        assert_eq!(size_at(&map, &groups, 0, 0), 2);
        assert_eq!(size_at(&map, &groups, 2, 0), 2);
        assert_eq!(size_at(&map, &groups, 5, 0), 1);
    }

    #[test]
    fn one_cell_shrubs_group_only_when_they_touch() {
        let apart = map_with(&[('s', 0, 0), ('s', 2, 0)]);
        let groups = blocking_groups(&apart);
        assert_eq!(size_at(&apart, &groups, 0, 0), 1);
        let touching = map_with(&[('s', 0, 0), ('s', 1, 1)]);
        let groups = blocking_groups(&touching);
        assert_eq!(size_at(&touching, &groups, 0, 0), 2);
    }

    #[test]
    fn kinds_of_different_sizes_form_one_group() {
        // The tree covers cells x 0..2, y 0..2. The shrub at (2, 2) touches its top right corner. The tall red tree
        // covers x 3, y 1..3 and touches the shrub.
        let map = map_with(&[('G', 0, 0), ('s', 2, 2), ('t', 3, 3), ('s', 6, 0)]);
        let groups = blocking_groups(&map);
        assert_eq!(size_at(&map, &groups, 0, 0), 3);
        assert_eq!(size_at(&map, &groups, 3, 3), 3);
        assert_eq!(size_at(&map, &groups, 6, 0), 1);
    }

    #[test]
    fn every_legend_name_has_its_own_kind_and_the_energy_names_give_every_energy() {
        // `Kind::of` panics on a name the parser accepts and `from_name` does not know.
        use crate::inventory::Energy;
        let mut kinds = Vec::new();
        let mut energies = Vec::new();
        for &name in KIND_NAMES {
            let kind = Kind::from_name(name).unwrap_or_else(|| panic!("`{name}` has no kind"));
            assert!(!kinds.contains(&kind), "`{name}` repeats {kind:?}");
            kinds.push(kind);
            let energy = Energy::from_kind(name);
            assert_eq!(energy.is_some(), kind.is_pickup(), "{name}");
            energies.extend(energy);
        }
        assert_eq!(energies, Energy::ALL);
    }

    #[test]
    fn a_blocking_object_knows_when_another_stands_on_its_top() {
        // The rock at (2, 2) touches the top of the rock at (1, 0), not the other way round. The tree at (6, 0) is
        // alone.
        let map = map_with(&[('R', 1, 0), ('R', 2, 2), ('G', 6, 0)]);
        let groups = blocking_groups(&map);
        let at = |x, y| group_at(&map, &groups, x, y);
        let group = |size, neighbour_above| Group {
            size,
            neighbour_above,
        };
        assert_eq!(at(1, 0), group(2, true));
        assert_eq!(at(2, 2), group(2, false));
        assert_eq!(at(6, 0), group(1, false));
    }

    #[test]
    fn the_top_of_a_footprint_is_touched_from_above_and_from_the_side_corners() {
        let two = IVec2::splat(2);
        let at = |x, y| touches_top(IVec2::ZERO, two, IVec2::new(x, y), two);
        assert!(at(0, 1) && at(1, 2) && at(-2, 2) && at(2, 2));
        assert!(!at(0, 3) && !at(3, 2) && !at(-3, 2) && !at(0, 0));
        // A 1 by 2 footprint covers rows 0 and 1, so the row above it is 2.
        assert!(touches_top(
            IVec2::ZERO,
            IVec2::new(1, 2),
            IVec2::new(0, 2),
            IVec2::ONE
        ));
        assert!(!touches_top(
            IVec2::ZERO,
            IVec2::new(1, 2),
            IVec2::new(0, 3),
            IVec2::ONE
        ));
    }
}
