# Bevy 0.20.1 notes

Verified patterns for the pinned version. Read before searching `bevy-src/`. Keep short.

## Camera

- `use bevy::camera::ScalingMode;`
- 2D camera with fixed height in art pixels: `Camera2d`, `Msaa::Off`, `Projection::Orthographic(OrthographicProjection { scaling_mode: ScalingMode::FixedVertical { viewport_height }, ..OrthographicProjection::default_2d() })`.
- Nearest sampling: `DefaultPlugins.set(ImagePlugin::default_nearest())`.

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
