# PLAN: particle spike with Enoki

Background and findings so far: `RESEARCH-particles.md`.

## Goal

Find out if `bevy_enoki` 0.8.0 can draw the four player weapon effects in Long Way, on native and on the web. The spike is a stand-alone app. It has no map, no kit collision and no Long Way code. It only answers: does it look right, run fast enough, and work in the browser?

Effects to build:

1. Fireball with a trailing tail, and an explosion on impact.
2. Ring of fire or ice crystals that grows outward from the player.
3. Wall of crystalline sparkles that stays in place.
4. Water ring that swirls like a whirlpool (three variants).

## Decisions needed before step 1

| # | Question | Recommendation |
|---|---|---|
| D1 | Where does the spike live? | A workspace member `games/particle-spike/`. It uses the root `[workspace.dependencies]` for Bevy, so versions stay in step. The `games/*` glob picks it up. `./dev-scripts/rust.sh run|web|lint particle-spike` all work. The alternative is a folder outside the workspace. That needs its own Bevy pin, a second `target/` and a second build to keep in step. |
| D2 | Keep or delete the spike afterwards? | Delete it when the assessment is done. The lasting content moves to docs (step 10). The final Long Way plugin is written fresh in `long-way`. |
| D3 | How do we run it in a browser? | **Decided: `wasm-server-runner`** (installed with `cargo install`). `.cargo/config.toml` sets it as the runner for `wasm32-unknown-unknown`. Run: `cargo run -p particle-spike --target wasm32-unknown-unknown`. It serves a page and prints a URL. No HTML file. Remove the config with the spike. `trunk` stays an option for the real web release (`rm10`). |

D1 and D3 are confirmed. D2 is not blocking.

## Progress

### Status for the next session (2026-11-10, end of day)

Steps 1 to 5 are done on native: harness, fireball (with sprite particles, step 4b) and the ring of fire or ice (step 5). Everything was seen in stills only. The user has tested the fireball by hand (R key works) and has not yet looked at the final ring in motion. Committed and pushed at the end of this session.

- **Resume with:** (1) the user tries the ring in motion (`2`, `I`, `Space`, arrows) and the fireball sprites (`B`), and says what to tune. (2) Step 6, the crystal wall: the sprites `wall_fire` and `wall_ice` are drawn and exported, and the ring already shows that 40 emitters cost no visible frame time. (3) Later steps already in the plan: 7 whirlpool, 7b additive blending, 7c easing of the ring expansion, 8 measure (needs the web fix below), 9 verify, 10 migrate and delete.
- **Decisions already made:** the ball (blob) ring is out, the triangles are the only ring look. The ring hurts anything in its band, every 0.5 s (a design value for the real game). The fireball trail and impact still use round blobs (not decided). Enoki facts learned are in the log below and go to `dev-docs/bevy/notes.md` in step 10.
- **Run it:** `./dev-scripts/rust.sh run particle-spike` (native). The sprite PNGs in `games/particle-spike/assets/generated/` are git-ignored. On a fresh clone, or after you edit `fireball_placeholder.aseprite`, run `/Applications/Aseprite.app/Contents/MacOS/aseprite -b games/particle-spike/assets/art/fireball_placeholder.aseprite --script-param out=games/particle-spike/assets/generated --script games/particle-spike/export_art.lua` (the `--script-param` must come before `--script`). With hot reload of effect files on save: `./dev-scripts/rust.sh run particle-spike --features hot-reload`. Keys: mouse aims, `0` to `4` pick an effect (`1` fireball, `2` ring), `Space` fires, `I` switches the ring between fire and ice, `B` switches the fireball between sprites and squares, arrows move the red block, `R` reloads effect files, `H` hides the text, `P` takes a screenshot. Env for scripted shots: `SPIKE_SHOT=<effect number>:<seconds>`, `SPIKE_ICE=1`, `SPIKE_ENEMY=<x>`. Effect files: `games/particle-spike/assets/effects/*.particle.ron`.
- **Web is untested for the effects.** The wasm build compiles. `wasm-server-runner` serves `./assets` of the directory it runs in (the workspace root), not the crate, so the effect files and the head art probably do not load in the browser. Fix this before the web checks in step 8. The wasm server is stopped. Start it with `./dev-scripts/rust.sh run particle-spike --target wasm32-unknown-unknown`. The Chrome extension was not connected today, so the user looked at the page.
- **Build cost.** The spike had its own copy of every Bevy build, because it enabled `file_watcher`, a Bevy feature that `long-way` does not use. Hot reload is now an opt-in feature, off by default. The default Bevy features are now identical to `long-way`'s (checked with `cargo tree`). Measured after the change: a default build of the spike compiles only the spike crate (10 s after the manifest change, 5.6 s after touching a file). Bevy is not rebuilt. The user said build time matters and disk use does not for now, so do NOT run `cargo clean` (it would cost about 10 minutes and the old duplicates only cost disk). `target/` is 11 GB, over the 8 GB limit of `adr-e8`. The user accepted this for now. Each switch between `--features hot-reload` and a normal build rebuilds Bevy (about 5 minutes), so use the feature only for a whole tuning session. The same holds for `--features bevy/dynamic_linking`.
- **For step 10:** `ui_bevy_render` is needed for UI text (`ui_api` alone draws nothing). Add it to `dev-docs/bevy/notes.md`, with the Enoki facts under "Learned" below.

