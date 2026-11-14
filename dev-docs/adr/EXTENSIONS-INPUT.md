# Extensions: input decisions

Pointer input and walking to a clicked place. Index of all extension decisions: `EXTENSIONS.md`.

## adr-x22. Input starts in the game and moves to the kit later (2026-11-04)

**Decision.** Input handling is a module of the game for now. R-Town has `src/input.rs` with a `PlayerInput` resource (move direction, fire) that is filled from the keyboard in `InputSet`. Gameplay systems read the resource and never `KeyCode`. The module moves to the kit when the mouse is added or a second game needs it (`rm8`).

**Rationale.** The resource isolates the device, so a mouse becomes a second writer of the same resource. A kit module now would have one user and a guessed API. The later move is a file move.

**How to apply.** Read `PlayerInput` with `.after(InputSet)`. The scripted run tool still works, because it writes key presses into `ButtonInput<KeyCode>` before the game reads them.

## adr-x42. The kit turns a mouse click into a world position and sends it as a message (2026-11-09)

**Decision.** `game_kit::pointer` has `PointerPlugin`, the message `WorldClick { position, button }` (world units) and `cursor_to_world`. A mouse button that goes down inside the game region of the window sends one message per button, converted through the `GameCamera`. A click on the HUD bar sends nothing. The system runs in `PointerSet` in `PreUpdate`, after `InputSystems`, so it uses the camera as it was drawn. The scripted run tool sends the same message with `click:<x>:<y>` (world units). The kit does not choose what a click does. This is the first step of `rm8`: there is still no `PlayerInput` resource, no key bindings and no touch.

**Rationale.** A message in world units is the smallest thing every game that clicks needs, and the conversion through the pixel camera and the letterbox is the part that is easy to get wrong, so it is written once (`adr-x12`). Running in `PreUpdate` matters because the camera follows the player in `Update`: a conversion after it would use a camera that is one step ahead of the picture the player clicked on. The script sends the message and not a fake window cursor, because a script has no window position. The alternative, a `PlayerInput` resource that hides the device (`adr-x22`), needs the keyboard rework of R-Town as well, and Long Way has no keyboard movement left, so it waits.

**How to apply.** `dev-docs/extensions/pointer.md`. Add `PointerPlugin`, read `MessageReader<WorldClick>` in `Update`.

## adr-x43. Walking to a clicked place: `Mover::feet_offset`, `NavInset` and `NavGrid::nearest_reachable` (2026-11-09)

**Decision.** (1) `Mover` has `feet_offset`, the distance of the feet below the `Transform` (as `DepthSort::feet_offset`), default 0, so an entity whose `Transform` is the middle of its sprite (the Long Way player) can use the mover. (2) `NavInset { min, max }` is an optional resource: the nav grid covers the map rectangle shrunk by it, so paths stay where the actor can stand. (3) `NavGrid::nearest_reachable(from, to)` gives `to` when a path exists, and otherwise the nearest grid point that `from` can walk to (a goal in a blocked area, outside the map, or enclosed). (4) `segment_blocked` now blocks a segment that starts exactly on the edge line at a convex end and heads on into the shape. The game keeps the walkable rule of its maps (`limit_player` applies `clamp_to_union` after the mover), and the kit mover keeps clamping to `CameraBounds` only.

**Rationale.** (1) and (3) are small additions that the ghost and R-Town do not notice. (2): the player stands half a body width from the sides and a body height from the top, but the grid ran to the map edge, where a zero width gap beside a tree block let a path use ground the player cannot reach, and the player stopped under the tree. Shrinking the grid is simpler than teaching the mover about margins on each side. (4) was found in the real game: from a box corner the straight line through the box was judged free and the player walked into the box (this was a roadmap item before stable IDs). The check looks 0.02 units along the segment, so a segment that leaves the corner along the outside, or away from the box, still passes, as `collision::resolve` allows. The click goes to the nearest reachable point because a click on a tree should still move the player, and one flood fill per click costs little. Keeping the map rule in the game follows `adr-x12`: a seam rule is a map rule, not a mover rule.

**How to apply.** `dev-docs/extensions/nav.md` (`NavInset`, `nearest_reachable`) and `dev-docs/extensions/enemy-movement.md` (`feet_offset`). `rm3` covers a better answer for maps that join.

## adr-x44. A click on a UI panel over the game region is not a world click: `BlocksWorldClick` (2026-11-11)

**Decision.** `game_kit::pointer` has the marker component `BlocksWorldClick`. A mouse press sends no `WorldClick` when the picking `HoverMap` shows the mouse pointer over an entity that has the marker, or whose parent chain has it. The cursor position now comes from the picking pointer of the mouse (`PointerLocation`) and not from the window, and `send_clicks` runs after `PickingSystems::Hover`. The marker goes on the root of a panel, not on every child.

**Rationale.** A panel drawn over the game region (the weapon drawer of Long Way) is inside the viewport of the `GameCamera`, so the rule "the HUD sends nothing" does not cover it, and a click on a button would also walk the player. A marker keeps the choice with the game: the kit does not know what a panel is, and a UI node that should let clicks through (the F2 info box) has no marker. Reading the `HoverMap` uses the hit test of Bevy picking, so there is no second hit test to keep in step with the layout. The pointer location replaces `Window::cursor_position` so that the scripted run tool can place the pointer with the same input messages as a real mouse (`ui_click`), and a test then goes through the same path as a player. For a real mouse both are the same position.

**How to apply.** `dev-docs/extensions/pointer.md`. Add `BlocksWorldClick` to the root node of the panel.
