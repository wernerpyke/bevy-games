# Map files

A text file that a human edits to draw a whole map. One character is one cell. The game reads the file and builds objects from the characters.

## Summary

- One grid shows every cell of the map and what is on it.
- A legend in the front matter says what each character means.
- Rulers and row labels help a human find a cell. Row labels are checked.
- Collision and sprites are built from the placed objects. No outline is drawn by hand.
- The parser is plain Rust behind one function, `parse`, so the format can change without touching the code that spawns objects.

## Status

| Part | Code | Shared? |
|---|---|---|
| Parser: `parse(file, source, kinds)`, `Map`, `MapObject`, `MapError` | `crates/game-kit/src/map.rs` | Shared. Object kinds are names (adr-x13). |
| Exits and entries: `Area`, `MapExit`, `MapEntry`, the in-file checks | `crates/game-kit/src/map_areas.rs` | Shared |
| Background: `Background`, `BackgroundMode`, and `place_tiles` (which cells get which image) | `crates/game-kit/src/map.rs`, `crates/game-kit/src/map_background.rs` | Shared. The game draws the images |
| Links between maps: `validate_links` | `crates/game-kit/src/map_links.rs` | Shared. Run by the map registry. |
| Map edges: `EdgePiece`, `choose_pieces`, `EdgeMode` and the quarter rules, `edge_sprites`, `edge_blocked_rects`, `edge_start_feet` | `crates/game-kit/src/map_edge.rs` (the edge cells are found in `map.rs`) | Shared. The game draws the frames and chooses the mode and layer |
| Loading and `Kind` | `games/<game>/src/map.rs` | Per game |
| Spawning | per game | Per game |

Decisions: `adr-x18`, `adr-x30` (exits and entries), `adr-x33` (exit objects), `adr-x36` (backgrounds), `adr-x49`, `adr-x50` and `adr-x52` (map edges), `adr-lw8`, `adr-rt7`. `adr-x11` is the markdown format that this one replaced. The research on other formats was deleted after the decision. It is in git history as `dev-docs/RESEARCH-map-layouts.md`.

## Specification

### Example

```
---
map: demo
cell: 16
origin: bottom-left
size: 6x4
legend:
  ".": empty
  "@": player_start
  "g": tree_big
---
   |0 1 2 3 4 5|
 3 |. . . . . .|
 2 |.     . . .|
 1 |. g   . @ .|
 0 |. . . . . .|
```

The tree is at (1,1). Its 2x2 footprint is drawn with blanks. The player starts at (4,1).

The same map with `origin: top-left` has the same drawing and different row labels:

```
   |0 1 2 3 4 5|
 0 |. . . . . .|
 1 |.     . . .|
 2 |. g   . @ .|
 3 |. . . . . .|
```

The tree is at (1,2) in this file, and the game still sees it at (1,1). See rule 6.

### File layout

1. **Front matter** between two `---` lines, one `key: value` per line. A `#` starts a comment at the start of a line or after a space.
2. **Grid** after the second `---`. Each line is one of:
   - a **comment**: starts with `#`. Ignored.
   - a **blank line**. Ignored.
   - a **ruler line**: the text before the first `|` is blank. Ignored. Rulers may be at the top, at the bottom, both, or absent.
   - a **grid row**: `<label>|<cells>|`.

### Front matter keys

| Key | Needed | Meaning |
|---|---|---|
| `map` | yes | Map name, used in log and error messages |
| `cell` | yes | Grid cell size in pixels |
| `origin` | yes | `bottom-left` or `top-left`: the corner where row labels start at 0. See rule 6 |
| `size` | yes | Width and height in cells, written `WxH` |
| `legend` | yes | Maps each character in the grid to an object kind |
| `draw` | no | Drawn size of an object in pixels. The game gives the default |
| `jitter` | no | Largest random shift of each object in pixels. Default 0 |
| `exits` | no | Zones that lead to other maps. See "Exits and entries" |
| `entries` | no | Places where the player arrives from another map. See "Exits and entries" |
| `background` | no | The ground of the map: a colour and tile images. See rule 25 |