### Log

- Step 1 done: `games/particle-spike/` builds natively (opens a window on Metal, no errors), passes lint, and compiles for wasm. It shows the default Enoki effect rising from a white square.
- Step 2 done (2026-11-09): the wasm build, served by `wasm-server-runner`, renders the default Enoki particles in a browser. The user saw them. `bevy_render` builds with its `webgl` feature, so the backend should be WebGL2. Not checked: browser name and version, the console log, and frame rate. The Chrome extension was not connected, so Claude did not see the page. Step 3 adds an on-screen frame time and particle count, and logs the backend name at startup, so later runs record this.
- Step 2 console check: the user saw no errors in DevTools.
- Step 3 done (2026-11-09): harness in `games/particle-spike/src/harness.rs`, rules in `rules.rs` (4 unit tests). Keys: 0-4 pick, Space fire at the cursor, H hide the text, R reload effect files, P screenshot (native). `SPIKE_SHOT=<effect>:<seconds>` fires an effect, saves `target/screenshots/<name>.png` and quits. Effect 0 is a smoke test (20 default particles). Effects 1 to 4 are names only until steps 4 to 7. Each effect reads the `FireEffect` message. Native run seen on screen: HUD, stand-ins, crosshair, particle count 20, 59 fps. The R key and the web build of the harness are not run yet. Bevy already logs the graphics backend at startup (`AdapterInfo`), so no code was added for that.
- Learned: `ui_api` alone gives no UI rendering. The spike needs `ui_bevy_render`. Add this to `dev-docs/bevy/notes.md` in step 10.
- Step 4 done, first version (2026-11-09): `games/particle-spike/src/fireball.rs`, effect files `assets/effects/*.particle.ron` (10 files), head art copied from Long Way `pickups.aseprite` (tag `pickup_red`). The spike view is now 512 px high, the same as Long Way, so sizes carry over.
  - Gameplay: a point at 420 px/s, hit circle 14 px, swept test (`rules::first_hit`, 8 unit tests in all), 1100 px range. Hits an enemy or a blocker, enemy flashes white.
  - Trail: four emitters (smoke, flame, core, embers) that follow the head, rotated so the particles stream backwards. When the ball is gone they stop and remove themselves after the last particle (`OneShot::Despawn`).
  - Impact: five one-shot emitters (flash, fire, sparks, smoke, shockwave) and a muzzle burst at launch.
  - Seen on native: flight reads well. 61 particles in flight, 160 at impact, 59 to 62 fps. Impact is weak: flat hard-edged squares of 14 to 40 px look crude next to the pixel-art head. One tuning round done.
  - Learned: `scale_curve` and `color_curve` set the absolute size and colour (not multipliers). Rotating the emitter entity rotates the particle direction. A full-circle `direction` spread (1.0) gives a radial burst. A burst of equal-speed particles from one point forms a ring front. Particles are spawned at the emitter position once per frame, so fast emitters leave gaps unless particles are large. The emitter follows the ball with one frame of lag (Enoki reads `GlobalTransform`).
  - Tail shape: a cone. The flame starts 26 px wide (the ball is about 25 px) and `scale_curve` narrows it to 1 px at the end of its life. Core and smoke taper the same way. Seen on screen: about 32 art px at the ball, a point at the far end.
  - Reload (fixed 2026-11-09): the R key found nothing to reload, because `AssetServer::reload` only reloads files with a live handle and the files were dropped between shots. Now `EffectLibrary` keeps every effect file loaded. Enoki refreshes running emitters only on a first load, so `refresh_instances` copies a changed effect into the emitters on `AssetEvent::Modified`. The `hot-reload` cargo feature (`bevy/file_watcher`, native only, off by default since it adds a second Bevy build) makes saving a file reload it. Verified: an in-place save logs `effect reloaded` during a run. An atomic save (write a temp file, then rename, as `sed -i` does) is NOT seen by the watcher. Use R after such a save. The R key itself was not pressed in a test (a script cannot press it yet).
  - Not checked: the web build (asset path: `wasm-server-runner` serves `./assets` of the directory it runs in, which is the workspace root, not the crate), hot reload while running, additive blending (fire normally wants it).
