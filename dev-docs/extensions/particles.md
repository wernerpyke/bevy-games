# Particles

## Summary

- `game_kit::fx` wraps `bevy_enoki` 0.8.0: one plugin, additive ("glow") materials, helpers that spawn an emitter, a one-shot burst or an emitter that follows an entity, and a library that keeps effect files loaded.
- The kit holds no effect files, no art and no values. A game supplies them.
- The plugin is opt-in. A game that does not add `FxPlugin` runs no Enoki code.

## Status

| Part | Code today | Shared? |
|---|---|---|
| `FxPlugin`, `Fx` (system parameter), `Follows`, `EffectLibrary`, `Look`, `SpriteLook`, `AdditiveSprite`, `AdditiveColor` | `crates/game-kit/src/fx.rs` | Shared. Used by Long Way only. |
| `FxLooks`, `strip_from_atlas` (particle sprite strips cut from an `.aseprite` tag at run time) | `crates/game-kit/src/fx_strip.rs` | Shared. |
| Patched Enoki (`blend_state()` on `Particle2dMaterial`, about 10 lines) | `vendor/bevy_enoki`, root `[patch.crates-io]` | Workspace. |
| Effect files, sprite sheets, weapons | `games/long-way/` (`games/long-way/docs/weapons.md`) | No. |

Cost accepted by the user: the kit depends on Enoki, so R-Town also compiles the 5 extra crates (`bevy_enoki`, `wide`, `chacha20`, `getrandom` 0.3.4 and 0.4.3). There are no Cargo features in the kit, because a feature would build the kit twice (`--workspace` against `-p`). If the cost grows, split a `game-fx` crate (`adr-x47`).

## Build guard

Measured on 2026-11-10 with `cargo tree`: Enoki adds only the 5 crates above and no Bevy feature. The build stays that way while these rules hold (from `PLAN-weapons.md`, 2026-11-11):

| # | Rule |
|---|---|
| G1 | `bevy_enoki` is a plain dependency of `crates/game-kit` only. The games get it through the kit and do not name it in their `Cargo.toml`. No Cargo `[features]` in `game-kit`. |
| G2 | No new Bevy feature. Games keep `bevy.workspace = true`. |
| G3 | Not used: `bevy/file_watcher` (hot reload), `bevy/dynamic_linking` as a default, Enoki's `dev` feature. Effect files are reloaded by restarting. |
| G4 | `bevy_enoki` stays pinned `=0.8.0` in `[workspace.dependencies]`. The patch (`adr-e9`) changes the source of that one crate only. |

Gates to run after any change to a `Cargo.toml`. Stop and ask when one fails.

| Gate | Pass when |
|---|---|
| `cargo tree -p long-way`, `-p r-town`, `-p game-kit` | No crate is new except the ones the change adds on purpose |
| `cargo tree -e features -i bevy -p long-way` | The Bevy features did not change |
| `git diff Cargo.lock` | Only added packages. No existing crate changes version. |
| `./dev-scripts/rust.sh build long-way` | The "Compiling" lines name no Bevy crate |
| `./dev-scripts/rust.sh lint`, run twice | The second run compiles nothing |
| `./dev-scripts/rust.sh size` | Report the change in `target/`. Over 8 GB: say so (`adr-e8`). |
| Rebuild time after touching one Long Way file | Not worse by more than noise (4.8 s on 2026-11-11) |

## Terms

- **Emitter:** an entity with a `ParticleEffectHandle` (the effect file) and a `ParticleSpawner` (the material).
- **One-shot:** an emitter with `OneShot::Despawn`: it plays one burst and removes itself after the last particle.
- **Glow:** additive blending. Particles add to what is already drawn, so fire and sparks look bright.

## Parameters the game supplies

Effect files (`*.particle.ron`), the `.aseprite` file and tag of each particle sprite (Enoki cannot read `.aseprite`, so `FxLooks` cuts a strip from the tag), which effects glow, and the draw order of each emitter (`DepthSort`, `adr-x9`).

## Specification

1. `FxPlugin` adds Enoki, the two additive materials, the `EffectLibrary` resource and the follow system. The additive colour material is made at start-up (`FxMaterials`).
2. `Fx` is a system parameter. `fx.emitter(file, transform, &look, glow)`, `fx.one_shot(...)` and `fx.following(owner, ...)` spawn emitters and return the entity. `glow` and `look` are chosen per call, so one weapon can glow and another not.
2b. `game_kit::fx` re-exports the Enoki types a game needs, so a game does not depend on Enoki itself (G1 of the weapons plan): `OneShot`, `Particle2dEffect`, `ParticleEffectHandle`, `ParticleSpawnerState`, and for per-emitter changes `ParticleEffectInstance` and `Attractor`. An emitter's `ParticleEffectInstance` holds its own copy of the effect once loaded; set its `attractors` there (Long Way's tornado, `weapons/tornado.rs`).
3. `Look::Squares` draws flat colour squares. `Look::Sprites(SpriteLook)` draws a sprite strip, made once as a normal and an additive material.
4. `Follows { owner, offset, rotation }` moves an emitter with its owner: the owner's x and y plus `offset`, and the owner's rotation turned by `rotation`. It never writes z. When the owner is gone, the emitter stops spawning, loses `Follows` and gets `OneShot::Despawn`.
5. `EffectLibrary::keep(&server, file)` keeps a file loaded. Without it a file is dropped when its last emitter dies.
6. `FxLooks::request(&server, name, file, tag, square)` (in `Startup`) asks for a look made from the frames of a tag. When the file has loaded, a system copies the frames out of the Aseprite atlas into one row of equal cells, cropped to the union of the visible pixels of the tag, padded to square cells when `square` is set (Enoki draws every particle as a square quad). `FxLooks::get(name)` returns the `Look`, or `None` until it is ready, so a caller uses `unwrap_or_default()` (flat squares) for the first frames. A failed look (no such tag, no visible pixel) logs an error and is never ready. The spike's exported sheets and these strips have the same cell sizes. The frames of the tag are the particle's life, in order.
7. Effect files are not reloaded while the game runs (no file watcher, G3 in "Build guard"). Restart the game to see a change.

## Usage

```rust
app.add_plugins(game_kit::fx::FxPlugin);

fn burst(mut fx: Fx, look: Res<MyLook>) {
    fx.one_shot("effects/impact_fire.particle.ron", Transform::from_xyz(x, y, 0.0), &look.0, true);
}
```

## Limits

- Enoki draws with alpha blending only. Additive blending needs the vendored patch. Re-apply it on every Enoki upgrade, or drop glow.
- The strip is built once, when the art loads. A changed `.aseprite` needs a restart. The atlas keeps its pixels on the CPU (the loader's default), which the builder reads.
- Not measured on the web.

## Tests

`fx.rs` has unit tests for the follow system (position, offset, rotation, owner gone) and the blend state. `fx_strip.rs` has unit tests on plain pixel buffers (union of the visible pixels, cell contents, frame order, square padding, empty input). Rendering is checked by a scripted run.
