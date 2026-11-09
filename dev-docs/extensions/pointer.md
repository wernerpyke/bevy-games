# Pointer (mouse clicks)

## Summary

The kit turns a mouse click into a world position and sends it as a message. A game reads the message and decides what a click does. Long Way uses it to set the goal of the player (`adr-lw30`).

## Status

`crates/game-kit/src/pointer.rs`, used by Long Way. Decision: `adr-x42`. Ready to share. Only the mouse is covered: no touch, no key bindings, no `PlayerInput` resource (`rm8`).

## Terms

- **Game region.** The part of the window that the `GameCamera` draws (`screen-layout.md`). The HUD bar is outside it.
- **World click.** A mouse button going down inside the game region, with the cursor position in world units.

## Specification

1. `PointerPlugin` adds the message `WorldClick { position: Vec2, button: MouseButton }` and the system `send_clicks` in `PointerSet`, in `PreUpdate` after `InputSystems`.
2. For every button that went down this frame, one `WorldClick` is sent, all at the same position.
3. The position is `Camera::viewport_to_world_2d` of the `GameCamera` at the window cursor position (logical pixels). Bevy subtracts the viewport origin, so the cursor position of the window is passed as it is.
4. A cursor outside the viewport of the `GameCamera` (the HUD bar, the letterbox) sends nothing. `pointer::cursor_to_world` returns `None` there.
5. No window, no cursor (it left the window) or no `GameCamera`: nothing is sent.
6. The system runs in `PreUpdate`, so the camera `GlobalTransform` is the one that was drawn. The camera follows the player in `Update`, so a conversion after it would be one step off while the player walks.
7. The scripted run tool sends `WorldClick` with `click:<world_x>:<world_y>` (left button), in `PreUpdate` after `InputSystems`.

## Usage

- Add `PointerPlugin`. Read `MessageReader<WorldClick>` in `Update`. Order a system `.after(PointerSet)` only if it runs in `PreUpdate`.
- Long Way: `set_goal` in `player.rs` takes the last left click of the frame and sets `MoveGoal`.

## Limits

- Touch taps (the web build on a phone) send nothing yet.
- Holding a button sends one message, on the frame it goes down.
- Only the primary window.

## Tests

`pointer.rs`: the viewport rule (corners, the HUD bar) and no message without a window. The conversion through a real camera and window was not tested by a program: check it by clicking in the game, including a click on the HUD bar. `script.rs` tests the `click:` command.
