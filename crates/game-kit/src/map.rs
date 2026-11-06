//! Map files: front matter and a drawn grid, one character per cell. See `dev-docs/extensions/map-files.md` for the
//! format. The parser is plain Rust and has no Bevy dependency except `IVec2`, so it is unit tested without a
//! running app. Object kinds are names (adr-x13): the game passes the names it knows, and converts the names of
//! the parsed objects to its own type.
//!
//! Supported front matter (a small subset of YAML, one `key: value` per line):
//! - `map`: name used in log and error messages. Required.
//! - `cell`: cell size in pixels. Required.
//! - `origin`: `bottom-left` or `top-left`, the corner where row labels start at 0. Required.
//! - `size`: width and height in cells, written `WxH`. Required.
//! - `draw`: drawn size of an object in pixels. Optional, the game gives the default.
//! - `jitter`: largest random offset in pixels. Default 0.
//! - `legend`: indented `char: kind` lines. A kind may be followed by `key=value` parameters, for example
//!   `"S": spawn_skeleton every=10 radius=120` (`Params`). The game reads them by name. `.` is always empty. Quote the character
//!   (for example `"#"`), because an unquoted `#` starts a comment.
//!
//! - `exits` and `entries`: indented `name: key=value ...` lines for the zones that lead to other maps (`map_areas`).
//!   Both are optional.
//!
//! Grid: rows of the form `<label>|<cells>|`. Cell k is the character at offset 2k between the bars, and a
//! blank or `.` is empty. Ruler lines, blank lines and `#` comment lines are ignored.
//!
//! Comments in the front matter start with `#` at the start of a line or after a space.

use std::collections::HashMap;
use std::fmt;

use bevy::math::IVec2;

use crate::map_areas::{self, RawLink};
pub use crate::map_areas::{Area, Edge, Facing, MapEntry, MapExit, PLAYER_START, Transition};

/// One painted character: an object whose bottom left corner is at `cell`.
#[derive(Clone, Debug, PartialEq)]
pub struct MapObject {
    /// The legend name of the object, for example `rock_0`.
    pub kind: String,
    pub cell: IVec2,
    /// The `key=value` parameters that follow the kind in the legend. Empty for most objects.
    pub params: Params,
}

/// Parameters written after a kind name in the legend, in file order. Values are text, and the game says what each
/// key means.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Params {
    pairs: Vec<(String, String)>,
    line: usize,
}

impl Params {
    pub fn from_pairs(pairs: &[(&str, &str)]) -> Self {
        Self {
            pairs: pairs
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
            line: 0,
        }
    }

    /// The line of the legend entry in the file, 0 when the parameters were not read from a file.
    pub fn line(&self) -> usize {
        self.line
    }

    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.pairs
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }

    /// The text of a parameter, if the legend entry has it.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| value)
    }

    /// A parameter as a number. `Ok(None)` when it is absent. An error message when it is not a number.
    pub fn f32(&self, key: &str) -> Result<Option<f32>, String> {
        match self.get(key) {
            None => Ok(None),
            Some(text) => text
                .parse::<f32>()
                .ok()
                .filter(|number| number.is_finite())
                .map(Some)
                .ok_or_else(|| format!("parameter `{key}` must be a number, found `{text}`")),
        }
    }
}

/// The corner of the grid where the row labels start at 0. It only changes how a file is written. Cells in
/// `MapObject` are always numbered from the bottom left, with y up, whichever origin the file uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    BottomLeft,
    TopLeft,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Map {
    pub name: String,
    pub cell_size: f32,
    /// The `origin` key: how the file numbers its rows.
    pub origin: Origin,
    /// Width and height in cells, from the `size` key.
    pub size: IVec2,
    /// The `draw` key, if the file has one.
    pub draw_size: Option<f32>,
    pub jitter: f32,
    pub objects: Vec<MapObject>,
    /// Zones that lead to other maps, from the `exits:` block. Empty when the file has none.
    pub exits: Vec<MapExit>,
    /// Places where the player can arrive, from the `entries:` block. Empty when the file has none.
    pub entries: Vec<MapEntry>,
}

