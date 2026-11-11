# Bevy 0.20.1 notes

Verified patterns for the pinned version. Read before searching `bevy-src/`. Keep short and about Bevy only. Our own designs are in `dev-docs/extensions/`. Plain explanations are in `concepts.md`.

## Camera

- `use bevy::camera::ScalingMode;`
- 2D camera with fixed height in art pixels: `Camera2d`, `Msaa::Off`, `Projection::Orthographic(OrthographicProjection { scaling_mode: ScalingMode::FixedVertical { viewport_height }, ..OrthographicProjection::default_2d() })`.
- Nearest sampling: `DefaultPlugins.set(ImagePlugin::default_nearest())`.

## Draw order and gizmos

- Sprites sort by `Transform` z. Higher is in front. Equal z has no fixed order, so overlapping sprites flicker. Always give overlapping sprites different z.
- `RenderLayers` only chooses which camera sees an entity.
- `mut gizmos: Gizmos` as a system parameter. `gizmos.rect_2d(Isometry2d::from_translation(centre), size, color)` and `gizmos.circle_2d(Isometry2d::from_translation(p), radius, color)`. Import `bevy::math::Isometry2d`. Included in the `2d` feature.
- 2D gizmos use sort key infinity (`bevy_gizmos_render`, `pipeline_2d.rs`), so they are always on top of sprites and cannot sit in a layer.
- `bevy_ui` nodes are drawn after sprites and gizmos, ordered by `GlobalZIndex` (root nodes default to 0, higher is in front).
- For a debug drawing that must sit in a layer, use thin sprites: `Sprite::from_color(color, size)` with a `Transform` at the layer z, as children of one root entity with a `Visibility`. Show and hide the root with `Single<&mut Visibility, With<Root>>`.

## Ordering and startup

- `Startup` commands are applied before `PostStartup`, so a `PostStartup` system can read entities and resources that `Startup` systems created.
- Order systems from different plugins with `my_system.after(other_plugin::pub_system_fn)`. Bevy inserts a sync point between them, so commands of the first are applied before the second runs.
- To run a system after Bevy has updated keyboard state, use `PreUpdate` with `.after(bevy::input::InputSystems)`. To write keys into `ButtonInput<KeyCode>`, use `press` and `release`.
- `Query<Entity, Added<T>>` finds entities that got `T` since the last run of the system. Use it to attach helper entities to new spawns.
- Let chains work in this edition: `else if cond && let Some(x) = value { .. }`.
- Exiting from a system: `MessageWriter<AppExit>` then `exit.write(AppExit::Success)`.

- Messages between systems (`examples/ecs/message.rs`): `#[derive(Message)] struct Hit { .. }`, `app.add_message::<Hit>()`, send with `MessageWriter<Hit>` and `.write(Hit { .. })`, read with `MessageReader<Hit>` and `.read()`.
- One keypress: `input.just_pressed(KeyCode::KeyX)`. System set: `#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)] struct MySet;`, then `app.configure_sets(Update, MySet)` and `.in_set(MySet)` / `.after(MySet)` (`examples/ecs/ecs_guide.rs`).
- Headless test (no window): `App::new().add_plugins(MinimalPlugins)`, `app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(16)))`, then `app.update()` adds 16 ms to `Time` per call (`bevy_time` `TimeUpdateStrategy`). Note the first update only sets the start time.
- A system with `Single<..>` is skipped without a warning when no entity matches or when there is more than one (`SystemParamValidationError::skipped`, `system_param.rs`). Removing the player is therefore safe for systems that use `Single` for it. A `Query` is empty and needs no care.
- `Timer::from_seconds(10.0, TimerMode::Repeating)` works as a field of a component. Call `timer.tick(time.delta())` and read `timer.just_finished()`.

## Pitfalls

- `Vec2::normalize` can return 0.99999994 or 1.0 for the same direction depending on the length, so comparing normalised vectors between frames fires randomly. Compare signs or an enum.
- Stored and recomputed positions drift by millionths (for example 76.8 comes back as 76.800003). Do not test exact equality against a coordinate. Use a tolerance.

