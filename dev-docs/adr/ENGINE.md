# Engine: decisions

## adr-e1. Bevy (Rust) as the game engine (2026-10-30)

**Decision.** Use Bevy, written in Rust.

**Rationale.** Alternatives were LOVE2D (Lua) and Pyxel (Python).

| Option | Verdict |
|---|---|
| LOVE2D | Rejected. Stable and fast to iterate, but Lua has no strict compiler and the web export is the least reliable. |
| Pyxel | Rejected. Easy web export, but the 16 colour palette and 256x256 image bank limits are real constraints, and Python gives no compile-time safety. |
| **Bevy** | **Chosen.** Rust's compiler and clippy give Claude a strong feedback loop. macOS native and WebAssembly are both supported. No palette or resolution limits. |

Accepted costs: breaking API changes every Bevy release, slow compiles, larger web builds.

**How to apply.** Keep native macOS and `wasm32-unknown-unknown` builds working. Disable unused Bevy default features to limit web build size.

## adr-e2. Pin Bevy 0.20.1 (2026-10-30)

**Decision.** Pin the latest stable release, 0.20.1 (crates.io, 2026-10-30).

**Rationale.** Latest stable gives the newest features and the longest time before a forced upgrade. Ecosystem plugins often lag one release, so each plugin is checked before use.

**How to apply.** Check every Bevy ecosystem crate for 0.20 support before adding it. Record plugin results in this file. Upgrading Bevy means updating `bevy-src/` and `dev-docs/bevy/notes.md`.

## adr-e3. Reference source in `bevy-src/` inside the project (2026-10-30)

**Decision.** Keep a shallow clone of the Bevy repo at tag `v0.20.1` in `bevy-src/` at the project root. It is git-ignored and is not a dependency.

**Rationale.** Claude's memory of Bevy APIs is unreliable across versions. The clone gives version-exact `examples/` and migration guides. It is inside the project so the developer can browse it. It is git-ignored so it stays out of project history.

**How to apply.** Search it with targeted grep. Do not read whole files. Re-clone at the new tag when Bevy is upgraded.

## adr-e4. Plugin compatibility with Bevy 0.20 (2026-10-31)

**Decision.** All candidate crates have a release that depends on Bevy 0.20. Use these versions (checked against crates.io dependency metadata).

| Crate | Version | Bevy requirement | Use |
|---|---|---|---|
| `bevy_aseprite_ultra` | 0.10.0 | `^0.20` | Aseprite import and animation |
| `bevy_ecs_tilemap` | 0.20.0 | `^0.20` | Tilemap rendering |
| `bevy_ecs_tiled` | 0.15.0 | `^0.20` | Tiled editor maps (uses `bevy_ecs_tilemap`) |
| `bevy_northstar` | 0.8.0 | `^0.20` | Grid pathfinding |
| `pathfinding` | 4.16.0 | none (engine independent) | Fallback pathfinding |
| `leafwing-input-manager` | 0.22.0 | `^0.20.0` | Input mapping |
| `bevy_asset_loader` | 0.28.1 | `bevy_app`/`bevy_ecs ^0.20.0` | Loading states |
| `bevy_egui` | 0.43.1 | `bevy_app`/`bevy_ecs ^0.20.0` | Debug UI |
| `bevy-inspector-egui` | 0.38.0 | `bevy_app`/`bevy_ecs ^0.20.0` | Entity inspector |
| `bevy_enoki` | 0.8.0 | `^0.20` | Particle effects for weapons (accepted, `adr-e9`). Local patch for additive blending. |

**Rationale.** Each crate moved to 0.20 within about five weeks of the Bevy release, so no plugin blocks Bevy 0.20.1.

**How to apply.** This checks declared dependency versions only. It does not prove a crate compiles for `wasm32-unknown-unknown` or behaves correctly. Run `./dev-scripts/rust.sh web` when adding each crate. Add a crate only when a feature needs it. Re-check on every Bevy upgrade.

## adr-e5. Pin the Rust toolchain in `rust-toolchain.toml` (2026-10-31)

**Decision.** Pin Rust 1.97.1 with clippy, rustfmt, rust-analyzer and the `wasm32-unknown-unknown` target in `rust-toolchain.toml`.

**Rationale.** Bevy 0.20.1 requires rustc 1.97.1 or newer. The global default was 1.96.0 and failed to build. A project pin installs the right version alongside the default without changing it, and gives the same compiler on every machine.

**How to apply.** Raise the version when a Bevy or crate upgrade requires it.

## adr-e6. Bevy features and build profiles (2026-10-31)

**Decision.** Depend on `bevy` with `default-features = false` and `features = ["2d"]`, pinned with `=0.20.1`. Dev builds compile dependencies at `opt-level = 3` (debug info: `adr-e8`). Web release builds use the `wasm-release` profile (size optimised, fat LTO).

**Rationale.** Bevy's defaults include 3D and UI, which add compile time and web size for a 2D game. The `2d` profile includes WebGL2. Unoptimised Bevy runs too slowly to develop against.

