# Development feedback

How behaviour is checked when Claude cannot play the game: plain function tests, logging, scripted input and screenshots.

## Summary

1. Game rules are plain functions with unit tests.
2. Game code logs state changes, not every frame.
3. A scripted run presses keys in the real game and logs the result.
4. A script command saves a screenshot that Claude opens.

The order of work for a behaviour that fails in the real game is in `CLAUDE.md`, working rule 10 (`adr-lw5`).

## Status

| Part | Code today | Shared? |
|---|---|---|
| Scripted input | `games/long-way/src/script.rs`, `dev-scripts/rust.sh` | Needs per-game keys, or a shared version |
| Logging | `collision.rs` and others | Pattern only |
| Plain function tests | inline in each module | Pattern only |

## Specification

### Rules as plain functions

1. Decision logic lives in functions that take plain values and return plain values, and use only `Vec2` and `Rect` from Bevy. They run without a window or an `App`.
2. Systems gather inputs, call the function and write the result back.
3. Tests are inline at the bottom of the module (`#[cfg(test)] mod tests`). Long Way is a binary crate, so a `tests/` folder could not reach private functions.

### Logging

4. `debug!` for game logic transitions (blocked, sliding started, no longer pushing). `info!` for script progress.
5. The module path is the filter: `RUST_LOG=long_way=debug`. Use `RUST_LOG=info,wgpu=error,naga=warn` to quiet Bevy.
6. State used only to log once (such as `SlideAssist.phase`) is kept next to the logic.
7. Check the log volume before adding any `info!` that runs every frame.

### Scripted input

8. An environment variable (`LONG_WAY_SCRIPT`) switches the plugin on. Without it the plugin does nothing.
9. Commands are separated by `;`:

| Command | Effect |
|---|---|
| `teleport:<feet_x>:<feet_y>` | Moves the entity marked `Player` |
| `hold:<keys>:<seconds>` | Holds keys, for example `hold:up+left:2` |
| `wait:<seconds>` | Waits |
| `screenshot:<name>` | Saves the window to `target/screenshots/<name>.png` (folder set by `LONG_WAY_SCREENSHOT_DIR`) |
| `quit` | Exits |

10. The plugin logs the feet position every 0.25 seconds. It runs in `PreUpdate` after Bevy's input systems and writes keys with `press` and `release`.
11. Add a `wait` before `screenshot` after a `teleport`.

### Flicker check

12. Take 12 screenshots in a row (`screenshot:` with `wait:0.05` between) and count changed pixels between neighbours. Any non zero count in a static area is a flicker.

## Usage

```
./dev-scripts/rust.sh script long-way "teleport:225:-150;hold:up:3;quit"
./dev-scripts/rust.sh script long-way "wait:0.5;screenshot:start;quit"
./dev-scripts/rust.sh test long-way
```

The script run keeps log lines that match `script:|collision:|WARN|ERROR|panicked`. Set `LOG_FILTER` to another pattern (for example `rocks:`) to see more. The game opens its window while the script runs.

## Limits

- Arrow keys only.
- It teleports the entity marked `Player`, so each game needs its own `script.rs` or a shared version.
- Screenshots work on native builds only.
- Unit tests do not cover frame timing, drawing or input.