## Screenshots

- `commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path))` with `use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk};`.
- The file is written a few frames later, inside the observer. A second `.observe(|_: On<ScreenshotCaptured>, ..| ..)` added after it runs once the file is saved.
- Saves the window at physical size (2560x1440 on this Mac). Includes UI, sprites and gizmos.

## Text

- World text: `Text2d::new("1")`, `TextFont::from_font_size(20.0)`, `TextColor(c)`, `Anchor::TOP_LEFT` (`bevy::sprite::Anchor`), `Transform`.
- UI text: `Text::new("")`, same font and colour, `Node { position_type: PositionType::Absolute, top: px(8), left: px(8), .. }`, optional `BackgroundColor`. Edit `Text.0` to change it.
- `default_font` and `bevy_ui_render` are already on through the `2d` feature chain, so no new dependency.
- Remove all children of an entity: `commands.entity(e).despawn_children().with_children(|p| { p.spawn(..); })`.

## Input and time

- Keyboard: `Res<ButtonInput<KeyCode>>`, `.pressed(KeyCode::ArrowLeft)`.
- Mouse: `Res<ButtonInput<MouseButton>>`, `.just_pressed(MouseButton::Left)`. Cursor: `Window::cursor_position()` (logical pixels, top left origin, `None` outside the window). To world: `camera.viewport_to_world_2d(&global_transform, cursor)`. Bevy subtracts the viewport origin itself (`viewport_to_ndc`), so pass the window position as it is. With a split screen the cursor can lie outside the camera viewport: test `camera.logical_viewport_rect()` first (`pointer::cursor_to_world`). `GlobalTransform` is the one of the last `PostUpdate`, so convert in `PreUpdate` when the camera moves in `Update`.
- Frame time: `time.delta_secs()`.
- Window resize events are `MessageReader<WindowResized>` (messages, not events).

## Features and assets

- `bevy = { default-features = false, features = ["2d"] }` compiles for native and `wasm32-unknown-unknown` (WebGL2).
- Assets load from the game's own `assets/` folder (`games/<name>/assets/`). Through `cargo run`, Bevy finds it from `CARGO_MANIFEST_DIR`. A binary started directly looks next to the executable. Set `BEVY_ASSET_ROOT=<game folder>` in that case.

## bevy_aseprite_ultra 0.10.0

- `app.add_plugins(AsepriteUltraPlugin)`; `use bevy_aseprite_ultra::prelude::*;`. It pulls in `bevy_ui` as a dependency.
- Spawn `AseAnimation { aseprite: server.load("art/characters/Hunter.aseprite"), animation: Animation::tag("Idle").with_repeat(AnimationRepeat::Loop) }` with `Sprite::default()`.
- Switch animation: `ase.animation.play_loop("Walk")`. Tag names are case sensitive and match the Aseprite tags. Call it only when the tag changes.
- A file with no tags: `animation: Animation::default()` plays all frames in a loop. A named tag that does not exist is an error.
- Frame state is the public component `AnimationState { relative_frame, current_frame, elapsed, .. }`, added with `AseAnimation`. The frame range of a tag is `Assets<Aseprite>::get(handle).tags[tag].range` (`RangeInclusive<u16>`), known only after the asset has loaded. Setting `current_frame` outside the range makes the animation system reset it to the start. Used by `game-kit` `animation.rs`.
- Play a tag once: `ase.animation.play("Die", AnimationRepeat::Count(0))`. The message `AnimationEvents::Finished(entity)` is sent when it ends (the entity is the one with the `AseAnimation`; the enum also has `LoopCycleFinished`, so match it, do not destructure it). Used for the ghost `Attack` and `Die` (`ghost.rs`) and the player `Hurt` (`player.rs`).

## One-shot systems and UI overlays (verified in `map_transition.rs`)

