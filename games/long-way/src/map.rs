//! Map files: markdown with frontmatter and painted `cells` blocks. See
//! `dev-docs/PLAN-forest-collision.md` for the format. The parser is plain Rust and
//! has no Bevy dependency except `IVec2`, so it is unit tested without a running app.
//!
//! Supported frontmatter (a small subset of YAML, one `key: value` per line):
//! - `map`: name used in log and error messages. Required.
//! - `cell`: cell size in pixels. Required.
//! - `origin`: must be `bottom-left`. Required.
//! - `pitch`: cells per character. Default 1.
//! - `draw`: drawn size of a rock in pixels. Default 128.
//! - `jitter`: largest random offset in pixels. Default 0.
//! - `legend`: indented `char: kind` lines. `.` is always empty. Quote the character
//!   (for example `"#"`), because an unquoted `#` starts a comment.
//!
//! Comments start with `#` at the start of a line or after a space. Lines in the body
//! are ignored except fenced `cells at=x,y` blocks.

use std::collections::HashMap;
use std::fmt;

use bevy::math::IVec2;
use bevy::prelude::*;

/// Map files built into the game. Edit one and rebuild. The first is the default. Set `LONG_WAY_MAP` to a file
/// name (for example `rocks-v2`) to load another one. The v2 and v3 files are for the placement experiment
/// in `dev-docs/PLAN-forest-collision.md` and are removed when it is decided.
const MAPS: [(&str, &str); 3] = [
    (
        "assets/maps/rocks-test.md",
        include_str!("../assets/maps/rocks-test.md"),
    ),
    (
        "assets/maps/rocks-v2.md",
        include_str!("../assets/maps/rocks-v2.md"),
    ),
    (
        "assets/maps/rocks-v3.md",
        include_str!("../assets/maps/rocks-v3.md"),
    ),
];

fn selected_map() -> (&'static str, &'static str) {
    let Ok(wanted) = std::env::var("LONG_WAY_MAP") else {
        return MAPS[0];
    };
    MAPS.iter()
        .copied()
        .find(|(name, _)| name.contains(wanted.as_str()))
        .unwrap_or_else(|| {
            let known: Vec<_> = MAPS.iter().map(|(name, _)| *name).collect();
            panic!("LONG_WAY_MAP={wanted} matches no map. Known maps: {known:?}")
        })
}

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        let (name, source) = selected_map();
        let map = parse(name, source).unwrap_or_else(|error| panic!("{error}"));
        info!(
            "map: {} cell {} pitch {} draw {} jitter {}, {} objects",
            map.name,
            map.cell_size,
            map.pitch,
            map.draw_size,
            map.jitter,
            map.objects.len()
        );
        for object in &map.objects {
            debug!("map: {:?} at cell {}", object.kind, object.cell);
        }
        app.insert_resource(CurrentMap(map));
    }
}

#[derive(Resource)]
pub struct CurrentMap(pub Map);

/// What an object is. Art, footprint and collision rule for each kind are set in code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Rock0,
    Rock1,
}

impl Kind {
    fn from_name(name: &str) -> Option<Kind> {
        match name {
            "rock_0" => Some(Kind::Rock0),
            "rock_1" => Some(Kind::Rock1),
            _ => None,
        }
    }
}

