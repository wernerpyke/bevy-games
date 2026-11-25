# Extensions: map decisions

Map files, exits and transitions, backgrounds and the legend. Index of all extension decisions: `EXTENSIONS.md`.

## adr-x11. Map file layout: Features and Map sections (2026-11-01)

**Superseded by `adr-x18` (2026-11-02): a map is now one drawn grid. This record keeps the reasons it gave.**

**Decision.** A map file is markdown with frontmatter and two sections the game reads (`dev-docs/extensions/map-files.md`):
1. `# Features`: each `## Name` heading defines a feature with exactly one `cells` block. The bottom left character is the anchor.
2. `# Map`: a bulleted list that places features, `- 'Name' at (x,y), (x,y) and (x,y)`. Positions are the anchor cell, in F3 grid numbers.
Everything else in the file is notes for people. Trees are map kinds (`green_tree`, `red_tree`) placed the same way as rocks. Every format error stops loading and names the file and line.

**Rationale.** The first format put `at=x,y` on each `cells` block, with a separate `name=` and `place` block for reuse. The developer wanted features described once, with comments, and placement in one readable list. A feature used several times is one bullet. It also lets the same file place trees, which need no change to the format, only a new kind in code.

**How to apply.** The parser is `parse` in `map.rs` (plain Rust, unit tested). Add a new object kind in `Kind` and in the module that spawns it (`rocks.rs`, `trees.rs`), and name it in the legend. Do not add per-block options to `cells`. `rm5` considers other formats (YAML, TOML) if maps outgrow markdown. `rm5` covers a sturdier loader.

## adr-x13. Map object kinds are strings in the kit (2026-11-02)

**Decision.** The map parser returns each object's kind as the legend name (`String`). Each game converts names to its own enum.

**Rationale.** The kit must not change when a game adds an object. A generic parser over the game's enum was rejected: it spreads generics through the parser and plugin for little gain. A shared enum was rejected: every new object in any game would edit the kit.

**How to apply.** An unknown name is reported by the game's conversion with the map file line, as the parser reports it today.

## adr-x18. Map files are one drawn grid in a text file (2026-11-02)

**Decision.** A map file is plain text with front matter and a grid (`dev-docs/extensions/map-files.md`). It replaces the markdown format of `adr-x11`.
1. The grid shows every cell of the map. One character is one cell. A legend in the front matter maps characters to object kinds. A blank or `.` is empty.
2. Each grid row is `<label>|<cells>|`, and cell k is the character at offset 2k. Rulers help a reader. Row labels are zero-based and checked. Both `.` and blanks may draw a footprint, and the parser treats them the same.
3. A character marks the anchor cell (bottom left) of an object. The footprint size belongs to the game. There are no layers, and one cell holds one object.
4. `size: WxH` is in the file. `origin` is `bottom-left` or `top-left` and only sets how rows are labelled. `parse` always returns cells numbered from the bottom left, with y up.
5. A legend entry may be a marker (R-Town `"@": player_start`). It is an ordinary object that the game reads and does not draw.
6. The F3 grid can number rows from the top (`DebugSettings::rows_from_top`) so that it matches a `top-left` file.
7. Parsing stays a small hand parser in `game-kit` with no new dependency. Every error names the file and line.

**Rationale.** The developer wanted to see a whole map in one file, edit it in a text editor, and not install an editor or tool. A drawn grid shows gaps and clusters at a glance. The markdown format split a map into features and a placement list, so no one place showed the map. Research found no standard text map format, and editor formats (Tiled, LDtk, Ogmo) need their editors. A seeded generator can later return the same `Map` value as the parser, so the format does not block generated maps.

**How to apply.** Add a new kind to the game's name list and `Kind` enum and to the legend. Use fixed-width font and turn off word wrap when editing wide maps. Layers, larger objects with own settings, chunk files and a visual editor are roadmap items. The visual editor must keep this format as the source of truth.

## adr-x30. Exits and entries are front matter blocks, and `player_start` marks a start (2026-11-06)

**Decision.** A map file can have two more nested front matter blocks, `exits:` and `entries:`, next to `legend:`. One line is `name: key=value ...`. An exit has `area`, `to` (map id), `entry` (entry name in the target map) and `transition` (`seamless` or `scene`). An entry has `area` and an optional `facing`. An `area` is a straight strip of cells: `x0..x1,y`, `x,y0..y1` or one cell `x,y`. A range on both axes is an error. Cells are numbered like the row labels of the file. The grid marks area cells with a blank by convention, as a reading aid. The parser reads the blocks (`crates/game-kit/src/map_areas.rs`) and a second function checks the files against each other (`map_links.rs`, `validate_links`): the target map and entry exist, and an exit and its entry have the same length. A seamless exit needs an entry on the opposite edge. A start marker is `player_start`, an ordinary legend object (`"@": player_start`), and the game reads its cell. A later map may also have one, so a scripted run can start in that map.