3. A legend character is one character, in quotes. `.` is written `".": empty` or `".": edge` (see "Map edges"), and any other value is an error. The keys `.`, `|` and a space are reserved and cannot name an object kind.
4. The game defines the set of kinds. It passes the names it knows to `parse`, which reports any other name with its line. Objects carry the name as a `String`, and the game converts it to its own enum (adr-x13).
5. A legend entry can be a marker, such as `"@": player_start`. A marker is an ordinary object with that kind. The game reads its cell and does not spawn art for it.
   A legend entry may have parameters after the kind: `"S": spawn_skeleton every=10 strategy=patrol radius=120`. Each parameter is `key=value` with no spaces, the key has letters, digits and `_`, and a key is not given twice. The parser reads them into `MapObject.params` (`Params`: `get`, `f32`, `line`) and does not know what they mean. The game checks the keys it needs and reports problems. An entry without parameters is unchanged.

### Grid rows

6. The label is the row number, right-aligned and zero-based. The parser checks every label. The first row on the page is the top row. Which number it has depends on `origin`:
   - `bottom-left`: the top row is `H-1` and the last row is 0, so labels count down the page. Labels match the F3 grid in a game that numbers cells from the bottom left.
   - `top-left`: the top row is 0 and the last row is `H-1`, so labels count down the page from 0, like rows of text.

   `origin` only changes how a file is written. `parse` always returns cells numbered from the bottom left with y up (`MapObject.cell`), so a game does not change when a file changes its origin. `Map.origin` says which one the file used, for tools that label cells.
7. The cells are the text between the first and the last `|`. Cell k, counting from 0 at the left, is the character at offset 2k. Characters at odd offsets must be spaces. The cell text is exactly `2W-1` characters long.
8. A blank is an empty cell. A `.` is an empty cell with `".": empty`, and an edge cell with `".": edge`. Any other character must be in the legend. A cell with an object is the anchor of that object.
9. The grid has exactly `H` rows, and each row has exactly `W` cells. Both numbers come from `size`.
10. In the game, cell (0,0) is the bottom-left cell. It is a fixed world position set by the game, so cells do not move when the window is resized.

### Footprints

11. An object occupies its anchor cell in the file. The size of its footprint belongs to the game, not to the file. The anchor is the cell at the bottom left of the footprint as drawn on the page, so a 2x2 footprint stretches one cell right and one cell up the page. This holds for both origins.
12. With `".": empty`, a person may draw the footprint cells with `.` or with blanks. The parser treats both as empty, so both give the same result. Blanks help a reader see how far an object reaches. With `".": edge`, the file is the truth: a `.` footprint cell is an edge cell. So draw footprint cells inside the map with blanks. The kit does not know footprints (`adr-x13`).
13. The parser does not check that a drawing matches the real footprint, and it does not report objects that overlap. Objects that are close enough merge into one collision cluster, and that is intended.

### Errors

14. Any format error stops loading and names the file and line, for example `assets/maps/1.txt:15: character x is not in the legend`.
15. Errors are:
    - exits and entries: see rules 16 to 22, each error names the line of the item
    - front matter: a missing key, a bad `size`, an `origin` that is not `bottom-left` or `top-left`, a legend kind the game does not know, or a reserved legend key, or a bad or repeated legend parameter
    - a row with no `|`, or no closing `|`
    - a wrong row label
    - a row of the wrong width, or a non-space character at an odd offset
    - a character that is not in the legend
    - a row count that does not match the height
    - with `".": edge`: see rule 32

### Exits and entries

Two optional blocks in the front matter describe how maps join. The game reads them when it moves the player between maps (`dev-docs/extensions/map-transitions.md`). The parser reads and checks them now.

```
exits:
  to_2: area=3..6,9 to=2 entry=from_1 transition=scene
entries:
  from_1: area=3..6,0 facing=up
```