/// One painted character: an object whose bottom left corner is at `cell`.
#[derive(Clone, Debug, PartialEq)]
pub struct MapObject {
    pub kind: Kind,
    pub cell: IVec2,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Map {
    pub name: String,
    pub cell_size: f32,
    pub pitch: i32,
    pub draw_size: f32,
    pub jitter: f32,
    pub objects: Vec<MapObject>,
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
    origin_ok: bool,
    pitch: Option<i32>,
    draw_size: Option<f32>,
    jitter: Option<f32>,
    legend: HashMap<char, Kind>,
}

/// Parses a map file. `file` is only used in error messages.
pub fn parse(file: &str, source: &str) -> Result<Map, MapError> {
    let fail = |line: usize, message: String| MapError {
        file: file.to_string(),
        line,
        message,
    };
    let lines: Vec<&str> = source.lines().collect();

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

    let header = parse_header(&lines[1..close], 2, &fail)?;
    let name = header
        .name
        .clone()
        .ok_or_else(|| fail(1, "missing required key `map`".into()))?;
    let cell_size = header
        .cell_size
        .ok_or_else(|| fail(1, "missing required key `cell`".into()))?;
    if !header.origin_ok {
        return Err(fail(
            1,
            "missing required key `origin` (use `origin: bottom-left`)".into(),
        ));
    }
    let pitch = header.pitch.unwrap_or(1);

    let mut objects = Vec::new();
    let mut i = close + 1;
    while i < lines.len() {
        let trimmed = lines[i].trim_start();
        let Some(info) = trimmed.strip_prefix("```") else {
            i += 1;
            continue;
        };
        let info = info.trim();
        let start = i;
        // Find the closing fence of this block.
        let Some(end) = lines
            .iter()
            .enumerate()
            .skip(start + 1)
            .find(|(_, l)| l.trim() == "```")
            .map(|(index, _)| index)
        else {
            return Err(fail(start + 1, "code block is not closed".into()));
        };
        if info == "cells" || info.starts_with("cells ") {
            let at = parse_at(info, start + 1, &fail)?;
            parse_cells(
                &lines[start + 1..end],
                start + 2,
                at,
                pitch,
                &header.legend,
                &fail,
                &mut objects,
            )?;
        }
        i = end + 1;
    }

    Ok(Map {
        name,
        cell_size,
        pitch,
        draw_size: header.draw_size.unwrap_or(128.0),
        jitter: header.jitter.unwrap_or(0.0),
        objects,
    })
}

type Fail<'a> = dyn Fn(usize, String) -> MapError + 'a;

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

fn parse_header(lines: &[&str], first_line: usize, fail: &Fail) -> Result<Header, MapError> {
    let mut header = Header::default();
    let mut in_legend = false;
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
            if !in_legend {
                return Err(fail(line, "indented line outside `legend`".into()));
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
            if character == '.' {
                if value != "empty" {
                    return Err(fail(line, "`.` is always empty, use `\".\": empty`".into()));
                }
                continue;
            }
            let Some(kind) = Kind::from_name(value) else {
                return Err(fail(
                    line,
                    format!("unknown kind `{value}` (known: rock_0, rock_1)"),
                ));
            };
            header.legend.insert(character, kind);
            continue;
        }
        in_legend = false;
        match key.as_str() {
            "map" => header.name = Some(value.to_string()),
            "cell" => header.cell_size = Some(positive(value, "cell", line, fail)?),
            "origin" => {
                if value != "bottom-left" {
                    return Err(fail(
                        line,
                        format!("`origin` must be `bottom-left`, found `{value}`"),
                    ));
                }
                header.origin_ok = true;
            }
            "pitch" => {
                let pitch = value
                    .parse::<i32>()
                    .ok()
                    .filter(|p| *p >= 1)
                    .ok_or_else(|| {
                        fail(
                            line,
                            format!(
                                "`pitch` must be a whole number of at least 1, found `{value}`"
                            ),
                        )
                    })?;
                header.pitch = Some(pitch);
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
            "legend" => {
                if !value.is_empty() {
                    return Err(fail(
                        line,
                        "`legend:` takes indented `char: kind` lines".into(),
                    ));
                }
                in_legend = true;
            }
            other => return Err(fail(line, format!("unknown key `{other}`"))),
        }
    }
    Ok(header)
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

/// Reads `at=x,y` from the info string of a `cells` block.
fn parse_at(info: &str, line: usize, fail: &Fail) -> Result<IVec2, MapError> {
    let Some(value) = info.split_whitespace().find_map(|t| t.strip_prefix("at=")) else {
        return Err(fail(
            line,
            "`cells` block needs `at=x,y`, for example `cells at=10,6`".into(),
        ));
    };
    let bad = || {
        fail(
            line,
            format!("`at={value}` must be two whole numbers, for example `at=10,6`"),
        )
    };
    let (x, y) = value.split_once(',').ok_or_else(bad)?;
    Ok(IVec2::new(
        x.trim().parse().map_err(|_| bad())?,
        y.trim().parse().map_err(|_| bad())?,
    ))
}

/// Rows run top to bottom, so the first row has the highest y. Character `k` of a row is at
/// cell `at.x + k * pitch`.
fn parse_cells(
    rows: &[&str],
    first_line: usize,
    at: IVec2,
    pitch: i32,
    legend: &HashMap<char, Kind>,
    fail: &Fail,
    objects: &mut Vec<MapObject>,
) -> Result<(), MapError> {
    let height = rows.len() as i32;
    let mut width = None;
    for (row_index, row) in rows.iter().enumerate() {
        let line = first_line + row_index;
        if row.trim().is_empty() {
            return Err(fail(line, "blank line in `cells` block".into()));
        }
        let tokens: Vec<&str> = row.split_whitespace().collect();
        if *width.get_or_insert(tokens.len()) != tokens.len() {
            return Err(fail(
                line,
                format!(
                    "row has {} characters but the first row has {}",
                    tokens.len(),
                    width.unwrap_or(0)
                ),
            ));
        }
        for (column, token) in tokens.iter().enumerate() {
            let mut chars = token.chars();
            let (Some(character), None) = (chars.next(), chars.next()) else {
                return Err(fail(
                    line,
                    format!(
                        "expected one character per cell, found `{token}` (separate cells with spaces)"
                    ),
                ));
            };
            if character == '.' {
                continue;
            }
            let Some(kind) = legend.get(&character) else {
                return Err(fail(
                    line,
                    format!("character `{character}` is not in the legend"),
                ));
            };
            objects.push(MapObject {
                kind: *kind,
                cell: IVec2::new(
                    at.x + column as i32 * pitch,
                    at.y + (height - 1 - row_index as i32) * pitch,
                ),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "---\nmap: t\ncell: 64\norigin: bottom-left\nlegend:\n  \"0\": rock_0\n  \"1\": rock_1\n---\n";

    fn parse_ok(source: &str) -> Map {
        parse("t.md", source).unwrap_or_else(|e| panic!("{e}"))
    }

    fn error(source: &str) -> MapError {
        parse("t.md", source).expect_err("expected an error")
    }

    fn cells(map: &Map, kind: Kind) -> Vec<(i32, i32)> {
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
    fn the_bundled_map_parses() {
        let map = parse_ok(MAPS[0].1);
        assert_eq!(map.name, "rocks-test");
        assert_eq!(map.cell_size, 64.0);
        assert_eq!(map.pitch, 1);
        assert_eq!(map.draw_size, 128.0);
        assert_eq!(map.jitter, 0.0);
        assert_eq!(map.objects.len(), 10);
    }

    #[test]
    fn rows_run_top_to_bottom_from_the_bottom_left_cell() {
        let map = parse_ok(&format!(
            "{HEADER}```cells at=10,6\n. . 1 1 . .\n. 0 0 1 1 .\n. 0 0 . . .\n. . 0 0 . .\n```\n"
        ));
        assert_eq!(
            cells(&map, Kind::Rock0),
            vec![(11, 7), (11, 8), (12, 6), (12, 7), (12, 8), (13, 6)]
        );
        assert_eq!(
            cells(&map, Kind::Rock1),
            vec![(12, 9), (13, 8), (13, 9), (14, 8)]
        );
    }

    #[test]
    fn pitch_spaces_the_characters() {
        let map = parse_ok(
            "---\nmap: t\ncell: 64\norigin: bottom-left\npitch: 2\nlegend:\n  \"0\": rock_0\n---\n```cells at=4,2\n0 0\n. 0\n```\n",
        );
        assert_eq!(cells(&map, Kind::Rock0), vec![(4, 4), (6, 2), (6, 4)]);
    }

    #[test]
    fn several_blocks_and_other_code_blocks() {
        let map = parse_ok(&format!(
            "{HEADER}# Notes\n```rust\n0 0 0\n```\n```cells at=0,0\n0\n```\ntext\n```cells at=-3,5\n1\n```\n"
        ));
        assert_eq!(cells(&map, Kind::Rock0), vec![(0, 0)]);
        assert_eq!(cells(&map, Kind::Rock1), vec![(-3, 5)]);
    }

    #[test]
    fn optional_keys_and_comments() {
        let map = parse_ok(
            "---\n# a comment\nmap: t # trailing\ncell: 32\norigin: bottom-left\ndraw: 160\njitter: 8\nlegend:\n  \"#\": rock_0 # quoted hash key\n---\n```cells at=0,0\n#\n```\n",
        );
        assert_eq!(map.name, "t");
        assert_eq!(map.cell_size, 32.0);
        assert_eq!(map.draw_size, 160.0);
        assert_eq!(map.jitter, 8.0);
        assert_eq!(cells(&map, Kind::Rock0), vec![(0, 0)]);
    }

    #[test]
    fn windows_line_endings_work() {
        let source = format!("{HEADER}```cells at=1,1\n0\n```\n").replace('\n', "\r\n");
        assert_eq!(parse_ok(&source).objects.len(), 1);
    }

    #[test]
    fn an_unknown_character_names_its_line() {
        let e = error(&format!("{HEADER}text\n```cells at=0,0\n0 0\n0 x\n```\n"));
        assert_eq!(e.line, 12);
        assert!(e.message.contains("`x`"), "{}", e.message);
        assert_eq!(e.to_string(), format!("t.md:12: {}", e.message));
    }

    #[test]
    fn a_ragged_row_is_an_error() {
        let e = error(&format!("{HEADER}```cells at=0,0\n0 0\n0\n```\n"));
        assert_eq!(e.line, 11);
        assert!(e.message.contains("first row"), "{}", e.message);
    }

    #[test]
    fn two_characters_without_a_space_is_an_error() {
        let e = error(&format!("{HEADER}```cells at=0,0\n00\n```\n"));
        assert!(
            e.message.contains("one character per cell"),
            "{}",
            e.message
        );
    }

    #[test]
    fn a_blank_line_in_a_block_is_an_error() {
        let e = error(&format!("{HEADER}```cells at=0,0\n0\n\n0\n```\n"));
        assert!(e.message.contains("blank line"), "{}", e.message);
    }

    #[test]
    fn a_missing_or_bad_at_is_an_error() {
        let e = error(&format!("{HEADER}```cells\n0\n```\n"));
        assert!(e.message.contains("at=x,y"), "{}", e.message);
        let e = error(&format!("{HEADER}```cells at=1\n0\n```\n"));
        assert!(e.message.contains("two whole numbers"), "{}", e.message);
    }

    #[test]
    fn an_unclosed_block_is_an_error() {
        let e = error(&format!("{HEADER}```cells at=0,0\n0\n"));
        assert_eq!(e.line, 9);
        assert!(e.message.contains("not closed"), "{}", e.message);
    }

    #[test]
    fn frontmatter_problems_are_errors() {
        assert_eq!(error("# no frontmatter\n").line, 1);
        assert!(error("---\nmap: t\n").message.contains("not closed"));
        assert!(
            error("---\ncell: 64\norigin: bottom-left\n---\n")
                .message
                .contains("`map`")
        );
        assert!(
            error("---\nmap: t\norigin: bottom-left\n---\n")
                .message
                .contains("`cell`")
        );
        assert!(
            error("---\nmap: t\ncell: 64\n---\n")
                .message
                .contains("`origin`")
        );
        let e = error("---\nmap: t\ncell: 64\norigin: top-left\n---\n");
        assert_eq!(e.line, 4);
        assert!(e.message.contains("bottom-left"));
        let e = error("---\nmap: t\ncell: 64\norigin: bottom-left\nsize: 3\n---\n");
        assert_eq!(e.line, 5);
        assert!(e.message.contains("unknown key `size`"));
        let e = error("---\nmap: t\ncell: -1\norigin: bottom-left\n---\n");
        assert_eq!(e.line, 3);
        let e = error("---\nmap: t\ncell: 64\norigin: bottom-left\npitch: 0\n---\n");
        assert_eq!(e.line, 5);
    }

    #[test]
    fn legend_problems_are_errors() {
        let base = "---\nmap: t\ncell: 64\norigin: bottom-left\nlegend:\n";
        let e = error(&format!("{base}  \"0\": boulder\n---\n"));
        assert_eq!(e.line, 6);
        assert!(e.message.contains("unknown kind `boulder`"));
        let e = error(&format!("{base}  \"ab\": rock_0\n---\n"));
        assert!(e.message.contains("exactly one character"));
        let e = error(&format!("{base}  \".\": rock_0\n---\n"));
        assert!(e.message.contains("always empty"));
        assert!(
            error("---\nmap: t\ncell: 64\norigin: bottom-left\n  \"0\": rock_0\n---\n")
                .message
                .contains("outside `legend`")
        );
    }
}