**Rationale.** Exits are rectangles on a map edge, usually several cells wide. A block in the front matter says so in one line and keeps legend characters free for objects that block. A strip cannot be a diagonal or a block, so a wrong exit is not possible to write. The grid marks stay a reading aid because blank and `.` are already the same empty cell (rule 12 of the format). Every exit and entry has the same length so the position along the strip carries over to the other map, for scene transitions as well as seamless ones. Checking the links when the maps are registered turns a typo in a map file into an error with the file and line. `player_start` already existed in R-Town, so the games share one name for a start.

**How to apply.** Write the blocks as in `dev-docs/extensions/map-files.md`. A map without the blocks parses as before, so R-Town is unchanged. A game that registers several maps calls `validate_links`. Moving between maps (the registry, the lifecycle, the camera) is in `dev-docs/extensions/map-transitions.md` (`adr-x31`, `adr-x32`).

## adr-x31. A map transition is run by the kit through three game hooks (2026-11-06)

**Decision.** `crates/game-kit/src/map_transition/` holds the lifecycle of a scene transition (modules since `adr-x59`). The game registers its maps (`MapSource`, read and checked once by `MapRegistry`) and gives the kit three one-shot systems (`MapHooks`): `load` (spawn one map, every entity tagged `MapId`), `rebuild` (rebuild the collision world) and `arrive` (place the controlled entity and set its facing). The kit sends a `RequestMapChange` message when the feet of the `Controlled` entity enter an exit area (it arms again when the feet leave every exit), fades a full-screen overlay out, runs `unload` (removes every `MapId` entity), `load`, `rebuild` and `arrive` in one frame, and fades in. The player keeps the same position along the strip: the offset from the low end of the exit strip is carried to the entry strip, and moves to the other axis when the strips are on different kinds of edge (a transpose, not a rotation). Exit and entry have the same length for every transition. The state is a plain resource (`MapTransition`) and not a Bevy sub state. The F2 box shows the current map id. The script tool has `goto:<map>:<entry>`.

**Rationale.** The order of load, rebuild and arrive matters, and all of it must happen while the screen is covered. Hooks run in one fixed order inside one frame, so no frame shows a half loaded map. Messages between separate systems would need system sets and sync points for the same result. The kit stays free of game types, because the game does the spawning and the facing. The game has no app states yet, and a sub state needs a parent state, so a resource carries the three phases. Keeping the position along the strip makes a wide door feel like one door. The transpose is easy to follow when editing maps: the start cell of an exit always maps to the start cell of its entry.

**How to apply.** Add `MapTransitionPlugin { sources, kinds, start, origin, fade }`, insert `MapHooks`. The kit tags every entity the `load` hook spawns with `MapId` under a `MapRoot` (`adr-x58`). Stop the player with `MapTransition::locks_input`. `reload_current_map` (F1) reads the current file again and runs `load` and `rebuild`. The first map is loaded in `PostStartup` after `rebuild_blockers`; use `collision::rebuild_blockers` as the `rebuild` hook, which clears the world first. Seamless transitions use the same lifecycle (`adr-x32`). See `dev-docs/extensions/map-transitions.md`.

## adr-x32. A seamless exit joins the next map into the world, the player crosses a doorway (2026-11-06)

**Decision.** A `transition=seamless` exit loads the target map beside the current one and does not fade or lock the player. The new map is placed on the edge of the exit, shifted along it until the exit strip and the entry strip lie on top of each other (`seamless_origin`). The maps are never turned, so the entry must be on the opposite edge (`adr-x30`). Both maps are in `ActiveMaps`, with a `Doorway` for the link, and the game hooks run as for a scene transition (`adr-x31`). The current map is the one that holds the feet, and it changes when the player crosses the seam. A map that is not the current one is removed after it has been out of the camera view for 1 second (`UNLOAD_AFTER_SECS`). The player may leave a map only through a doorway: `ActiveMaps::walkable` gives the map areas shrunk by the body size plus a bridge over each doorway, and `clamp_to_union` keeps a move inside them. It chooses among the areas that hold the old feet and the ones that touch them, so a long step cannot jump a wall. The camera shows the current map only (changed 2026-11-06, before this it showed the box around both maps). The camera limits are the current map, plus the joined map while the feet are in the exit that leads to it (`camera_target`), so the player sees ahead and is not cut off at the seam. `CameraBounds` is that rectangle at once. The camera glides: it keeps following the player exactly, and when the bounds change only the difference that the change adds is eased out (`CameraSmoothing`, time constant `camera_ease_secs`, Long Way 0.9 s). After the player crosses the seam the bounds are the new map alone, and the camera moves on until none of the old map is in view. A map that would overlap an active map is refused. The F3 grid is drawn once per active map with its own numbers (`DebugGrids`).