16. One line is `name: key=value ...`, indented under `exits:` or `entries:`. Names are unique inside one block. An entry and an exit may share a name.
17. `area` is a straight strip of cells: `x0..x1,y`, `x,y0..y1` or one cell `x,y`. A range on both axes is an error, so an area is never a block or a diagonal. A range goes from low to high. Cells are numbered like the row labels of the file, so a `top-left` file counts rows from the top. The area must be inside the map.
18. An **exit** needs `area`, `to` (the id of the target map, set by the game), `entry` (the name of an entry in that map) and `transition` (`seamless` or `scene`). Its area lies along a map edge: a horizontal strip on the top or bottom row, a vertical strip in the left or right column. A single cell in a corner is an error, because it touches two edges.
19. An **entry** needs `area` and may have `facing` (`up`, `down`, `left` or `right`).
20. Mark the cells of an area with a blank (`' '`) in the grid, not `.`. With `".": empty` this is a reading aid that the parser does not check. With `".": edge` a `.` would close the area, so it is an error that names the line of the area.
21. No object may stand in an area. The one exception is a `player_start` marker in an entry area. Two exits may not overlap, and an entry may not overlap an exit (the player would be sent back at once).
22. Between maps (`validate_links`, when a game registers its maps): the exit leads to a registered map and an entry of that map, and the two have the same length, so the position along the strip carries over. A seamless exit needs its entry on the opposite edge (a top exit joins a bottom entry) and may not lead to its own map.
23. **Start marker.** `"@": player_start` is an ordinary marker (rule 5). A game reads its cell as the start of the player. A map that is only entered from another map needs none. Add one to a later map to be able to start in it directly, for example for a scripted run.
   `start_feet(map, origin)` gives its position (the centre of the cell). On an edge cell, use `edge_start_feet` (rule 33).
24. **Exit objects.** A legend entry may have `exit_to=<map id>`, for example `"m": marker exit_to=2` (`adr-x33`). Touching the object sends the player to that map, at its `player_start`. The parser reads it as an ordinary parameter (`MapObject::exit_to`). Between maps (`validate_links`) the map must be registered and have exactly one `player_start`, and the error names the legend line. The game decides where the object can be touched (`ExitTrigger`). There is no `transition` key: it is always a scene transition. Do not put such an object inside an exit area (rule 21). A map may use exit areas, exit objects, or both.

### Background

25. **Background.** An optional `background:` block in the front matter gives the ground of the map: a colour, and tile images drawn over it (`adr-x36`). Indented `key: value` lines, like the legend:

    ```
    background:
      color: "#292829"
      mode: random
      fill: 0.2
      jitter: 16
      scale: 2
      tiles: grass_0 grass_1 flowers_0
    ```

    | Key | Needed | Meaning |
    |---|---|---|
    | `color` | yes | Hex colour `#rrggbb`. Quote it: an unquoted `#` starts a comment |
    | `mode` | yes | `random` or `pattern`. No default. `pattern` (a repeating fill) is reserved: the parser rejects it with "not implemented yet" |
    | `fill` | no | Chance, 0 to 1, that a cell gets an image. Default 0 (colour only) |
    | `tiles` | with `fill` above 0 | Image names separated by blanks, each equally likely. The game checks the names, the kit does not know the art |
    | `jitter` | no | Largest random shift of an image in pixels on each axis. Default 0. Separate from the map key `jitter` |
    | `scale` | no | A number above 0, the size of an image as a multiple of its art size. Default 1. `0.5` is allowed |
    | `seed` | no | A whole number that changes the dice. Default 0 |

    The parser (`map.rs`, `Map::background`) checks the syntax and names the line of each error: a missing `color` or `mode`, a bad `mode`, colour, `fill`, `jitter`, `scale` or `seed`, a `fill` above 0 without `tiles`, a repeated tile name, an unknown key, a key or the block given twice.
    `map_background::place_tiles` decides which cells get an image, which image, and where. Each cell rolls its own dice from a hash of the map name, the `seed` and the cell number. So the ground is the same on every load, on F1 reload and on the web, it does not depend on the order of the cells, and a map that grows keeps the images of its old cells. The number of images varies from map to map, because it is a chance per cell. An image is centred on its cell, shifted by the jitter, and moved inwards so it stays inside the map. The game draws the colour as a rectangle of the map size and the images above it (`layers.rs`, `BACKGROUND_Z` and `BACKGROUND_TILES_Z`).

