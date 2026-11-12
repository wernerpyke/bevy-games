# Pixel art rendering and animation

How art size on screen is set, and how Aseprite tags drive animation.

## Summary

- The camera shows a fixed height of world units. Art scale on screen comes from that number, not from the window size or from scaling sprites.
- A character state maps to an Aseprite tag name. The animation switches only when the tag changes.

## Status

| Part | Code today | Shared? |
|---|---|---|
| Camera: `PixelCameraPlugin { view_height, clear_color }`, `CameraFollowPlugin`, `CameraBounds`, `visible_rect`, `half_view` | `crates/game-kit/src/camera.rs` | Shared |
| `VIEW_HEIGHT`, `CELL_SIZE`, `MAP_ORIGIN` | `games/long-way/src/camera.rs` | One set of values per game |
| Tag switching: `play_if_changed`. Spawn helpers: `tag_sprite`, `tag_animation`, `all_frames_sprite`, `all_frames_animation` | `crates/game-kit/src/animation.rs` | Shared (`adr-x39`) |
| Still frame: `StaticFrame` |  `crates/game-kit/src/animation.rs` | Shared (`adr-x39`) |
| Random start frame: `RandomStartFrame`, `AnimationHelpPlugin` | `crates/game-kit/src/animation.rs` | Shared (`adr-x39`) |

Decision: `adr-e7` for the camera. Art pipeline: the game's `art-pipeline.md` and `art-tags.md`.

## Specification

### View height sets on-screen scale

1. The camera uses `ScalingMode::FixedVertical { viewport_height: VIEW_HEIGHT }`. Width follows the window shape.
2. World units equal art pixels. A sprite that is 60 pixels tall is 60 units tall.
3. On-screen size is `art height / VIEW_HEIGHT` of the screen height. A bigger window does not make a sprite smaller. A bigger `VIEW_HEIGHT` does.
4. A window whose height is not `VIEW_HEIGHT` scales the art. Scales that are not whole numbers can make pixels uneven.
5. Sprites are not resized in the engine, except where a game states a whole number scale for one asset (for example a 32x32 rock drawn at 4x).

### Animation tag switching

6. Tag names in the `.aseprite` file are the contract between art and code. They are case sensitive. A tag that does not exist is an error.
7. The character component keeps the current tag name. Code compares it with the wanted tag and calls `play_loop` only when it differs. Calling `play_loop` every frame restarts the animation.
8. Playback speed is set in the same branch (`ase.animation.speed`).
9. A file with no tags plays all frames with `Animation::default()`.
10. A tag may also name one frame, to pick a variant (a rock type) and not to animate.
11. Tags must not overlap. A script that makes tags makes them after all frames exist, then `./dev-scripts/aseprite.sh check <file>` shows the frame ranges to read (`games/long-way/docs/art-pipeline.md`, rule 9).

### Animation speed

11. The timing of an animation is art data. Set it as frame durations in the `.aseprite` file (Aseprite: select the frames, Frame Properties, Duration). The file is then the one place that says how fast a tag plays, and the Aseprite preview matches the game. Per frame and per tag timing is possible only this way.
12. Do not slow down or speed up a fixed animation with `Animation::with_speed` or `ase.animation.speed`. A code constant that assumes the durations in the file breaks without a message when the art changes. Use `speed` only when the rate depends on the game state (for example a run animation that follows the walk speed, rule 8).
13. Check durations with `./dev-scripts/aseprite.sh check <file>` (tags, frames and ms) and set them with `./dev-scripts/aseprite.sh set_durations <file> tags=<a,b|all> ms=<n> [apply=1]` (`dev-scripts.md`). It is a dry run until `apply=1`. Close the file in Aseprite before applying, and note that it overwrites uncommitted art changes in that file.
14. A tag loops by default (`AnimationRepeat::Loop`). Say so in the code only when a tag must play once or a set number of times.

### Random start frame

15. Copies of one looping animation (trees, pickups) move in step and look uniform. Add `RandomStartFrame { seed }` to the entity with its `AseAnimation`. When the Aseprite file has loaded, `AnimationHelpPlugin` moves the animation to a frame of its tag (all frames when it has no tag) picked by the seed, and removes the component.
16. Use `RandomStartFrame::for_cell(map_name, cell)` for map objects. The seed comes from the map name and the cell, so the frame is the same on every load, on F1 reload and when the map is entered again. Two objects in different cells differ.
17. It sets the start only. Every copy then runs at the speed set in the file (rule 11). It does not work for a one-shot animation.

### Still frame

18. `StaticFrame { relative }` shows one frame and never plays, for an icon. It sets the frame (counted from the first frame of the tag, kept inside the range) once the file has loaded, then stops the animation. Long Way draws its weapon icons and the frames of its HUD buttons this way.

## Usage

- Add `PixelCameraPlugin { view_height, clear_color }` with the game's own `VIEW_HEIGHT` constant. Change that constant to change how large everything looks. The camera is spawned at the world origin. A game that scrolls adds `CameraFollowPlugin` and inserts `CameraBounds(Rect)` once the map size is known. The camera follows the `Controlled` entity, stops at the bounds and is rounded to whole world units (adr-x17).
- Add `AnimationHelpPlugin` (and `RandomStartFrame` on entities) to vary the start frame of copies of one animation. Long Way does this for trees and pickups.
- For a character, call `play_if_changed(&mut ase, tag, speed)` every frame with the tag the state asks for. It switches (and sets the speed) only when the tag differs from the one playing, so the animation does not restart. Long Way: `animate_player`.
- Spawn with `tag_sprite(&server, path, tag)` (a tag, plus a default `Sprite`) or `all_frames_sprite(&server, path)` (a file without tags). Use the `_animation` variants when the entity has its own sprite or an `ImageNode`. Paths are `&'static str`.

## Limits

- Not whole number scales (for example a 720 high window against 1200 units gives 0.6) can make pixels uneven. A render target at a fixed low resolution is the alternative and is not built.
- The scale and speed constants are Long Way values. See the game docs.
