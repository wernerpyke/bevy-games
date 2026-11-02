# Pixel art rendering and animation

How art size on screen is set, and how Aseprite tags drive animation.

## Summary

- The camera shows a fixed height of world units. Art scale on screen comes from that number, not from the window size or from scaling sprites.
- A character state maps to an Aseprite tag name. The animation switches only when the tag changes.

## Status

| Part | Code today | Shared? |
|---|---|---|
| View height | `games/long-way/src/camera.rs` | Yes, one constant per game |
| Tag switching | `games/long-way/src/player.rs` | Pattern only |

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

## Usage

- Change `VIEW_HEIGHT` in the camera module to change how large everything looks.
- For a character, keep a `playing: &'static str` field and write `if wanted != player.playing { ase.animation.play_loop(wanted); ase.animation.speed = speed; player.playing = wanted; }`.

## Limits

- Not whole number scales (for example a 720 high window against 1200 units gives 0.6) can make pixels uneven. A render target at a fixed low resolution is the alternative and is not built.
- The scale and speed constants are Long Way values. See the game docs.