### Map edges

With `".": edge` in the legend, each `.` is an edge cell and a blank is free space (`adr-x50`). The kit gives each edge cell a piece, and the game draws a frame per piece (`adr-x49`). Code: `crates/game-kit/src/map_edge.rs`.

```
   |0 1 2 3 4 5 6 7 8 9|          | 0  1  2  3  4  5  6  7  8  9|
 9 |. . . . . . . . . .|        9 | c  n  n  c  c  c  n  n  c  c|
 8 |@     . . .     . .|        8 | w        ne c  nw       ne c|
 7 |.                 .|        7 | w                          e|
 6 |.                 .|        6 | w                          e|
 5 |.                 .|        5 | w                          e|
 4 |. .             . .|        4 | c  sw                   se c|
 3 |. . .         . . .|        3 | c  c  w              se c  c|
 2 |.           . . . .|        2 | w                 se c  c  c|
 1 |. . .       . . . .|        1 | c  c  sw          c  c  c  c|
 0 |. . . . . . . . . .|        0 | c  c  c  s  s  s  c  c  c  c|
```

The grid on the left gives the pieces on the right (`c` is `center`, `n` is `north`, `ne` is `north_east`, and so on). The `@` at (0,8) is on the outer ring, so it is an edge cell (rule 27).

26. **Pieces.** The nine pieces are named by their opaque side: `north`, `north_east`, `east`, `south_east`, `south`, `south_west`, `west`, `north_west`, `center`. `north` is opaque in the top half, `north_east` in the top right triangle, `center` everywhere (`EdgePiece`, `EdgePiece::name` gives the names).
27. **Objects.** A cell with an object has no `.` or blank, so its place decides what it is. On the outer ring (the first or last row or column) it is an ordinary edge cell: it gets a piece, its neighbours treat it as edge, and it blocks. The object is drawn as usual and blocks as before. In an entry area, or inside the map, it is free space. This holds for every kind of object, `@` too.
28. **Step 1: the free sides give the piece.** A neighbour is free when it is a blank, an object inside the map, or in an entry area. A cell outside the map counts as edge.

    | Free sides | Piece |
    |---|---|
    | none | `center` |
    | S | `north` |
    | N | `south` |
    | W | `east` |
    | E | `west` |
    | S and W | `north_east` |
    | S and E | `north_west` |
    | N and W | `south_east` |
    | N and E | `south_west` |
    | three sides | the piece of the closed side, for example N, E and S give `west` |

29. **Step 2.** A straight piece (`north`, `east`, `south`, `west`) from step 1 becomes `center` when a neighbour along its wall has a piece other than `center` or the same straight piece. The neighbours along the wall of `north` and `south` are west and east, of `east` and `west` they are north and south. Step 2 reads the pieces of step 1, so a change does not spread. The reason: a straight piece is half a cell high where it meets its neighbour, and a corner or a straight piece facing another way is full height there, which would leave a notch. In the example, (4,8), (1,3), (6,1) and (1,1) become `center`.
30. **Modes.** A game passes `EdgeMode::Basic` (steps 1 and 2) or `EdgeMode::Smooth` to `edge_quarters` (`adr-x49`). Smooth mode then applies the quarter rules to the whole map. They read the pieces of step 2 one quarter of a cell at a time, so a change does not spread. A side of a quarter is closed when the touching edge of the neighbouring quarter is fully opaque, and open when that quarter is fully clear. Outside the map is closed.
    - **Cut.** A fully opaque quarter with two adjacent open sides, whose other two sides are closed, gets the half-size corner piece of its closed sides. It removes a square corner of the wall.
    - **Fill.** A fully clear quarter with two adjacent closed sides, whose other two sides are not closed, gets the half-size corner piece of its closed sides. It fills a square inside corner.
    - A fill next to a cut quarter is skipped (a step one quarter high, where both would fire).

    In the example, the cuts are (0,9) bottom right and (2,3) both left quarters. The fills are (2,9), (7,9) and (0,5) bottom right, (6,9) and (9,5) bottom left, (9,7) and (3,0) top left, (5,0) top right, and (0,2) both right quarters. The fills at (1,9) bottom left and (0,8) top right are skipped next to the cut at (0,9). A corner where a diagonal meets a straight wall stays as it is, because its quarters are not fully opaque or fully clear.
