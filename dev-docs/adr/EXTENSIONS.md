# Extensions: decisions

Decisions about reusable code that Long Way implements first. They were `adr-lw2` to `adr-lw11` until 2026-11-02, when the prefix changed to `x` and the numbers stayed (`adr-lw2` is now `adr-x2`). Game only decisions are in `games/long-way/docs/adr.md` and `games/r-town/docs/adr.md`.

The decisions are split by area, all with the prefix `x`. This file holds `adr-x12` and the index of the others. To find a decision, search for its ID: `grep -rn "^## adr-x23" dev-docs/adr`.

## adr-x12. Shared code lives in one kit crate, `crates/game-kit` (2026-11-02)

**Decision.** Code that more than one game uses moves into the library crate `crates/game-kit`. One crate, not several. Games depend on it through `[workspace.dependencies]`. The kit has no assets and no game values.

**Rationale.** R-Town is the second game (adr-w6). One crate means one manifest and one import path. Splitting into small crates can happen later if compile times or dependencies demand it.

**How to apply.** A game supplies its values through plugin fields and marker components, not generics or Cargo features (`dev-docs/extensions/plugin-configuration.md`). Ask before adding game specific code to the kit.


## Index

| ID | Decision | File |
|---|---|---|
| x2 | Draw order sorted by feet | EXTENSIONS-SCREEN.md |
| x3 | Collision debug overlay | EXTENSIONS-DEBUG.md |
| x4 | Forgiving collision: `allow_sliding` | EXTENSIONS-COLLISION.md |
| x5 | Debug strategy: logging and scripted input | EXTENSIONS-DEBUG.md |
| x6 | Reusable visual debug tools in `debug.rs` | EXTENSIONS-DEBUG.md |
| x9 | Draw order layers | EXTENSIONS-SCREEN.md |
| x10 | Sliding along merged outlines, into notches | EXTENSIONS-COLLISION.md |
| x11 | Map file layout: Features and Map sections | EXTENSIONS-MAPS.md |
| x12 | Shared code lives in one kit crate, `crates/game-kit` | EXTENSIONS.md |
| x13 | Map object kinds are strings in the kit | EXTENSIONS-MAPS.md |
| x14 | Kit holds default layers, games override | EXTENSIONS-SCREEN.md |
| x15 | Scripted run tool is in the kit and extensible | EXTENSIONS-DEBUG.md |
| x16 | Collision blockers are a kit component, slide values scale with the cell size | EXTENSIONS-COLLISION.md |
| x17 | Camera follow in the kit, debug sizes scale with the cell size | EXTENSIONS-SCREEN.md |
| x18 | Map files are one drawn grid in a text file | EXTENSIONS-MAPS.md |
| x19 | Debug keys F2 info, F3 grid, F1 map reload | EXTENSIONS-DEBUG.md |
| x20 | Neighbour groups in the kit | EXTENSIONS-COLLISION.md |
| x21 | Projectiles are in the kit and collide as a ground point | EXTENSIONS-COLLISION.md |
| x22 | Input starts in the game and moves to the kit later | EXTENSIONS-INPUT.md |
| x23 | Path finding on a grid built from the collision edges | EXTENSIONS-COLLISION.md |
| x24 | Strategies are components that write a goal | EXTENSIONS-GAMEPLAY.md |
| x25 | Spawner settings are legend parameters | EXTENSIONS-GAMEPLAY.md |
| x26 | A seedable random number generator in the kit | EXTENSIONS-GAMEPLAY.md |
| x27 | Hit boxes, and projectiles hit at their flight height | EXTENSIONS-COLLISION.md |
| x28 | F1 sends `ReloadMap` from the kit, and `--debug` turns the debug tools on | EXTENSIONS-DEBUG.md |
| x29 | A corner contact between blocked areas is closed with a filler | EXTENSIONS-COLLISION.md |
| x30 | Exits and entries are front matter blocks, and `player_start` marks a start | EXTENSIONS-MAPS.md |
| x31 | A map transition is run by the kit through three game hooks | EXTENSIONS-MAPS.md |
| x32 | A seamless exit joins the next map into the world, the player crosses a doorway | EXTENSIONS-MAPS.md |
| x33 | An object can be an exit: `exit_to` names the map, the game makes the touch area | EXTENSIONS-MAPS.md |
| x34 | The window is split into screen regions, each with a camera and a UI root | EXTENSIONS-SCREEN.md |
| x35 | The kit knows the app flow states, and a game fills them in | EXTENSIONS-SCREEN.md |
| x36 | A map file can describe its ground in a `background:` block, and the kit places the images | EXTENSIONS-MAPS.md |
| x37 | Map legend characters follow a fixed convention | EXTENSIONS-MAPS.md |
| x38 | The kit has a touch trigger for collectable items, and the game keeps the inventory | EXTENSIONS-GAMEPLAY.md |
| x39 | The kit starts looping animations at a seeded random frame, and depends on `bevy_aseprite_ultra` | EXTENSIONS-SCREEN.md |
| x40 | `Sentry` is one strategy that patrols until it sees the player, then pursues for good | EXTENSIONS-GAMEPLAY.md |
| x41 | The kit reports touches on the controlled entity and plays facing tags | EXTENSIONS-GAMEPLAY.md |
| x42 | The kit turns a mouse click into a world position and sends it as a message | EXTENSIONS-INPUT.md |
| x43 | Walking to a clicked place: `Mover::feet_offset`, `NavInset` and `NavGrid::nearest_reachable` | EXTENSIONS-INPUT.md |
| x44 | A click on a UI panel over the game region is not a world click: `BlocksWorldClick` | EXTENSIONS-INPUT.md |
| x45 | The kit has an image button and a gauge for the HUD; the game handles the click | EXTENSIONS-SCREEN.md |
| x46 | The kit has a `Stunned` marker and a ray test against the collision outlines | EXTENSIONS-GAMEPLAY.md |
| x47 | The kit holds the particle code (`fx`), and builds particle sheets from `.aseprite` tags at run time | EXTENSIONS-SCREEN.md |
| x48 | The kit has area geometry and temporary blocked areas | EXTENSIONS-COLLISION.md |
