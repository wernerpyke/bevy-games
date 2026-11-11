# Development feedback

How behaviour is checked when Claude cannot play the game: plain function tests, logging, scripted input and screenshots.

## Summary

1. Game rules are plain functions with unit tests.
2. Game code logs state changes, not every frame.
3. A scripted run presses keys in the real game and logs the result.
4. A script command saves a screenshot that Claude opens.

The order of work for a behaviour that fails in the real game is in `CLAUDE.md`, working rule 9 (`adr-x5`).

## Status

| Part | Code today | Shared? |
|---|---|---|
| Scripted input | `crates/game-kit/src/script.rs`, `dev-scripts/rust.sh` | Shared. Game specific commands are not supported yet (adr-x15) |
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

8. An environment variable (`<PREFIX>_SCRIPT`, for example `LONG_WAY_SCRIPT`, set by `ScriptPlugin { env_prefix }`) switches the plugin on. Without it the plugin does nothing.
9. Commands are separated by `;`:

| Command | Effect |
|---|---|
| `teleport:<feet_x>:<feet_y>` | Moves the entity marked `Controlled` (its feet come from its `DepthSort`). Skipped with a log line when there is no such entity |
| `hold:<keys>:<seconds>` | Holds keys (`up`, `down`, `left`, `right`, `x`, `escape`, `f1` to `f3`), for example `hold:up+left:2`. `hold:x:0.1` is one key press |
| `wait:<seconds>` | Waits |
| `screenshot:<name>` | Saves the window to `target/screenshots/<name>.png` (folder set by `<PREFIX>_SCREENSHOT_DIR`) |
| `goto:<map>:<entry>` | Starts a scene transition to an entry of a registered map and waits until it is done. The player is put in the middle of the entry (`map_transition.rs`) |
| `quit` | Exits |

10. The plugin logs the feet position every 0.25 seconds. When the controlled entity is gone (the player died) it logs `none` and the script goes on. It runs in `PreUpdate` after Bevy's input systems and writes keys with `press` and `release`.
11. Add a `wait` before `screenshot` after a `teleport`.
11b. A scripted run sets `GAME_KIT_SKIP_TITLE`, so the title screen is skipped, and the script clock uses real time, so it goes on while the game is paused (`screen-layout.md`).
12. The wrapper `rust.sh script` shows only log lines that match `LOG_FILTER` (default: `script:`, `collision:`, warnings and errors). Set `LOG_FILTER="projectile:|enemy:|script:"` to see the lines of a feature. `<PREFIX>_SEED` (for example `R_TOWN_SEED=3`) fixes the random numbers of the game (`adr-x26`). A run that needs more than the 30 s limit of the wrapper can start the binary directly with `CARGO_MANIFEST_DIR` set to the game folder and `perl -e 'alarm 60; exec @ARGV'`.

### Flicker check

13. Take 12 screenshots in a row (`screenshot:` with `wait:0.05` between) and count changed pixels between neighbours. Any non zero count in a static area is a flicker.

## Usage

```
./dev-scripts/rust.sh script long-way "teleport:225:-150;hold:up:3;quit"
./dev-scripts/rust.sh script long-way "wait:0.5;click:-704:-96;wait:4;screenshot:walk;quit" --debug
./dev-scripts/rust.sh script long-way "wait:0.5;screenshot:start;quit"
./dev-scripts/rust.sh test long-way
```

The script run keeps log lines that match `script:|collision:|WARN|ERROR|panicked`. Set `LOG_FILTER` to another pattern (for example `rocks:`) to see more. The game opens its window while the script runs.

## Limits

- Keys: the arrow keys, `x`, `escape` and `f1` to `f3`. `click:<x>:<y>` sends a left click as a world position (`pointer.md`), so a game that moves by clicking can be scripted. It does not move a real cursor, so the window to world conversion is not tested by a script. `ui_click:<x>:<y>` clicks the UI at a window position in art pixels from the top left (for example `ui_click:66:588` is the red energy icon in Long Way): it moves the picking pointer, presses and releases, through the same path as a mouse, so buttons, hover and `BlocksWorldClick` work. It sends no `WorldClick` unless the node under it lets the click through. A real cursor over the window cannot disturb it. The wrapper `rust.sh script` prints only lines that match `script:|collision:|WARN|ERROR|panicked`: set `LOG_FILTER="<pattern>"` to see other log lines.
- It teleports the entity marked `Controlled`, which also needs a `DepthSort`. A game specific command needs a registration API in the kit (adr-x15).
- Screenshots work on native builds only.
- Unit tests do not cover frame timing, drawing or input.