- Step 4b built (2026-11-10), first version: the particle quads use sprites. Not judged in motion yet. The user accepted the proposal and changed the head art.
  - Art: `games/particle-spike/assets/art/fireball_placeholder.aseprite`, one file for the head and the particle sprites (64x64 canvas, the user made the head spherical). Tags: `head` (frames 1 to 8) and `blob` (4 round puffs, frames 9 to 12). The `flame` frames (3 pointed shapes) were removed on 2026-11-10: no effect used them. Layers `Head` and `Particles`. The old `pickups.aseprite` copy in the spike was deleted. Head: scale 0.55, anchor at the centre.
  - Export: `games/particle-spike/export_art.lua` writes `assets/generated/blob.png` and `blob_last.png` (one cell size, frames side by side). The command is in the script header. `--script-param` must come before `--script`.
  - Use: every trail, impact and muzzle emitter uses `blob_last.png` as a `SpriteParticle2dMaterial::from_texture`. Key `B` switches to the old squares for the next shot (`Look` resource in `fireball.rs`). Enemy at x=410, blocker at x=-410 (the view is about 910 px wide, so about 455 is the edge).
  - First blobs had a hard dark red rim and looked like bubbles. Redrawn without the rim (white, pale, salmon, orange, then an orange edge dithered away). Still reads as separate discs at impact, but softer. Backup of the earlier file: scratchpad only.
  - Learned (Enoki sprites): the shader multiplies the sprite by the colour curve. The sprite already carries the palette, so colour curves must be near white (a dark tint makes dark discs). The material plays the sheet over the particle's life (`frame = lifetime_frac * frames`), so a multi-frame sheet changes the picture while it lives. For one fixed picture, use a one-frame PNG. A quad is drawn at `scale` px for the whole cell, so the sprite must fill its cell, or the visible size is smaller than `scale`.
  - Learned (rotation): a particle starts with the rotation of the emitter entity. Its own direction does not matter, and `angular_speed` spins it from there. A pointed sprite works for the trail (the emitter points backwards) but not for a radial burst such as the sparks. A pointed sprite would work for the trail, but not for a radial burst. The `flame` frames were removed because nothing used them. Draw them again if a trail needs streaks.
  - Not done: additive blending (step 7b), the sprite look on the web.
