//! Checks that the exits of a set of maps lead somewhere. A file can only check itself (`map_areas`). This module
//! checks the files against each other, once, when a game registers its maps. See
//! `dev-docs/extensions/map-transitions.md`.
//!
//! Rules, for every exit:
//! - `to` is the id of a registered map, and `entry` names an entry of that map.
//! - The exit and the entry have the same length, so the position along the strip carries over (every transition).
//! - A seamless exit joins another map: the entry lies along the opposite edge (a top exit joins a bottom entry),
//!   because the maps are placed side by side and never turned.

use crate::map::{Map, MapError};
use crate::map_areas::Transition;

/// A parsed map with the id the game registered it under and the file it came from (for error messages).
pub struct LinkedMap<'a> {
    pub id: &'a str,
    pub file: &'a str,
    pub map: &'a Map,
}

/// Checks every exit of every map. Returns the first problem, with the file and the line of the exit.
pub fn validate_links(maps: &[LinkedMap]) -> Result<(), MapError> {
    for from in maps {
        for exit in &from.map.exits {
            let fail = |message: String| MapError {
                file: from.file.to_string(),
                line: exit.line,
                message,
            };
            let Some(to) = maps.iter().find(|m| m.id == exit.to) else {
                let known: Vec<&str> = maps.iter().map(|m| m.id).collect();
                return Err(fail(format!(
                    "exit `{}` leads to map `{}`, which is not registered (registered: {})",
                    exit.name,
                    exit.to,
                    known.join(", ")
                )));
            };
            let Some(entry) = to.map.entries.iter().find(|e| e.name == exit.entry) else {
                let known: Vec<&str> = to.map.entries.iter().map(|e| e.name.as_str()).collect();
                return Err(fail(format!(
                    "exit `{}` leads to entry `{}`, but map `{}` has no such entry (entries: {})",
                    exit.name,
                    exit.entry,
                    to.id,
                    if known.is_empty() {
                        "none".to_string()
                    } else {
                        known.join(", ")
                    }
                )));
            };
            if exit.area.length() != entry.area.length() {
                return Err(fail(format!(
                    "exit `{}` is {} cells long but entry `{}` of map `{}` is {} cells long, they must match",
                    exit.name,
                    exit.area.length(),
                    entry.name,
                    to.id,
                    entry.area.length()
                )));
            }
            if exit.transition != Transition::Seamless {
                continue;
            }
            if to.id == from.id {
                return Err(fail(format!(
                    "exit `{}` is seamless and leads to its own map, which would lie on top of itself",
                    exit.name
                )));
            }
            // The parser guarantees that an exit lies along exactly one edge.
            let Some(exit_edge) = exit.area.edges(from.map.size).first().copied() else {
                continue;
            };
            let entry_edges = entry.area.edges(to.map.size);
            let wanted = exit_edge.opposite();
            match entry_edges.as_slice() {
                [edge] if *edge == wanted => {}
                [edge] => {
                    return Err(fail(format!(
                        "exit `{}` is seamless on the {} edge, so entry `{}` of map `{}` must be on the {} edge, not the {} edge",
                        exit.name,
                        exit_edge.name(),
                        entry.name,
                        to.id,
                        wanted.name(),
                        edge.name()
                    )));
                }
                [] => {
                    return Err(fail(format!(
                        "exit `{}` is seamless, so entry `{}` of map `{}` must lie along the {} edge",
                        exit.name,
                        entry.name,
                        to.id,
                        wanted.name()
                    )));
                }
                _ => {
                    return Err(fail(format!(
                        "exit `{}` is seamless, but entry `{}` of map `{}` is a single cell in a corner and touches two edges, use a strip along the {} edge",
                        exit.name,
                        entry.name,
                        to.id,
                        wanted.name()
                    )));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::parse;

    /// An empty map of `w` by `h` cells with the given `exits:` and `entries:` blocks.
    fn map(w: i32, h: i32, blocks: &str) -> Map {
        let mut text = format!(
            "---\nmap: t\ncell: 16\norigin: bottom-left\nsize: {w}x{h}\nlegend:\n  \".\": empty\n{blocks}---\n"
        );
        let row = vec!["."; w as usize].join(" ");
        for y in (0..h).rev() {
            text.push_str(&format!("{y:>2} |{row}|\n"));
        }
        parse("t.txt", &text, &[]).unwrap_or_else(|e| panic!("{e}"))
    }

    fn check(maps: &[(&str, &Map)]) -> Result<(), MapError> {
        let linked: Vec<LinkedMap> = maps
            .iter()
            .map(|(id, map)| LinkedMap {
                id,
                file: "m.txt",
                map,
            })
            .collect();
        validate_links(&linked)
    }

    fn problem(maps: &[(&str, &Map)]) -> MapError {
        check(maps).expect_err("expected an error")
    }

    /// An exit on the top edge of a 10 by 10 map that leads to `entry` of map `to`.
    fn top_exit(to: &str, entry: &str, transition: &str) -> Map {
        map(
            10,
            10,
            &format!("exits:\n  x: area=3..6,9 to={to} entry={entry} transition={transition}\n"),
        )
    }

    fn entry_map(area: &str) -> Map {
        map(10, 10, &format!("entries:\n  e: area={area}\n"))
    }

    #[test]
    fn a_chain_of_maps_with_matching_strips_is_fine() {
        let first = top_exit("2", "e", "seamless");
        let second = entry_map("3..6,0");
        check(&[("1", &first), ("2", &second)]).unwrap();
    }

    #[test]
    fn maps_without_exits_always_pass() {
        let plain = map(4, 4, "");
        check(&[("1", &plain)]).unwrap();
        check(&[]).unwrap();
    }

    #[test]
    fn an_unknown_target_map_names_the_exit_and_lists_the_maps() {
        let first = top_exit("9", "e", "scene");
        let second = entry_map("3..6,0");
        let e = problem(&[("1", &first), ("2", &second)]);
        assert_eq!((e.file.as_str(), e.line), ("m.txt", 9));
        assert!(
            e.message.contains("map `9`") && e.message.contains("registered: 1, 2"),
            "{e}"
        );
    }

    #[test]
    fn an_unknown_entry_lists_the_entries_of_the_target() {
        let first = top_exit("2", "nope", "scene");
        let second = entry_map("3..6,0");
        let e = problem(&[("1", &first), ("2", &second)]);
        assert!(
            e.message.contains("no such entry") && e.message.contains("entries: e"),
            "{e}"
        );
        let bare = map(10, 10, "");
        let e = problem(&[("1", &first), ("2", &bare)]);
        assert!(e.message.contains("entries: none"), "{e}");
    }

    #[test]
    fn an_exit_and_its_entry_must_have_the_same_length_for_every_transition() {
        for transition in ["scene", "seamless"] {
            let first = top_exit("2", "e", transition);
            let second = entry_map("3..5,0");
            let e = problem(&[("1", &first), ("2", &second)]);
            assert!(
                e.message.contains("4 cells long") && e.message.contains("3 cells long"),
                "{e}"
            );
        }
    }

    #[test]
    fn a_scene_entry_may_be_anywhere_in_the_map() {
        let first = top_exit("2", "e", "scene");
        let upright = entry_map("2,5..8");
        let inner = entry_map("3..6,5");
        check(&[("1", &first), ("2", &upright)]).unwrap();
        check(&[("1", &first), ("2", &inner)]).unwrap();
    }

    #[test]
    fn a_seamless_exit_needs_the_entry_on_the_opposite_edge() {
        // Each exit edge with its opposite entry, strips of 2 cells.
        let pairs = [
            ("3..4,9", "3..4,0"),
            ("3..4,0", "3..4,9"),
            ("9,3..4", "0,3..4"),
            ("0,3..4", "9,3..4"),
        ];
        for (exit, entry) in pairs {
            let first = map(
                10,
                10,
                &format!("exits:\n  x: area={exit} to=2 entry=e transition=seamless\n"),
            );
            let second = entry_map(entry);
            check(&[("1", &first), ("2", &second)])
                .unwrap_or_else(|e| panic!("{exit} {entry}: {e}"));
        }
    }

    #[test]
    fn a_seamless_entry_on_the_wrong_edge_is_an_error() {
        let first = map(
            10,
            10,
            "exits:\n  x: area=3..4,9 to=2 entry=e transition=seamless\n",
        );
        let same_side = entry_map("3..4,9");
        let e = problem(&[("1", &first), ("2", &same_side)]);
        assert!(
            e.message
                .contains("must be on the bottom edge, not the top edge"),
            "{e}"
        );
        let beside = entry_map("0,3..4");
        let e = problem(&[("1", &first), ("2", &beside)]);
        assert!(e.message.contains("not the left edge"), "{e}");
        let inside = entry_map("3..4,5");
        let e = problem(&[("1", &first), ("2", &inside)]);
        assert!(e.message.contains("must lie along the bottom edge"), "{e}");
    }

    #[test]
    fn a_seamless_single_cell_entry_in_a_corner_is_ambiguous() {
        let first = map(
            10,
            10,
            "exits:\n  x: area=3,9 to=2 entry=e transition=seamless\n",
        );
        let corner = entry_map("0,0");
        let e = problem(&[("1", &first), ("2", &corner)]);
        assert!(e.message.contains("single cell in a corner"), "{e}");
        let edge = entry_map("3,0");
        check(&[("1", &first), ("2", &edge)]).unwrap();
    }

    #[test]
    fn a_seamless_exit_cannot_lead_to_its_own_map() {
        let own = map(
            10,
            10,
            "exits:\n  x: area=3..4,9 to=1 entry=e transition=seamless\nentries:\n  e: area=3..4,0\n",
        );
        let e = problem(&[("1", &own)]);
        assert!(e.message.contains("its own map"), "{e}");
    }

    #[test]
    fn a_scene_exit_may_lead_to_its_own_map() {
        let own = map(
            10,
            10,
            "exits:\n  x: area=3..4,9 to=1 entry=e transition=scene\nentries:\n  e: area=3..4,0\n",
        );
        check(&[("1", &own)]).unwrap();
    }

    #[test]
    fn maps_of_different_sizes_link_by_strip_length() {
        let small = map(
            6,
            4,
            "exits:\n  x: area=1..2,3 to=2 entry=e transition=seamless\n",
        );
        let large = entry_map("3..4,0");
        check(&[("1", &small), ("2", &large)]).unwrap();
        let wide = entry_map("3..5,0");
        check(&[("1", &small), ("2", &wide)]).unwrap_err();
    }
}
