# Map files

A markdown file that a human edits to place objects. The game reads three parts of it and ignores the rest.

## Summary

- A `# Features` section defines painted features, one `cells` block each.
- A `# Map` section places features with a bulleted list.
- Collision and sprites are built from the placed objects. No outline is drawn by hand.
- The parser is plain Rust behind one function, `parse`, so the format can change without touching the code that spawns objects.

## Status

| Part | Code today | Shared? |
|---|---|---|
| Parser | `games/long-way/src/map.rs` | The format yes. Object kinds are per game. |
| Spawning | `rocks.rs`, `trees.rs` | Per game |

Decisions: `adr-lw8`, `adr-x11`. How to write a map for Long Way: `games/long-way/docs/maps.md`. Other formats and a loader with hot reload: roadmap items 5 and 4.

## Specification

### File layout

1. **Frontmatter** between two `---` lines, one `key: value` per line. A `#` starts a comment at the start of a line or after a space.
2. **Features**: the section `# Features`.
3. **Map**: the section `# Map`. The two sections may be in either order.
4. Any other heading, text, list or code block is a note for people and is ignored.

### Frontmatter keys

| Key | Needed | Meaning |
|---|---|---|
| `map` | yes | Map name, used in log and error messages |
| `cell` | yes | Grid cell size in pixels |
| `origin` | yes | Must be `bottom-left` |
| `legend` | yes | Maps each character in a `cells` block to an object kind |
| `pitch` | no | Cells between two characters. Default 1 |
| `draw` | no | Drawn size of a rock in pixels. Default 128 |
| `jitter` | no | Largest random shift of each object in pixels. Default 0 |

5. A legend character is one character, in quotes. `.` always means empty and is written `".": empty`.
6. The set of kinds is defined in code (`Kind` in `map.rs`).

### Features

7. A feature is a `##` heading under `# Features`, with exactly one `cells` block. The heading is the feature name.
8. Each character in a `cells` block is one object. Characters are separated by a space. All rows have the same count. There are no blank lines inside the block.
9. Rows run from top to bottom. The bottom left character is the anchor.

### Map placements

10. One bullet per feature: `- 'Name' at (x,y), (x,y) and (x,y)`. The name is in single or double quotes and matches the heading exactly.
11. Positions are anchor cell numbers on the grid. They may be negative. Copies may overlap.
12. Cell (0, 0) is a fixed world position set by the game, so cells do not move when the window is resized.

### Errors

13. Any format error stops loading and names the file and line, for example `assets/maps/rocks-test.md:15: character x is not in the legend`.
14. Errors are: unknown legend character, rows of unequal length, no space between characters, a blank line in a block, a `cells` block outside a feature, a feature with no block or two, duplicate feature names, a placement of an undefined feature, a bad position, an unclosed code block.

## Usage

- Add a new object kind in `Kind` in `map.rs`, in the module that spawns it, and in the legend.
- Do not add per block options to `cells`.
- The map is read with `include_str!`, so a change needs a rebuild.

## Limits

- One map per game build.
- Maps with many object kinds, triggers, paths or nested data may outgrow markdown (roadmap items 4 and 5).

## Tests

Inline in `map.rs`: the parser, every error message, and the bundled map file.
