# Bevy 0.20.1 notes

Verified patterns for the pinned version. Read before searching `bevy-src/`. Keep short.

## Camera

- `use bevy::camera::ScalingMode;`
- 2D camera with fixed height in art pixels: `Camera2d`, `Msaa::Off`, `Projection::Orthographic(OrthographicProjection { scaling_mode: ScalingMode::FixedVertical { viewport_height }, ..OrthographicProjection::default_2d() })`.
- Nearest sampling: `DefaultPlugins.set(ImagePlugin::default_nearest())`.

## Pitfalls

- Do not compare normalised float vectors for equality between frames. `Vec2::normalize` can return 0.99999994 or 1.0 for the same direction depending on the length, so a "did the direction change" check fires randomly. Compare signs or an enum.
- To run a system after Bevy has updated keyboard state, use `PreUpdate` with `.after(bevy::input::InputSystems)`. To write keys into `ButtonInput<KeyCode>`, use `press` and `release`.
- Exiting from a system: `MessageWriter<AppExit>` then `exit.write(AppExit::Success)`.

## Screenshots

- `commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path))` with `use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk};`. The file is written a few frames later, inside the observer. A second `.observe(|_: On<ScreenshotCaptured>, ..| ..)` added after it runs once the file is saved. Saves the window at physical size (2560x1440 on this Mac). Includes UI, sprites and gizmos.

## Gizmos

- `mut gizmos: Gizmos` as a system parameter. `gizmos.rect_2d(Isometry2d::from_translation(centre), size, color)` and `gizmos.circle_2d(Isometry2d::from_translation(p), radius, color)`. Import `bevy::math::Isometry2d`. Included in the `2d` feature.

## Text

- World text: `Text2d::new("1")`, `TextFont::from_font_size(20.0)`, `TextColor(c)`, `Anchor::TOP_LEFT` (`bevy::sprite::Anchor`), `Transform`. UI text: `Text::new("")`, same font and colour, `Node { position_type: PositionType::Absolute, top: px(8), left: px(8), .. }`, optional `BackgroundColor`. Edit `Text.0` to change it.
- `default_font` and `bevy_ui_render` are already on through the `2d` feature chain, so no new dependency.
- Remove all children of an entity: `commands.entity(e).despawn_children().with_children(|p| { p.spawn(..); })`.

## Input and time

- Keyboard: `Res<ButtonInput<KeyCode>>`, `.pressed(KeyCode::ArrowLeft)`.
- Frame time: `time.delta_secs()`.
- Window resize events are `MessageReader<WindowResized>` (messages, not events).

## Features

- `bevy = { default-features = false, features = ["2d"] }` compiles for native and `wasm32-unknown-unknown` (WebGL2).
- `bevy_aseprite_ultra` 0.10.0 pulls in `bevy_ui` as a dependency.

## Assets

- Assets load from the game's own `assets/` folder (`games/<name>/assets/`). Through `cargo run`, Bevy finds it from `CARGO_MANIFEST_DIR`. A binary started directly looks next to the executable. Set `BEVY_ASSET_ROOT=<game folder>` (for example `games/long-way`) in that case.

## bevy_aseprite_ultra 0.10.0

- A file with no tags: `animation: Animation::default()` plays all frames in a loop. A named tag that does not exist is an error.
- 2D draw order: higher `Transform` z draws in front. Equal z has no guaranteed order.

- `app.add_plugins(AsepriteUltraPlugin)`; `use bevy_aseprite_ultra::prelude::*;`
- Spawn `AseAnimation { aseprite: server.load("art/characters/Guy.aseprite"), animation: Animation::tag("Idle").with_repeat(AnimationRepeat::Loop) }` with `Sprite::default()`.
- Switch animation: `ase.animation.play_loop("Walk")`. Tag names are case sensitive and match the Aseprite tags. Call it only when the tag changes.

## Build times

Measured on an Apple M1, 8 GB RAM, Rust 1.97.1, `dev` profile (`opt-level = 1`, dependencies at `opt-level = 3`), no `dynamic_linking`.

| Build | Time |
|---|---|
| Clean native build, first time (`build`) | about 8.5 min |
| Add `bevy_aseprite_ultra` and build | about 5.5 min |
| `lint` (fmt check and clippy) after a small edit | 2.5 s |
| `build` after a small edit (about 30 lines changed in 2 files) | 7.8 s |
| `build` after touching one file, no change | 5.9 s |
| `build` after touching `main.rs`, no change | 5.4 s |

Adding a dependency or changing Cargo features rebuilds dependencies and takes minutes. Web (`wasm32`) builds have a separate `target` folder per target and need their own first build (about 3 min for a `check`).
