//! Long Way's map: loads the map file and names the object kinds. The parser is `game_kit::map`. See
//! `games/long-way/docs/maps.md` for the kinds.

use crate::rocks::FOOTPRINT_CELLS;
use bevy::prelude::*;
use game_kit::groups::{Adjacency, group_sizes, neighbour_groups};
use game_kit::map::{Map, MapObject, parse};

/// The map file Long Way loads. Edit it and rebuild.
const MAP_NAME: &str = "assets/maps/rocks-test.txt";
const MAP_SOURCE: &str = include_str!("../assets/maps/rocks-test.txt");

/// The legend names the parser accepts.
const KIND_NAMES: &[&str] = &["rock_0", "rock_1", "green_tree", "red_tree"];

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        let map = parse(MAP_NAME, MAP_SOURCE, KIND_NAMES).unwrap_or_else(|error| panic!("{error}"));
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
        app.insert_resource(CurrentMap(map));
    }
}

#[derive(Resource)]
pub struct CurrentMap(pub Map);

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
    #[cfg(test)]
    pub const ALONE: Group = Group {
        size: 1,
        neighbour_above: false,
    };

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

#[cfg(test)]
mod tests {
    use super::*;

    fn cells(map: &Map, kind: Kind) -> Vec<(i32, i32)> {
        let mut found: Vec<_> = map
            .objects
            .iter()
            .filter(|o| Kind::of(o) == kind)
            .map(|o| (o.cell.x, o.cell.y))
            .collect();
        found.sort();
        found
    }

    #[test]
    fn the_bundled_map_parses() {
        let map = parse(MAP_NAME, MAP_SOURCE, KIND_NAMES).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(map.name, "rocks-test");
        assert_eq!(map.cell_size, 64.0);
        assert_eq!(map.size, IVec2::new(32, 18));
        assert_eq!(map.draw_size, None);
        assert_eq!(map.jitter, 0.0);
        // 10 rocks in the first feature, three copies of the 8 rock patch, two single rocks, two trees and a grove of four red trees.
        assert_eq!(map.objects.len(), 42);
        assert_eq!(cells(&map, Kind::GreenTree), vec![(12, 8)]);
        assert_eq!(
            cells(&map, Kind::RedTree),
            vec![(12, 2), (12, 4), (14, 2), (14, 4), (18, 8)]
        );
    }

    #[test]
    fn every_kind_name_has_a_kind() {
        for name in KIND_NAMES {
            assert!(Kind::from_name(name).is_some(), "{name}");
        }
    }

    #[test]
    fn rocks_are_rocks_and_trees_are_not() {
        assert!(Kind::Rock0.is_rock() && Kind::Rock1.is_rock());
        assert!(!Kind::GreenTree.is_rock() && !Kind::RedTree.is_rock());
    }
}
