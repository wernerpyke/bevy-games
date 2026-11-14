# Extensions: screen, draw order and animation decisions

Screen regions, app states, camera, draw order layers and animation. Index of all extension decisions: `EXTENSIONS.md`.

## adr-x2. Draw order sorted by feet (2026-10-31, amended 2026-11-14: ties break at the footprint centre)

**Decision.** Entities with a `DepthSort` component get their z set from the y position of their feet: the lower the feet on screen, the closer to the camera.

**Rationale.** The player must draw behind a tree when its feet are above its base and in front when below. A fixed z cannot do both. The same sort line also decides collision, so what the player sees matches where it can walk.

**How to apply.** Give any sprite that should sort a `DepthSort { feet_offset }`, the distance from its centre down to its feet. `depth.rs` sets `z = -feet_y * 0.001`. Sprites with the same feet y are ordered by x (`z += x * 1e-8`), the one further right in front, because equal z flickers (found with the rock rows, 2026-11-01). The x is the middle of the footprint: a sprite whose `Transform` is elsewhere (anchored at a corner) adds `TieBreakX(offset)`. Without it a corner-anchored object and a centred one can share the same x by chance and tie: on 2026-11-14 a Long Way medium tree (bottom-left anchor) and the big tree one cell to its left had exactly the same z, and their order changed with Bevy's batching (it flipped once per run, and not with the F3 overlay on). The right-in-front rule suits a top-down RPG (the user, 2026-11-14). Another game may want a different tie rule; make it a `DepthPlugin` setting then. The sort runs in `Update` with no ordering against movement, so z can lag one frame.

## adr-x9. Draw order layers (2026-11-01)

**Decision.** Everything on screen belongs to a named layer, from back to front, defined in the kit's `layers.rs` (`crates/game-kit`), which a game may override or extend (`adr-x14`, changed 2026-11-02; before this each game had its own file). Sprite layers are bands of z. HUD and UI layers use `GlobalZIndex`. Features and characters share one world layer, sorted together by their feet (`depth.rs`). Debug drawings have their own layers above the world. The layers Long Way uses are in `adr-lw12` (`games/long-way/docs/adr.md`).

**Rationale.** Bevy has no layer system for 2D draw order: sprites are sorted by z, equal z has no fixed order, `RenderLayers` only picks which camera sees an entity, and 2D gizmos are always drawn last, on top of everything. The developer asked for formal layers on 2026-11-01. Features and characters share one layer because a character must walk behind a tree or rock when its feet are above the feature's sort line (`adr-lw1`, `adr-x2`). Separate layers would stop that. The debug layer is above the world, so debug lines also draw over the player.

**How to apply.** Never write a bare z number outside the kit's `layers.rs` or the game's own layers module (`adr-x14`). `bevy_ui` draws after all sprites and gizmos, so the HUD and UI are always above the feature debug layer. A background must use a z below `GRID_DEBUG_Z`. `depth.rs` has a debug assertion that feet stay inside the world band (y within about 100000). Debug drawings that must follow the layers are sprites, not gizmos. The collision overlay in `collision_debug.rs` follows this rule.

## adr-x14. Kit holds default layers, games override (2026-11-02)

**Decision.** The kit has a `layers` module with the default z values. Plugins take the values from plugin fields that default to those constants. A game overrides a value or adds its own layers in its own module. This changes adr-x9, which said every z number is in one file per game.

**Rationale.** Both games start with the same draw order without copying it. A game that needs another layer can add it.

**How to apply.** Still never write a bare z number in game code: use a name from the kit or the game's layers module. Keep the compile-time ordering checks in the kit. A game that overrides a value must keep the checks true.

## adr-x17. Camera follow in the kit, debug sizes scale with the cell size (2026-11-02)

**Decision.** `CameraFollowPlugin` moves the `Camera2d` to the `Controlled` entity and stops it at `CameraBounds` (a resource a game inserts once the map size is known). The camera position is rounded to a whole world unit. The debug grid and the collision overlay take their line thickness, tick length, label size and feet marker size from base values tuned for a cell size of 64, multiplied by `DebugSettings::scale` (cell size over 64).

**Rationale.** A scrolling camera is needed by every game with a map larger than the view, and rounding keeps pixel art from shimmering when it scrolls. The debug sizes were world units chosen for Long Way's 1200 unit view. At R-Town's 240 unit view they were five times too large on screen.

**How to apply.** Insert `CameraBounds` in the plugin that loads the map. Do not move the camera elsewhere, or the follow system overwrites it each frame.

## adr-x34. The window is split into screen regions, each with a camera and a UI root (2026-11-07)

