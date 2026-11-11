//! Exits and entries of a map: the `exits:` and `entries:` front matter blocks. See
//! `dev-docs/extensions/map-transitions.md` for the design.
//!
//! ```text
//! exits:
//!   to_2: area=3..6,9 to=2 entry=from_1 transition=scene
//! entries:
//!   from_1: area=3..6,0 facing=up
//! ```
//!
//! An area is a straight strip of cells: `x0..x1,y`, `x,y0..y1` or one cell `x,y`. A range on both axes is an
//! error, so an area can never be a block or a diagonal. Cells are numbered like the row labels of the file, so a
//! `top-left` file counts rows from the top. The parser converts them to cells from the bottom left, like
//! `MapObject.cell`. This module only reads and checks one file. The links between maps are checked elsewhere.

use bevy::math::IVec2;

use crate::map::{Fail, MapError, MapObject, Origin, Params};

/// Legend kind name that marks where the player starts. The games share the name. The kit only needs it to allow
/// the marker inside an entry area.
pub const PLAYER_START: &str = "player_start";

/// A straight strip of cells. `min` and `max` are the first and last cell, both included, numbered from the bottom
/// left with y up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Area {
    pub min: IVec2,
    pub max: IVec2,
}

/// A side of a map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Bottom,
    Top,
}

impl Edge {
    /// The edge a neighbour that is placed beside this one touches: top and bottom, left and right.
    pub fn opposite(self) -> Edge {
        match self {
            Edge::Left => Edge::Right,
            Edge::Right => Edge::Left,
            Edge::Bottom => Edge::Top,
            Edge::Top => Edge::Bottom,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Edge::Left => "left",
            Edge::Right => "right",
            Edge::Bottom => "bottom",
            Edge::Top => "top",
        }
    }
}

impl Area {
    pub fn new(a: IVec2, b: IVec2) -> Self {
        Self {
            min: a.min(b),
            max: a.max(b),
        }
    }

    /// Number of cells in the strip.
    pub fn length(&self) -> i32 {
        (self.max.x - self.min.x + 1).max(self.max.y - self.min.y + 1)
    }

    /// A strip that runs along x. A single cell is not horizontal.
    pub fn is_horizontal(&self) -> bool {
        self.max.x > self.min.x
    }

    pub fn contains(&self, cell: IVec2) -> bool {
        cell.cmpge(self.min).all() && cell.cmple(self.max).all()
    }

    pub fn overlaps(&self, other: &Area) -> bool {
        self.min.cmple(other.max).all() && other.min.cmple(self.max).all()
    }

    /// The edges of a map of `size` cells that the strip lies along. A horizontal strip can lie along the top or
    /// bottom, a vertical strip along the left or right, and a single cell along any edge it touches (two in a
    /// corner).
    pub fn edges(&self, size: IVec2) -> Vec<Edge> {
        let mut edges = Vec::new();
        let horizontal = self.max.x > self.min.x;
        let vertical = self.max.y > self.min.y;
        if !horizontal {
            if self.min.x == 0 {
                edges.push(Edge::Left);
            }
            if self.min.x == size.x - 1 {
                edges.push(Edge::Right);
            }
        }
        if !vertical {
            if self.min.y == 0 {
                edges.push(Edge::Bottom);
            }
            if self.min.y == size.y - 1 {
                edges.push(Edge::Top);
            }
        }
        edges
    }
}

/// How the player gets from one map to the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transition {
    /// The two maps are in the world together and the player walks across.
    Seamless,
    /// The screen fades, the new map loads and the screen fades in.
    Scene,
}

/// The way the player looks after an entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Facing {
    Up,
    Down,
    Left,
    Right,
}

/// A zone on a map edge that sends the player to another map.
#[derive(Clone, Debug, PartialEq)]
pub struct MapExit {
    pub name: String,
    pub area: Area,
    /// Id of the target map. The game registers the maps by id.
    pub to: String,
    /// Name of the entry in the target map.
    pub entry: String,
    pub transition: Transition,
    /// The line of the exit in the file, for later error messages.
    pub line: usize,
}

/// A place where the player can arrive from another map.
#[derive(Clone, Debug, PartialEq)]
pub struct MapEntry {
    pub name: String,
    pub area: Area,
    pub facing: Option<Facing>,
    pub line: usize,
}