31. **Drawing.** `edge_sprites` lists the sprites, and `EdgeSprite::place` gives each one a centre, a scale and an optional source rectangle (`Sprite::rect`):
    - A cell without quarter pieces: its whole frame.
    - A quarter piece: the whole frame of its corner piece at half size in that quarter. The frame is scaled down, so no new art is needed and its pixels are half the size.
    - A cell with a cut: only its kept quarters that are not clear, each a quarter of its frame at full size. Art that reaches past the half of a frame does not show beside a cut. Beside a fill it does, so draw the frames so that nothing reaches past the half.

    Variants: a tag may have more than one frame. Put `RandomStaticFrame::for_cell(map_name, cell)` (`animation.rs`) on each sprite. When the file has loaded, it shows one frame of the tag, chosen from the map name and the cell, and the tag does not play (`adr-x52`). So the frame is the same on every load. All sprites of one cell use the same frame number. A tag with one frame shows that frame.

    Draw order: a game uses one of two layers from `layers.rs`, `EDGE_GROUND_Z` (below the world and the grid debug) or `EDGE_CANOPY_Z` (above the world, below the debug drawings), with half-size pieces `EDGE_HALF_Z_OFFSET` above.
32. **Errors.** Each stops loading and names its line:
    - a cell with exactly two free sides that are opposite (N and S, or E and W), or with four free sides: a wall one cell thick, or a single edge cell. The line is the grid row of the cell. Three free sides are allowed.
    - a `.` inside an exit or entry area. The line is the line of the area.
    - a map with no free cell. The line is the `".": edge` line.
33. **Start.** `edge_start_feet(map, origin, step, half_width)` places the player at its `@`. On an edge cell it is the point nearest the cell centre, away from the opaque side, outside the blocked area grown by the half width: on a `west` piece, the half width to the right of the half line. On a `center` piece it is the centre of the nearest free cell (no edge, no object), by straight-line distance, with ties taken clockwise from north. Elsewhere it is the centre of the cell. A game that uses edges also passes `edge_fit: Some(EdgeFit { step, half_width })` to `MapTransitionPlugin`, so an arrival at `player_start` through an exit object uses the same rule.
34. **Blocking.** `edge_blocked_rects` gives the blocked areas, which the game grows sideways by its actor half width like every other blocker. A straight piece blocks its half, `center` the whole cell. A corner blocks a stair of columns about `step` wide: the top of each column meets the diagonal at the middle of the column, so the stair covers exactly half the cell. A half-size corner uses the same step, so it has half as many columns. A cut removes the blocking of its quarter. Neighbouring cells merge into one shape in `outline::build`.

R-Town keeps `".": empty` and has no edges.

## Legend conventions

Characters are chosen so a map can be read at a glance (`adr-x37`). Games follow them for new legends. The parser does not check them.