/// A problem in a map file, with the 1-based line it was found on.
#[derive(Debug, PartialEq)]
pub struct MapError {
    pub file: String,
    pub line: usize,
    pub message: String,
}

impl fmt::Display for MapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.file, self.line, self.message)
    }
}

impl std::error::Error for MapError {}

/// Frontmatter values that have been read so far.
#[derive(Default)]
struct Header {
    name: Option<String>,
    cell_size: Option<f32>,
    origin: Option<Origin>,
    size: Option<IVec2>,
    draw_size: Option<f32>,
    jitter: Option<f32>,
    legend: HashMap<char, (String, Params)>,
    exits: Vec<RawLink>,
    entries: Vec<RawLink>,
}

/// The nested block that the indented lines belong to.
#[derive(Clone, Copy, PartialEq)]
enum Block {
    None,
    Legend,
    Exits,
    Entries,
}

/// Reads the frontmatter. Returns the index of the closing `---` line, the header, the map name and the cell size.
fn front_matter(
    lines: &[&str],
    kinds: &[&str],
    fail: &Fail,
) -> Result<(usize, Header, String, f32), MapError> {
    if lines.first().map(|l| l.trim()) != Some("---") {
        return Err(fail(
            1,
            "the file must start with a `---` frontmatter line".into(),
        ));
    }
    let Some(close) = lines.iter().skip(1).position(|l| l.trim() == "---") else {
        return Err(fail(1, "frontmatter is not closed by a `---` line".into()));
    };
    let close = close + 1;

    let header = parse_header(&lines[1..close], 2, kinds, fail)?;
    let name = header
        .name
        .clone()
        .ok_or_else(|| fail(1, "missing required key `map`".into()))?;
    let cell_size = header
        .cell_size
        .ok_or_else(|| fail(1, "missing required key `cell`".into()))?;
    if header.origin.is_none() {
        return Err(fail(
            1,
            "missing required key `origin` (`bottom-left` or `top-left`)".into(),
        ));
    }
    Ok((close, header, name, cell_size))
}

/// Parses a text map file: frontmatter, then a drawn grid (`dev-docs/extensions/map-files.md`). `file` is only
/// used in error messages. `kinds` are the legend names the game knows.
pub fn parse(file: &str, source: &str, kinds: &[&str]) -> Result<Map, MapError> {
    let fail = |line: usize, message: String| MapError {
        file: file.to_string(),
        line,
        message,
    };
    let lines: Vec<&str> = source.lines().collect();
    let (close, header, name, cell_size) = front_matter(&lines, kinds, &fail)?;
    let size = header
        .size
        .ok_or_else(|| fail(1, "missing required key `size`".into()))?;

    let origin = header.origin.unwrap_or(Origin::BottomLeft);
    let objects = parse_grid(&lines, close + 1, size, origin, &header.legend, &fail)?;
    let exits = map_areas::build_exits(&header.exits, origin, size, &fail)?;
    let entries = map_areas::build_entries(&header.entries, origin, size, &fail)?;
    map_areas::check_areas(&exits, &entries, &objects, &fail)?;

    Ok(Map {
        name,
        cell_size,
        origin,
        size,
        draw_size: header.draw_size,
        jitter: header.jitter.unwrap_or(0.0),
        objects,
        exits,
        entries,
    })
}

