//! Map files: markdown with frontmatter, a `# Features` section and a `# Map` section. See
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
//! Body:
//! - `# Features`: each `## Name` heading defines a feature, with one fenced `cells` block. Rows run
//!   top to bottom, one character per `pitch` cells. The bottom left character is the anchor.
//! - `# Map`: a bulleted list that places features, for example `- 'Name' at (2,12), (2,3) and (6,3)`.
//!   Positions are the cell of the anchor. Features may be defined before or after the `# Map` section.
//! - Any other heading, text or code block is ignored.
//!
//! Comments in the frontmatter start with `#` at the start of a line or after a space.

use std::collections::HashMap;
use std::fmt;

use bevy::math::IVec2;
use bevy::prelude::*;

/// The map file Long Way loads. Edit it and rebuild.
const MAP_NAME: &str = "assets/maps/rocks-test.md";
const MAP_SOURCE: &str = include_str!("../assets/maps/rocks-test.md");

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        let map = parse(MAP_NAME, MAP_SOURCE).unwrap_or_else(|error| panic!("{error}"));
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

    let objects = parse_body(&lines, close + 1, pitch, &header.legend, &fail)?;

    Ok(Map {
        name,
        cell_size,
        pitch,
        draw_size: header.draw_size.unwrap_or(128.0),
        jitter: header.jitter.unwrap_or(0.0),
        objects,
    })
}

/// Where the body of the file is, as it is read.
#[derive(Clone, Copy, PartialEq)]
enum Section {
    Other,
    Features,
    Map,
}

/// A feature: a painted `cells` block under a `## Name` heading, with positions relative to its anchor.
struct Feature {
    line: usize,
    objects: Option<Vec<MapObject>>,
}

/// One bullet of the `# Map` section.
struct Placement {
    line: usize,
    name: String,
    at: Vec<IVec2>,
}

/// Reads the body: `# Features` holds `## Name` headings with a `cells` block each, `# Map` holds a bulleted
/// list that places features. Other headings, text and code blocks are ignored.
fn parse_body(
    lines: &[&str],
    first: usize,
    pitch: i32,
    legend: &HashMap<char, Kind>,
    fail: &Fail,
) -> Result<Vec<MapObject>, MapError> {
    let mut section = Section::Other;
    let mut features: HashMap<String, Feature> = HashMap::new();
    let mut current: Option<String> = None;
    let mut placements: Vec<Placement> = Vec::new();
    let mut i = first;
    while i < lines.len() {
        let trimmed = lines[i].trim_start();
        if let Some(info) = trimmed.strip_prefix("```") {
            let start = i;
            let Some(end) = (start + 1..lines.len()).find(|&index| lines[index].trim() == "```")
            else {
                return Err(fail(start + 1, "code block is not closed".into()));
            };
            let words: Vec<&str> = info.split_whitespace().collect();
            if words.first() == Some(&"cells") {
                if words.len() > 1 {
                    return Err(fail(
                        start + 1,
                        "a `cells` block takes no options: positions go in the `# Map` section and the name is the `##` heading"
                            .into(),
                    ));
                }
                let feature = current
                    .as_ref()
                    .filter(|_| section == Section::Features)
                    .and_then(|name| features.get_mut(name.as_str()))
                    .ok_or_else(|| {
                        fail(
                            start + 1,
                            "a `cells` block belongs under a `## Feature name` heading in the `# Features` section"
                                .into(),
                        )
                    })?;
                if feature.objects.is_some() {
                    return Err(fail(
                        start + 1,
                        "a feature has one `cells` block, this is a second".into(),
                    ));
                }
                let mut objects = Vec::new();
                parse_cells(
                    &lines[start + 1..end],
                    start + 2,
                    IVec2::ZERO,
                    pitch,
                    legend,
                    fail,
                    &mut objects,
                )?;
                feature.objects = Some(objects);
            }
            i = end + 1;
            continue;
        }
        if let Some(title) = trimmed.strip_prefix("# ") {
            section = match title.trim().to_lowercase().as_str() {
                "features" => Section::Features,
                "map" => Section::Map,
                _ => Section::Other,
            };
            current = None;
        } else if let Some(title) = trimmed.strip_prefix("## ") {
            if section == Section::Features {
                let name = title.trim().to_string();
                if features.contains_key(&name) {
                    return Err(fail(i + 1, format!("feature `{name}` is already defined")));
                }
                features.insert(
                    name.clone(),
                    Feature {
                        line: i + 1,
                        objects: None,
                    },
                );
                current = Some(name);
            }
        } else if section == Section::Map
            && let Some(item) = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
        {
            placements.push(parse_placement(item, i + 1, fail)?);
        }
        i += 1;
    }

    if let Some((name, feature)) = features
        .iter()
        .filter(|(_, f)| f.objects.is_none())
        .min_by_key(|(_, f)| f.line)
    {
        return Err(fail(
            feature.line,
            format!("feature `{name}` has no `cells` block"),
        ));
    }
    let mut objects = Vec::new();
    for placement in placements {
        let Some(feature) = features.get(&placement.name) else {
            return Err(fail(
                placement.line,
                format!(
                    "feature `{}` is not defined under `# Features`",
                    placement.name
                ),
            ));
        };
        for at in placement.at {
            objects.extend(feature.objects.iter().flatten().map(|o| MapObject {
                kind: o.kind,
                cell: o.cell + at,
            }));
        }
    }
    Ok(objects)
}

