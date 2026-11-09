# Screen layout, HUD and app states

How the window is shared between the game and a HUD, and how the app flow (loading, title, playing, pause, game over) changes what is on screen.

## Summary

- The window is split into regions. Each region has a camera and a UI root. A game with a HUD bar has a game region and a HUD region.
- The split is a pure function of the window size and a mode (`regions`), so it is tested without a window.
- UI is written in art pixels. `UiScale` makes one UI unit as large as one art pixel of the game view.
- The kit knows the app states `Loading`, `Title`, `Playing` and `GameOver`, and pause inside `Playing`. A state chooses the layout and which UI roots exist. The kit knows nothing about what a title screen shows.

## Status

| Part | Code today | Shared? |
|---|---|---|
| `GameCamera`, `PixelCameraPlugin` | `crates/game-kit/src/camera.rs` | Shared. |
| `ScreenMode`, `regions`, `ui_scale`, `ScreenPlugin`, `ScreenLayout` | `crates/game-kit/src/screen.rs` | Shared. Long Way uses a bar above the game. |
| `Hud`, `HudCamera`, `label` | `crates/game-kit/src/hud.rs` | Shared. |
| `AppState`, `PlayState`, `AppFlowPlugin`, `state_root`, `PauseDialog` | `crates/game-kit/src/app_state.rs` | Shared. Long Way starts in `Playing` and has no other screens. R-Town does not use it yet. |

Decisions: `adr-x34`, `adr-x35`, and `adr-lw24` for Long Way.

## Terms

- **Region**: a rectangle of the window in physical pixels, origin at the top left: `full`, `game`, and `hud` (none when there is no HUD).
- **Bar mode**: the HUD has its own region above (`BarTop`) or below (`BarBottom`) the game region. `ratio` is the bar height as a share of the game height.
- **Overlay mode**: the game region is the whole window and the HUD is drawn over it. The layout is computed and tested. The HUD root is transparent. It is not used by a game yet.

## Parameters the game supplies

| Parameter | Where | Long Way |
|---|---|---|
| View height in art pixels | `PixelCameraPlugin::view_height` | `VIEW_HEIGHT` 512 |
| Layout while playing | `ScreenPlugin::mode` | `BarTop { ratio: HUD_BAR_RATIO }`, ratio 0.3125 (bar 160 art pixels, 576 by 180 on screen) |
| Window size | the game's `WindowPlugin` | `WINDOW_WIDTH` 576, `WINDOW_HEIGHT` 756 (`camera.rs`) |
| HUD content | children of the `Hud` entity | one `label`, `Long Way` |
| First state, pause key, dim colour | `AppFlowPlugin` | `Playing`, no pause key, default dim |

## Specification

### Regions and cameras

1. `regions(window, mode)` returns the rectangles in physical pixels. A bar is the full window width and `ratio` times the game height high. The bar and the game fill the window height with no gap (the bar is rounded to whole pixels).
2. The `GameCamera` marks the camera that shows the world. Every kit system that needs the world camera uses `With<GameCamera>`. Never use `With<Camera2d>`: a second `Camera2d` makes `Single<.., With<Camera2d>>` match two entities, and Bevy skips the system without a warning.
3. `ScreenPlugin` sets the `viewport` of the game camera, spawns the `HudCamera` (order 1, no clear, render layer 1, so it draws UI only) and the `Hud` root, and updates both when the window, its scale factor or `ScreenLayout::mode` changes.
4. The game camera is the default UI camera (`IsDefaultUiCamera`). A UI root without `UiTargetCamera` (the debug info box, the map fade, a title screen) is therefore in the game region. The `Hud` root targets the HUD camera.
5. `UiScale` is `game region height in logical pixels / view height`. It applies to all UI, so every kit and game UI is written in art pixels.
6. `half_view` takes the size of the region, not of the window.

### HUD

7. The `Hud` root fills the HUD region and draws no background. A game puts its own background (a picture or a colour) inside it or on it.
8. A game adds its content as children of the `Hud` entity, for example in a `PostStartup` system with `Single<Entity, With<Hud>>`. The kit has generic widgets only (`label`). Icons and gauges are added when a game needs them.

### App states

9. `AppState` is `Loading`, `Title`, `Playing` or `GameOver`. `PlayState` (`Running`, `Paused`) is a sub state of `Playing` and does not exist in the other states.
10. `AppFlowPlugin::start` is the first state. A game that loads assets starts in `Loading` and sends `LoadingFinished` when it is ready: the state goes to `Title`.
11. The layout follows the state. `Loading`, `Title` and `GameOver` use `ScreenMode::Full` (the HUD camera is off, the game region is the whole window). `Playing` uses `ScreenPlugin::mode`. Without `ScreenPlugin` only the states and pause exist.
12. A screen is a UI root made with `state_root(state, background)` in `OnEnter(state)`. It fills the game region, is drawn above the HUD, and is removed when the state is left. A game that shows a title over a world should note that the game camera still draws behind an opaque root.
13. Pause (key `pause_key`, `Escape` by default, `None` for no key): on entering `Paused` the virtual clock stops (`Time<Virtual>::pause`), a dim layer covers the game region and the HUD, and an empty, centred `PauseDialog` is added in the game region. On leaving `Paused` for any reason, including leaving `Playing`, the clock runs again and all three are removed.
14. While paused the dim layer blocks picking below it. The game fills the `PauseDialog` (`Added<PauseDialog>`). Systems that use `Res<Time>` in `Update` freeze with the virtual clock (movement, the camera glide, the map fade and its timers). The scripted run tool uses real time, so a script keeps running while the game is paused. Systems that must stop for another reason use `run_if(in_state(PlayState::Running))`.
15. Skipping the title is for development. `./dev-scripts/rust.sh run <game> --skip-title` sets `GAME_KIT_SKIP_TITLE`, and `script` always sets it. The skip is used once: at the start when `start` is `Title`, or when loading finishes. A later visit to the title is shown.

## Usage

1. Add `PixelCameraPlugin`, then `ScreenPlugin { mode }`, then `AppFlowPlugin::new(start)`.
2. Set the window size so that the game region has the shape you want. At scale factor 2 the window must fit the monitor (macOS shrinks a taller window, and the game region then shows more than the view). Long Way: 576 by 756.
3. Spawn the HUD content under `Hud`. Spawn each screen with `state_root` in `OnEnter`.
4. Convert a mouse position with the game camera and its viewport (`Camera::viewport_to_world_2d`). Nothing does this yet.

## Limits

- The overlay mode is computed but not used by a game.
- There is no title, game over or pause content in any game. Long Way starts in `Playing` and has no pause key.
- A free window shape, monitor fitting and fullscreen are not handled: roadmap item 13.
- The pause dialog is in the game region, not over the whole window.
- Kit systems are not gated by state. In `Title` or `GameOver` a game must not spawn the player, or the kit systems that need it are skipped.
- Not tried: rendering the game into a UI node with `ViewportNode` (one UI tree for every mode). It is not checked on wasm.

## Tests

- `screen.rs`: the regions for each mode and at odd sizes, the UI scale, and the plugin setting both cameras and `UiScale`, and switching the HUD camera off.
- `app_state.rs`: the start state with and without the skip, loading to title or game, the skip used once, the layout for each state, screen roots removed with their state, pause (clock, dim layers, dialog slot, HUD dim), leaving the game while paused, and no pause key.
- `camera.rs`: a second camera does not stop the game camera following.