/// Reads the grid rows. `first` is the index in `lines` of the first line after the frontmatter, so line
/// numbers in errors are `index + 1`. Row labels count down from the top for `BottomLeft` and up from the top for
/// `TopLeft`. Objects come back top row first, left to right, with cells numbered from the bottom left.
fn parse_grid(
    lines: &[&str],
    first: usize,
    size: IVec2,
    origin: Origin,
    legend: &HashMap<char, (String, Params)>,
    fail: &Fail,
) -> Result<Vec<MapObject>, MapError> {
    let width = size.x as usize;
    let mut objects = Vec::new();
    let mut rows = 0;
    for (index, raw) in lines.iter().enumerate().skip(first) {
        let line = index + 1;
        let text = raw.trim();
        if text.is_empty() || text.starts_with('#') {
            continue;
        }
        let Some(open) = raw.find('|') else {
            return Err(fail(
                line,
                format!("expected a grid row like ` 3 |. . .|`, found `{text}`"),
            ));
        };
        let label = raw[..open].trim();
        if label.is_empty() {
            continue;
        }
        let rest = raw[open + 1..].trim_end();
        let Some(cells) = rest.strip_suffix('|') else {
            return Err(fail(line, "grid row has no closing `|`".into()));
        };
        if rows >= size.y {
            return Err(fail(
                line,
                format!("more rows than the height {} in `size`", size.y),
            ));
        }
        let y = size.y - 1 - rows;
        let expected = match origin {
            Origin::BottomLeft => y,
            Origin::TopLeft => rows,
        };
        if label.parse::<i32>().ok() != Some(expected) {
            return Err(fail(
                line,
                format!("row label `{label}` is wrong, expected {expected}"),
            ));
        }
        rows += 1;
        let chars: Vec<char> = cells.chars().collect();
        if chars.len() != 2 * width - 1 {
            return Err(fail(
                line,
                format!(
                    "row has {} characters between the bars but {} are needed for {width} cells",
                    chars.len(),
                    2 * width - 1
                ),
            ));
        }
        for x in 0..width {
            if x > 0 && chars[2 * x - 1] != ' ' {
                return Err(fail(
                    line,
                    format!(
                        "expected a space after cell {}, found `{}`",
                        x - 1,
                        chars[2 * x - 1]
                    ),
                ));
            }
            let character = chars[2 * x];
            if character == ' ' || character == '.' {
                continue;
            }
            let Some((kind, params)) = legend.get(&character) else {
                return Err(fail(
                    line,
                    format!("character `{character}` is not in the legend"),
                ));
            };
            objects.push(MapObject {
                kind: kind.clone(),
                cell: IVec2::new(x as i32, y),
                params: params.clone(),
            });
        }
    }
    if rows != size.y {
        return Err(fail(
            lines.len().max(first),
            format!("the grid needs {} rows, found {rows}", size.y),
        ));
    }
    Ok(objects)
}

pub(crate) type Fail<'a> = dyn Fn(usize, String) -> MapError + 'a;

/// Removes a comment: `#` at the start of the line or after a space.
fn strip_comment(line: &str) -> &str {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
        return "";
    }
    match line.find(" #") {
        Some(position) => &line[..position],
        None => line,
    }
}

/// Splits `key: value`. The key may be quoted, so `":": rock_0` works.
fn split_key(text: &str) -> Option<(String, &str)> {
    let text = text.trim();
    if let Some(rest) = text.strip_prefix('"') {
        let end = rest.find('"')?;
        let value = rest[end + 1..].trim_start().strip_prefix(':')?;
        return Some((rest[..end].to_string(), value.trim()));
    }
    let (key, value) = text.split_once(':')?;
    Some((key.trim().to_string(), value.trim()))
}