/// Reads `'Feature name' at (x,y), (x,y) and (x,y)`. The name may be in single or double quotes.
fn parse_placement(item: &str, line: usize, fail: &Fail) -> Result<Placement, MapError> {
    let item = item.trim();
    let Some(quote) = item.chars().next().filter(|c| *c == '\'' || *c == '"') else {
        return Err(fail(
            line,
            "expected a quoted feature name, for example `- 'Rocks test' at (21,9)`".into(),
        ));
    };
    let rest = &item[1..];
    let Some(close) = rest.find(quote) else {
        return Err(fail(line, "the feature name has no closing quote".into()));
    };
    let name = rest[..close].trim().to_string();
    let after = rest[close + 1..].trim_start();
    let Some(positions) = after
        .strip_prefix("at")
        .filter(|r| r.is_empty() || r.starts_with(char::is_whitespace) || r.starts_with('('))
    else {
        return Err(fail(
            line,
            "expected `at` and positions after the name, for example `at (2,12), (2,3) and (6,3)`"
                .into(),
        ));
    };
    Ok(Placement {
        line,
        name,
        at: parse_positions(positions, line, fail)?,
    })
}

/// Reads positions in parentheses, separated by commas and `and`: `(2,12), (2,3) and (6,3)`.
fn parse_positions(text: &str, line: usize, fail: &Fail) -> Result<Vec<IVec2>, MapError> {
    let mut positions = Vec::new();
    let mut rest = text.trim();
    while let Some(open) = rest.find('(') {
        let between = rest[..open].replace(',', " ");
        let words: Vec<&str> = between.split_whitespace().collect();
        let allowed = if positions.is_empty() {
            words.is_empty()
        } else {
            words.is_empty() || words == ["and"]
        };
        if !allowed {
            return Err(fail(
                line,
                format!(
                    "unexpected text `{}` between positions",
                    rest[..open].trim()
                ),
            ));
        }
        let Some(close) = rest.find(')') else {
            return Err(fail(line, "a position has no closing `)`".into()));
        };
        let inner = &rest[open + 1..close];
        let bad = || {
            fail(
                line,
                format!("`({inner})` must be two whole numbers, for example `(10,6)`"),
            )
        };
        let (x, y) = inner.split_once(',').ok_or_else(bad)?;
        positions.push(IVec2::new(
            x.trim().parse().map_err(|_| bad())?,
            y.trim().parse().map_err(|_| bad())?,
        ));
        rest = rest[close + 1..].trim_start();
    }
    if positions.is_empty() {
        return Err(fail(
            line,
            "expected at least one position like `(10,6)`".into(),
        ));
    }
    if !rest.trim_end_matches('.').trim().is_empty() {
        return Err(fail(
            line,
            format!("unexpected text `{rest}` after the positions"),
        ));
    }
    Ok(positions)
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
                    format!("unknown kind `{value}` (known: rock_0, rock_1, green_tree, red_tree)"),
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

    /// A body with a `# Features` section and a `# Map` section. `features` starts on line 11 and the
    /// `# Map` heading follows it.
    fn source(features: &str, map: &str) -> String {
        format!("{HEADER}# Features\n\n{features}# Map\n\n{map}")
    }

    /// A feature of three rocks: two on the left column, one on the bottom right. Lines 11 to 17.
    const PATCH: &str = "## Patch\n\n```cells\n0 .\n0 1\n```\n\n";

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
        let map = parse_ok(MAP_SOURCE);
        assert_eq!(map.name, "rocks-test");
        assert_eq!(map.cell_size, 64.0);
        assert_eq!(map.pitch, 1);
        assert_eq!(map.draw_size, 128.0);
        assert_eq!(map.jitter, 0.0);
        // 10 rocks in the first feature, three copies of the 8 rock patch, two single rocks and two trees.
        assert_eq!(map.objects.len(), 38);
        assert_eq!(cells(&map, Kind::GreenTree), vec![(12, 8)]);
        assert_eq!(cells(&map, Kind::RedTree), vec![(18, 8)]);
    }

    #[test]
    fn rows_run_top_to_bottom_from_the_bottom_left_cell() {
        let map = parse_ok(&source(
            "## Cluster\n\n```cells\n. . 1 1 . .\n. 0 0 1 1 .\n. 0 0 . . .\n. . 0 0 . .\n```\n\n",
            "- 'Cluster' at (10,6)\n",
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
    fn a_feature_is_placed_at_every_listed_position() {
        let map = parse_ok(&source(PATCH, "- 'Patch' at (10,6), (20,2) and (-3,5)\n"));
        assert_eq!(
            cells(&map, Kind::Rock0),
            vec![(-3, 5), (-3, 6), (10, 6), (10, 7), (20, 2), (20, 3)]
        );
        assert_eq!(cells(&map, Kind::Rock1), vec![(-2, 5), (11, 6), (21, 2)]);
    }

    #[test]
    fn a_feature_with_no_placement_draws_nothing() {
        assert!(parse_ok(&source(PATCH, "")).objects.is_empty());
    }

    #[test]
    fn bullets_may_vary_in_style() {
        let map = parse_ok(&source(
            PATCH,
            "* \"Patch\" at (1, 1) , (5,5).\n- 'Patch' at (9,9)\nText that is not a bullet.\n",
        ));
        assert_eq!(map.objects.len(), 9);
    }

    #[test]
    fn features_may_come_after_the_map_section() {
        let map = parse_ok(&format!(
            "{HEADER}# Map\n\n- 'Patch' at (0,0)\n\n# Features\n\n{PATCH}"
        ));
        assert_eq!(map.objects.len(), 3);
    }

    #[test]
    fn overlapping_placements_keep_every_object() {
        let map = parse_ok(&source(PATCH, "- 'Patch' at (0,0), (1,0)\n"));
        assert_eq!(map.objects.len(), 6);
    }

    #[test]
    fn pitch_spaces_the_characters() {
        let map = parse_ok(
            "---\nmap: t\ncell: 64\norigin: bottom-left\npitch: 2\nlegend:\n  \"0\": rock_0\n---\n# Features\n\n## A\n\n```cells\n0 0\n. 0\n```\n\n# Map\n\n- 'A' at (4,2)\n",
        );
        assert_eq!(cells(&map, Kind::Rock0), vec![(4, 4), (6, 2), (6, 4)]);
    }

    #[test]
    fn other_headings_text_and_code_blocks_are_ignored() {
        let map = parse_ok(&format!(
            "{HEADER}# Title\n\n- a bullet in prose\n```rust\n0 0 0\n```\n# Features\n\n{PATCH}### Detail\n\ntext\n\n# Map\n\nProse.\n\n- 'Patch' at (0,0)\n\n# Later\n\n- 'not a placement'\n"
        ));
        assert_eq!(map.objects.len(), 3);
    }

    #[test]
    fn optional_keys_and_comments() {
        let map = parse_ok(
            "---\n# a comment\nmap: t # trailing\ncell: 32\norigin: bottom-left\ndraw: 160\njitter: 8\nlegend:\n  \"#\": rock_0 # quoted hash key\n---\n# Features\n\n## A\n\n```cells\n#\n```\n\n# Map\n\n- 'A' at (0,0)\n",
        );
        assert_eq!(map.name, "t");
        assert_eq!(map.cell_size, 32.0);
        assert_eq!(map.draw_size, 160.0);
        assert_eq!(map.jitter, 8.0);
        assert_eq!(cells(&map, Kind::Rock0), vec![(0, 0)]);
    }

    #[test]
    fn windows_line_endings_work() {
        let text = source("## A\n\n```cells\n0\n```\n\n", "- 'A' at (1,1)\n").replace('\n', "\r\n");
        assert_eq!(parse_ok(&text).objects.len(), 1);
    }

    #[test]
    fn an_unknown_character_names_its_line() {
        let e = error(&source(
            "## A\n\n```cells\n0 0\n0 x\n```\n\n",
            "- 'A' at (0,0)\n",
        ));
        assert_eq!(e.line, 15);
        assert!(e.message.contains("`x`"), "{}", e.message);
        assert_eq!(e.to_string(), format!("t.md:15: {}", e.message));
    }

    #[test]
    fn a_ragged_row_is_an_error() {
        let e = error(&source("## A\n\n```cells\n0 0\n0\n```\n\n", ""));
        assert_eq!(e.line, 15);
        assert!(e.message.contains("first row"), "{}", e.message);
    }

    #[test]
    fn two_characters_without_a_space_is_an_error() {
        let e = error(&source("## A\n\n```cells\n00\n```\n\n", ""));
        assert!(
            e.message.contains("one character per cell"),
            "{}",
            e.message
        );
    }

    #[test]
    fn a_blank_line_in_a_block_is_an_error() {
        let e = error(&source("## A\n\n```cells\n0\n\n0\n```\n\n", ""));
        assert!(e.message.contains("blank line"), "{}", e.message);
    }

    #[test]
    fn an_unclosed_block_is_an_error() {
        let e = error(&format!("{HEADER}# Features\n\n## A\n\n```cells\n0\n"));
        assert_eq!(e.line, 13);
        assert!(e.message.contains("not closed"), "{}", e.message);
    }

    #[test]
    fn a_cells_block_outside_a_feature_is_an_error() {
        let e = error(&format!("{HEADER}# Features\n\n```cells\n0\n```\n"));
        assert_eq!(e.line, 11);
        assert!(e.message.contains("belongs under"), "{}", e.message);
        let e = error(&format!("{HEADER}# Map\n\n## A\n\n```cells\n0\n```\n"));
        assert!(e.message.contains("belongs under"), "{}", e.message);
    }

    #[test]
    fn cells_options_are_an_error() {
        for info in ["cells at=1,1", "cells name=a"] {
            let e = error(&source(&format!("## A\n\n```{info}\n0\n```\n\n"), ""));
            assert_eq!(e.line, 13);
            assert!(e.message.contains("takes no options"), "{}", e.message);
        }
    }

    #[test]
    fn a_feature_has_exactly_one_cells_block() {
        let e = error(&source(
            "## A\n\n```cells\n0\n```\n\n```cells\n1\n```\n\n",
            "",
        ));
        assert_eq!(e.line, 17);
        assert!(e.message.contains("second"), "{}", e.message);
        let e = error(&source("## A\n\ntext only\n\n", ""));
        assert_eq!(e.line, 11);
        assert!(e.message.contains("no `cells` block"), "{}", e.message);
    }

    #[test]
    fn a_feature_name_cannot_be_defined_twice() {
        let e = error(&source(&format!("{PATCH}{PATCH}"), ""));
        assert_eq!(e.line, 18);
        assert!(e.message.contains("already defined"), "{}", e.message);
    }

    #[test]
    fn an_unknown_feature_names_the_bullet_line() {
        let e = error(&source(PATCH, "- 'Patch' at (0,0)\n- 'Shrubs' at (1,1)\n"));
        assert_eq!(e.line, 21);
        assert!(
            e.message.contains("`Shrubs` is not defined"),
            "{}",
            e.message
        );
    }

    #[test]
    fn bad_bullets_are_errors_with_their_line() {
        let cases = [
            ("- Patch at (0,0)", "quoted feature name"),
            ("- 'Patch at (0,0)", "closing quote"),
            ("- 'Patch' (0,0)", "expected `at`"),
            ("- 'Patch' at", "at least one position"),
            ("- 'Patch' at (x,2)", "two whole numbers"),
            ("- 'Patch' at (1,2", "closing `)`"),
            ("- 'Patch' at (1,2) there", "after the positions"),
            ("- 'Patch' at and (1,2)", "unexpected text"),
        ];
        for (bullet, expected) in cases {
            let e = error(&source(PATCH, &format!("{bullet}\n")));
            assert_eq!(e.line, 20, "{bullet}");
            assert!(e.message.contains(expected), "{bullet}: {}", e.message);
        }
        // Two positions with no separator are accepted: the separator is optional.
        assert_eq!(
            parse_ok(&source(PATCH, "- 'Patch' at (1,2) (3,4)\n"))
                .objects
                .len(),
            6
        );
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