**Rationale.** The player walks on without a cut, and the two maps share one world space, so their collision, depth sort and camera work with no extra code. Collision is already in world units, so rocks and trees near the seam form clusters across it. The doorway rule keeps the map edges solid where there is no exit. Choosing the area by where the feet were stops a frame of lag from carrying the player through a wall into a map that is nearer than the one they were in. Maps that sit just outside the view are removed late so the one that was just loaded does not vanish before the camera reaches it. Easing the limits themselves was tried first. The camera follows the player at once, so it moved with the player and stopped when the player stopped, then crept on as the limits caught up: a quick move, a stop and a slow move. Easing out only the offset that the change of bounds adds gives one smooth glide that carries on while the player walks, and keeps the player in view. Labels at the top and right edge line of a map are not drawn, because the line belongs to the next map.

**How to apply.** Set `transition=seamless` on an exit whose entry is on the opposite edge and has the same length. A game uses `ActiveMaps::walkable` and `clamp_to_union` for the player instead of one map rectangle (Long Way: `move_player`). Every system that used the map rectangle must read `ActiveMaps`. `CameraBounds` is an eased box and only fits systems that use it for the camera: a game with enemies that read it for their area should not use it with seamless maps yet. The `rebuild` hook runs after every load and unload and must rebuild from all entities. Not built: an exit from the second map back to the first (the walk back works while the first map is still loaded), enemies and projectiles across a seam, and a layout of three maps around one map. `dev-docs/extensions/map-transitions.md`.

## adr-x33. An object can be an exit: `exit_to` names the map, the game makes the touch area (2026-11-07)

**Decision.** A legend entry may have the parameter `exit_to=<map id>` (`"m": marker exit_to=2`). The kit reads it (`MapObject::exit_to`) and checks it when the maps are registered: the map must exist and have exactly one `player_start`. The game adds an `ExitTrigger { area, to }` component to the entity of such an object (the kit adds its `MapId`, `adr-x58`). `detect_exit` tests the feet against the triggers of the current map as well as the exit areas, with the same `armed` rule. A touch sends `RequestMapChange` with `entry: None` and a scene transition. `entry: None` puts the player at the `player_start` of the target map and keeps their facing. A marker exit has no `transition` key and is never seamless. The kit also owns `start_feet(map, origin)` now (it was in Long Way). The script command `goto:<map>` uses the same arrival.

**Rationale.** Some games want a door in the middle of a map, not a strip on its edge. The file knows where the object leads, and only the game knows how big the touch area is, so each side keeps its part and the kit stays free of game types (`adr-x13`). A seamless link needs an exit strip to place the next map, so an object cannot start one. Arrival at `player_start` needs no entry strip, which matches maps that have one way in. The shared `armed` flag stops an arrival next to an exit object from sending the player back at once.

**How to apply.** Write `exit_to` on the legend entry, give the target map one `@`, and add `ExitTrigger` when the game spawns the object (Long Way: `markers.rs`). Do not put an exit object inside an exit area (rule 21 of `map-files.md`). `dev-docs/extensions/map-transitions.md`, `dev-docs/extensions/map-files.md`.

## adr-x36. A map file can describe its ground in a `background:` block, and the kit places the images (2026-11-08)

**Decision.** The front matter has an optional `background:` block with `color` (hex), a mandatory `mode` (`random` or `pattern`), and for `random` the keys `fill` (a chance per cell), `tiles` (image names), `jitter` (pixels), `scale` (any number above 0) and `seed`. The parser keeps it in `Map::background` and checks the syntax. `map_background::place_tiles` in the kit decides, for each cell, whether it gets an image, which image and where. The game draws the colour as a rectangle of the map size and the images above it, and checks the image names. `mode: pattern` is reserved for a repeating fill and is rejected as not implemented.