fn parse_header(
    lines: &[&str],
    first_line: usize,
    kinds: &[&str],
    fail: &Fail,
) -> Result<Header, MapError> {
    let mut header = Header::default();
    let mut block = Block::None;
    for (offset, raw) in lines.iter().enumerate() {
        let line = first_line + offset;
        let text = strip_comment(raw);
        if text.trim().is_empty() {
            continue;
        }
        let Some((key, value)) = split_key(text) else {
            return Err(fail(
                line,
                format!("expected `key: value`, found `{}`", text.trim()),
            ));
        };
        let indented = text.starts_with(' ') || text.starts_with('\t');
        if indented {
            if block == Block::None {
                return Err(fail(
                    line,
                    "indented line outside `legend`, `exits` and `entries`".into(),
                ));
            }
            if block != Block::Legend {
                if key.is_empty() {
                    return Err(fail(line, "the name of an exit or entry is empty".into()));
                }
                let params = params_from(value.split_whitespace(), line, fail)?;
                let link = RawLink {
                    name: key,
                    line,
                    params,
                };
                if block == Block::Exits {
                    header.exits.push(link);
                } else {
                    header.entries.push(link);
                }
                continue;
            }
            let mut chars = key.chars();
            let (Some(character), None) = (chars.next(), chars.next()) else {
                return Err(fail(
                    line,
                    format!("legend key `{key}` must be exactly one character"),
                ));
            };
            if character.is_whitespace() {
                return Err(fail(line, "legend key cannot be a space".into()));
            }
            if character == '|' {
                return Err(fail(
                    line,
                    "legend key `|` is reserved for the grid bars".into(),
                ));
            }
            if character == '.' {
                if value != "empty" {
                    return Err(fail(line, "`.` is always empty, use `\".\": empty`".into()));
                }
                continue;
            }
            let (kind, params) = legend_value(value, line, fail)?;
            if !kinds.contains(&kind) {
                return Err(fail(
                    line,
                    format!("unknown kind `{kind}` (known: {})", kinds.join(", ")),
                ));
            }
            header.legend.insert(character, (kind.to_string(), params));
            continue;
        }
        block = Block::None;
        match key.as_str() {
            "map" => header.name = Some(value.to_string()),
            "cell" => header.cell_size = Some(positive(value, "cell", line, fail)?),
            "origin" => {
                header.origin = Some(match value {
                    "bottom-left" => Origin::BottomLeft,
                    "top-left" => Origin::TopLeft,
                    _ => {
                        return Err(fail(
                            line,
                            format!(
                                "`origin` must be `bottom-left` or `top-left`, found `{value}`"
                            ),
                        ));
                    }
                });
            }
            "size" => {
                let size = value
                    .split_once('x')
                    .and_then(|(w, h)| Some(IVec2::new(w.trim().parse().ok()?, h.trim().parse().ok()?)))
                    .filter(|s| s.x >= 1 && s.y >= 1)
                    .ok_or_else(|| {
                        fail(
                            line,
                            format!(
                                "`size` must be written `WxH` with whole numbers of at least 1, found `{value}`"
                            ),
                        )
                    })?;
                header.size = Some(size);
            }
            "draw" => header.draw_size = Some(positive(value, "draw", line, fail)?),
            "jitter" => {
                let jitter = value
                    .parse::<f32>()
                    .ok()
                    .filter(|j| *j >= 0.0)
                    .ok_or_else(|| {
                        fail(
                            line,
                            format!("`jitter` must be a number of at least 0, found `{value}`"),
                        )
                    })?;
                header.jitter = Some(jitter);
            }
            "legend" | "exits" | "entries" => {
                if !value.is_empty() {
                    return Err(fail(line, format!("`{key}:` takes indented lines")));
                }
                block = match key.as_str() {
                    "legend" => Block::Legend,
                    "exits" => Block::Exits,
                    _ => Block::Entries,
                };
            }
            other => return Err(fail(line, format!("unknown key `{other}`"))),
        }
    }
    Ok(header)
}

/// Splits a legend value into the kind and its `key=value` parameters.
fn legend_value<'a>(
    value: &'a str,
    line: usize,
    fail: &Fail,
) -> Result<(&'a str, Params), MapError> {
    let mut words = value.split_whitespace();
    let kind = words.next().unwrap_or("");
    let params = params_from(words, line, fail)?;
    Ok((kind, params))
}

/// Reads `key=value` words into `Params`.
fn params_from<'a>(
    words: impl Iterator<Item = &'a str>,
    line: usize,
    fail: &Fail,
) -> Result<Params, MapError> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    for word in words {
        let Some((key, text)) = word.split_once('=') else {
            return Err(fail(
                line,
                format!("expected `key=value` after the kind, found `{word}`"),
            ));
        };
        let key_ok = !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !key_ok || text.is_empty() {
            return Err(fail(
                line,
                format!(
                    "bad parameter `{word}`: use `key=value` with letters, digits and `_` in the key"
                ),
            ));
        }
        if pairs.iter().any(|(name, _)| name == key) {
            return Err(fail(line, format!("parameter `{key}` is given twice")));
        }
        pairs.push((key.to_string(), text.to_string()));
    }
    Ok(Params { pairs, line })
}

fn positive(value: &str, key: &str, line: usize, fail: &Fail) -> Result<f32, MapError> {
    value
        .parse::<f32>()
        .ok()
        .filter(|v| *v > 0.0)
        .ok_or_else(|| {
            fail(
                line,
                format!("`{key}` must be a number above 0, found `{value}`"),
            )
        })
}