**Decision.** The kit owns a `ScreenLayout`: a set of named rectangles of the window (`Game`, `Hud`, `Full`), computed by a pure function from the window size, the layout mode and the bar size. Each region used by a layout has a `Camera2d` with a `viewport` and a UI root that targets that camera (`UiTargetCamera`). The kit marks the world camera `GameCamera`, the HUD camera `HudCamera`, and every kit query for the world camera uses `GameCamera`. Two layout modes: **bar** (the HUD has its own rectangle above or below the game rectangle) and **overlay** (the game rectangle is the whole window and the HUD is drawn over it). Only bar mode is built first. HUD content is one `bevy_ui` tree under one `Hud` root and does not depend on the mode. UI is written in art pixels and scaled by the global `UiScale` (window height of the region divided by the view height). The kit has generic widgets (label, icon, gauge). The game has the content and the values. The map-transition fade covers the game region only.

**Rationale.** Bevy has no HUD concept, only `bevy_ui`, camera viewports and `UiTargetCamera`. `FixedVertical` maps the view height to the viewport height, so a camera in a smaller viewport keeps its art scale, and `follow_controlled` and `visible_rect` need no change. A second `Camera2d` would make `Single<.., With<Camera2d>>` match two entities, and Bevy skips such a system without a warning, so camera following would stop. The marker prevents that. A pure layout function is unit testable without a window. One scale rule and one font for all UI keeps the HUD, title screen, dialogs and info box alike. A fade that covers the HUD would hide values the player still needs.

**How to apply.** Recompute the viewports on `WindowResized` and when the layout changes. Code that turns a window position into a world position must use the game viewport (`Camera::viewport_to_world_2d`), and `half_view` takes the region size, not the window size. `layers.rs` order stays: HUD below UI below fade. Approach considered and not chosen now: render the game into a UI node with `ViewportNode`. It would give one UI tree for every mode, but it is not tried on wasm. `rm12`.

## adr-x35. The kit knows the app flow states, and a game fills them in (2026-11-07)

**Decision.** The kit defines the app states `Loading`, `Title`, `Playing` and `GameOver` as Bevy `States` (`AppState`) and `Running` and `Paused` as a sub state of `Playing` (`PlayState`), and no behaviour beyond them: the kit does not know what a title screen shows. A state chooses the screen layout (`Title` and `GameOver` use the `Full` region with their own background, `Playing` uses the layout of the game, `Paused` keeps it) and which UI roots exist (spawned `OnEnter`, removed `OnExit`). Gameplay systems run in `Playing` and not `Paused`. Pause freezes game time (`Time<Virtual>`); UI uses real time. While paused the HUD and the game stay visible, dimmed and not interactable, and a floating dialog is the only interactive part. A debug switch (`--skip-title`, the variable `GAME_KIT_SKIP_TITLE`, always set for scripted runs) skips the title once, so a developer does not walk through it.

**Rationale.** Title, pause and game over change what is on screen, and the screen layout (`adr-x34`) must be a function of the state, or each new screen needs its own camera code. The kit has no states yet, so now is the cheapest time to set the names. One call that pauses virtual time freezes all movement that uses `time.delta_secs()`, which is all real-time movement (`adr-lw7`). A title screen on its own background needs no active game camera.

**How to apply.** Add `AppFlowPlugin` after `ScreenPlugin`. A game builds a screen with `state_root` in `OnEnter`, fills the `PauseDialog` slot, and sends `LoadingFinished` when it has loaded. Map-transition timers and the camera glide use `Res<Time>`, so they freeze while paused. The scripted run tool uses real time. Long Way uses the plugin with `Playing` as its first state and no pause key. `dev-docs/extensions/screen-layout.md`.

## adr-x39. The kit starts looping animations at a seeded random frame, and depends on `bevy_aseprite_ultra` (2026-11-08)

**Decision.** `game_kit::animation` has `RandomStartFrame { seed }` and `AnimationHelpPlugin`. When the Aseprite file of an entity has loaded, the plugin sets `AnimationState` to a frame of the tag (or of the whole file) chosen from the seed, and removes the component. `RandomStartFrame::for_cell(map_name, cell)` builds the seed from the map name and the cell with the hash of `map_background`. The kit now depends on `bevy_aseprite_ultra` (the version is pinned once in the root `Cargo.toml`).

**Rationale.** Copies of one animation that start together look uniform. A seed from the map and cell gives the same start on every load, F1 reload and platform, as the map backgrounds do (`adr-x36`). The kit takes the dependency at the developer's request (`adr-x12`): R-Town's animated water can use the same component. Both games already compile the crate, so no new crate reaches the web build, but the kit now cannot be used without it.