- Step 5 variant A built (2026-11-10): `games/particle-spike/src/ring.rs`, effect files `ring_front` and `ring_wake`, key `2` picks it, `Space` fires it at the player, `I` switches fire and ice. Seen on native in stills only (not in motion): 590 particles, 55 to 60 fps. Not checked: the web.
  - Gameplay: `radius = 300 * t` up to 420 px, each body hit once when the ring reaches its nearest point (`rules::circle_hits_rect`, tested already). The enemy flashes white (seen).
  - Look: two point emitters at the player. Every front particle flies at the ring speed with no damping, so it stays on the ring front by construction (seen: front radius 200 px at age 0.7 s, as predicted). The wake emitter is slower, so it trails inside. Both emit for 0.12 s, which gives the band its thickness.
  - Ice: its own blue sprite (`ice` tag, same shape as the blob, frames 13 to 16 in the aseprite file, exported as `ice_last.png`), plus a replaced colour curve set at runtime on `ParticleEffectInstance` (works).
  - Learned: `EmissionShape::Circle(r)` fills a disc (distance = r x random), it is not a ring outline, so it cannot place particles on a ring front. A tinted sprite cannot change hue: the colour curve multiplies the sprite, so a red sprite tinted blue goes black. Ice needs its own sprite or a neutral grey sprite. `Attractor` exists in Enoki (`attractors` in the effect), so the attractor comparison is still possible.
  - Seen: the band reads as separate beads with gaps. More particles or bigger sprites would close the gaps. The wake is too sparse.
  - Changed (2026-11-10, user): the ring grows to a maximum radius, stays there and burns for 2 s, then disappears. Max radius 230 px (the whole ring is inside the view; 420 left the view, so only the side arcs showed). Growing: as above (front lifetime 0.767 s = 230 / 300). Burning: 40 small emitters (`ring_burn.particle.ron`) placed on the circle at the max radius, 36 px apart, started 40 px before the max radius so the front hands over to them. They twinkle (scale and alpha curves up and down, lifetime spread) and drift up a little. After 2 s the ring entity is gone, the emitters stop and remove themselves, and the last particles die within 0.9 s (seen: 0 particles at 5 s). Seen on native in stills only: 400 to 590 particles, 53 to 62 fps. Fire and ice both work. The stand-ins at x = ±410 are out of the ring's reach now, so the hit-once rule is only unit-tested, not seen with this radius (it was seen with 420).
  - Learned: Enoki damping is `velocity *= 1 + life_fraction * (acceleration - damp) * dt`, so it depends on the life fraction and the frame rate. A particle cannot be stopped at an exact radius, so gameplay and look cannot match through damping. A ring of emitters at a fixed radius is exact. 40 emitters with 1 particle every 0.07 s cost no visible frame time.
  - Decided (user, 2026-11-10): the ring also hurts anything that walks into the band. Built: the ring is a band (growing: the thickness of the front, burning: 24 px each side of the max radius). A body that overlaps it is hurt, and again every 0.5 s while it stays (`rules::rect_overlaps_band`, 4 new unit tests, 12 in all). Seen in the log with `SPIKE_ENEMY=<x>` (start x of the red block): at x=230 hurt at 0.65 s and then every 0.5 s until the ring ended (5 hits); at x=100 (inside the ring) hurt once, when the front passed; at x=410 never. The arrow keys move the red block, so you can walk it in. The key walk itself was not tested by a script. In the real game the 0.5 s interval is a damage tick, a design value for you.
  - Art for step 6 drawn (2026-11-10): tags `wall_fire` (frames 17 to 20, 4 flicker frames of a softened triangle flame pointing up, sway differs per frame) and `wall_ice` (frames 21 to 24, 4 frames of a sharp triangle ice shard pointing up, growing from 18 to 48 px with a bright ridge, a light left face, a mid right face and a darker lower right). Exported by `export_art.lua` as `wall_fire.png` (19x38 cells) and `wall_ice.png` (19x48 cells). Not used in an effect yet. Remember: Enoki plays the frames in order over a particle's life, so ice grows as the particle ages, and fire flickers.
  - Variant B built (2026-11-10): the ring with the upward triangles (the wall sprites) instead of blobs. Key `V` switches between A (blobs) and B (triangles) for the next shot, `I` fire or ice, env `SPIKE_TRI` starts with B. Effect files `ring_front_tri` and `ring_burn_tri` (bigger particles, fewer of them, no wake). Seen on native in stills only: about 310 particles, 59 to 62 fps, both fire and ice. The shards and flames all point up (they do not turn to the ring), and a flame or shard looks like a tree or a cone on the ring more than like a wall of flame. The ice reads well. The wall sheets are now exported with square cells (`wall_fire.png` 38x38, `wall_ice.png` 48x48), because Enoki draws every particle as a square quad and tall cells would be stretched.
  - Learned: the particle quad is square (`scale` is one number), so a sprite sheet needs square cells. Particles do not turn to face the ring unless the emitter is rotated, so a ring of upright sprites needs one rotated emitter per angle (the burners could be rotated to point outward: a possible next test).
  - Decided (user, 2026-11-10): the ball (blob) ring is out, the triangles are the only ring look. Removed: variant A code, the `V` key, `SPIKE_TRI`, the wake emitter, the blue blob frames (tag `ice`, frames 13 to 16), `ice.png` and the runtime colour change on `ParticleEffectInstance` (it worked, it is not needed once fire and ice have their own sprites). `ring_front` and `ring_burn` are now the triangle versions. The fireball trail and impact still use the round blob `blob_last.png` (not decided). The aseprite file now has 20 frames: `head` 1 to 8, `blob` 9 to 12, `wall_fire` 13 to 16, `wall_ice` 17 to 20.
  - Flames redrawn (user): irregular outline (noise on each side), a lick on one side, and the tip curls to the right (4 frames, curl 4 to 9 px). Cell is now 40x40 (was 38x38). Seen on the ring in a still: reads as flames, not cones. The shards were kept as they were (user: they look good).
