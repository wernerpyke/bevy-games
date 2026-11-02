# Research: map layout formats

Status: research done. The decisions are in `adr-x18` (`dev-docs/adr/EXTENSIONS.md`) and the format is in `dev-docs/extensions/map-files.md`. Section 1 describes the first draft of `test.txt`. The final file has rulers, row labels, `origin: top-left` and a `player_start` marker. Section 5 is done. Date: 2026-11-02.

Goal: define a whole map in one plain text file, with no separate editor. Keep the door open for seeded random maps later.

Evidence labels: **[web]** came from a web search in this session. **[memory]** is general knowledge and was not checked in this session. Check it before you rely on it.

## 1. Notes on `games/r-town/assets/maps/test.txt`

The file is a draft of the new format. `test.md` is still the format the game loads.

### Format as written

```
---
map: test
cell: 16
origin: bottom-left
size: 80x60
legend:
  ".": empty
  "@": player_start
  "g": tree_big
  "d": tree_dead
---
. . . g . .   (60 rows of 80 cells)
```

- A front matter block (YAML style) sits between two `---` lines. It holds the name, cell size in pixels, origin, size and legend.
- The grid follows. One character is one cell. Cells are separated by one space. The first row is the top row (y=59). The last row is y=0.
- `size: 80x60` is new. In `test.md` the size is `MAP_CELLS` in `src/map.rs`. The parser can now check the row and column counts against it.
- `"@": player_start` is new. It marks a spawn point, not an object. A legend entry can therefore be a marker as well as an object kind.
- `test.txt` was generated from `test.md`. It holds 68 big trees, 22 dead trees and one `@`. The `@` is at (40,30).

### What the format gains

- One block shows the whole map. You can see gaps and clusters without reading coordinate lists.
- Edits are plain text. Any editor works, and a diff in git shows exactly which cells changed.
- A tree is one character, not a coordinate pair, so the file is easy to edit by hand.

### Known limits and open questions

- **Layers.** One grid holds one object per cell. Ground, objects and markers cannot overlap. Options: several grids in one file (`layer: ground`, `layer: objects`), or keep ground out of scope.
- **Multi-cell objects.** Long Way uses 2x2 rocks. Options: one character marks the anchor cell only (as `g` does now), or the parser merges neighbouring equal characters. Anchor-only is simpler.
- **Row alignment.** With spaces the file is 159 characters wide. A row with a missing cell is easy to miss by eye. The `size` check in the parser catches it. A coordinate ruler comment line above the grid would help reading, but the parser must ignore it.
- **Large maps.** A 200x200 map is 40,000 characters. This is fine for one file. Bigger maps want chunk files or generation (see section 3).
- **Notes and prose.** `test.md` allowed free text. `test.txt` has no place for notes. Option: ignore lines that start with `#` before the grid.
- **Legend key clash.** `"@"`, `"."` and `"#"` are common in the roguelike convention. Keep them for the same meanings in every game.
- **Front matter.** The parser can read it with a YAML crate or a small hand parser. `legend` is the only nested part. A hand parser avoids a new dependency and any wasm risk.

## 2. Existing plain text and simple map formats

### Plain text grids