- Register a system that takes input: `let id = app.world_mut().register_system(my_system);` with `fn my_system(In(x): In<MyInput>, ..)`. Store `SystemId<In<MyInput>>` (Copy) in a resource. Run it with `world.run_system_with(id, MyInput { .. })`. Run one without input with `world.run_system(id)`. Both return a `Result`, so log the error. Commands queued by the system are applied when it returns.
- Inside an exclusive system (`fn(world: &mut World)`) copy the ids out first (`world.resource::<Hooks>().clone()`, or `get_resource::<Hooks>().copied()`), because running a system needs `&mut World`.
- Write a message from a test or from `&mut World`: `world.write_message(msg)` (`bevy_ecs` `World::write_message`).
- A full-screen overlay: `commands.spawn((Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), ..default() }, BackgroundColor(colour), GlobalZIndex(z)))`. Change `BackgroundColor.0` to fade it. Screenshots include UI.
- `#[derive(SystemParam)] struct MapLink<'w> { writer: MessageWriter<'w, M>, res: Option<Res<'w, R>> }` bundles parameters when a system has more than 7 (clippy `too_many_arguments`).
- A test app for a plugin that spawns UI needs only `MinimalPlugins`: the UI components can be spawned without the UI plugins.

## Pitfalls found in the seamless map tests

- `Time` in `Update` is virtual time, and it never advances more than 0.25 seconds per frame (`Time<Virtual>` max delta). `TimeUpdateStrategy::ManualDuration(Duration::from_millis(600))` therefore gives 0.25 s per `app.update()`. Count updates for timers in a test, or use a step of 250 ms or less.
- A `Camera2d` in a `MinimalPlugins` test has no transform propagation: insert `GlobalTransform` yourself, and set `OrthographicProjection::area` (it is zero until the render systems run), or `visible_rect` returns `None`.
- A map or tool that needs both `&mut World` and queries in one system: read what you need into owned values first (ids, rectangles), then change the world.

## Viewports and a HUD camera (verified in `screen.rs`, `hud.rs`)

- `Camera::viewport = Some(Viewport { physical_position, physical_size, ..default() })` limits a camera to a rectangle of the window (origin top left, physical pixels). `ScalingMode::FixedVertical` then maps the view height to the viewport height.
- The first camera on a window clears the whole window, whatever its viewport. A second camera for a region sets `clear_color: ClearColorConfig::None` and a `Camera::order` above the first, and the region gets its background from a UI node.
- A second `Camera2d` also draws world sprites in its region. Give it `RenderLayers::layer(1)` (the world is on layer 0).
- A UI root without `UiTargetCamera` goes to the camera with `IsDefaultUiCamera`, or else to the highest `order` camera on the primary window. Put `IsDefaultUiCamera` on the game camera, so the debug box and the fade stay in the game region, and give the HUD root `UiTargetCamera(hud_camera)`.
- A root node with `width: percent(100)` fills the viewport of its target camera, and its drawing is clipped to it (seen in a screenshot).
- `UiScale` (`bevy::ui::UiScale`, a resource) multiplies the window scale factor for all UI and for `TextFont` sizes. Art pixel UI: `game_region_physical_height / (view_height * window.scale_factor())`.
- A picture on a padded UI node shrinks into the padding, and `NodeImageMode::Auto` keeps the aspect ratio. For a full-bleed background use a child with `position_type: Absolute`, `left` and `top` 0 (auto offsets start inside the padding), width and height 100%, and `ImageNode::with_mode(NodeImageMode::Stretch)` (`games/long-way/src/hud.rs`).
- `TextFont::from_font_size(32.0).with_font(server.load("fonts/X.ttf"))`. `font` is a `FontSource` in 0.20.
- `app.insert_state(S::X)`, `app.add_sub_state::<Sub>()`, `OnEnter(S::X)`, `OnExit(S::X)`, `in_state(S::X)`, `DespawnOnExit(S::X)`, `NextState<S>` with `.set(..)` (verified in `app_state.rs`, `examples/state/sub_states.rs`). A headless test needs `bevy::state::app::StatesPlugin` and `init_resource::<ButtonInput<KeyCode>>()`, and the state change shows after two `app.update()` calls. A pause toggle needs `Time<Virtual>::pause()` and `unpause()`.
- A sub state is removed with its source, and its `OnExit` runs, so leaving `Playing` while `Paused` still runs the `OnExit(Paused)` system.
- `bevy::picking::Pickable { should_block_lower: true, is_hoverable: false }` on a UI node stops picking below it.
- A system with `Single<(&mut Camera, ..), With<A>>` and `Single<&mut Camera, With<B>>` needs `Without<B>` and `Without<A>` filters, or Bevy reports a query conflict. A long tuple in a `Single` trips clippy `type_complexity`: use a `type` alias.
- Test without a window: `App::new().add_plugins(MinimalPlugins)`, `init_resource::<UiScale>()`, and spawn `(Window { resolution: (720, 1080).into(), ..default() }, PrimaryWindow)`. `Window::physical_size()` works without a backend.

