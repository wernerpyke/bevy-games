# PLAN: weapons for Long Way (migrate from the spike, then add them inactive)

Source of the code and the findings: `PLAN-particle-spike.md` and `RESEARCH-particles.md`. The spike (`games/particle-spike/`) stays in place until the last step of Part 2. It is the porting source.

## Goal

1. **Part 1, migrate.** Move what is reusable into `game-kit`. Move the rest into `long-way`. Add `bevy_enoki` to Long Way without making the main build slower or `target/` bigger than it must be.
2. **Part 2, weapons.** Add the four weapons (fireball, ring, wall, whirlpool) to Long Way as clean code, assets, tests and docs. They stay **inactive**: no player input, no cost, no damage, no HUD. A separate plan switches them on.

## Out of scope (the activation plan)

Input and aiming, energy cost, cooldown, damage values, enemy hit points and death, sound, HUD, which weapon the player owns, glow on or off per weapon, the final gameplay numbers (all values from the spike are defaults, not design decisions). `rm9` (Combat basics) owns damage and enemy death. This plan only builds the layer under it: effects, hit detection and the blocker.

## Assessment: what goes where

Rule (`CLAUDE.md`, `adr-x12`): code goes into the kit only when R-Town could use it with no Long Way value in it **and** it adds no new dependency to R-Town. Everything else stays in the game.

| Spike piece | Goes to | Why |
|---|---|---|
| `rules.rs`: `ease_out`, `ring_radius`, `circle_hits_rect`, `rect_overlaps_band`, `push_delta` | **Kit**, new `area.rs` (Bevy `Rect`, no new dependency) | Plain geometry with no game value. A blast or shield in R-Town needs the same. Tests move with it (`adr-w7`). |
| `rules.rs`: `Rect2`, `rects_overlap` | **Dropped** | The kit has `Rect` and `hitbox::overlaps`. |
| `rules.rs`: `move_rect`, `sweep_circle`, `first_hit` | **Dropped** | The kit has `collision::move_feet` and `projectile` (the swept test against a `HitBox` and blocked areas). |
| Fireball flight and hits | **Kit, already there**: `projectile.rs` (`Projectile`, `ProjectileTarget`, `ProjectileHit`) | Needs no change. The range limit (1100 px in the spike) is a Long Way system that despawns the shot. The spike hit circle (14 px) becomes the target's `HitBox`. |
| Wall as a blocker | **Kit**, `collision.rs`: add and remove a temporary blocked area at run time | `CollisionWorld` is only rebuilt on map load today (`rocks.rs` `rebuild_collision`). Needs a design, see D5. |
| Draw order (z numbers in the spike) | **Kit `layers.rs` only if needed** | Plan: every effect uses `DepthSort` in the world layer, so no bare z numbers. Add a named layer only if a ring or the whirlpool must draw below the player. |
| Enoki integration: `EnokiPlugin`, effect library, one-shot and follow-the-owner helpers, additive materials (`glow.rs`) | **Kit**, new `fx.rs` (user decision D3, 2026-11-10) | Generic: no weapon values, no assets. **Cost accepted by the user:** the kit now depends on Enoki, so R-Town also compiles the 5 extra crates, though it does not use them. The plugin is opt-in (`FxPlugin`), so R-Town runs no Enoki code. If this cost grows, split a `game-fx` crate (`adr-x12` allows it). Record this in the ADR. |
| `fireball.rs`, `ring.rs`, `wall.rs`, `whirlpool.rs` | **Long Way**, `src/weapons/` | Weapons, values and art are game content. |
| `harness.rs`, `main.rs`, `SPIKE_*` env, benchmark mode | **Dropped** | Spike tooling. Long Way has the script runner. Numbers stay in the research file. |
| Whirlpool variants A and C, `whirlpool_placeholder.aseprite` | **Dropped** | Variant B is the pick (D6). |
| Effect files (16 `.particle.ron`) | **Long Way**, `assets/effects/` | Copy. Drop any file no kept weapon uses. |
| Art: `fireball_placeholder`, `ice_wall_placeholder` | **Long Way**, `assets/art/weapons/` | Copy. Keep the `_placeholder` suffix until you replace the art. |
| `export_art.lua` | **Shared**, `dev-scripts/aseprite/`, if no existing script does the job | Check the shared scripts first. |
| `draw_*.lua` | **Dropped** | They only made the placeholders. The `.aseprite` files are the source of truth. |

## Dependency and build guard

