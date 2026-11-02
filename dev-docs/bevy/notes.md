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
- Frame time: `time.delta_secs()`.
- Window resize events are `MessageReader<WindowResized>` (messages, not events).

## Features and assets

- `bevy = { default-features = false, features = ["2d"] }` compiles for native and `wasm32-unknown-unknown` (WebGL2).
- Assets load from the game's own `assets/` folder (`games/<name>/assets/`). Through `cargo run`, Bevy finds it from `CARGO_MANIFEST_DIR`. A binary started directly looks next to the executable. Set `BEVY_ASSET_ROOT=<game folder>` in that case.

## bevy_aseprite_ultra 0.10.0

- `app.add_plugins(AsepriteUltraPlugin)`; `use bevy_aseprite_ultra::prelude::*;`. It pulls in `bevy_ui` as a dependency.
- Spawn `AseAnimation { aseprite: server.load("art/characters/Guy.aseprite"), animation: Animation::tag("Idle").with_repeat(AnimationRepeat::Loop) }` with `Sprite::default()`.
- Switch animation: `ase.animation.play_loop("Walk")`. Tag names are case sensitive and match the Aseprite tags. Call it only when the tag changes.
- A file with no tags: `animation: Animation::default()` plays all frames in a loop. A named tag that does not exist is an error.