/// One line of an `exits:` or `entries:` block as read, before the values are checked.
#[derive(Debug)]
pub(crate) struct RawLink {
    pub name: String,
    pub line: usize,
    pub params: Params,
}

/// Reads `x0..x1,y`, `x,y0..y1` or `x,y` into the first and last cell, with the numbers as written in the file.
pub fn parse_area(text: &str) -> Result<(IVec2, IVec2), String> {
    let bad = || format!("`area` must be `x0..x1,y`, `x,y0..y1` or `x,y`, found `{text}`");
    let (xs, ys) = text.split_once(',').ok_or_else(bad)?;
    let (x0, x1, x_range) = parse_span(xs).ok_or_else(bad)?;
    let (y0, y1, y_range) = parse_span(ys).ok_or_else(bad)?;
    if x_range && y_range {
        return Err(format!(
            "`area` can have a range on one axis only, found `{text}`"
        ));
    }
    if x0 > x1 || y0 > y1 {
        return Err(format!(
            "`area` range must go from low to high, found `{text}`"
        ));
    }
    Ok((IVec2::new(x0, y0), IVec2::new(x1, y1)))
}

/// One number, or `a..b`. The flag says whether it was a range.
fn parse_span(text: &str) -> Option<(i32, i32, bool)> {
    match text.split_once("..") {
        Some((a, b)) => Some((a.parse().ok()?, b.parse().ok()?, true)),
        None => {
            let value = text.parse().ok()?;
            Some((value, value, false))
        }
    }
}

const EXIT_KEYS: &[&str] = &["area", "to", "entry", "transition"];
const ENTRY_KEYS: &[&str] = &["area", "facing"];

/// Checks the keys of one line: all are known and the required ones are there.
fn check_keys(
    link: &RawLink,
    what: &str,
    known: &[&str],
    required: &[&str],
    fail: &Fail,
) -> Result<(), MapError> {
    for (key, _) in link.params.iter() {
        if !known.contains(&key) {
            return Err(fail(
                link.line,
                format!(
                    "unknown parameter `{key}` for {what} `{}` (known: {})",
                    link.name,
                    known.join(", ")
                ),
            ));
        }
    }
    for key in required {
        if link.params.get(key).is_none() {
            return Err(fail(
                link.line,
                format!("{what} `{}` needs `{key}=`", link.name),
            ));
        }
    }
    Ok(())
}

/// Reads the `area` of a line and numbers it from the bottom left. It must lie inside the map.
fn read_area(
    link: &RawLink,
    what: &str,
    origin: Origin,
    size: IVec2,
    fail: &Fail,
) -> Result<Area, MapError> {
    let text = link.params.get("area").unwrap_or("");
    let (first, last) = parse_area(text).map_err(|message| fail(link.line, message))?;
    let (min, max) = match origin {
        Origin::BottomLeft => (first, last),
        Origin::TopLeft => (
            IVec2::new(first.x, size.y - 1 - last.y),
            IVec2::new(last.x, size.y - 1 - first.y),
        ),
    };
    if min.cmplt(IVec2::ZERO).any() || max.cmpge(size).any() {
        return Err(fail(
            link.line,
            format!(
                "{what} `{}` has area `{text}` outside the map ({}x{} cells)",
                link.name, size.x, size.y
            ),
        ));
    }
    Ok(Area { min, max })
}

fn check_name_is_new<'a>(
    link: &RawLink,
    what: &str,
    mut names: impl Iterator<Item = &'a str>,
    fail: &Fail,
) -> Result<(), MapError> {
    if names.any(|name| name == link.name) {
        return Err(fail(
            link.line,
            format!("{what} name `{}` is used twice", link.name),
        ));
    }
    Ok(())
}