Measured on 2026-11-10 with `cargo tree` on the wasm target: the spike and Long Way trees differ by **5 crates** only (`bevy_enoki`, `wide`, `chacha20`, `getrandom` 0.3.4 and 0.4.3). They have the same Bevy features. So the main build does not spiral as long as these rules hold:

| # | Rule |
|---|---|
| G1 | `bevy_enoki` is a plain dependency of `crates/game-kit` (D3), and `long-way` and `r-town` get it through the kit. **No Cargo `[features]` in `game-kit`**, not even an optional `particles` feature: a feature would build the kit twice (`--workspace` against `-p`), the same trap as the spike's `file_watcher`. The cost is that R-Town compiles Enoki too. |
| G2 | No new Bevy feature. Long Way keeps `bevy.workspace = true`. |
| G3 | Not carried over: the spike feature `hot-reload` (`bevy/file_watcher`), `bevy/dynamic_linking`, the Enoki `dev` feature (tracing, Tracy). Effect files are reloaded by restarting, or by a reload key that uses no watcher. |
| G4 | `bevy_enoki` stays pinned `=0.8.0` in `[workspace.dependencies]`. The patch (D1) changes the source of that one crate only. |
| G5 | Gates, measured before (P0) and after every phase that touches a `Cargo.toml`: see the table below. Stop and ask when one fails. |

| Gate | Pass when |
|---|---|
| Tree diff, `cargo tree -p long-way`, native and wasm | Only the 5 crates above are new |
| `cargo tree -p r-town` and `-p game-kit` | Only the same 5 crates are new (the kit carries Enoki, D3). Nothing else. |
| `cargo tree -e features -i bevy -p long-way` | Identical to the baseline |
| `git diff Cargo.lock` | Only added packages. No existing crate changes version. |
| `./dev-scripts/rust.sh build long-way` | The "Compiling" lines name no Bevy crate |
| `./dev-scripts/rust.sh lint`, run twice | The second run compiles nothing |
| `./dev-scripts/rust.sh size` | Report the change in `target/`. Over 8 GB: say so (`adr-e8`). |
| Web release size (`--profile wasm-release`) | Report the change in the `.wasm`. Grows by more than 1 MB: ask you. |
| Time to rebuild after touching one Long Way file | Report before and after. Not worse by more than noise. |

`target/` is 11 GB today (over the 8 GB limit, accepted for now). The spike's Bevy builds stay in it as dead weight after the spike is deleted. Ask before `cargo clean` (about 10 minutes).

## Decisions needed

