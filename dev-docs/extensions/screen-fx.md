# Screen effects

Effects on the whole game view. The camera shake is the first one.

## Summary

- A game asks for an effect with a message and gives every value. The kit has no game values.
- `CameraShake { duration, distance, direction }` pushes the `GameCamera` by `distance` along `direction`, then to the other side, flipping each frame, for `duration` seconds of game time.
- The kit puts the camera back exactly when the shake ends. A camera that follows the player and a fixed camera both work.

## Status

| Part | Code today | Shared? |
|---|---|---|
| `ScreenFxPlugin`, `CameraShake`, `shake_offset` | `crates/game-kit/src/screen_fx.rs` | Shared. Jet uses the shake at its field edge (`adr-jt10`). |
| `CameraFollowSet` | `crates/game-kit/src/camera.rs` | Shared. The set of the camera follow, so effects run after it. |

Decision: `adr-x61`.

## Terms

- **Offset**: the distance the effect moves the camera away from where the game or the follow put it.
- **Forward frame**: a frame that pushes along `direction`. The first frame of a shake is a forward frame. The next pushes the other way, and so on.

## Parameters the game supplies

| Parameter | Meaning | Jet |
|---|---|---|
| `duration` | Seconds of game time | 0.1 |
| `distance` | World units (art pixels) | 2 |
| `direction` | The direction of the first push. Need not be a unit vector | The player's direction of travel |

## Specification

1. Add `ScreenFxPlugin`. Write a `CameraShake` message to start a shake.
2. A new shake replaces one that is running. When several are written in one frame, the last one wins.
3. The offset is `distance` along the unit direction, rounded to whole world units, so the pixel art stays on the pixel grid (`adr-x17`).
4. The shake lasts `duration` of game time (`Time`, so `Time<Virtual>`). A frame pushes while more than 1 ms of the time is left, so float noise in frame times does not add a frame: 0.1 s at 60 fps is six frames.
5. A paused frame (no game time passes) keeps the offset. It does not flip and uses no time. The shake goes on after the pause.
6. The offset is taken off in `First` and added again in `PostUpdate`, after `CameraFollowSet` and before the transform propagation. So:
   - game systems in `Update` see the camera where it really is (for example `visible_rect`);
   - the camera follow and its smoothing never see the offset, and the shake is added to the followed position;
   - when the shake ends, the camera is exactly where it would be with no shake.
7. A game never moves the `GameCamera` for an effect itself (`adr-x61`).

## Usage

```rust
app.add_plugins(game_kit::screen_fx::ScreenFxPlugin);

fn on_wall_hit(mut shake: MessageWriter<CameraShake>) {
    shake.write(CameraShake { duration: 0.1, distance: 2.0, direction: Vec2::X });
}
```

## Limits

- One effect only: the shake. A flash or a fade of the whole view goes in this module later.
- The shake moves the camera only. A HUD camera does not shake.
- A system that moves the camera in `PostUpdate` without `CameraFollowSet`, after the shake, would overwrite the offset.

## Tests

Inline in `crates/game-kit/src/screen_fx.rs`: the offset and its rounding, the flip, the exact end position, a new shake while one runs, a pause, and a shake on top of `CameraFollowPlugin` while the target moves.
