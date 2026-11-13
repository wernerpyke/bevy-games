# Research: particles for player weapons

Date: 2026-11-09. Status: spike done, verdict below (2026-11-10). Plan for the port: `PLAN-weapons.md` (done 2026-11-13, in git history); the walls: `PLAN-walls.md`, which deletes the spike.

## Goal

Draw player weapons in Long Way with a particle system:

1. **Fireball.** A ball with a trailing tail. It explodes when it hits an enemy or a blocked feature.
2. **Ring.** A ring of fire or ice crystals that grows outward from the player.
3. **Crystal wall.** Crystalline sparkles that stay in place and block enemies.
4. **Water ring.** A ring of water around the player that swirls like a whirlpool.

## Constraints

- Bevy 0.20.1, 2D only (`CLAUDE.md`).
- Two targets: macOS native and Web (`wasm32-unknown-unknown`). Both must work.
- The web build is assumed to use the WebGL2 backend. WebGL2 has no compute shaders.
- Pixel art look: nearest-neighbour sampling, fixed low-resolution camera.

## Candidates

Versions and Bevy requirements come from the crates.io dependency metadata (checked 2026-11-09). Nothing was compiled.

| Crate | Version | Bevy requirement | Web | 2D | Verdict |
|---|---|---|---|---|---|
| `bevy_hanabi` | 0.20.0 | `^0.20` | WebGPU only (needs compute shaders) | Yes | Rejected |
| `bevy_sprinkles` | 0.3.0 | `^0.19` | Not documented | No (planned) | Rejected |
| `bevy_enoki` | 0.8.0 | `^0.20` | README says it works on WebGL2 and mobile. Not verified by us. | Yes (2D only) | **Chosen for the spike** |
| `bevy_particle_systems` | 0.13.0 | Not checked | Not checked | Yes | Not assessed. Last release 2024-07. Treat as stale. |

### Why not Hanabi

Hanabi is the most capable (GPU simulation, modifier graph). But it needs compute shaders. On the web that means WebGPU only. Browsers without WebGPU would show nothing. `CLAUDE.md` says the web build must keep working.

### Why not Sprinkles

It targets Bevy 0.19, and it is 3D first. 2D support is only planned.

### Enoki

Enoki simulates on the CPU, with SIMD. That is a good fit for pixel art effects with a few hundred particles. Its cost is CPU time at high particle counts. The facts below are from the 0.8.0 source (`src/lib.rs`, `src/update.rs`).

**Effect fields** (`Particle2dEffect`, loaded from RON assets):

| Field | Meaning |
|---|---|
| `spawn_rate`, `spawn_amount` | How often, and how many particles per burst |
| `emission_shape` | `Point` or `Circle(radius)`. Nothing else. |
| `lifetime` | Seconds, with random spread (`Rval`) |
| `linear_speed`, `linear_acceleration`, `linear_damp` | Movement along one `direction` |
| `direction` | One direction per effect, with random spread. There is no "away from centre" or "along the edge" mode. |
| `angular_speed`, `angular_acceleration`, `angular_damp` | Appears to turn each particle sprite. Not confirmed. It does not orbit a point. |
| `scale`, `scale_curve` | Size, and size over lifetime |
| `color`, `color_curve` | Colour, and colour over lifetime |
| `gravity_direction`, `gravity_speed` | Constant pull |
| `attractors` | List of points with a strength and a minimum distance. The force is radial only. There is no sideways force. |
| `relative_positioning` | When true, particles move with the emitter |

**Spawner components:** `ParticleSpawner(material handle)`, `ParticleEffectHandle` (shared asset) or `ParticleEffectInstance` (own copy per spawner, changeable at runtime), `ParticleSpawnerState` (has `active`), `OneShot` (despawns the spawner after the burst and the last particle).

**Materials:** `ColorParticle2dMaterial` (plain colour) and `SpriteParticle2dMaterial` (sprite sheet with a fixed grid, set when the material is created). Custom materials need our own fragment shader.

**Editor:** separate install, not part of the crate.

## Weapon mapping

Particles are only visuals. Enoki has no particle collision and sends no events. Each weapon is two parts:

1. **Gameplay entity.** Position, velocity or radius, hitbox, lifetime. Plain Rust, unit-testable, moved with `time.delta_secs()`.
2. **Emitter.** Follows or sits on the gameplay entity and only draws the effect.

| Weapon | Gameplay entity | Emitter plan | Open question |
|---|---|---|---|
| Fireball | Moving point. A straight-line projectile already exists in the kit (`extensions/projectiles.md`). It stops at blocked areas and sends `ProjectileHit`. | A continuous emitter on the ball for the tail, with `relative_positioning` off so the tail stays in the world. A `OneShot` burst at the hit point. | Does the kit projectile hit enemies as well as features? It has `ProjectileTarget` for that (`adr-x27`). Check before the real build. |
| Fire or ice ring | An expanding radius. Each enemy is hit once, when the ring front passes it. | `Circle` emission gives a ring, but particles cannot be sent outward from the circle edge with one `direction`. Options: many short-lived particles with `linear_damp`, or a sprite-sheet ring. | How to keep the particles in step with the radius. The radius should drive both. |
| Crystal wall | A collision shape that enemies treat as solid, for a set time. | A row of small emitters along the wall, or one wide emitter with sparkles. Sprite sheet for the crystals. | Particles cannot block. The kit collision and nav must accept a dynamic blocker. Not checked yet. |
| Water ring | Ring with an inner and outer radius, following the player. | See below. | Which of the three options looks best. |