| # | Question | Recommendation |
|---|---|---|
| D1 | Additive blending (glow) needs a patched Enoki (`blend_state()` on the material trait, about 10 lines). How do we carry it? | **A. Keep a vendored copy** at the repo root `vendor/bevy_enoki`, with the root `[patch.crates-io]`. Open an upstream issue or PR. Cost: re-apply 10 lines on every Enoki upgrade. **B. Drop glow.** No patch, no vendor folder, plain alpha blending. You said the gain is small on some effects. **C. Fork on GitHub** as a git dependency: needs network at build time and a pinned revision. |
| D2 | Enoki cannot read `.aseprite`. Sprite sheets must be PNG strips, and `assets/generated/` is git-ignored (`adr-a1`), so a fresh clone and the web build lack them. | First check (P3) whether the `Aseprite` asset of `bevy_aseprite_ultra` can feed Enoki directly. If not, add an export step to the shared scripts and a start-up check that reports a missing sheet. |
| D3 | Where does the Enoki code live? | **Decided: `game-kit`**, as `fx.rs` (user, 2026-11-10; I had recommended `long-way`). See G1. |
| D4 | The wall must block ghosts, the player and projectiles. Bodies may stand inside its rectangle when it appears. Ghost paths (`nav.rs`) are built from the collision outlines. | Needs your input in P1: place the wall only where no body overlaps (shift it), or push bodies out, or let bodies inside walk out (the kit already ignores a start inside an area). Recommendation: let them walk out, no push. |
| D5 | How do we trigger a weapon in a test while nothing in the game is active? | **Decided:** an env variable `LONG_WAY_WEAPON_TEST` (a weapon name, or `all`) read by a test system, off in normal play. It is **temporary**: the activation plan MUST remove it and use the normal command structure (the game's input and command path, and the script runner commands) instead. See P10. |
| D6 | Whirlpool variant? | B (attractor, spiral outward, with push). Variants A and C are dropped. |

**Decided (user, 2026-11-10):**
- **D1: keep the patch for now** (option A, vendored copy at `vendor/bevy_enoki`). Revisit when Enoki upgrades or upstream accepts the change. D1 B (drop glow) stays the fallback if the patch causes trouble.
- **D2: agreed.** Check first whether the Aseprite asset can feed Enoki directly. Fall back to an export step and a start-up check.
- **D4: agreed.** A body inside the wall rectangle when it appears walks out. No push, no shifting of the wall.
- **D3: Enoki code in `game-kit`.** **D5: env variable test trigger, temporary, replaced by the normal command structure in the activation plan.**
- **Update 2026-11-11 (weapon drawer, `adr-lw31`):** the player now has a real way to pick a weapon: the drawer in the HUD sends `WeaponChosen(WeaponKind)` (`games/long-way/src/weapon_menu.rs`, six kinds: fireball, fire ring, ice ring, fire wall, ice wall, whirlpool; red energy has the fire weapons, blue the ice ones and the whirlpool). `CastWeapon { weapon, origin, aim }` is built from it: P-steps below add a system that turns `WeaponChosen` into `CastWeapon` for the player. The weapon kinds are the ones in `weapon_menu.rs`, so reuse that enum instead of making a second one. Scripted runs click the drawer with `ui_click:<x>:<y>` (`dev-feedback.md`; the red icon is `ui_click:66:588`, the first button `ui_click:136:465`). The env variable `LONG_WAY_WEAPON_TEST` is then only needed for the all-four stress run, and the activation plan removes it. **Aim from a button is not decided:** the cursor is on the button, so the options are (a) the last cursor position seen in the game area, (b) the direction the player faces, (c) the next click in the game area sets the aim. The activation plan decides (P10).
- D6 was not answered. Treat the recommendation as the plan (whirlpool variant B) unless you say otherwise.

Glow on or off per weapon is **not** decided here. The weapons get a `glow: bool` in their values, set from the spike result (fire and fireball on, water off), and the activation plan chooses.

## Part 1: Migrate

### P0. Baseline (before any change)

- Run every gate in the table above on the current tree. Write the numbers into a "Baseline" table in this file.
- Record how long a rebuild takes after touching `games/long-way/src/main.rs`, and the `.wasm` size of `long-way` with `wasm-release`.
- Note the Cargo.lock state (`git status` must be clean).

### P1. Kit changes (no Enoki)

- `crates/game-kit/src/area.rs`: port the five functions on Bevy `Rect`. Tests moved from the spike (`ease_out`, ring radius, band overlap including the "consecutive frames overlap, so no body is skipped" test, push). Export in `lib.rs`.
- `crates/game-kit/src/collision.rs`: a temporary blocker. Design after D4. Needs: add at run time, remove on expiry, no stale edges, cost of one outline rebuild per change (measure), and `nav.rs` picking it up. Unit tests in the kit (a shot, a body and a path against a wall that appears and expires).
- `layers.rs`: change only if P4 shows a need.
- Docs: `dev-docs/extensions/area-effects.md` (new) and an update to `collision.md` and `projectiles.md` (no range limit: the game despawns). Add both to `dev-docs/extensions/README.md`.
- Check: `./dev-scripts/rust.sh test game-kit`, `lint`, `web`. R-Town still builds (`./dev-scripts/rust.sh build r-town`). Gate G5 (no `Cargo.toml` change here, so the tree must not move).

### P2. Enoki into Long Way (no weapons)

- Move `games/particle-spike/vendor/bevy_enoki` to `vendor/bevy_enoki` (D1 A) and change the root `[patch.crates-io]` path. Check whether the root `exclude` line is still needed (a folder outside `games/*` and `crates/*` may not be a member). The spike keeps compiling against the moved copy. With D1 B: skip, and remove the patch with the spike.
- `games/long-way/Cargo.toml`: add `bevy_enoki.workspace = true`. Nothing else.
- `crates/game-kit/Cargo.toml`: add `bevy_enoki.workspace = true` (a plain dependency, no feature, G1). Nothing else. `games/long-way/Cargo.toml` does not change. Run `./dev-scripts/rust.sh build r-town` and read the "Compiling" lines: Enoki's 5 crates appear, no Bevy crate.
- `crates/game-kit/src/fx.rs` (add `pub mod fx;` to `lib.rs`): `FxPlugin` (adds `EnokiPlugin` and the two additive materials, opt-in so R-Town runs no Enoki code), `EffectLibrary` (keeps effect files loaded, so a reload works), one-shot spawn helper, "emitter follows its owner" system, `Glow` helpers. Port from `glow.rs`, `fireball.rs` (`spawn_emitter`, `spawn_one_shot`, `emitters_follow`) and the library code.
- Add `FxPlugin` to `main.rs`. Long Way must look and run the same: run a scripted shot before and after (`script long-way "wait:0.5;screenshot:start;quit" --debug`) and compare.
- Run all gates. This is the phase where the build cost shows. Stop here and report if any gate fails.
- Ask before committing (commit 1: kit, commit 2: Enoki).

### P3. Assets and sheets

- Copy art to `games/long-way/assets/art/weapons/` and effect files to `games/long-way/assets/effects/`. Keep only what the four kept weapons use.
- Decide D2 (direct from Aseprite, or an export step). If an export step: one shared Lua script in `dev-scripts/aseprite/` (do not copy `export_art.lua` as it is, it hard-codes the spike's tags), a line in `dev-docs/extensions/dev-scripts.md`, and the start-up check.
- Art rule from the spike (step 10 there): after a script makes tags, run `./dev-scripts/aseprite.sh check <file>` and read the frame ranges. Tags must not overlap.
- Update `games/long-way/docs/art-pipeline.md` (files copied) and `art-tags.md` (tags of the new files). Do not touch `diodrama-assets`.

## Part 2: Weapons (inactive)

### Shape of the code

```
games/long-way/src/weapons/
├── mod.rs        WeaponPlugin: the messages, the sub-plugins, the TEMPORARY test trigger (removed by the activation plan)
├── values.rs     every number of every weapon in one place, with its source (spike default)
├── fireball.rs   FireballPlugin
├── ring.rs       RingPlugin (fire and ice)
├── wall.rs       WallPlugin (fire and ice)
└── whirlpool.rs  WhirlpoolPlugin (variant B, with push)
```

- **Two messages are the whole public surface.** `CastWeapon { weapon, origin, aim }` asks for a weapon. `WeaponHit { target, weapon, kind }` reports an impact, a damage tick or a push. In the game, **nothing sends the first and nothing reads the second.** The activation plan connects them to input, cost and enemy damage.
- **Gameplay and visuals are separate systems.** Gameplay owns the state (the ring radius, the wall rectangle and timer, the whirlpool band) and sends `WeaponHit`. Visuals read that state and spawn emitters and sprites. The gameplay half runs and is tested with no particles.
- Hit targets use the existing `HitBox` of a ghost. Add `ProjectileTarget` to ghosts only if the fireball needs it (inert while no shot exists).
- Hurt ticks (0.5 s in the spike) are a per-target timer inside the weapon, not a damage rule.
- Draw order: `DepthSort` on every emitter and sprite. No bare z numbers (`adr-x9`).
- Per weapon, port the spike behaviour and nothing more:
  - **Fireball:** `Projectile` from the kit, head sprite, four trail emitters that follow it and stop with it, impact bursts on `ProjectileHit`, muzzle burst, range limit.
  - **Ring:** eased growth (`area::ring_radius`), 40 moving front emitters, 40 burners at the maximum radius, band hit test, fire and ice from two sprite sheets.
  - **Wall:** rectangle across the aim, temporary blocker from P1, fire as Enoki flames, ice as animated sprites (one tag per cube height, played once, fade by alpha).
  - **Whirlpool:** variant B only: 10 emitters with attractors set per emitter, spiral outward, band hit test, push away from the player (blocked by the collision world).
- No weapon touches `Inventory`, `Hp`, the HUD or the player's movement.

### P4. Fireball

- Port and test: unit tests for the range limit and for the muzzle point. A scripted run with the test trigger shows flight, hit on a tree, hit on a ghost.
- Check frame time and the particle count with the fireball fired every 0.7 s.

### P5. Ring

- Port with `area` functions. Tests: the band reaches back far enough that no body is skipped between frames, one hit when the front passes, repeat ticks while in the burn band. Scripted run with the ghost placed at three distances.

### P6. Wall

- Port with the P1 blocker. Tests (kit side done in P1): the Long Way side tests the lifetime (3.3 s), the placement across the aim, and removal of the blocker on expiry. Scripted run: the ghost stops at the wall, then walks on after it expires.
- Check the horizontal and vertical wall, and the ghost path (`nav.rs`) around it.

### P7. Whirlpool

- Port variant B. Keep the unit test that integrates the Enoki attractor formula (droplets spiral outward across the band). Tests for the push (blocked by a tree). Scripted run with a ghost walking at the player.

### P8. Inactive check and all-four stress

- `grep` shows that only the test trigger sends `CastWeapon`.
- With the env variable unset, a normal scripted run is identical to the P2 run (screenshot and log).
- Stress: all four, fired again every 0.7 s with `LONG_WAY_WEAPON_TEST=all`. Compare the frame time with the spike figures in `PLAN-particle-spike.md` (about 3.4 ms on native at about 3400 particles). The web is not measured (you skipped it); say so.
- Run all gates once more. Report the final `target/` size and `.wasm` size.

### P9. Verify

- `./dev-scripts/rust.sh lint`, `test` (all crates), `web long-way`, `build r-town`.
- Screenshots for each weapon with `--debug`. State what was not seen on screen: motion, timing, the web run.
- Delete files in `target/screenshots/`.

### P10. Write the activation plan (required, before P11)

Write `dev-docs/PLAN-weapons-activation.md` as a fully built out plan, in the same style as this one. Ask the user for the design first (`CLAUDE.md`: do not invent mechanics). It MUST contain:

- **Remove the temporary test trigger** (`LONG_WAY_WEAPON_TEST` and its system, D5) and replace it with the **normal command structure**: the game's real input path (the player input and aim that send `CastWeapon`), and the scripted run commands so `rust.sh script long-way "..."` can cast a weapon like a player does (`crates/game-kit/src/script.rs`, `dev-feedback.md`). A step to check by `grep` that no env trigger is left.
- **Connect the drawer:** `WeaponChosen` (already sent by the weapon drawer, `adr-lw31`) to `CastWeapon`, with the aim decision of the update note in D5 (ask the user). Energy cost and a disabled button when the count is 0 are drawer features that belong here too.
- The design questions that this plan left open: how the player gets and selects a weapon, aim and input, energy cost (`Inventory`, `adr-lw27`), cooldown, damage and enemy hit points (`rm9`), death and removal of a ghost, the HUD, sound, the final values in `values.rs` (replacing the spike defaults), glow per weapon (D1), the whirlpool push rules and whether it also hurts, whether weapons affect the player's own movement.
- Wiring `WeaponHit` to damage, and `ProjectileTarget` on ghosts if P4 left it out.
- Tests, scripted runs with `--debug`, the "what was not seen on screen" statement, docs and ADRs, a final migrate step, and commits that ask first.
- Link it from `rm9` in `dev-docs/ROADMAP.md`.

### P11. Migrate and delete

- ADRs (next free number in each file, record why, not how): `ENGINE.md` (Enoki accepted, added to the adr-e4 table, the patch decision from D1, G1 to G4 as rules) is done in step 10 of the spike plan; add here: `EXTENSIONS-GAMEPLAY.md` (area geometry and the temporary blocker in the kit), `EXTENSIONS-GAMEPLAY.md` also gets the Enoki-in-the-kit decision (D3: why, the 5-crate cost to R-Town, no Cargo features, split to `game-fx` if the cost grows), and `games/long-way/docs/adr.md` (weapons are inactive and built behind two messages, gameplay and visuals separate, the env test trigger is temporary).
- Docs: `games/long-way/docs/weapons.md` (values, files, the test trigger), `source-layout.md` (the `weapons/` folder and the extension table), `dev-docs/extensions/area-effects.md` (final), `dev-docs/README.md` map, `docs/README.md` (the web run instructions from the spike plan, and the effect export step if D2 needs it). Check `CLAUDE.md` for lines to change (commands table, game table).
- Update `rm9` in `dev-docs/ROADMAP.md`: say that the weapons exist, are inactive, and cite this plan's ADR and `PLAN-weapons-activation.md` (written in P10). Do not delete this plan until P10 is done.
- Delete `games/particle-spike/` (D2 of the spike plan), the root `[patch.crates-io]` and `exclude` lines if no longer needed, and `PLAN-particle-spike.md`. Then delete this plan. Git history keeps both.
- Run `./dev-scripts/rust.sh prune`. Ask before `cargo clean`.
- Ask before each commit. Do not push.

## Risks

| Risk | Handling |
|---|---|
| The patched Enoki breaks on an upgrade | The patch is 10 lines. The ADR lists them. Upstream issue. D1 B removes the risk. |
| The wall blocker costs too much (outline and nav rebuild per change) | Measured in P1. A wall appears once per cast, not every frame. |
| Enoki adds more to the `.wasm` than expected | Gate G5, measured in P2. Ask at more than 1 MB. |
| The fresh-clone and web builds miss the generated sheets | D2, with a start-up check. |
| Spike values are mistaken for design | `values.rs` names the source of each number. The activation plan owns them. |
| Effect files cannot reload without the watcher | Accepted. Authoring happens in the spike until it is deleted, or by restart. |