| Group | Character | Example (Long Way) |
|---|---|---|
| Player | `@` | `"@": player_start` |
| Features (rocks, trees, shrubs, lanterns, markers) | Letters. Upper and lower case tell variants of the same family apart | `"G": green_tree`, `"g": green_tree_medium`, `"O": rock_0`, `"o": rock_1` |
| Enemies | Special characters: any ASCII punctuation that is not reserved, `! # $ % & ' ( ) * + , - / : ; < = > ? [ \ ] ^ _ ` { } ~` | `"!": spawn_ghost`, `"^": spawn_volcano` |
| Pickups | Digits | `"1": energy_blue` |

- `.`, `|` and a space stay reserved (rule 3). `.` is `empty` or `edge`. `"` cannot be a key: the parser reads it as the start of a quoted key.
- Write special characters in quotes. `#` and `:` must be quoted: unquoted, `#` starts a comment and `:` splits the key from the kind.
- R-Town keeps its older legend (digits for water) until the user asks for a change.

## Usage

- Add a new object kind to the game's name list and `Kind` enum, to the module that spawns it, and to the legend.
- Items that touch form a solid group in R-Town and Long Way, and an item that stands alone can be walked behind (`groups.md`).
- Keep rows in a fixed-width font and turn off word wrap. Large maps have long rows.
- Write blank cells as spaces inside the bars. The closing `|` keeps blanks at the end of a row safe from editors that remove trailing spaces.
- Both games read the map file from disk at start and on F1, so a change needs no rebuild (`adr-rt8`, `adr-e11`).

## Limits

- One grid, one object per cell. There are no layers and no ground tiles in the file.
- One map file is one map. How a game moves between maps is in `dev-docs/extensions/map-transitions.md`.
- Maps with triggers, paths or nested data may outgrow this format (`rm3`).
- A visual editor is part of `rm3`. It must keep this format as the source of truth.

## Tests

Inline in `game-kit` `map.rs`. Each game tests that its own map file parses (counts of objects, size and the start marker). The parser tests cover:

- Footprints: a dotted and a blank footprint give the same objects, one grid may mix both styles, a blank in the middle of a row does not shift later cells, and blanks before the closing `|` are kept.
- Grid shape: rows run top to bottom and cells are numbered from the bottom left, the row count and the width must match `size`, a character at an odd offset must be a space, and a row needs both bars.
- Labels and rulers: wrong row labels name their line (for both origins), and rulers are ignored at the top, at the bottom, at both ends or absent.
- Origin: `top-left` and `bottom-left` files with the same drawing give the same objects, and an unknown origin is an error.
- Legend: a character that is not in the legend names its line, reserved keys (`|`, `.`, space) are errors, and a marker such as `player_start` comes back as an ordinary object.
- Front matter: missing keys, a bad `size`, unknown keys (including the removed `pitch`), bad `draw`, `jitter` and `cell` values, an unknown kind, comments and optional keys.
- Exits and entries: `map_areas.rs` tests the area notation, the edge rules, objects in areas, overlaps and a `top-left` file. `map_links.rs` tests the checks between maps. It also tests `exit_to` (unknown map, a target without exactly one start). `map.rs` tests `exit_to` and `start_feet`.
- Background: a full block, defaults, a block with only a colour and a mode, no block, every error with its line, `scale` (including `0.5`), a repeated key or block, and the map key `jitter` kept apart. `map_background.rs` tests the placement: fill 0 and 1, the same result for the same input, a different seed, the share close to `fill`, jitter and the clamp inside the map, a map that grows, and a fixed expected list so the hash never changes.
- Other: comments, blank lines and Windows line endings, and objects come back in a fixed order (top row first, then left to right).
- Map edges: `map.rs` tests that `.` is an edge only with `".": edge`, objects on the ring and inside the map (both origins), and the errors with their lines. `map_areas.rs` tests a `.` in an exit or entry and `@` in an entry on the ring. `map_edge.rs` tests every row of the step 1 table, the whole example of rule 29 and its quarter pieces (rule 30), the errors, the blocked areas of each piece at two cell sizes and step sizes, the start on each piece and on a `center` piece (one and two cells away, a tie), the sprites and their placement in both modes, and that cuts free and fills block. `map_transition.rs` tests an arrival on an edge cell.