**Also in the kit (same day).** The spawn helpers `tag_sprite`, `tag_animation`, `all_frames_sprite` and `all_frames_animation`, and `play_if_changed` (switch tag only when it differs, set the speed). `StaticFrame { relative }` shows one frame without playing, for icons (Long Way HUD). Both games used the spawn boilerplate in many places, and the tag switch came from Long Way `animate_player`, which no longer keeps its own `playing` field. R-Town's walk flip and bob stays in R-Town (one game uses it).

**How to apply.** `dev-docs/extensions/pixel-art.md`, "Random start frame". Add `AnimationHelpPlugin` and put `RandomStartFrame::for_cell(..)` in the spawn bundle. Long Way: trees (`trees.rs`) and pickups (`pickups.rs`).


## adr-x45. The kit has an image button and a gauge for the HUD; the game handles the click (2026-11-12, amended 2026-11-14: a disabled state and a fill speed)

**Decision.** `hud.rs` has two widgets besides `label`. `image_button(server, path, up, down, disabled)` draws a button from tags of an Aseprite file: `up` normally, `down` while a primary press holds it, with Bevy's `Pressed` marker on it then, and the `disabled` tag (`up` when it is `None`) while the entity has Bevy's `InteractionDisabled` marker. A disabled button takes no press, and a press in progress ends when it becomes disabled. A release, the pointer leaving or a cancelled press puts it back up. `gauge(GaugeStyle)` is a track bar with a fill that moves towards `Gauge::fraction` (0 to 1, clamped) at a constant speed (`GaugeStyle::fill_secs` from empty to full, 0 for at once), in whole art pixels, a click pad above and below, and an optional line below the bar while the pointer is over it. The game spawns them with its own size, values and marker, sets the fraction, and observes `PointerClick` on the entity for what a click does. The kit sends no messages for them. It cannot stop a `PointerClick`, so the game ignores a click on a button that has `InteractionDisabled`. `approach(shown, target, max_step)`, the step rule of the fill, is public for other values that move the same way. `ScreenPlugin`, which spawns the `Hud`, adds their behaviour, so a game needs no extra plugin.

**Rationale.** The user asked for the Long Way button and gauge to be reusable (2026-11-12). Bevy 0.20 has a headless `Button` in `bevy_ui_widgets`, but it is in the `ui` feature collection and the workspace builds only `2d`: a new Bevy feature means a full rebuild and about 2 GB more in `target/` (`adr-e8`). It has no gauge or progress bar. The two widgets are small, and they use only `bevy_ui` (`Pressed` included) and the Aseprite plugin the kit already has (`adr-x39`). Leaving the click to the game keeps game types out of the kit and lets a game decide what a click on an empty button means. The disabled state uses Bevy's own `InteractionDisabled` marker, so it needs no kit type and matches Bevy's widgets. The disabled tag is optional, so a button whose art has no such frame still works (the user, 2026-11-14). The game decides when a button is disabled: a blank Long Way slot is not, so a click on it still closes the drawer. The fill speed shows a change instead of a jump, and 0 keeps the old behaviour for a game that does not want it.

**How to apply.** Code and tests: `crates/game-kit/src/hud.rs`. Use and limits: `dev-docs/extensions/screen-layout.md`, "HUD". Long Way uses both (`adr-lw34`). Move to `bevy_ui_widgets` if keyboard or gamepad control of the HUD is needed.

## adr-x47. The kit holds the particle code (`fx`), and builds particle sheets from `.aseprite` tags at run time (2026-11-11)

**Decision.** `game_kit::fx` wraps `bevy_enoki` (`adr-e9`): `FxPlugin` (opt-in), the `Fx` system parameter that spawns emitters, one-shots and emitters that follow an owner, the additive (glow) materials and `EffectLibrary`. It re-exports the Enoki types a game needs, so a game never depends on Enoki itself. `bevy_enoki` is a plain dependency of the kit only, with no Cargo feature. `fx_strip.rs` (`FxLooks`) cuts the frames of an `.aseprite` tag into a one-row particle sheet when the art loads; there is no export step and no generated PNG. The kit holds no effect files, art or values.

**Rationale.** The user chose the kit for the Enoki code (2026-11-11), so a second game gets particles without copying the glow patch handling and the helpers. The cost, accepted by the user: R-Town compiles Enoki's 5 extra crates, though it runs no Enoki code. A Cargo feature would avoid that but builds the kit twice (`--workspace` against `-p`), which costs more. If the cost grows, split a `game-fx` crate (`adr-x12` allows it). Enoki cannot read `.aseprite` and the Aseprite asset is a packed 2D atlas of full-canvas frames, so the sheets are cut at run time: a fresh clone works and the `.aseprite` file stays the only source (`adr-a1`).

**How to apply.** `dev-docs/extensions/particles.md` (including the build guard to run after any `Cargo.toml` change). Long Way: `weapons/ring.rs`, `tornado.rs`, `fireball.rs`.