**How to apply.** Add Bevy features only when needed. Web builds use `--profile wasm-release`. The web release pipeline (`wasm-bindgen`, HTML shell, hosting) is not decided yet.

## adr-e7. Fixed-height camera, 1200 art pixels (2026-10-31)

**Decision.** The 2D camera shows a fixed 1200 art pixels of height (`ScalingMode::FixedVertical`), with nearest sampling and no MSAA. Width follows the window aspect ratio. The value is `VIEW_HEIGHT` in `games/long-way/src/camera.rs`.

**Rationale.** The player was Guy, 60 pixels tall, and should be one twentieth of the screen height (60 x 20 = 1200). The developer does not want to resize the art in Aseprite or scale sprites when drawing, so the view size sets the on-screen size. This replaced an earlier value of 180.

Updated 2026-11-05 (later): the view height is now 576 and the camera follows the player (`adr-lw16`). The text below is the earlier state.

Updated 2026-11-05: the player is now the Hunter, a 128x128 canvas with a body about 50 pixels tall, so about one 24th of the screen height (`adr-lw15`). The value 1200 stays. The trees and rocks (128 px) and the 64 px map cell were drawn for it, and the developer sizes a character by scaling the art file in Aseprite (the Hunter was doubled there) and not by changing the view.

**How to apply.** Art pixels map 1:1 to screen pixels only when the window is 1200 high. At smaller windows (a 720 high window gives 0.6x) the art is downscaled, which can make nearest-neighbour pixels uneven. If that looks bad, try linear sampling for downscaling, or a render-to-texture pixel camera. Revisit when the game design sets the screen layout.

## adr-e8. No debug info for dependencies, line tables for our crates (2026-11-05)

**Decision.** `[profile.dev.package."*"]` sets `debug = false` and `[profile.dev]` sets `debug = "line-tables-only"` in the root `Cargo.toml`. `target/` is kept under 8 GB with a short routine (`CLAUDE.md`, "Disk use"): `rust.sh size`, then `rust.sh prune`, then ask before `cargo clean`. Further savings are `rm90`.

**Rationale.** The developer does not review or debug Rust code by hand, and Claude does not use a debugger. Debug info was most of the size: `target/` was 25 GB, and 2.2 GB after a clean build without it (3.2 GB with `lint`, `test` and `web`). Panics still show file and line, because line tables stay. Only stepping through code in a debugger is lost. The limit of 8 GB is about twice the normal size, so it signals stale outputs.

**How to apply.** Do not add `debug = true` or `bevy/dynamic_linking` as a default without asking. After a Bevy version or feature change, run `cargo clean` once: the old outputs are never used again. Run the routine at the start of a session that builds Rust, and after any `Cargo.toml` change.

## adr-e9. Particle effects with `bevy_enoki`, with a local patch for additive blending (2026-11-10)

**Decision.** Weapon effects that are many short-lived random bits (trails, bursts, embers, flames) use `bevy_enoki` 0.8.0, pinned `=0.8.0` in `[workspace.dependencies]`. A placed object with a fixed life and a place in the depth order (the ice wall) is built from plain animated sprites (`bevy_aseprite_ultra`), not particles. Additive blending ("glow") needs a patched copy of Enoki: one trait method `blend_state()` on `Particle2dMaterial` (default alpha blending) and two shader handles made public, about 10 lines. The patch stays for now (user, 2026-11-10) through the root `[patch.crates-io]`. The fallback is to drop glow, which removes the patch. The Enoki code lives in `crates/game-kit/src/fx.rs` as a plain dependency of the kit, with **no Cargo features** (user, 2026-11-10; the cost is that R-Town also compiles Enoki's 5 extra crates). Not carried into any game: `bevy/file_watcher` (hot reload), `bevy/dynamic_linking` and the Enoki `dev` feature.

**Rationale.** `bevy_hanabi` needs compute shaders, so WebGPU only. `bevy_sprinkles` targets Bevy 0.19 and is 3D first. Enoki simulates on the CPU and runs on WebGL2: the user ran the spike in a browser with all four weapons, saw no console errors and no problem (the web frame time was not measured). On native, all four weapons together (about 3400 particles, 5 times normal use) cost about 3.4 ms a frame on the `dev` profile. Enoki adds 5 crates (Enoki, `wide`, `chacha20`, `getrandom` 0.3 and 0.4) and no Bevy feature, so the build stays small. A feature in the kit would build the kit twice (`--workspace` against `-p`), and `bevy/file_watcher` already doubled every Bevy build in the spike. Enoki hard-codes alpha blending, so glow needs the patch.

**How to apply.** Gates after any change to a `Cargo.toml` are in `dev-docs/PLAN-weapons.md` (dependency tree, `Cargo.lock`, no Bevy crate recompiled, sizes). Re-apply the 10 lines when Enoki is upgraded. Facts about the crate: `dev-docs/bevy/notes.md`, "bevy_enoki". Measurements and the comparison: `dev-docs/RESEARCH-particles.md`.