- Step 5 next: tuning from your look, then step 6 (the wall can reuse these sprites; the attractor comparison is optional now that the ring does not need it).


## Steps

### 1. Scaffold the crate

- Add `games/particle-spike/` with `Cargo.toml` (`bevy.workspace = true`, `bevy_enoki` pinned exact in `[workspace.dependencies]`, in the same style as the other pins).
- `main.rs`: a fixed low-resolution camera (for example 320x180), integer scaling, nearest-neighbour sampling, as in Long Way. Add `EnokiPlugin`.
- A flat background and a fixed "player" marker square in the middle.
- Run `./dev-scripts/rust.sh size`, then check disk use as `CLAUDE.md` says.

### 2. Web gate (go or no-go)

- Run `./dev-scripts/rust.sh web particle-spike`. It must compile.
- Run the one-particle-effect scene in a browser using the method from D3. Use WebGL2, not WebGPU.
- If particles do not render on WebGL2, stop. Record this in the research file. The decision is then "not Enoki". Look at a custom sprite-based system or `bevy_particle_systems` next.

### 3. Test harness

- Keys 1 to 6 pick an effect. Space fires or triggers it. The mouse sets the aim point.
- On-screen text: effect name, live particle count, frame time. A key to toggle the text.
- A key to save a screenshot to `target/screenshots/<name>.png` (Bevy `Screenshot`, native only). This lets Claude see the result.
- A key to reload effect RON files at runtime (asset hot reload), to test authoring speed.
- Plain rectangles stand in for "enemy" and "blocked feature". Rules stay in plain Rust functions with a few unit tests.

### 4. Fireball

- Gameplay: a moving point, `delta_secs()`, a hitbox, a hit test against the rectangles.
- Tail: continuous emitter on the ball, `relative_positioning` off, `scale_curve` and `color_curve` from white-yellow to red to nothing.
- Impact: a `OneShot::Despawn` burst at the hit point, plus a short second burst of sparks.
- Check: does the tail look pixel-clean? Does it stay in the world when the ball turns?

### 5. Ring of fire or ice crystals

- Gameplay: `radius = speed * t`. A hit test per rectangle, once each.
- Variant A: `Circle` emission with short life and `linear_damp`. Compare it with a ring of attractors.
- Variant B: a sprite-sheet material with crystal frames.
- Check: can the radius drive the effect, so the particles stay on the ring front? Can we change the effect at runtime with `ParticleEffectInstance`?
- Build fire and ice from one effect with different colour curves.

### 6. Crystal wall

- Gameplay: a solid rectangle that lasts T seconds. A dummy enemy moves into it and stops.
- Visual: a row of small emitters along the wall, with sparkles that twinkle (`scale_curve`), on a sprite-sheet material.
- Check: does the wall look solid and stable? What does the cost per emitter look like? Note what the real build needs from the kit collision and nav (not built here).

### 7. Water ring (whirlpool)

Build all three, so we can compare:

- **A. Rotating pivot.** N child emitters on a circle, pivot turns with `delta_secs()`. Try inward spiral (shrinking R) and two counter-rotating pivots.
- **B. Attractor plus start speed.**
- **C. Animated sprite ring plus spray.** Draw a placeholder ring animation in Aseprite (`_placeholder` suffix, timing in the file, `adr-a6`). Read `games/long-way/docs/art-pipeline.md` first. Do not touch `diodrama-assets`.
- Pick the best one for the assessment.

### 7b. Additive blending

