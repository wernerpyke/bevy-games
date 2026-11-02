# Debug tools

Visual debug tools drawn in the game: a grid, an info box and a collision overlay.

## Summary

- **Grid.** A red grid with one line per cell, numbered along the edges.
- **Info box.** A text box where any system shows live values.
- **Collision overlay.** The collision outline, interior shapes and feet markers, drawn as sprites.

All tools are sprites or UI on named layers ([draw-order.md](draw-order.md)), not gizmos, because gizmos always draw on top.

## Status

| Tool | Code today | Shared? |
|---|---|---|
| Grid and info box | `crates/game-kit/src/debug.rs` | Shared, configured by `DebugPlugin` fields |
| Collision overlay | `crates/game-kit/src/collision_debug.rs` | Shared, uses the collision extension |

Decisions: `adr-x3`, `adr-x6`, `adr-x9`, `adr-x12`, `adr-x14`.

## Specification

### Grid (key F2)

1. Lines at every `cell_size`, in `grid_color`, over the area the camera shows (the `Camera2d` with an orthographic projection). Thin sprites on `grid_z` (default `GRID_DEBUG_Z`). The grid follows a scrolling camera: it is rebuilt when the camera crosses a line.
2. Column numbers stay on the top edge and row numbers on the left edge of the view, also while the camera moves.
3. `grid_origin` sets where (0, 0) is: `Center`, `BottomLeft` or `World(Vec2)`. `BottomLeft` is the corner of the visible area, which moves when the window is resized or the camera moves. Use `World(Vec2)` for a fixed origin.

### Info box (key F3)

4. A text box at the top left, in the HUD layer (`hud_z_index`). It shows the grid position, cell and world position of the feet of the entity with the `Controlled` component, named by `target_name` (Long Way: "guy").
5. A system shows a value with `ResMut<DebugInfo>` and `info.set("label", value)`. Rows keep the order of first use.

### Collision overlay (follows F2)

6. Yellow lines are every outline edge, with a tick on the blocked side. Cyan boxes are interior shapes. Green squares are the feet of every `DepthSort` entity.
7. All are sprites under one root entity with a `Visibility`, on the feature debug layers. The overlay is drawn once in `PostStartup`, after every blocked area is registered.

## Usage

- Add `Controlled` to the player entity. Add `DebugPlugin::new(cell_size, grid_color, grid_origin)` (set `target_name`, `grid_z` or `hud_z_index` with struct update syntax) and `CollisionDebugPlugin::default()` (fields `outline_z`, `interior_z`, `feet_z`).
- Report a value from any system through `DebugInfo`.
- Add a new tool as a new system in the kit `debug.rs`, with a key toggle in `toggle_tools` and a field in `DebugSettings`.

## Limits

- Not seen on screen by Claude except through scripted screenshots ([dev-feedback.md](dev-feedback.md)).
- The overlay is static. Areas added after `PostStartup` are not shown.
- Remove the tools or gate them behind a debug feature before a release build.
- With a bottom left origin, the corner labels overlap (roadmap item 99).