pub(crate) fn build_exits(
    raw: &[RawLink],
    origin: Origin,
    size: IVec2,
    fail: &Fail,
) -> Result<Vec<MapExit>, MapError> {
    let mut exits: Vec<MapExit> = Vec::new();
    for link in raw {
        check_name_is_new(link, "exit", exits.iter().map(|e| e.name.as_str()), fail)?;
        check_keys(link, "exit", EXIT_KEYS, EXIT_KEYS, fail)?;
        let area = read_area(link, "exit", origin, size, fail)?;
        match area.edges(size).len() {
            1 => {}
            0 => {
                return Err(fail(
                    link.line,
                    format!("exit `{}` must lie along a map edge", link.name),
                ));
            }
            _ => {
                return Err(fail(
                    link.line,
                    format!(
                        "exit `{}` is a single cell in a corner and touches two edges, use a strip along one edge",
                        link.name
                    ),
                ));
            }
        }
        let transition = match link.params.get("transition").unwrap_or("") {
            "seamless" => Transition::Seamless,
            "scene" => Transition::Scene,
            other => {
                return Err(fail(
                    link.line,
                    format!("`transition` must be `seamless` or `scene`, found `{other}`"),
                ));
            }
        };
        exits.push(MapExit {
            name: link.name.clone(),
            area,
            to: link.params.get("to").unwrap_or("").to_string(),
            entry: link.params.get("entry").unwrap_or("").to_string(),
            transition,
            line: link.line,
        });
    }
    Ok(exits)
}

pub(crate) fn build_entries(
    raw: &[RawLink],
    origin: Origin,
    size: IVec2,
    fail: &Fail,
) -> Result<Vec<MapEntry>, MapError> {
    let mut entries: Vec<MapEntry> = Vec::new();
    for link in raw {
        check_name_is_new(link, "entry", entries.iter().map(|e| e.name.as_str()), fail)?;
        check_keys(link, "entry", ENTRY_KEYS, &["area"], fail)?;
        let area = read_area(link, "entry", origin, size, fail)?;
        let facing = match link.params.get("facing") {
            None => None,
            Some("up") => Some(Facing::Up),
            Some("down") => Some(Facing::Down),
            Some("left") => Some(Facing::Left),
            Some("right") => Some(Facing::Right),
            Some(other) => {
                return Err(fail(
                    link.line,
                    format!("`facing` must be `up`, `down`, `left` or `right`, found `{other}`"),
                ));
            }
        };
        entries.push(MapEntry {
            name: link.name.clone(),
            area,
            facing,
            line: link.line,
        });
    }
    Ok(entries)
}