## bevy_enoki 0.8.0 (verified in the particle spike, 2026-11-10)

- Spawners: `ParticleSpawner(material handle)` plus `ParticleEffectHandle` (shared asset) or `ParticleEffectInstance` (own copy, changeable at run time). `OneShot::Despawn` removes the spawner after the burst and its last particle. `spawn_rate: 0.001` with `spawn_amount: 1` gives exactly one particle.
- `scale_curve` and `color_curve` set the absolute size and colour, not multipliers. With a sprite material the shader multiplies the sprite by the colour, so keep colour curves near white (a dark tint makes dark discs). A tint cannot change hue: a red sprite tinted blue goes black. Fire and ice need their own sprites.
- A sprite sheet is played in order over the particle life: `frame = floor(life fraction x frames)`. A fast growth and a long hold need a sheet padded with repeated cells. For one fixed picture use a one-frame PNG. Cells must be square (the quad is square, `scale` is one number) and the art must fill its cell.
- A particle starts with the rotation of the emitter entity. Its own direction does not matter, and `angular_speed` spins it from there. A full-circle `direction` spread (1.0) gives a radial burst. A burst of equal-speed particles from one point forms a ring front.
- `EmissionShape::Circle(r)` fills a disc. It is not a ring outline.
- Damping is `velocity *= 1 + life_fraction * (acceleration - damp) * dt`: it depends on the life fraction and the frame rate, so a particle cannot be stopped at an exact radius. For an exact curve, move small emitters along it each frame (40 moving emitters cost no visible frame time).
- `Attractor.position` is an offset from the emitter, not a world point. Set it per emitter on `ParticleEffectInstance`. A reloaded effect file loses it, so set it again. An emitter turned by `Quat::from_rotation_z(angle)` shoots its particles along that angle (tangent launches).
- Particles spawn at the emitter position once per frame, so a fast emitter leaves gaps unless the particles are large. A follower reads `GlobalTransform`, so it lags one frame.
- Enoki refreshes running emitters only on a first load. After `AssetEvent::Modified`, copy the changed effect into the emitters yourself. `AssetServer::reload` only reloads files with a live handle, so keep every effect file loaded (an `EffectLibrary` resource).
- Blending is fixed to alpha in 0.8.0. Additive needs the local patch (`blend_state()` on `Particle2dMaterial`, `adr-e9`) and a material with the same shader and bind group as the Enoki one.
- Hot reload of effect files needs `bevy/file_watcher`, which gives Bevy a feature set Long Way does not use and rebuilds every Bevy crate. Do not enable it in a game. An atomic save (temp file and rename, as `sed -i` does) is not seen by the watcher.
- `ui_bevy_render` is needed to draw UI text. `ui_api` alone draws nothing.
- Web: `wasm-server-runner` serves `./assets` of the directory it runs in. Start it from the crate folder: `cd games/<crate> && cargo run -p <crate> --target wasm32-unknown-unknown`.
- Two systems that both write an `AseAnimation` and are not ordered run in an order that depends on the schedule, and a command (such as `remove::<FacingTags>`) is not applied until a sync point. A system that takes over a sprite's animation must be ordered `.before` the system that plays the default animation, or the default can overwrite it in the same frame. Found when the ghost did not die: `die_when_killed` played `Die`, but `animate_facing` ran after it and set `Down` again (`game_kit::animation::FacingSet`).
