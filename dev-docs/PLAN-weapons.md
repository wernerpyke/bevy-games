# PLAN: weapons for Long Way (migrate from the spike, then add and activate one weapon at a time)

Source of the code and the findings: `PLAN-particle-spike.md` and `RESEARCH-particles.md`. The spike (`games/particle-spike/`) stays in place until the last step. It is the porting source.

## Status (2026-11-11, end of session): resume here

Done and committed: P0 to P4, W1 (fire ring, built, **values not tuned yet**), the ghost `Hurt` frames (`HurtFrames`, `EnemyHurt`), the `FacingSet` fix (a ghost did not die because `animate_facing` overwrote `Die`), the `info!` lifecycle logs (`dev-feedback.md` rule 4b, `games/long-way/docs/weapons.md` "Reporting a problem"). Next: tune W1 with the user's numbers, then **W2 (ice ring)**: reuse `weapons/ring.rs` (add an ice element, the four `ice_ring1` to `ice_ring4` looks from `FxLooks`, the two ice effect files `ring_front_ice` and `ring_burn_ice` from the spike, two emitters per position), `ToEdge` pushback. Design questions for W2 are in its section. WASM is out of scope (see Out of scope). Not seen on screen yet: a ghost pinned against a tree by a push, a cast near the map edge.

## Goal

1. **Part 1, migrate.** Move what is reusable into `game-kit` (geometry, the wall blocker, **and all Enoki code**). Move the rest into `long-way`. Add `bevy_enoki` to `game-kit` without making the main build slower or `target/` bigger than it must be.
2. **Part 2, weapons.** First build the shared base: enemy hit points, damage, pushback and the link from the weapon drawer. Then add the weapons **one at a time**. Each weapon is built, wired into the drawer (`WeaponChosen`), tested and tuned before the next one starts. Order set by the user: (1) fire ring, (2) ice ring, (3) fire wall, (4) ice wall, (5) whirlpool, (6) fireball.

## Out of scope