### Whirlpool options

Enoki has no swirl motion, so the effect needs a workaround.

1. **Rotating pivot with child emitters.** N small emitters sit on a circle of radius R around a pivot entity. The pivot turns each frame. Short-lived droplets leave arcs behind the emitters. Shrinking R over time makes a whirlpool. Uses plain transforms.
2. **Attractor plus start speed.** An attractor at the player and a start speed give loose elliptical orbits. Cheap, hard to control, uneven.
3. **Animated sprite ring plus spray.** An Aseprite animation draws the ring (timing set in the file, `adr-a6`). Enoki adds only splash and foam.

## Risks to test in the spike

| Risk | Why it matters |
|---|---|
| Web run | The WebGL2 claim is unverified. A compile check (`./dev-scripts/rust.sh web`) does not prove it renders. |
| Pixel look | Enoki particles are quads with smooth positions and scaling. They may not look pixel-aligned at a low-resolution camera. |
| CPU cost | CPU simulation. Several emitters at once (a ring plus a wall plus a fireball) may cost frame time, most of all in wasm. |
| Palette control | Colours come from `color` and `color_curve`, not from our palette. The art pipeline wants palette-exact colours (`games/long-way/docs/art-pipeline.md`). |
| Authoring effort | RON files need hand editing, or the separate editor. |
| Runtime control | We must change an effect while it runs (ring radius, wall length). `ParticleEffectInstance` should allow it. |
| Blocking wall | Needs a dynamic blocker in the collision and nav code. Outside the spike. |

## Additive blending (spike step 7b, 2026-11-10)

- Enoki hard-codes `BlendState::ALPHA_BLENDING` in its pipeline (`material.rs`), and the material trait has no way to change it. Additive needs a patch: one trait method `blend_state()` (default alpha) and two public shader constants. The spike uses a vendored copy (`games/particle-spike/vendor/bevy_enoki`, root `[patch.crates-io]`). Worth an upstream PR. A custom material alone is not enough.
- Two materials in `glow.rs` (sprite and colour) use `SrcAlpha, One, Add`. G toggles it for the next effects.
- Seen on native in stills: fire and the fireball tail gain hot yellow spots where particles overlap, and low-alpha particles fade to nothing instead of a muddy brown. Water goes to pure white where droplets overlap, so the blue palette is lost. Flame shapes merge where they overlap.
- Not checked: WebGL2, frame time (step 8), motion.

## Easing of the ring expansion (spike step 7c, 2026-11-10)

- Enoki cannot ease a particle to an exact radius: damping depends on the life fraction and the frame rate. Emitters moved along the curve can. The ring is now 40 emitters whose position is set each frame from a pure function (`rules::ring_radius`); the particles stay where they are born and fade, so the band behind the front is the trail. The hurt band uses the same function, one step back in time.
- Native cost: not measurable (ring stress run 3.2 ms before, 3.1 to 3.9 ms after, about 14 % more particles). Web cost not measured yet.

## Verdict (2026-11-10)

**Use Enoki.** Recorded in `adr-e9` (`dev-docs/adr/ENGINE.md`) and in the adr-e4 table.

| Check | Result |
|---|---|
| Web (WebGL2) renders | **Pass.** The user ran all four weapons in a browser through `wasm-server-runner`: no problem, no console errors (step 2). Browser name and version not recorded. |
| Frame time, native | **Pass.** All four together, about 3400 particles (5 times normal use): about 3.4 ms mean (about 300 fps uncapped), `dev` profile. Normal use (690 particles): 4 to 4.4 ms with vsync off, noise about 0.5 ms. Glow on or off: no measurable difference. Moving emitters (the eased ring): no measurable cost. |
| Frame time, web | **Not measured.** The user skipped it ("it works well"). The risk (one wasm thread) is untested in numbers. |
| Pixel look | **Accepted by the user** for the fireball and the ring in motion. No sub-pixel shimmer measurement was made. |
| Palette | **Partly.** Sprites carry the palette and colour curves stay near white. Particle colours from curves are not palette-exact. |
| Runtime control | **Pass.** Ring radius (eased, 40 moving emitters), wall length and per-emitter attractors change while running (`ParticleEffectInstance`). |
| Authoring | **Not timed.** Hot reload works natively, but only with `bevy/file_watcher`, which rebuilds Bevy (about 5 minutes per switch), so it is not carried into the games. |

What Enoki is good for: trails, bursts, embers, flames. What it is not good for: a placed object with a fixed life and a place in the depth order. The ice wall is plain animated sprites. A swirl has no built-in motion: the whirlpool is variant B (attractors with a start speed above the circular speed, so droplets spiral outward). The pivot (A) and sprite ring (C) variants were built and dropped.

Remaining risks: web frame time under stress, Enoki upgrades (the 10-line additive patch), palette-exact colour from curves, and the wall needing a dynamic blocker in the collision and nav code (built: `adr-x48`). Facts learned about the crate: `dev-docs/bevy/notes.md`, "bevy_enoki".

## Sources

- `bevy_enoki` 0.8.0 source and README (downloaded and read): <https://github.com/Lommix/bevy_enoki>
- `bevy_hanabi`: <https://github.com/djeedai/bevy_hanabi>, <https://docs.rs/bevy_hanabi>
- Sprinkles: <https://github.com/doceazedo/sprinkles>, <https://doce.sh/blog/bevy-sprinkles>
- crates.io dependency metadata: `https://crates.io/api/v1/crates/<crate>/<version>/dependencies`
