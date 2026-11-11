# Particles

## Summary

- `game_kit::fx` wraps `bevy_enoki` 0.8.0: one plugin, additive ("glow") materials, helpers that spawn an emitter, a one-shot burst or an emitter that follows an entity, and a library that keeps effect files loaded.
- The kit holds no effect files, no art and no values. A game supplies them.
- The plugin is opt-in. A game that does not add `FxPlugin` runs no Enoki code.

## Status

| Part | Code today | Shared? |
|---|---|---|
| `FxPlugin`, `Fx` (system parameter), `Follows`, `EffectLibrary`, `Look`, `SpriteLook`, `AdditiveSprite`, `AdditiveColor` | `crates/game-kit/src/fx.rs` | Shared. Used by Long Way only. |
| Patched Enoki (`blend_state()` on `Particle2dMaterial`, about 10 lines) | `vendor/bevy_enoki`, root `[patch.crates-io]` | Workspace. |
| Effect files, sprite sheets, weapons | `games/long-way/` (`PLAN-weapons.md`) | No. |

Cost accepted by the user: the kit depends on Enoki, so R-Town also compiles the 5 extra crates (`bevy_enoki`, `wide`, `chacha20`, `getrandom` 0.3.4 and 0.4.3). There are no Cargo features in the kit, because a feature would build the kit twice (`--workspace` against `-p`). If the cost grows, split a `game-fx` crate.

## Terms

- **Emitter:** an entity with a `ParticleEffectHandle` (the effect file) and a `ParticleSpawner` (the material).
- **One-shot:** an emitter with `OneShot::Despawn`: it plays one burst and removes itself after the last particle.
- **Glow:** additive blending. Particles add to what is already drawn, so fire and sparks look bright.

## Parameters the game supplies

Effect files (`*.particle.ron`), sprite sheets as PNG strips (Enoki cannot read `.aseprite`), which effects glow, and the draw order of each emitter (`DepthSort`, `adr-x9`).

## Specification

1. `FxPlugin` adds Enoki, the two additive materials, the `EffectLibrary` resource and the follow system. The additive colour material is made at start-up (`FxMaterials`).
2. `Fx` is a system parameter. `fx.emitter(file, transform, &look, glow)`, `fx.one_shot(...)` and `fx.following(owner, ...)` spawn emitters and return the entity. `glow` and `look` are chosen per call, so one weapon can glow and another not.
3. `Look::Squares` draws flat colour squares. `Look::Sprites(SpriteLook)` draws a sprite strip, made once as a normal and an additive material.
4. `Follows { owner, offset, rotation }` moves an emitter with its owner: the owner's x and y plus `offset`, and the owner's rotation turned by `rotation`. It never writes z. When the owner is gone, the emitter stops spawning, loses `Follows` and gets `OneShot::Despawn`.
5. `EffectLibrary::keep(&server, file)` keeps a file loaded. Without it a file is dropped when its last emitter dies.
6. Effect files are not reloaded while the game runs (no file watcher, `PLAN-weapons.md` G3). Restart the game to see a change.

## Usage

```rust
app.add_plugins(game_kit::fx::FxPlugin);

fn burst(mut fx: Fx, look: Res<MyLook>) {
    fx.one_shot("effects/impact_fire.particle.ron", Transform::from_xyz(x, y, 0.0), &look.0, true);
}
```

## Limits

- Enoki draws with alpha blending only. Additive blending needs the vendored patch. Re-apply it on every Enoki upgrade, or drop glow.
- Sprite sheets must be PNG strips. `assets/generated/` is git-ignored (`adr-a1`), so a fresh clone must export them first.
- Not measured on the web.

## Tests

`fx.rs` has unit tests for the follow system (position, offset, rotation, owner gone) and the blend state. Rendering is checked by a scripted run.