- **ASCII grid with a legend is a normal approach.** The r/roguelikedev FAQ Friday on map representation lists the usual symbols (NetHack, Brogue, Angband) and says to reuse them. [web] [FAQ Friday #28](https://redlib.hbubli.cc/r/roguelikedev/comments/3y4z3x/faq_friday_28_map_object_representation/cyawfji/?context=3)
- **No formal standard exists.** I found no widely used open specification for a text map with a legend. Every game defines its own. The test.txt format is close to what the web results describe. [web]
- **ASCII Mapper** (MIT licence). A browser and desktop editor. Its output is a plain grid of symbols that you read in your own code. Maps up to 100x100. [web] [itch.io page](https://notimetoplay.itch.io/ascii-mapper), [devlog on plain text maps](https://notimetoplay.itch.io/ascii-mapper/devlog/17472/making-game-maps-with-plain-old-text)
- **stmn/ascii-map-editor** (MIT licence). Exports nested arrays, arrays of text rows, or plain text. Up to 999x999. [web] [GitHub](https://github.com/stmn/ascii-map-editor)
- **Playscii.** An ASCII art tool that can show each character as a graphic. [web]
- **Roguelike prefab files.** Dungeon Crawl Stash (`.des` files) and Brogue blueprints describe small rooms ("vaults") as text grids with a legend, and the generator places them. [memory] This matches the pattern in section 3, and it matches the "feature" idea in the Long Way `rocks-test.md`.

These editors need a UI, so they are not needed. A text editor is enough for the format above.

### Editor based formats (for comparison)

| Format | Notes |
|---|---|
| Tiled TMX, JSON, Lua | Full feature set. CSV export is reduced: tile ids only, with `-1` for empty. A forum answer for clean ASCII output is a custom export script. [web] [Tiled forum](https://discourse.mapeditor.org/t/how-to-map-graphic-tileset-to-ascii-symbols/3752) |
| LDtk | JSON. Several levels in one file. Each layer has its own grid size, and tiles can stack in a cell. [web] [LDtk issue 126](https://github.com/deepnight/ldtk/issues/126) |
| Ogmo | `.ogmo` and JSON. Orthogonal maps only. [web] |

- All three need their editor to author maps, which you do not want.
- The Bevy crate `bevy_ecs_ldtk` was last listed for Bevy 0.18 in my search. It sits on `bevy_ecs_tilemap`. I found no release for Bevy 0.20, and this project pins 0.20.1. [web] [docs.rs](https://docs.rs/crate/bevy_ecs_ldtk/latest)
- Result: the editor formats do not help now. A hand parser for the text format is small.

## 3. How other engines approach map layout

### Authored maps

- **Godot** (TileMapLayer) and **Unity** (Tilemap) store tiles in the scene or asset. The editor is the main tool. [memory]
- **Tiled and LDtk** are external editors. Engines read their files through a library. [web]
- **Roguelikes** usually keep the map as a 2D array in memory. The Rust roguelike tutorial (bracket-lib) uses a flat `Vec<TileType>` and a chain of "map builders". [memory]

### Generated maps

These patterns appear in most sources. [web] unless marked.

1. **One master seed.** The whole world comes from one number. Same seed gives the same world.
2. **Per-chunk seed.** Each chunk gets its own random generator from `hash(master_seed, chunk_x, chunk_y)`. Chunks can load in any order, in parallel, and on demand, and the result does not change. Minecraft style worlds use this idea. [Atomic Object article](https://spin.atomicobject.com/infinite-procedurally-generated-world/), [seeds and determinism](https://abratabia.com/procedural-generation/seeds-and-determinism.php)
3. **Chunk borders.** A chunk can read its neighbours but must not write to them. Otherwise the result depends on load order. [web] [gamedev.net thread](https://gamedev.net/forums/topic/684981-splitting-world-generation-into-chunks)
4. **Hash quality.** Use a proper integer hash (MurmurHash3, xxHash, Wang hash), and 64-bit seeds, to avoid patterns and collisions. [web]
5. **Common algorithms** [memory]: value or simplex noise for terrain, cellular automata for caves, BSP or room placement for dungeons, random walk, Wave Function Collapse (WFC) for tile rules, and hand-made prefabs placed by a generator.

### Rust and Bevy notes [memory, check versions before use]

- `rand` with a seedable generator such as `rand_chacha::ChaCha8Rng` gives the same output on every platform. This matters for native and wasm giving the same map. Avoid `StdRng` and `SmallRng` for this, because their algorithm can change between versions.
- Noise crates exist (`noise`, `fastnoise-lite`). Check Bevy 0.20 and `wasm32-unknown-unknown` support before adding one.
- Keep the generator in plain Rust with no Bevy types. This fits the project rule that game rules are unit testable.

## 4. What this means for the text format

A seeded generator and the text format fit together without a redesign.

```
generate(seed, size, rules) -> Grid        (plain Rust)
parse(text)                  -> Grid        (plain Rust)
Grid -> spawn objects                       (Bevy system, same for both)
```

- A generated map is the same `Grid` value as a parsed one. The spawn code does not know which one it received.
- A generator can write its output as a `.txt` file. This helps debugging: you can read what seed 42 made, and you can edit it by hand.
- Hand-drawn prefabs (a village, a ruin) can use the same text format and be stamped into a generated map. This is the "feature" idea from `rocks-test.md`, and the text format can hold it as a small grid.
- Front matter can gain `seed: 12345` and `generator: forest` later, with no grid, or with a grid that holds fixed parts only.

## 5. Suggested next steps (done 2026-11-02)

1. Decide the open questions in section 1: layers, multi-cell objects, notes.
2. Write a parser for the text format in `crates/game-kit/` (ask first, per the shared code rule) and switch R-Town to `test.txt`.
3. Record the format decision in `dev-docs/adr/` and document the format in `dev-docs/extensions/`.
4. Keep generation out of scope until needed. When it is needed, start with a per-chunk seed and `ChaCha8Rng`.

## Sources

- [ASCII Mapper](https://notimetoplay.itch.io/ascii-mapper)
- [Making game maps with plain old text](https://notimetoplay.itch.io/ascii-mapper/devlog/17472/making-game-maps-with-plain-old-text)
- [stmn/ascii-map-editor](https://github.com/stmn/ascii-map-editor)
- [Tiled forum: map tileset to ASCII symbols](https://discourse.mapeditor.org/t/how-to-map-graphic-tileset-to-ascii-symbols/3752)
- [r/roguelikedev FAQ Friday #28](https://redlib.hbubli.cc/r/roguelikedev/comments/3y4z3x/faq_friday_28_map_object_representation/cyawfji/?context=3)
- [LDtk issue 126](https://github.com/deepnight/ldtk/issues/126)
- [Tiled forum: JSON vs TMX](https://discourse.mapeditor.org/t/questions-about-json-vs-tmx/4431)
- [bevy_ecs_ldtk on docs.rs](https://docs.rs/crate/bevy_ecs_ldtk/latest)
- [Building an Infinite Procedurally-Generated World](https://spin.atomicobject.com/infinite-procedurally-generated-world/)
- [Seeds and determinism](https://abratabia.com/procedural-generation/seeds-and-determinism.php)
- [Splitting world generation into chunks (gamedev.net)](https://gamedev.net/forums/topic/684981-splitting-world-generation-into-chunks)