#[cfg(test)]
mod text_tests {
    use super::*;

    const KINDS: &[&str] = &["player_start", "tree_big", "red"];
    const RULERS: &str = "   |0 1 2 3 4 5|\n";
    const DOTS: &str = ". . . . . .";

    /// The front matter of a 6 by 4 test map. It is 11 lines, so the grid starts on line 12.
    fn head(size: &str) -> String {
        format!(
            "---\nmap: t\ncell: 16\norigin: bottom-left\nsize: {size}\nlegend:\n  \".\": empty\n  \"@\": player_start\n  \"g\": tree_big\n  \"r\": red\n---\n"
        )
    }

    /// Four labelled rows, top row first. `rows` is the cell text between the bars.
    fn body(rows: [&str; 4]) -> String {
        format!(
            " 3 |{}|\n 2 |{}|\n 1 |{}|\n 0 |{}|\n",
            rows[0], rows[1], rows[2], rows[3]
        )
    }

    /// A whole test file with the top ruler. Rulers are on line 12 and the rows on lines 13 to 16.
    fn grid(rows: [&str; 4]) -> String {
        format!("{}{RULERS}{}", head("6x4"), body(rows))
    }

    fn try_parse(source: &str) -> Result<Map, MapError> {
        parse("t.txt", source, KINDS)
    }

    fn parse_ok(source: &str) -> Map {
        try_parse(source).unwrap_or_else(|e| panic!("{e}"))
    }

    fn error(source: &str) -> MapError {
        try_parse(source).expect_err("expected an error")
    }

    fn cells(map: &Map, kind: &str) -> Vec<(i32, i32)> {
        let mut found: Vec<_> = map
            .objects
            .iter()
            .filter(|o| o.kind == kind)
            .map(|o| (o.cell.x, o.cell.y))
            .collect();
        found.sort();
        found
    }

    #[test]
    fn dotted_and_blank_footprints_give_the_same_objects() {
        let dotted = parse_ok(&grid([DOTS, DOTS, ". g . . . .", DOTS]));
        let blank = parse_ok(&grid([DOTS, ".     . . .", ". g   . . .", DOTS]));
        assert_eq!(dotted.objects, blank.objects);
        assert_eq!(cells(&blank, "tree_big"), vec![(1, 1)]);
    }

    #[test]
    fn one_grid_may_mix_both_styles() {
        let map = parse_ok(&grid([DOTS, ".     . . .", ". g   . . .", ". . . . g ."]));
        assert_eq!(cells(&map, "tree_big"), vec![(1, 1), (4, 0)]);
    }

    #[test]
    fn a_blank_in_the_middle_of_a_row_does_not_shift_later_cells() {
        let map = parse_ok(&grid([DOTS, DOTS, DOTS, "r   r . . ."]));
        assert_eq!(cells(&map, "red"), vec![(0, 0), (2, 0)]);
    }

    #[test]
    fn blanks_before_the_closing_bar_are_kept() {
        let map = parse_ok(&grid([DOTS, DOTS, DOTS, ". . .      "]));
        assert!(map.objects.is_empty());
        let spaced = grid([DOTS, DOTS, DOTS, ". . .      "]).replace("|\n", "| \n");
        assert!(parse_ok(&spaced).objects.is_empty());
        let stripped = error(&grid([DOTS, DOTS, DOTS, ". . ."]));
        assert_eq!(stripped.line, 16);
        assert!(stripped.message.contains("characters between the bars"));
    }

    #[test]
    fn a_row_of_the_wrong_width_names_its_line() {
        let e = error(&grid([DOTS, DOTS, ". . . . .", DOTS]));
        assert_eq!(e.line, 15);
        assert!(e.message.contains("characters between the bars"), "{e}");
    }

    #[test]
    fn a_wrong_row_label_names_its_line() {
        let e = error(&grid([DOTS; 4]).replace(" 2 |", " 5 |"));
        assert_eq!(e.line, 14);
        assert!(
            e.message.contains("label") && e.message.contains("expected 2"),
            "{e}"
        );
        let e = error(&grid([DOTS; 4]).replace(" 2 |", " b |"));
        assert!(e.message.contains("label"), "{e}");
    }