Do this after step 7, when the ring, the wall and the whirlpool exist, so the test has more than the fireball.

- Fire and sparks normally glow where particles overlap. Enoki draws with alpha blending. Additive blending needs a custom material (`Particle2dMaterial` with its own shader and blend state). Check the Enoki source and `bevy-src/` first.
- Build one additive material. Switch the effects over with the `B` look key (or a second key) and compare, on all four weapons together.
- Check: does it look better on fire, ice and water? Does it work on WebGL2? Does it change the frame time? Does it break the palette (overlap goes toward white)?
- Record the result in the step 8 table (add a row if the answer matters) and in `RESEARCH-particles.md`.

### 7c. Easing of the ring expansion

Consider this after step 7. Do not build it before. The ring now grows at a constant 300 px/s. The wish: it expands quickly at the start and slows down as it reaches the max radius (an ease-out curve).

- Gameplay first: the radius becomes `MAX_RADIUS * ease(t / GROW_SECS)` in a pure function in `rules.rs` (for example a quadratic or cubic ease-out), with unit tests. The hurt band follows it, so check that a body is still hurt once when the front passes it.
- The look is the hard part. The front particles fly at a constant speed, so they cannot follow an eased radius. Enoki damping is `velocity *= 1 + life_fraction * (acceleration - damp) * dt`: it depends on the life fraction and the frame rate, so it will not match an exact curve. Options to compare:
  - Emitters on the circle, as the burners are: move a ring of about 40 to 60 emitters outward each frame with the eased radius. Exact, and it makes the growing front and the burning ring one system. Cost: more emitters moving every frame, so check the frame time with all four weapons.
  - Keep the constant-speed front, tune `linear_damp` and `linear_speed` by eye, and drive the gameplay radius from a measured curve. Simple, but the front and the band can drift apart.
  - Several short pulses with decreasing speed. Looks stepped, so probably not.
- Check: does the front stay on the hurt band during the whole growth (take stills at three times and measure the radius)? Does it look right with the flame and shard sprites (the shards grow over the particle life, so the sheet timing may need a change)? Frame time on native and web.
- Record the result in `RESEARCH-particles.md`.

### 8. Measure

Fill this table in the research file. Run each effect alone, then fireball + ring + wall + whirlpool together.

| Check | Pass when |
|---|---|
| Web (WebGL2) renders | Visible, no console errors |
| Frame time, native | 60 fps with all four effects |
| Frame time, web | At least 30 fps with all four effects |
| Pixel look | No visible sub-pixel shimmer at 1x integer scale, or a snap fix is simple |
| Palette | Colours can be set to palette values |
| Runtime control | Radius and length change while running |
| Authoring | A new variant takes less than 10 minutes with hot reload |

### 9. Verify

- `./dev-scripts/rust.sh lint` and `./dev-scripts/rust.sh test particle-spike` (rules only).
- `./dev-scripts/rust.sh web particle-spike`.
- Screenshots from step 3 for each effect. State which parts Claude did not see on screen (motion, timing and the web run cannot be judged from a still image).
- Delete files in `target/screenshots/` when done.

### 10. Decide and migrate

- Write the verdict in `RESEARCH-particles.md` (measurements, chosen whirlpool option, risks that remain).
- Add an ADR in `dev-docs/adr/ENGINE.md` (use or reject Enoki, and why) and add `bevy_enoki` to the adr-e4 table if accepted.
- Verified Enoki patterns go to `dev-docs/bevy/notes.md`. A new Bevy concept (custom particle materials) goes to `dev-docs/bevy/concepts.md`.
- If accepted, add a roadmap item for the real weapons in Long Way (next free ID, with the gameplay-versus-visual split and the dynamic blocker for the wall). Check `rm9` (Combat basics) for overlap.
- If a reusable design comes out of it (for example a weapon effect spawner), write it up in `dev-docs/extensions/` and link its ADR. Keep it in the game until a second game needs it.
- Move web-run instructions to `docs/README.md` if D3 produced any.
- Delete `games/particle-spike/` (D2), then delete this plan file. Git history keeps it.
- Ask before committing. Do not push.

## Out of scope

- Real weapon gameplay in Long Way: damage, cooldowns, mana, input, aiming. These are design questions for you.
- Kit collision and nav changes for the dynamic wall blocker.
- Sound.