**WASM.** This plan does not build, check or measure the web target: no `rust.sh web`, no `wasm-release`, no `.wasm` size, no web run. The spike showed that Enoki runs on the web. A later plan does the WASM compatibility check (this overrides `CLAUDE.md` working rule 8 for this plan, by the user's decision on 2026-11-11). `target/wasm32-unknown-unknown` was deleted.

Energy cost, cooldown and a disabled drawer button at 0 energy (D12: a separate plan later), sound, new HUD elements (the drawer exists, `adr-lw31`), the game over screen and restart flow (`rm9` keeps them), and R-Town. Final numbers are set by tuning in each weapon step (the spike values are only the first defaults).

## Assessment: what goes where

Rule (`CLAUDE.md`, `adr-x12`): code goes into the kit only when R-Town could use it with no Long Way value in it **and** it adds no new dependency to R-Town. **Exception, decided by the user (D3): the Enoki code goes into the kit even though R-Town then compiles the extra crates.** Everything else stays in the game.

| Spike piece                                                                                                             | Goes to                                                                      | Why                                                                                                                                                                                                                                                                                      |
| ----------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rules.rs`: `ease_out`, `ring_radius`, `circle_hits_rect`, `rect_overlaps_band`, `push_delta`                           | **Kit**, new `area.rs` (Bevy `Rect`, no new dependency)                      | Plain geometry with no game value. Tests move with it (`adr-w7`).                                                                                                                                                                                                                        |
| `rules.rs`: `Rect2`, `rects_overlap`                                                                                    | **Dropped**                                                                  | The kit has `Rect` and `hitbox::overlaps`.                                                                                                                                                                                                                                               |
| `rules.rs`: `move_rect`, `sweep_circle`, `first_hit`                                                                    | **Dropped**                                                                  | The kit has `collision::move_feet` and `projectile`.                                                                                                                                                                                                                                     |
| Fireball flight and hits                                                                                                | **Kit, already there**: `projectile.rs`                                      | Needs no change. The range limit is a Long Way system that despawns the shot.                                                                                                                                                                                                            |
| Wall as a blocker                                                                                                       | **Kit**, `collision.rs`: add and remove a temporary blocked area at run time | `CollisionWorld` is only rebuilt on map load today (`rocks.rs` `rebuild_collision`). See D4 and P1.                                                                                                                                                                                      |
| Draw order                                                                                                              | **Kit `layers.rs` only if needed**                                           | Every effect uses `DepthSort` in the world layer. Add a named layer only if a ring or the whirlpool must draw below the player.                                                                                                                                                          |
| Enoki integration: `EnokiPlugin`, effect library, one-shot and follow-the-owner helpers, additive materials (`glow.rs`) | **Kit**, new `fx.rs` (D3)                                                    | Generic: no weapon values, no assets. **Cost accepted by the user:** the kit depends on Enoki, so R-Town also compiles the 5 extra crates. `FxPlugin` is opt-in, so R-Town runs no Enoki code. If the cost grows, split a `game-fx` crate (`adr-x12` allows it). Record this in the ADR. |
| `fireball.rs`, `ring.rs`, `wall.rs`, `whirlpool.rs`                                                                     | **Long Way**, `src/weapons/`                                                 | Weapons, values and art are game content.                                                                                                                                                                                                                                                |
| `harness.rs`, `main.rs`, `SPIKE_*` env, benchmark mode                                                                  | **Dropped**                                                                  | Spike tooling. Long Way has the script runner.                                                                                                                                                                                                                                           |
| Whirlpool variants A and C                                                                                              | **Dropped**                                                                  | Variant B is the pick (D6).                                                                                                                                                                                                                                                              |
| Effect files (16 `.particle.ron`)                                                                                       | **Long Way**, `assets/effects/`                                              | Copied **in the step of the weapon that uses them**.                                                                                                                                                                                                                                     |
| Art: `fireball_placeholder` (now `fireball_and_rings`), `ice_wall_placeholder` (now `ice_wall`), ring sheets                                                        | **Long Way**, `assets/art/weapons/`                                          | Copied in the step of the weapon that uses them. The `_placeholder` suffix is dropped from the Long Way copies (user, 2026-11-11).                                                                                                                                                                                                     |
| `export_art.lua`                                                                                                        | **Dropped**                                                                  | Replaced by run-time strips in the kit (`fx_strip.rs`, D2). No export step.                                                                                                                                                                                                              |
| `draw_*.lua`                                                                                                            | **Dropped**                                                                  | They only made the placeholders.                                                                                                                                                                                                                                                         |

## Dependency and build guard

Measured on 2026-11-10 with `cargo tree`: the spike and Long Way trees differ by **5 crates** only (`bevy_enoki`, `wide`, `chacha20`, `getrandom` 0.3.4 and 0.4.3). Same Bevy features. The main build does not spiral as long as these rules hold:

| # | Rule |
|---|---|
| G1 | `bevy_enoki` is a plain dependency of **`crates/game-kit` only** (D3). `long-way` and `r-town` get it through the kit. **No change to `games/long-way/Cargo.toml`.** **No Cargo `[features]` in `game-kit`**, not even an optional `particles` feature: a feature would build the kit twice (`--workspace` against `-p`), the same trap as the spike's `file_watcher`. The cost is that R-Town compiles Enoki too. |
| G2 | No new Bevy feature. Games keep `bevy.workspace = true`. |
| G3 | Not carried over: the spike feature `hot-reload` (`bevy/file_watcher`), `bevy/dynamic_linking`, the Enoki `dev` feature. Effect files are reloaded by restarting. |
| G4 | `bevy_enoki` stays pinned `=0.8.0` in `[workspace.dependencies]`. The patch (D1) changes the source of that one crate only. |
| G5 | Gates, measured before (P0) and after every step that touches a `Cargo.toml`: see the table below. Stop and ask when one fails. |

| Gate | Pass when |
|---|---|
| `cargo tree -p long-way` | Only the 5 crates above are new |
| `cargo tree -p r-town` and `-p game-kit` | Only the same 5 crates are new. Nothing else. |
| `cargo tree -e features -i bevy -p long-way` | Identical to the baseline |
| `git diff Cargo.lock` | Only added packages. No existing crate changes version. |
| `./dev-scripts/rust.sh build long-way` | The "Compiling" lines name no Bevy crate |
| `./dev-scripts/rust.sh lint`, run twice | The second run compiles nothing |
| `./dev-scripts/rust.sh size` | Report the change in `target/`. Over 8 GB: say so (`adr-e8`). |
| Time to rebuild after touching one Long Way file | Report before and after. Not worse by more than noise. |

`target/` was 11 GB at the time of writing (over the 8 GB limit, accepted for now). Ask before `cargo clean` (about 10 minutes).

## Decisions

| # | Question | Status |
|---|---|---|
| D1 | Additive blending (glow) needs a patched Enoki (about 10 lines). | **Decided:** keep a vendored copy at `vendor/bevy_enoki` with the root `[patch.crates-io]`. Fallback: drop glow. |
| D2 | Enoki cannot read `.aseprite`. Sheets must be one row of equal cells. | **Decided (2026-11-11):** the `Aseprite` asset cannot feed Enoki as it is (a packed 2D atlas of full-canvas frames). **Build the strips at run time** in the kit: `fx_strip.rs` cuts a tag into a cropped strip when the art loads. No export script, no generated PNG, no start-up check, and a fresh clone works. |
| D3 | Where does the Enoki code live? | **Decided: `game-kit`**, as `fx.rs`. See G1. |
| D4 | The wall blocks ghosts, the player and projectiles. | **Decided:** a body inside the wall rectangle when it appears walks out. No push, no shifting. |
| D5 | How do we trigger a weapon in a test? | **Replaced.** The drawer exists (`WeaponChosen`, `adr-lw31`). Scripted runs click it with `ui_click:<x>:<y>` (`dev-feedback.md`). **No env variable trigger.** For an all-weapons stress run, a script casts every weapon in turn. |
| D6 | Whirlpool variant | **Decided (user, 2026-11-11): variant B** (attractor, spiral outward, with push). |
| D7 | Enemy hit points | **Decided: enemies need HP.** Design in P4. |
| D8 | Damage model | **Decided:** every weapon has a tunable **damage per second** (`dps`). A weapon that hits once (the fireball) has a tunable **impact damage** instead. |
| D9 | Pushback model | **Decided:** every weapon has a tunable pushback, one of: `None`, `Pixels(n)` (push back by a set distance, then stop), `ToEdge` (push all the way to the weapon's final radius or distance). The fireball has `None` and ends on its first hit. Pushback stops at features and walls (the collision world). |
| D10 | How a ring interacts with features (trees, rocks, markers, scenery) | **Open. Ask the user in step W1.** See the questions there. |
| D11 | Aim from a drawer button (the cursor is on the button) | **Decided: no cursor aim. Each weapon has its own aim rule**, taken from the player (facing, position) or the world (nearest enemy). See the table below. |
| D12 | Energy cost and cooldown per cast (`Inventory`, `adr-lw27`) | **Decided: out of scope.** A separate plan will build them later. Weapons in this plan cost nothing and have no cooldown. |
| D13 | What a ghost does when its HP reaches 0 | **Decided: it dies.** The ghost is removed (despawned) when `EnemyDied` fires. No drop. A death animation or effect is not part of this plan unless the user asks. |

### Aim rules (D11)

`CastWeapon { weapon, origin, facing }` is built from the player's state at the moment of the click: the player's feet and the direction the player last moved in. The aim is a rule per weapon, not a cursor point. The fireball's nearest-enemy search is done by the fireball itself (W6).

| Weapon | Anchor | Aim |
|---|---|---|
| W1 fire ring, W2 ice ring | The player's position at cast time. **Does not follow the player.** | None (a full circle). |
| W3 fire wall, W4 ice wall | In front of the player. | The player's facing direction sets the orientation (horizontal or vertical) and the travel direction. |
| W5 whirlpool | The player. **Follows the player** while it lasts. | None (surrounds the player). |
| W6 fireball | The player's muzzle point. | **Auto-target the nearest visible enemy.** |

Open details, asked in the weapon step: what "facing" gives when the player faces a diagonal (W3), what "visible" means and what the fireball does with no enemy visible (W6).

## Part 1: Migrate

### P0. Baseline (before any change)

- Run every gate in the table above on the current tree. Write the numbers into a "Baseline" table in this file.
- Record how long a rebuild takes after touching `games/long-way/src/main.rs`.
- Note the Cargo.lock state (`git status` must be clean; `weapon_icons.aseprite` is modified now, so ask about it first).

**Baseline (2026-11-11, before P2; the Cargo tree is the P2 reference):**

| Gate | Value |
|---|---|
| Unique crates, `cargo tree -p long-way` / `r-town` / `game-kit`, native | 492 / 492 / 490 lines |
| Bevy features (`cargo tree -e features -i bevy -p long-way`) | 126 lines, saved in the scratchpad |
| `Cargo.lock` | clean |
| Rebuild after touching `games/long-way/src/main.rs` | 5.7 s, only `long-way` compiled |
| `lint`, second run | compiles nothing |
| `target/` | 5.3 GB (the 11 GB in the text above is out of date) |

### P1. Kit geometry and the wall blocker (no Enoki)

- `crates/game-kit/src/area.rs`: port the five functions on Bevy `Rect`. Tests moved from the spike (`ease_out`, ring radius, band overlap including the "consecutive frames overlap, so no body is skipped" test, push). Export in `lib.rs`.
- `crates/game-kit/src/collision.rs`: a temporary blocker. Needs: add at run time, remove on expiry, no stale edges, cost of one outline rebuild per change (measure), and `nav.rs` picking it up. Unit tests in the kit (a shot, a body and a path against a wall that appears and expires). Built here, used in W3.
- `layers.rs`: change only if a weapon step shows a need.
- Docs: `dev-docs/extensions/area-effects.md` (new), updates to `collision.md` and `projectiles.md` (no range limit: the game despawns). Add to `dev-docs/extensions/README.md`.
- Check: `./dev-scripts/rust.sh test game-kit`, `lint`, `build r-town`. No `Cargo.toml` change, so the tree must not move.

### P2. Enoki into `game-kit` (no weapons)

- Move `games/particle-spike/vendor/bevy_enoki` to `vendor/bevy_enoki` (D1) and change the root `[patch.crates-io]` path. Check whether the root `exclude` line is still needed. The spike keeps compiling against the moved copy.
- `crates/game-kit/Cargo.toml`: add `bevy_enoki.workspace = true` (plain dependency, no feature, G1). **Nothing else changes. `games/long-way/Cargo.toml` does not change.**
- `crates/game-kit/src/fx.rs` (add `pub mod fx;` to `lib.rs`): `FxPlugin` (adds `EnokiPlugin` and the two additive materials, opt-in), `EffectLibrary` (keeps effect files loaded), one-shot spawn helper, "emitter follows its owner" system, `Glow` helpers. Port from `glow.rs`, `fireball.rs` (`spawn_emitter`, `spawn_one_shot`, `emitters_follow`) and the library code. Tests in the kit where there is logic.
- Add `FxPlugin` to Long Way `main.rs` only. R-Town does not add it.
- Long Way must look and run the same: run a scripted shot before and after (`script long-way "wait:0.5;screenshot:start;quit" --debug`) and compare.
- Run all gates. Read the "Compiling" lines for `build long-way` and `build r-town`: Enoki's 5 crates appear, no Bevy crate. Stop and report if any gate fails.
- Ask before committing (commit 1: kit geometry and blocker, commit 2: Enoki in the kit).

**P2 result (2026-11-11):** tree diff is the 5 crates only (`bevy_enoki`, `wide`, `chacha20`, `getrandom` 0.4.3 and, on other targets, 0.3.4) for `long-way`, `r-town` and `game-kit`. Bevy features identical. `Cargo.lock` gains one line (`game-kit` depends on `bevy_enoki`). `build long-way` compiled no Bevy crate. Rebuild after touching `main.rs`: 4.8 s (baseline 5.7 s). Second `lint` compiles nothing. `target/` 7.9 GB after `prune`. A screenshot before and after matched except the line order of the debug info box.


### P3. Particle sprites from Aseprite (D2) (done)

- Result of the D2 check: not directly (see D2). `crates/game-kit/src/fx_strip.rs` builds the strips at run time (`strip_from_atlas`, `FxLooks`). Unit tests on plain pixel buffers. Checked on the real art: `blob`, `fire_ring` and `ice_ring1` to `ice_ring4` of `fireball_placeholder.aseprite` give the same cell sizes as the spike's exported PNGs (4×32, 6×64, 4×59, 4×60, 4×59, 4×60). Pixel by pixel equality was not compared.
- Art rule from the spike: after a script makes tags, run `./dev-scripts/aseprite.sh check <file>` and read the frame ranges. Tags must not overlap.
- `fireball_placeholder.aseprite` is copied as `fireball_and_rings.aseprite` to `games/long-way/assets/art/weapons/` (it holds the ring, blob and fireball head tags). `art-pipeline.md` and `art-tags.md` are updated. The other art and the effect files are copied

## Part 2: Weapons

### Shape of the code

```
games/long-way/src/weapons/
├── mod.rs        WeaponPlugin: the messages, the sub-plugins, the WeaponChosen to CastWeapon link
├── values.rs     every number of every weapon in one place (WeaponValues per WeaponKind)
├── damage.rs     applies WeaponHit to enemy HP, pushback application
├── ring.rs       RingPlugin (fire and ice)
├── wall.rs       WallPlugin (fire and ice)
├── whirlpool.rs  WhirlpoolPlugin (variant B)
└── fireball.rs   FireballPlugin
games/long-way/src/enemy_hp.rs   EnemyHp, DamageEnemy, EnemyDied, the hurt flash (P4)
```

Each weapon file is added in its own step. Do not create empty files for later weapons.

- **Weapon kinds:** reuse `WeaponKind` from `weapon_menu.rs` (fire ring, ice ring, fire wall, ice wall, whirlpool, fireball). Do not make a second enum.
- **Messages:**
  - `CastWeapon { weapon, origin, facing }`: asks for a weapon. Built from `WeaponChosen` for the player.
  - `WeaponHit { target, weapon, amount, push }`: a damage tick, an impact, or a push on one target. `amount` is already `dps * tick` or the impact damage. `damage.rs` turns it into HP loss and movement.
- **Gameplay and visuals are separate systems.** Gameplay owns the state (ring radius, wall rectangle and timer, whirlpool band) and sends `WeaponHit`. Visuals read that state and spawn emitters and sprites. The gameplay half runs and is tested with no particles.
- **Values in one place.** `values.rs` holds, per weapon: `damage` (`PerSecond(f32)` or `Impact(f32)`), `pushback` (`None`, `Pixels(f32)`, `ToEdge`), `tick` (seconds between hits on one target, 0.5 s in the spike), size, duration, speed, `glow: bool`. Each number names its source (spike default or tuned).
- **Pushback rules (D9):** push is a movement of the target's feet through `collision::move_feet`, so trees, rocks and walls stop it. `Pixels(n)`: the target is moved back by at most `n` pixels in total per cast, then left alone. `ToEdge`: the target is carried outward with the front until the final radius or distance. A target that is stopped by a feature stays there and keeps taking damage.
- Hit targets use the `HitBox` of a ghost.
- Draw order: `DepthSort` on every emitter and sprite. No bare z numbers (`adr-x9`).
- Weapons do not touch `Inventory`, the HUD or the player's movement. Energy cost and cooldown come in a later plan (D12).

### P4. Base: enemy HP, damage, pushback, drawer link (done, 2026-11-11)

**Result:** `enemy_hp.rs` (`EnemyHp` f32, `DamageEnemy`, `EnemyDied`, flash `#993D41` for 0.12 s on the child sprites; the ghost has `Hurt` frames since 2026-11-11 and plays them instead, via `HurtFrames` and `EnemyHurt`), `weapons/` (`mod.rs`, `values.rs` with placeholder rows for all six weapons, `damage.rs` with `Contact` (damage ticks and the pixel budget) and the push through `collision::resolve` inside the map), the ghost has 100 HP and dies through `Die` when killed, `log_choice` is replaced by the `CastWeapon` log. Not seen on screen: the flash and a weapon-kill death in the running game (no weapon exists yet, they are unit tested); the scripted run shows one `weapons: cast ...` line per drawer click.

D11 to D13 are decided. Then:

- **Enemy HP (D7).** `games/long-way/src/enemy_hp.rs`: `EnemyHp { current: f32, max: f32 }` (a float, because damage is per second; the player's `Hp` in `hp.rs` is a whole-number heart count and stays as it is). Added to every ghost in `spawn_ghosts`. A `max` per enemy kind in the game's values, so later enemies differ. Message `EnemyDied(Entity)` sent once when `current` reaches 0. Death handling per D13. Unit tests for the HP rules in plain Rust (loss, no go below 0, died once only).
- **Damage and pushback application** in `weapons/damage.rs`: read `WeaponHit`, lower `EnemyHp`, apply the push through `collision::move_feet`. Test: `dps` over a set time gives the expected HP loss (tick rounding included); `Pixels(n)` never pushes more than `n` in total; a push is stopped by a blocked area.
- **`values.rs`** with `WeaponValues` and its tests (every `WeaponKind` has a value set).
- **Drawer link:** a system turns `WeaponChosen` into `CastWeapon` for the player (origin and aim from the aim rules table, D11; the player's facing is read from the player components). The weapon systems read `CastWeapon`. Replace the `log_choice` system in `weapon_menu.rs` only if it is then redundant.
- **A debug line per hit** (target, weapon, amount, HP left) at `info!` level, so tuning can read it from the script log.
- **Scripted check:** `ui_click` on the red icon and then on a weapon button sends one `CastWeapon` (log line). No weapon exists yet, so nothing else happens.
- Docs: `games/long-way/docs/weapons.md` (new: values table, files, the tuning loop), `adr.md` entries for enemy HP and the damage and pushback model.
- Check: `lint`, `test long-way`, `test game-kit`. Ask before committing.

### Steps W1 to W6: one weapon at a time

Every weapon step follows the same checklist. Do not start the next weapon before the user has accepted the current one.

1. **Design questions** to the user (listed per weapon below).
2. **Assets:** copy this weapon's art and effect files from the spike (only what it uses), update `art-pipeline.md` and `art-tags.md`, request the particle looks from `FxLooks` (P3).
3. **Gameplay:** the weapon's state machine, hit test, `WeaponHit` messages with the values from `values.rs`. Unit tests in plain Rust for the mechanic (the spike tests move or are rewritten here).
4. **Visuals:** the emitters and sprites, reading the gameplay state. `DepthSort` on all of them.
5. **Wire into the drawer:** the button for this weapon (already in `weapon_menu.rs`) now produces a working cast through `CastWeapon`. Check the button after a cast (no double cast from one click) and that closing the drawer does not cancel the weapon.
6. **Scripted test:** `rust.sh script long-way "...;ui_click:...;ui_click:...;wait:...;screenshot:...;quit" --debug` with ghosts placed at set distances (a ghost inside, on the edge and outside the effect). Read the log for the per-hit lines. Say what was not seen on screen (motion, timing, feel).
7. **Tune with the user:** the user plays it. The values to tune are `dps`, `pushback`, size, duration, and `glow`. Change `values.rs`, rerun, repeat until the user accepts. Record the accepted values in `weapons.md`. A tuning note per weapon says which numbers moved and why.
8. **Performance check:** frame time and particle count with the weapon cast every 0.7 s (spike reference: about 3.4 ms native at about 3400 particles).
9. **Gates and docs:** `lint`, `test` (changed crates). Update `weapons.md`, `source-layout.md`. Delete files in `target/screenshots/`.
10. **Ask before committing** (one commit per weapon).

#### W1. Fire ring (built 2026-11-11, values not tuned yet)

- **Design questions:**
  - Pushback: `Pixels(n)`. How many pixels (start from the spike feel, tune)?
  - Damage: `dps` start value? Does a ghost at the burn band at the maximum radius keep burning after the front stops (the spike has 40 burners at the maximum radius and repeat ticks while in the burn band)?
  - **D10, features.** Ask these before any code:
    1. Does the ring pass over trees, rocks, markers and scenery, or is it stopped or cut by them (a body behind a tree is shielded)?
    2. Does the fire react to a feature (burn marks, a flame on a tree, a tree is removed)? Cosmetic only, or does it change the feature (the collision world must then change too)?
    3. Do features stop the pushback of a ghost (the base rule says yes), and does a ghost pinned against a feature take damage as normal?
    4. Does the ring hit the player's own features such as a lantern or an exit marker?
  - Recommendation: the ring passes over features (no shielding, simple and the band test stays a plain circle test), a pushed ghost stops at features, and feature reactions are cosmetic and a later step. Shielding needs a line-of-sight test per target against the collision outlines and costs more.
- Port with the `area` functions: eased growth (`ring_radius`), 40 moving front emitters, 40 burners at the maximum radius, band hit test.
- Tests: the band reaches back far enough that no body is skipped between frames, one hit when the front passes, repeat ticks while in the burn band, pushback totals `n` pixels at most and stops at a blocked area, the feature rule chosen in D10.
- Scripted run: a ghost at three distances. Check a ghost next to a tree (D10).
- This step also proves the base (P4): fix any gap in `damage.rs` here.
- Record D10 as an ADR in `games/long-way/docs/adr.md`.

#### W2. Ice ring

- **Design questions:**
  - Pushback: `ToEdge`. Does the ghost move with the front (carried) or jump to the final radius? Does a ghost stay frozen or slowed after (a status is a new mechanic: ask, do not invent)?
  - Damage: `dps` start value, and how it compares to the fire ring.
  - Do the D10 answers for the fire ring hold for the ice ring (an ice reaction on features)?
- Reuse the ring code from W1: one `RingPlugin`, two value sets and two sprite sheets and effect sets. Only add what differs.
- Tests: `ToEdge` carries a body to the final radius, a body behind a tree stops at the tree, damage per second matches `dps`.
- Scripted run: ghosts at three distances, and one with a tree between the player and the ghost.

#### W3. Fire wall

- Uses the temporary blocker from P1.
- **Design questions:**
  - Facing (D11): the wall's orientation and travel direction come from the player's facing. A diagonal facing: snap to the nearest axis, or use the dominant axis? Ask.
  - **Travel:** the rule says the wall has a travel direction. Does the wall move (how fast, how far), or does it only face that way? The spike wall is static, so this is a new mechanic: ask. A moving blocker needs the P1 blocker to move or be rebuilt each frame; measure the cost.
  - Damage: `dps` for a ghost standing in or touching the wall. Pushback: `None`, `Pixels(n)`, or the wall only blocks? Does the fire wall block the player too (D4 said it blocks ghosts, the player and projectiles)?
  - Lifetime (3.3 s in the spike) and length.
  - What if the wall would stand on a feature or outside the map?
- Port: rectangle placed by the facing rule, the blocker added and removed with the wall, fire as Enoki flames.
- Tests: the lifetime, the placement and orientation from the facing (all four directions), removal of the blocker on expiry, `dps` through the rectangle (Long Way side; the blocker is tested in the kit, P1).
- Scripted run: the ghost stops at the wall, burns, then walks on after it expires. Check a horizontal and a vertical wall and the ghost path (`nav.rs`).

#### W4. Ice wall

- **Design questions:** damage, pushback, lifetime, and whether the ice wall differs in mechanic from the fire wall (it blocks without damage? it is only a barrier?). Ask. Do not invent.
- Reuse `WallPlugin` from W3. Add the ice sprites: animated, one tag per cube height, played once, fade by alpha.
- Tests: the values differ as agreed and the shared rules hold.
- Scripted run: same as W3 with the ice art.

#### W5. Whirlpool

- Variant B only (D6): 10 emitters with attractors set per emitter, spiral outward, band hit test.
- **Design questions:**
  - Pushback: the spike pushes away from the player (blocked by the collision world). Is that `Pixels(n)` or `ToEdge`? Does it also deal `dps`, or only push?
  - Does it affect the player's movement (the plan says no unless you decide yes)?
  - Duration and size. The whirlpool is centred on the player and moves with it (D11): check that the band, the emitters and the push all follow, and what happens when the player walks into a tree.
- Keep the unit test that integrates the Enoki attractor formula (droplets spiral outward across the band).
- Tests: the push is blocked by a tree, the damage per second if any, the duration.
- Scripted run: a ghost walking at the player.

#### W6. Fireball

- Last, because it adds a moving shot, impact and no pushback.
- **Design questions:**
  - Impact damage (`Impact(f32)`, one hit), or also an area at the explosion (a radius and a damage)? The user said it explodes and ends on hitting an enemy: confirm the explosion hurts only the enemy hit or nearby enemies too.
  - Speed, range (1100 px in the spike), what it does when it hits a tree or rock (explode, as in the spike).
  - Target rule (D11): nearest visible enemy. What counts as visible (on screen, or in line of sight past trees and rocks, or both)? What happens when none is visible (no cast, or a shot straight ahead)? How is a tie or a moving target handled (fire at the position at cast time)?
- `Projectile` from the kit, head sprite, four trail emitters that follow it and stop with it, impact bursts on `ProjectileHit`, muzzle burst, range limit system that despawns the shot. The ghost's `HitBox` is the target (add `ProjectileTarget` to ghosts). Pushback is `None`. The shot ends on its first enemy hit and sends one `WeaponHit` with the impact damage.
- Tests: the range limit, the muzzle point, one hit and then the shot is gone, no push.
- Scripted run: flight, hit on a tree, hit on a ghost, miss to the range limit.

### P5. Verify all (after W6)

- `./dev-scripts/rust.sh lint`, `test` (all crates), `build r-town`.
- All six weapons through the drawer in one scripted run, then a stress run (each cast again every 0.7 s). Compare the frame time with the spike figures. Report the final `target/` size. Run all gates once more.
- Screenshots for each weapon with `--debug`. State what was not seen on screen: motion, timing.
- Delete files in `target/screenshots/`.

### P6. Migrate and delete

- ADRs (next free number in each file, record why, not how):
  - `ENGINE.md`: Enoki accepted, the patch decision (D1), G1 to G4 as rules (the spike plan step 10 may already have done part of it: check first).
  - `EXTENSIONS-GAMEPLAY.md`: area geometry and the temporary blocker in the kit; Enoki in the kit (D3: why, the 5-crate cost to R-Town, no Cargo features, split to `game-fx` if the cost grows).
  - `games/long-way/docs/adr.md`: enemy HP, the damage and pushback model (D7 to D9), the ring and feature rule (D10), weapons built behind `CastWeapon` and `WeaponHit`, gameplay and visuals separate.
- Docs: `games/long-way/docs/weapons.md` (final: accepted values per weapon, files, tuning loop), `source-layout.md` (the `weapons/` folder, `enemy_hp.rs`, the extension table), `dev-docs/extensions/area-effects.md` (final), `dev-docs/README.md` map, `docs/README.md` . Check `CLAUDE.md` for lines to change (commands table, game table).
- Update `rm9` in `dev-docs/ROADMAP.md`: weapons and enemy HP are done and live; what stays open is the death screen, the restart flow, enemy attacks, sound and the impact sound. Move finished parts to `ROADMAP-completed.md`.
- Delete `games/particle-spike/`, the root `[patch.crates-io]` and `exclude` lines if no longer needed (the patch stays if the kit still uses the vendored Enoki, D1), and `PLAN-particle-spike.md`. Then delete this plan. Git history keeps both.
- Run `./dev-scripts/rust.sh prune`. Ask before `cargo clean`.
- Ask before each commit. Do not push.

## Risks

| Risk | Handling |
|---|---|
| The patched Enoki breaks on an upgrade | The patch is 10 lines. The ADR lists them. Upstream issue. D1 fallback removes the risk. |
| The wall blocker costs too much (outline and nav rebuild per change) | Measured in P1. A wall appears once per cast, not every frame. |
| Enoki in the kit slows R-Town builds | Accepted (D3). Measured in P2. Split `game-fx` if it grows. |
| A particle look is not ready in the first frames, or its tag is missing | `FxLooks::get` gives `None` until ready (the caller falls back to squares); a missing tag logs an error (D2). |
| Spike values are mistaken for design | `values.rs` names the source of each number and each weapon step ends with the user's tuning. |
| Pushback and damage ticks double-apply when two weapons overlap | Per-target timers are per weapon cast. Tested in P4. |
| Pushing a ghost through a feature or out of the map | Push uses `move_feet`, so blocked areas stop it. Tested in P4 and W1. |
| Ring and feature interaction grows into a large feature system | D10 is decided in W1 before code. Feature reactions stay cosmetic unless the user says otherwise. |
| Effect files cannot reload without the watcher | Accepted. Restart the game to see a change. |