    #[test]
    fn rulers_are_ignored_and_may_be_anywhere() {
        let rows = [DOTS, DOTS, ". g . . . .", ". . . . @ ."];
        let plain = body(rows);
        let expected = parse_ok(&format!("{}{RULERS}{plain}", head("6x4"))).objects;
        for text in [
            plain.clone(),
            format!("{plain}{RULERS}"),
            format!("{RULERS}{plain}{RULERS}"),
        ] {
            assert_eq!(
                parse_ok(&format!("{}{text}", head("6x4"))).objects,
                expected
            );
        }
        assert_eq!(expected.len(), 2);
    }

    #[test]
    fn a_character_not_in_the_legend_names_its_line() {
        let e = error(&grid([DOTS, DOTS, ". x . . . .", DOTS]));
        assert_eq!(e.line, 15);
        assert!(e.message.contains("`x` is not in the legend"), "{e}");
    }

    #[test]
    fn rows_run_top_to_bottom_from_the_bottom_left_cell() {
        let map = parse_ok(&grid(["g . . . . .", DOTS, DOTS, "g . . . . ."]));
        assert_eq!(cells(&map, "tree_big"), vec![(0, 0), (0, 3)]);
        assert_eq!(map.size, IVec2::new(6, 4));
        assert_eq!(map.cell_size, 16.0);
        assert_eq!(map.name, "t");
    }

    #[test]
    fn the_row_count_must_match_the_height() {
        let few = format!(
            "{}{RULERS} 3 |{DOTS}|\n 2 |{DOTS}|\n 1 |{DOTS}|\n",
            head("6x4")
        );
        let e = error(&few);
        assert!(
            e.message.contains("4 rows") && e.message.contains("found 3"),
            "{e}"
        );
        let many = format!("{}{RULERS}{} -1 |{DOTS}|\n", head("6x4"), body([DOTS; 4]));
        let e = error(&many);
        assert_eq!(e.line, 17);
        assert!(e.message.contains("more rows"), "{e}");
    }

    #[test]
    fn the_width_in_size_sets_the_cell_count() {
        let e = error(&format!("{}{RULERS}{}", head("5x4"), body([DOTS; 4])));
        assert!(e.message.contains("characters between the bars"), "{e}");
    }

    #[test]
    fn a_character_between_cells_must_be_a_space() {
        let e = error(&grid([DOTS, DOTS, ".x. . . . .", DOTS]));
        assert_eq!(e.line, 15);
        assert!(e.message.contains("space"), "{e}");
    }

    #[test]
    fn a_row_needs_both_bars() {
        let no_close = format!("{}{}", head("6x4"), " 3 |. . . . . .\n");
        let e = error(&no_close);
        assert_eq!(e.line, 12);
        assert!(e.message.contains("closing"), "{e}");
        let no_bars = format!("{}{}", head("6x4"), ". . . . . .\n");
        let e = error(&no_bars);
        assert!(e.message.contains("grid row"), "{e}");
    }

    #[test]
    fn reserved_legend_keys_are_errors() {
        for key in ["|", ".", " "] {
            let source = head("6x4").replace("\"g\": tree_big", &format!("\"{key}\": tree_big"));
            let e = error(&format!("{source}{RULERS}{}", body([DOTS; 4])));
            assert_eq!(e.line, 9, "key `{key}`: {e}");
        }
    }

    #[test]
    fn player_start_is_an_ordinary_object() {
        let map = parse_ok(&grid([DOTS, DOTS, ". . . . @ .", DOTS]));
        assert_eq!(cells(&map, "player_start"), vec![(4, 1)]);
    }

    #[test]
    fn comments_blank_lines_and_windows_line_endings_work() {
        let plain = grid([DOTS, DOTS, ". g . . . .", DOTS]);
        let noisy = format!(
            "{}# a note\n\n{RULERS}# another note\n{}\n",
            head("6x4"),
            body([DOTS, DOTS, ". g . . . .", DOTS])
        );
        let expected = parse_ok(&plain).objects;
        assert_eq!(parse_ok(&noisy).objects, expected);
        assert_eq!(parse_ok(&plain.replace('\n', "\r\n")).objects, expected);
    }