/// Checks the areas against the rest of the map: no object inside an area (an entry may hold the `player_start`
/// marker), no two exits that overlap, and no entry on an exit, or the player would be sent back at once.
pub(crate) fn check_areas(
    exits: &[MapExit],
    entries: &[MapEntry],
    objects: &[MapObject],
    fail: &Fail,
) -> Result<(), MapError> {
    for exit in exits {
        if let Some(object) = objects.iter().find(|o| exit.area.contains(o.cell)) {
            return Err(fail(
                exit.line,
                format!(
                    "`{}` at cell ({},{}) is inside exit `{}`",
                    object.kind, object.cell.x, object.cell.y, exit.name
                ),
            ));
        }
    }
    for entry in entries {
        let found = objects
            .iter()
            .find(|o| o.kind != PLAYER_START && entry.area.contains(o.cell));
        if let Some(object) = found {
            return Err(fail(
                entry.line,
                format!(
                    "`{}` at cell ({},{}) is inside entry `{}`",
                    object.kind, object.cell.x, object.cell.y, entry.name
                ),
            ));
        }
    }
    for (i, first) in exits.iter().enumerate() {
        if let Some(second) = exits[i + 1..].iter().find(|e| e.area.overlaps(&first.area)) {
            return Err(fail(
                second.line,
                format!("exit `{}` overlaps exit `{}`", second.name, first.name),
            ));
        }
    }
    for entry in entries {
        if let Some(exit) = exits.iter().find(|e| e.area.overlaps(&entry.area)) {
            return Err(fail(
                entry.line,
                format!("entry `{}` overlaps exit `{}`", entry.name, exit.name),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{Map, parse};

    const KINDS: &[&str] = &["player_start", "tree_big"];

    /// A 6 by 4 map. `blocks` goes into the front matter after the legend. Its first line is line 10, so the first
    /// item is line 11. `rows` are the four grid rows, top row first.
    fn source(blocks: &str, rows: [&str; 4]) -> String {
        format!(
            "---\nmap: t\ncell: 16\norigin: bottom-left\nsize: 6x4\nlegend:\n  \".\": empty\n  \"@\": player_start\n  \"g\": tree_big\n{blocks}---\n   |0 1 2 3 4 5|\n 3 |{}|\n 2 |{}|\n 1 |{}|\n 0 |{}|\n",
            rows[0], rows[1], rows[2], rows[3]
        )
    }

    const DOTS: &str = ". . . . . .";

    fn open(blocks: &str) -> String {
        source(blocks, [DOTS; 4])
    }

    fn try_parse(text: &str) -> Result<Map, MapError> {
        parse("t.txt", text, KINDS)
    }

    fn parse_ok(text: &str) -> Map {
        try_parse(text).unwrap_or_else(|e| panic!("{e}"))
    }

    fn error(text: &str) -> MapError {
        try_parse(text).expect_err("expected an error")
    }

    fn cell(x: i32, y: i32) -> IVec2 {
        IVec2::new(x, y)
    }

    #[test]
    fn an_area_is_a_strip_or_a_cell() {
        assert_eq!(parse_area("3..6,9"), Ok((cell(3, 9), cell(6, 9))));
        assert_eq!(parse_area("9,8..9"), Ok((cell(9, 8), cell(9, 9))));
        assert_eq!(parse_area("4,0"), Ok((cell(4, 0), cell(4, 0))));
        assert_eq!(parse_area("4..4,0"), Ok((cell(4, 0), cell(4, 0))));
    }

    #[test]
    fn an_area_with_a_range_on_both_axes_is_an_error() {
        let error = parse_area("1..2,3..4").unwrap_err();
        assert!(error.contains("one axis only"), "{error}");
    }

    #[test]
    fn a_bad_or_reversed_area_is_an_error() {
        assert!(parse_area("6..3,9").unwrap_err().contains("low to high"));
        for text in [
            "", "3", "3,", ",3", "a,b", "3..,4", "1,2,3", "3,9..6,9", "x0..x1,y",
        ] {
            assert!(
                parse_area(text).unwrap_err().contains("`area` must be"),
                "{text}"
            );
        }
    }

    #[test]
    fn a_strip_knows_its_length_direction_and_cells() {
        let strip = Area::new(cell(6, 9), cell(3, 9));
        assert_eq!((strip.min, strip.max), (cell(3, 9), cell(6, 9)));
        assert_eq!(strip.length(), 4);
        assert!(strip.is_horizontal());
        assert!(strip.contains(cell(3, 9)) && strip.contains(cell(6, 9)));
        assert!(!strip.contains(cell(7, 9)) && !strip.contains(cell(4, 8)));
        let single = Area::new(cell(4, 0), cell(4, 0));
        assert_eq!(single.length(), 1);
        assert!(!single.is_horizontal());
    }

    #[test]
    fn areas_overlap_when_they_share_a_cell() {
        let a = Area::new(cell(3, 9), cell(6, 9));
        assert!(a.overlaps(&Area::new(cell(6, 9), cell(6, 9))));
        assert!(!a.overlaps(&Area::new(cell(7, 9), cell(9, 9))));
        assert!(!a.overlaps(&Area::new(cell(3, 8), cell(6, 8))));
    }

    #[test]
    fn a_strip_lies_along_the_edge_it_runs_beside() {
        let size = cell(10, 10);
        let along = |a: IVec2, b: IVec2| Area::new(a, b).edges(size);
        assert_eq!(along(cell(3, 9), cell(6, 9)), vec![Edge::Top]);
        assert_eq!(along(cell(3, 0), cell(6, 0)), vec![Edge::Bottom]);
        // x = 9 runs up the right edge. It reaches the top row but does not lie along the top edge.
        assert_eq!(along(cell(9, 8), cell(9, 9)), vec![Edge::Right]);
        assert_eq!(along(cell(0, 6), cell(0, 7)), vec![Edge::Left]);
        assert_eq!(along(cell(4, 4), cell(4, 6)), vec![]);
        assert_eq!(along(cell(4, 0), cell(4, 0)), vec![Edge::Bottom]);
        assert_eq!(along(cell(9, 9), cell(9, 9)), vec![Edge::Right, Edge::Top]);
    }

    const BOTH: &str = "exits:\n  to_2: area=3..4,3 to=2 entry=from_1 transition=seamless\n  to_3: area=5,1..2 to=3 entry=a transition=scene\nentries:\n  from_1: area=1..2,0 facing=up\n  other: area=0,2\n";

    #[test]
    fn exits_and_entries_are_read() {
        let map = parse_ok(&open(BOTH));
        assert_eq!(map.exits.len(), 2);
        assert_eq!(
            map.exits[0],
            MapExit {
                name: "to_2".into(),
                area: Area::new(cell(3, 3), cell(4, 3)),
                to: "2".into(),
                entry: "from_1".into(),
                transition: Transition::Seamless,
                line: 11,
            }
        );
        assert_eq!(map.exits[1].area, Area::new(cell(5, 1), cell(5, 2)));
        assert_eq!(map.exits[1].transition, Transition::Scene);
        assert_eq!(map.entries.len(), 2);
        assert_eq!(map.entries[0].area, Area::new(cell(1, 0), cell(2, 0)));
        assert_eq!(map.entries[0].facing, Some(Facing::Up));
        assert_eq!(map.entries[1].facing, None);
        for (text, facing) in [
            ("up", Facing::Up),
            ("down", Facing::Down),
            ("left", Facing::Left),
            ("right", Facing::Right),
        ] {
            let map = parse_ok(&open(&format!("entries:\n  e: area=0,1 facing={text}\n")));
            assert_eq!(map.entries[0].facing, Some(facing), "{text}");
        }
    }

    #[test]
    fn the_blocks_do_not_change_the_objects() {
        let plain = parse_ok(&source("", [DOTS, ". g . . . .", DOTS, DOTS]));
        let linked = parse_ok(&source(BOTH, [DOTS, ". g . . . .", DOTS, DOTS]));
        assert_eq!(plain.objects, linked.objects);
    }

    #[test]
    fn a_top_left_file_numbers_areas_like_its_row_labels() {
        let text = "---\nmap: t\ncell: 16\norigin: top-left\nsize: 6x4\nlegend:\n  \".\": empty\nexits:\n  up: area=1..2,0 to=2 entry=a transition=scene\nentries:\n  low: area=4,3\n  side: area=0,0..1\n---\n   |0 1 2 3 4 5|\n 0 |. . . . . .|\n 1 |. . . . . .|\n 2 |. . . . . .|\n 3 |. . . . . .|\n";
        let map = parse_ok(text);
        // Row 0 of this file is the top row, which is y = 3 from the bottom.
        assert_eq!(map.exits[0].area, Area::new(cell(1, 3), cell(2, 3)));
        assert_eq!(map.entries[0].area, Area::new(cell(4, 0), cell(4, 0)));
        // A vertical strip turns over: rows 0..1 from the top are y 2..3, and the low end is still `min`.
        assert_eq!(
            map.entries[1].area,
            Area {
                min: cell(0, 2),
                max: cell(0, 3)
            }
        );
    }

    #[test]
    fn an_exit_must_lie_along_an_edge() {
        let e = error(&open(
            "exits:\n  mid: area=2..3,2 to=2 entry=a transition=scene\n",
        ));
        assert_eq!(e.line, 11);
        assert!(e.message.contains("must lie along a map edge"), "{e}");
        // A vertical strip on the right edge is fine even when it reaches a corner.
        parse_ok(&open(
            "exits:\n  r: area=5,2..3 to=2 entry=a transition=scene\n",
        ));
        // A strip that runs along x on the right column is not along the right edge.
        let e = error(&open(
            "exits:\n  r: area=4..5,2 to=2 entry=a transition=scene\n",
        ));
        assert!(e.message.contains("must lie along a map edge"), "{e}");
    }

    #[test]
    fn a_single_cell_exit_in_a_corner_is_ambiguous() {
        let e = error(&open(
            "exits:\n  c: area=5,3 to=2 entry=a transition=scene\n",
        ));
        assert!(e.message.contains("corner"), "{e}");
    }

    #[test]
    fn an_area_outside_the_map_is_an_error() {
        for area in ["6,1", "0..6,0", "0,4", "-1,2", "0,0..4"] {
            let e = error(&open(&format!("entries:\n  e: area={area}\n")));
            assert_eq!(e.line, 11, "{area}");
            assert!(e.message.contains("outside the map"), "{area}: {e}");
        }
    }

    #[test]
    fn exit_and_entry_lines_are_checked() {
        let cases = [
            (
                "exits:\n  a: area=0,1 to=2 entry=x transition=maybe\n",
                "`transition` must be",
            ),
            (
                "exits:\n  a: area=0,1 to=2 entry=x\n",
                "needs `transition=`",
            ),
            (
                "exits:\n  a: area=0,1 entry=x transition=scene\n",
                "needs `to=`",
            ),
            (
                "exits:\n  a: area=0,1 to=2 transition=scene\n",
                "needs `entry=`",
            ),
            (
                "exits:\n  a: to=2 entry=x transition=scene\n",
                "needs `area=`",
            ),
            (
                "exits:\n  a: area=0,1 to=2 entry=x transition=scene colour=red\n",
                "unknown parameter `colour`",
            ),
            ("entries:\n  a: area=0,1 facing=north\n", "`facing` must be"),
            ("entries:\n  a: facing=up\n", "needs `area=`"),
            ("entries:\n  a: area=0,1 to=2\n", "unknown parameter `to`"),
            (
                "entries:\n  a: area=0,1\n  a: area=0,2\n",
                "entry name `a` is used twice",
            ),
            (
                "exits:\n  a: area=0,1 to=2 entry=x transition=scene\n  a: area=0,2 to=2 entry=x transition=scene\n",
                "exit name `a` is used twice",
            ),
            ("entries:\n  a: area=1..2,3..4\n", "one axis only"),
        ];
        for (blocks, message) in cases {
            let e = error(&open(blocks));
            assert!(e.message.contains(message), "{blocks}: {e}");
            assert!(e.line >= 11, "{blocks}: {e}");
        }
    }

    #[test]
    fn an_entry_and_an_exit_may_share_a_name() {
        parse_ok(&open(
            "exits:\n  a: area=0,1 to=2 entry=x transition=scene\nentries:\n  a: area=3..4,0\n",
        ));
    }

    #[test]
    fn an_object_inside_an_exit_is_an_error() {
        let rows = [DOTS, DOTS, DOTS, ". . . g . ."];
        let e = error(&source(
            "exits:\n  s: area=2..4,0 to=2 entry=a transition=scene\n",
            rows,
        ));
        assert_eq!(e.line, 11);
        assert!(
            e.message
                .contains("`tree_big` at cell (3,0) is inside exit `s`"),
            "{e}"
        );
    }

    #[test]
    fn only_player_start_may_stand_in_an_entry() {
        let blocks = "entries:\n  e: area=2..4,0\n";
        parse_ok(&source(blocks, [DOTS, DOTS, DOTS, ". . . @ . ."]));
        let e = error(&source(blocks, [DOTS, DOTS, DOTS, ". . . g . ."]));
        assert!(e.message.contains("is inside entry `e`"), "{e}");
    }

    #[test]
    fn player_start_in_an_exit_is_an_error() {
        let e = error(&source(
            "exits:\n  s: area=2..4,0 to=2 entry=a transition=scene\n",
            [DOTS, DOTS, DOTS, ". . . @ . ."],
        ));
        assert!(
            e.message.contains("`player_start`") && e.message.contains("inside exit"),
            "{e}"
        );
    }

    #[test]
    fn overlapping_exits_and_an_entry_on_an_exit_are_errors() {
        let e = error(&open(
            "exits:\n  a: area=1..3,0 to=2 entry=x transition=scene\n  b: area=3..4,0 to=2 entry=x transition=scene\n",
        ));
        assert_eq!(e.line, 12);
        assert!(e.message.contains("exit `b` overlaps exit `a`"), "{e}");
        let e = error(&open(
            "exits:\n  a: area=1..3,0 to=2 entry=x transition=scene\nentries:\n  e: area=3,0\n",
        ));
        assert_eq!(e.line, 13);
        assert!(e.message.contains("entry `e` overlaps exit `a`"), "{e}");
    }

    #[test]
    fn the_blocks_take_only_indented_lines() {
        let e = error(&open("exits: a\n"));
        assert!(e.message.contains("takes indented lines"), "{e}");
        let e = error(&open("exits:\n  \"\": area=0,1\n"));
        assert!(e.message.contains("name"), "{e}");
    }
}