**Rationale.** The ground belongs to the map, so it is written in the map file, and a designer can tune it without code. A block leaves room for the pattern mode. `mode` has no default so that adding the pattern mode never changes how an old map reads. Each cell rolls its own dice from a hash of the map name, the `seed` and the cell, so the ground is the same on every load, F1 reload and platform, does not depend on the order of the cells, and keeps the images of old cells when a map grows. An exact count was rejected because the dice roll is simpler and a changing count does not matter for decoration. The hash is FNV-1a and SplitMix64 written in the kit, because `DefaultHasher` does not promise the same numbers on every platform, and a test fixes the result. The placement is in the kit at the developer's request. Only Long Way calls it today. The rectangle is a sprite per map and not `ClearColor`, because the kit can place several maps in one world (`adr-x30`, `adr-x32`). Each image has its own tiny z step (`layers.rs`), so overlapping images do not flicker.

**How to apply.** Rule 25 of `dev-docs/extensions/map-files.md`. A game loads the block in its map load hook, calls `place_tiles` with the drawn size of one image (art size times `scale`), checks the tile names at start and draws the result. Values for Long Way: `games/long-way/docs/adr.md` `adr-lw26`.

## adr-x37. Map legend characters follow a fixed convention (2026-11-08)

**Decision.** In a map legend, the player is `@`, features are letters (upper and lower case separate variants of one family, for example `G` and `g`), enemies are special characters (`! # $ % & * +`) and pickups are digits. `.`, `|` and a space stay reserved. The parser does not enforce this. Long Way rocks moved from `0` and `1` to `O` and `o` so that digits are free for pickups.

**Rationale.** A reader can tell the group of an object from its character alone, so a large map is easier to read. Pickups are the group that grows with the game design, and ten digits are enough for their kinds. Enforcing the groups in the parser would tie the kit to game concepts (`adr-x13`).

**How to apply.** `dev-docs/extensions/map-files.md`, "Legend conventions". Use these characters when a new kind is added to any legend. R-Town keeps its legend until the user asks.


## adr-x49. Map edges: the kit owns the edge rules, and each game chooses basic or smooth edges (2026-11-15)

**Decision.** With `".": edge` in the legend, each `.` is an edge cell (`dev-docs/extensions/map-files.md`, "Map edges"). The kit chooses a piece for each edge cell, reports the shapes it does not support, and gives the sprites, the blocked areas and the player start. A game passes `EdgeMode::Basic` (one piece per cell) or `EdgeMode::Smooth` (the basic pieces, then the quarter rules: cuts and fills drawn as half-size pieces). The game maps each piece to a frame, places the sprites the kit lists, and sets the draw order. Long Way uses `Smooth`.

**Rationale.** The rules are the same for every game, so they are tested once in the kit, and a game only maps pieces to frames. A spike showed that smooth edges fit this split: the game draws a list of sprites, each a frame at a position and a scale, sometimes one quarter of a frame, and the art needs no new frames. Each mode suits some art: smooth edges round the corners, while basic edges keep every pixel the same size and show the art as drawn. So the choice belongs to the game, not the kit.

**How to apply.** `crates/game-kit/src/map_edge.rs`: `edge_quarters(mode, ...)`, `edge_sprites`, `EdgeSprite::place`, `edge_blocked_rects` (grow by the actor half width) and `edge_start_feet`. In smooth mode, art that reaches past the half of a frame can show beside a fill.

## adr-x50. `.` can be an edge cell: the legend value, objects on the ring, footprints and the start (2026-11-15)

**Decision.**
1. The legend value of `.` is `empty` or `edge`, a value the kit knows. Any other value is an error. With `edge`, a `.` is an edge cell and a blank is free space.
2. An object on the outer ring of the map stands on an ordinary edge cell, whatever its kind: it gets a piece, its neighbours treat it as edge, and it blocks. An object inside the map, or in an entry area, stands on free space.
3. Footprints follow the file: a `.` footprint cell is an edge cell, so footprints inside the map are drawn with blanks.
4. Shapes the pieces cannot draw are load errors that name their line: a wall one cell thick, a single edge cell, a `.` in an exit or entry area, a map with no free cell.
5. The player start on an edge cell is the point nearest the cell centre outside the grown blocked area. On a `center` piece it is the nearest free cell, ties clockwise from north. Arrivals through exit objects use the same rule (`EdgeFit` on `MapTransitionPlugin`).