    #[test]
    fn front_matter_problems_are_errors_with_their_line() {
        let rows = body([DOTS; 4]);
        let no_size = head("6x4").replace("size: 6x4\n", "");
        let e = error(&format!("{no_size}{rows}"));
        assert_eq!(e.line, 1);
        assert!(e.message.contains("missing required key `size`"), "{e}");
        for bad in ["6by4", "0x4", "6x", "6x-1"] {
            let e = error(&format!("{}{rows}", head(bad)));
            assert_eq!(e.line, 5, "size `{bad}`");
            assert!(e.message.contains("`size`"), "{e}");
        }
        let e = error(&format!(
            "{}{rows}",
            head("6x4").replace("bottom-left", "centre")
        ));
        assert_eq!(e.line, 4);
        assert!(e.message.contains("origin"), "{e}");
        let e = error(&format!(
            "{}{rows}",
            head("6x4").replace("tree_big", "bush")
        ));
        assert_eq!(e.line, 9);
        assert!(e.message.contains("unknown kind"), "{e}");
    }

    #[test]
    fn objects_come_back_top_row_first_then_left_to_right() {
        let map = parse_ok(&grid([". g . . g .", DOTS, DOTS, ". . g . . ."]));
        let order: Vec<_> = map.objects.iter().map(|o| (o.cell.x, o.cell.y)).collect();
        assert_eq!(order, vec![(1, 3), (4, 3), (2, 0)]);
    }

    #[test]
    fn optional_keys_and_comments() {
        let source = "---\n# a comment\nmap: t # trailing\ncell: 32\norigin: bottom-left\nsize: 1x1\ndraw: 160\njitter: 8\nlegend:\n  \"#\": tree_big # quoted hash key\n---\n# a note\n   |0|\n 0 |#|\n";
        let map = parse_ok(source);
        assert_eq!(map.name, "t");
        assert_eq!(map.cell_size, 32.0);
        assert_eq!(map.draw_size, Some(160.0));
        assert_eq!(map.jitter, 8.0);
        assert_eq!(cells(&map, "tree_big"), vec![(0, 0)]);
    }

    #[test]
    fn a_missing_or_broken_front_matter_is_an_error() {
        assert_eq!(error("# no front matter\n").line, 1);
        assert!(error("---\nmap: t\n").message.contains("not closed"));
        let base = "origin: bottom-left\nsize: 1x1\n---\n";
        assert!(
            error(&format!("---\ncell: 16\n{base}"))
                .message
                .contains("`map`")
        );
        assert!(
            error(&format!("---\nmap: t\n{base}"))
                .message
                .contains("`cell`")
        );
        assert!(
            error("---\nmap: t\ncell: 16\nsize: 1x1\n---\n")
                .message
                .contains("`origin`")
        );
        let head = "---\nmap: t\ncell: 16\norigin: bottom-left\n";
        for (extra, line, text) in [
            ("color: 3\n", 5, "unknown key `color`"),
            ("pitch: 2\n", 5, "unknown key `pitch`"),
            ("draw: 0\n", 5, "`draw`"),
            ("jitter: -1\n", 5, "`jitter`"),
        ] {
            let e = error(&format!("{head}{extra}size: 1x1\n---\n"));
            assert_eq!(e.line, line, "{extra}");
            assert!(e.message.contains(text), "{e}");
        }
        let e = error("---\nmap: t\ncell: -1\norigin: bottom-left\nsize: 1x1\n---\n");
        assert_eq!(e.line, 3);
    }

    #[test]
    fn legend_problems_are_errors() {
        let base = "---\nmap: t\ncell: 16\norigin: bottom-left\nsize: 1x1\nlegend:\n";
        let e = error(&format!("{base}  \"0\": boulder\n---\n"));
        assert_eq!(e.line, 7);
        assert!(e.message.contains("unknown kind `boulder`"), "{e}");
        let e = error(&format!("{base}  \"ab\": tree_big\n---\n"));
        assert!(e.message.contains("exactly one character"), "{e}");
        let e = error(&format!("{base}  \".\": tree_big\n---\n"));
        assert!(e.message.contains("always empty"), "{e}");
        let e = error("---\nmap: t\ncell: 16\norigin: bottom-left\n  \"g\": tree_big\n---\n");
        assert!(e.message.contains("outside `legend`"), "{e}");
    }

    fn legend_map(entries: &str) -> String {
        format!(
            "---\nmap: t\ncell: 16\norigin: bottom-left\nsize: 2x1\nlegend:\n  \".\": empty\n{entries}---\n   |0 1|\n 0 |a b|\n"
        )
    }

    #[test]
    fn legend_parameters_are_read_and_default_to_none() {
        let kinds = ["spawn_a", "tree_big"];
        let source = legend_map(
            "  \"a\": spawn_a every=10 strategy=patrol radius=120.5\n  \"b\": tree_big\n",
        );
        let map = parse("t.txt", &source, &kinds).unwrap();
        assert_eq!(map.objects.len(), 2);
        let spawner = &map.objects[0];
        assert_eq!(spawner.kind, "spawn_a");
        assert_eq!(spawner.params.get("every"), Some("10"));
        assert_eq!(spawner.params.get("strategy"), Some("patrol"));
        assert_eq!(spawner.params.f32("radius"), Ok(Some(120.5)));
        assert_eq!(spawner.params.f32("missing"), Ok(None));
        assert!(spawner.params.f32("strategy").is_err());
        assert!(map.objects[1].params.is_empty());
    }

    #[test]
    fn bad_legend_parameters_are_errors_with_their_line() {
        let kinds = ["spawn_a", "tree_big"];
        for (entry, text) in [
            ("spawn_a every", "expected `key=value`"),
            ("spawn_a =5", "bad parameter"),
            ("spawn_a every=", "bad parameter"),
            ("spawn_a ev-ery=5", "bad parameter"),
            ("spawn_a every=5 every=6", "given twice"),
            ("nope every=5", "unknown kind `nope`"),
        ] {
            let source = legend_map(&format!("  \"a\": {entry}\n  \"b\": tree_big\n"));
            let e = parse("t.txt", &source, &kinds).unwrap_err();
            assert_eq!(e.line, 8, "{entry}");
            assert!(e.message.contains(text), "{entry}: {e}");
        }
    }

    /// The same drawing as `grid`, with rows labelled from the top for `origin: top-left`.
    fn grid_top_left(rows: [&str; 4]) -> String {
        format!(
            "{}{RULERS} 0 |{}|\n 1 |{}|\n 2 |{}|\n 3 |{}|\n",
            head("6x4").replace("bottom-left", "top-left"),
            rows[0],
            rows[1],
            rows[2],
            rows[3]
        )
    }

    #[test]
    fn top_left_rows_count_down_the_page_and_cells_stay_bottom_left() {
        let rows = [". g . . . .", DOTS, DOTS, ". . . . @ ."];
        let top = parse_ok(&grid_top_left(rows));
        let bottom = parse_ok(&grid(rows));
        assert_eq!(top.origin, Origin::TopLeft);
        assert_eq!(bottom.origin, Origin::BottomLeft);
        assert_eq!(top.objects, bottom.objects);
        assert_eq!(cells(&top, "tree_big"), vec![(1, 3)]);
        assert_eq!(cells(&top, "player_start"), vec![(4, 0)]);
    }

    #[test]
    fn top_left_row_labels_are_checked() {
        let wrong = grid([DOTS; 4]).replace("origin: bottom-left", "origin: top-left");
        let e = error(&wrong);
        assert_eq!(e.line, 13);
        assert!(e.message.contains("expected 0"), "{e}");
        let e = error(&grid_top_left([DOTS; 4]).replace(" 2 |", " 7 |"));
        assert_eq!(e.line, 15);
        assert!(e.message.contains("expected 2"), "{e}");
    }

    #[test]
    fn an_unknown_origin_is_an_error() {
        let e = error(&grid([DOTS; 4]).replace("bottom-left", "centre"));
        assert_eq!(e.line, 4);
        assert!(e.message.contains("`bottom-left` or `top-left`"), "{e}");
    }
}