**Rationale.** A fixed legend value keeps a rule the kit owns free of game names. Objects on the ring are the edge, as the map border is, so the ring stays closed and there is one rule for every kind, `@` included. The kit does not know footprints (`adr-x13`), so the file decides. The two-step piece rule has no piece for a wall one cell thick, and an edge is always the border of the map, so such a cell is an error like other format errors. A start inside the blocked half would let the player walk into the wall, so the kit places it outside, and a `center` piece has no free point.

**How to apply.** `dev-docs/extensions/map-files.md`, rules 3, 8, 12, 20 and "Map edges" (rules 26 to 34). An object on the ring is blocked on one half: a pickup or an exit marker there must stay reachable. In Long Way, a marker on an `east` or `west` piece is touched only from the lower half of its cell.

## adr-x52. An edge tag with several frames is a set of variants, and the kit picks one per cell (2026-11-16)

**Decision.** A tag of the edge art may have more than one frame. Each frame is a variant of the piece and is not animated. The kit component `RandomStaticFrame` shows one frame of the tag, chosen from a seed, and stops the animation. A game seeds it with `RandomStaticFrame::for_cell(map_name, cell)`.

**Rationale.** Long edges built from one frame repeat in a visible pattern. Variants in the same tag need no new piece names and no code change when the artist adds a frame. Without a still frame, a tag with two frames would play as an animation. A seed from the map name and the cell keeps the edges the same on every load, like the background tiles (`adr-x36`) and `RandomStartFrame` (`adr-x39`). All sprites of one cell share the seed, so the quarters of a cut cell come from one frame.

**How to apply.** `dev-docs/extensions/map-files.md`, rule 31, and `dev-docs/extensions/pixel-art.md`, rule 19. The component works for any tag, not only edges.

## adr-x58. The kit puts everything a map's `load` hook spawns under a map root (2026-11-25)

**Decision.** Before the `load` hook runs, the kit spawns a `MapRoot` entity with the map's `MapId` and passes it in `MapLoad::root`. After the hook, every new top-level entity (not a Bevy system or observer) becomes a child of that root and gets the map's `MapId`. Removing a map despawns its root and so everything in it. The game no longer tags its map objects. It adds `MapId` by hand only to an entity it spawns later, outside the hook, that should go with the map.

**Rationale.** Each spawn site had to remember `MapId(load.id.clone())`, about twelve sites in Long Way, and a forgotten tag left an object in the world after its map was gone, with no warning (`adr-x56`). The kit knows when the hook runs, so it can tag by construction. Keeping `MapId` on each child means the code that reads it (collectables, exit triggers, the volcano) works unchanged. Adopting what the hook spawns, rather than asking the game to spawn under the root, also needs nothing at the spawn sites. The root sits at the world origin, so a child's `Transform` is still its world position.

**How to apply.** Spawn map objects in the `load` hook with no `MapId`. Anything spawned there belongs to the map: spawn persistent entities (the player, the camera) elsewhere. An entity spawned later that should go with a map (a shot from a volcano, a weapon effect) still needs a `MapId`. A test that calls a spawn function directly gets no adoption: check the objects, not their `MapId` (`test_util::map_load` in Long Way).

## adr-x59. The map lifecycle is a plugin of its own, and R-Town uses it (2026-11-25)

**Decision.** `map_transition` is a folder of modules: `registry` (map files and links), `lifecycle` (active maps, `MapId`, `MapRoot`, hooks, load, unload and the F1 reload), `transition` (exits, triggers, fades and the swap) and `seamless` (joined maps, removing maps out of view, camera limits). `MapLifecyclePlugin` is the lifecycle alone: it registers the maps, loads the start map and runs the hooks. `MapTransitionPlugin` adds itself on top of it. `MapHooks::arrive` is optional, and the origin of a lone load is the `MapOrigin` resource. R-Town uses `MapLifecyclePlugin` with one map and no `arrive` hook. Old `game_kit::map_transition::X` paths still work.

**Rationale.** The plugin was all or nothing, so R-Town (one map, no exits) wrote its own lifecycle: its own `MapEntity` tag, its own reload, and a second spawn list that every new kind had to be added to. One lifecycle means the map root, the adoption of what the load hook spawns (`adr-x58`) and the collision rebuild work the same in both games. The split also gives each module one job.

**How to apply.** A game with one map adds `MapLifecyclePlugin` and inserts `MapHooks` with `arrive: None`. Its `load` hook spawns everything the map needs, the player too when F1 should reset it (R-Town). An entity spawned later that should go with the map gets `MapId(active.current.clone())`. A game with exits adds `MapTransitionPlugin` only.
