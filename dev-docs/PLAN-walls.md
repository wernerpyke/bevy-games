# PLAN: fire wall and ice wall for Long Way, then check all weapons and remove the spike

Split from `PLAN-weapons.md` on 2026-11-13, when the fire ring, ice ring, tornado and fireball (W1 to W4) were accepted and that plan was closed (git history keeps it). The spike (`games/particle-spike/`, `PLAN-particle-spike.md`, `RESEARCH-particles.md`) is the porting source for the walls and stays until the last step.

## Status (2026-11-13): resume here

Not started. Next: W5 (fire wall), design questions first.

## What is already in place

- **Weapons:** the drawer and the active weapon buttons cast through `CastWeapon`, and weapons hit through `WeaponHit` (`games/long-way/docs/weapons.md`, "How a weapon works", `adr-lw32`, `adr-lw34`). Shared code: `weapons/values.rs` (every number), `weapons/damage.rs` (`Contact`, `pushed_feet`). `WeaponKind::FireWall` and `IceWall` exist with placeholder rows in `values.rs` (40 and 20 per second, no pushback) and are not built (`WeaponKind::is_built`).
- **How to add a weapon:** the checklist in `weapons.md`, "Adding a weapon". Every step ends with the user's play test, and a commit only when the user says so.
- **Rules every weapon keeps:** dead enemies are skipped, a long frame deals one tick (`weapons.md`, "Values"), the balance rules in `balance.md`.
- **Kit:** the temporary blocked area (`CollisionWorld::add_temporary`, `remove_temporary`, `adr-x48`, `dev-docs/extensions/collision.md`), built for the walls and not used yet. Particles: `game_kit::fx` and `FxLooks` (`adr-x47`, `dev-docs/extensions/particles.md`). The build gates after any `Cargo.toml` change are in `particles.md`, "Build guard".
- **Spike source for the walls:** `games/particle-spike/src/wall.rs`. The fire wall's flames are the effect `wall_shard.particle.ron`. The ice wall is cubes of three heights, tags `cube_s`, `cube_m`, `cube_t` of `art/ice_wall_placeholder.aseprite` (copy it as `ice_wall.aseprite` without the suffix, the user's rule of 2026-11-11).

## Out of scope

- **WASM** (carried over from `PLAN-weapons.md`, the user's decision of 2026-11-11): no `rust.sh web`, no web run or size check in this plan. A later plan does the WASM check. This overrides `CLAUDE.md` working rule 8 for this plan.
- Energy cost, cooldown and a disabled drawer button at 0 energy (a separate plan), sound, the game over screen and restart flow (`rm9`), R-Town.

## Decisions carried over

| Topic | Decision |
|---|---|
| A wall blocks (old D4) | A wall blocks ghosts, the player and projectiles. A body inside the wall rectangle when it appears walks out: no push, no shifting. To confirm with the user per wall (the design questions below ask whether each wall blocks the player). |
| Aim (old D11) | The player's facing sets the wall's orientation (horizontal or vertical) and its travel direction. Open: a diagonal facing (below). |
| Pushback model (old D9) | `None`, `Pixels(n)`, `ToEdge` or the tornado's `Outward`. A push stops at features and walls. |

### W5. Fire wall

- Uses the temporary blocker (`add_temporary` when it appears, `remove_temporary` when it ends; the game owns the timer).
- **Design questions** (ask the user for the intent first, as for W1 to W4, and quote it in `weapons.md`):
  - Facing: the wall's orientation and travel direction come from the player's facing. A diagonal facing: snap to the nearest axis, or use the dominant axis?
  - Travel: does the wall move (how fast, how far), or does it only face that way? The spike wall is static, so a moving wall is a new mechanic. A moving blocker needs the blocker rebuilt each frame: measure the cost.
  - Damage (`dps`) for a ghost standing in or touching the wall. Pushback: `None`, `Pixels(n)`, or does it only block? Does the fire wall block the player too?
  - Lifetime (3.3 s in the spike) and length.
  - What if the wall would stand on a feature or outside the map?
  - Balance rules for the walls (next free ID in `balance.md`).
- Port: the rectangle placed by the facing rule, the blocker added and removed with the wall, fire as Enoki flames.
- Tests: the lifetime, placement and orientation from the facing (all four directions), the blocker removed when the wall ends, damage through the rectangle, a dead enemy skipped. The blocker itself is tested in the kit.
- Scripted run: a ghost stops at the wall, burns, then walks on after it ends. A horizontal and a vertical wall, and the ghost's path around it (`nav.rs`).

### W6. Ice wall

- **Design questions:** the intent; damage, pushback, lifetime; does the ice wall differ in mechanic from the fire wall (it blocks without damage? it is only a barrier?). Ask. Do not invent.
- Reuse the fire wall's code. Add the ice sprites: animated, one tag per cube height, played once, fade by alpha.
- Tests: the values differ as agreed and the shared rules hold.
- Scripted run: as W5 with the ice art.

### V. Check all six weapons (after W6)

- `./dev-scripts/rust.sh lint`, `test` (all crates), `build r-town`.
- All six weapons through the drawer in one scripted run, then a stress run (each cast again every 0.7 s). Frame time and particle count against the spike figures (about 3.4 ms native at about 3400 particles). Not measured yet for the ice ring, the tornado and the fireball; the fire ring was 2.7 to 3.1 ms (`weapons.md`).
- Not seen on screen in W1 to W4, check where a scripted run can show it: a cast near the map edge; an ice ring pinning a ghost against a tree, and a stunned ghost's paused animation; a ghost killed by holding the tornado against it, and a ghost pinned against a tree by the tornado; a fireball blast kill, a fireball cast with no enemy, and with no enemy on screen.
- Screenshots for each weapon with `--debug`. Say what was not seen on screen (motion, timing). Delete `target/screenshots/` files after.
- Report the final `target/` size.

### Final step: migrate and delete

- ADRs for the wall decisions (`games/long-way/docs/adr.md`, next free number). `weapons.md`: the wall sections, final values and tuning notes. `balance.md`: the wall rules. `source-layout.md`: the wall file. `docs/README.md`: the weapons line under Controls if it changes.
- Update `rm9` in `dev-docs/ROADMAP.md` and the weapons entry in `ROADMAP-completed.md`.
- Delete `games/particle-spike/`, the root `[patch.crates-io]` and `exclude` lines only if no longer needed (the patch stays while the kit uses the vendored Enoki, `adr-e9`), `PLAN-particle-spike.md`, and `RESEARCH-particles.md` if nothing lasting is left in it (move its verdict to `adr-e9` or `particles.md` first). Then delete this plan. Git history keeps them.
- Run `./dev-scripts/rust.sh prune`. Ask before `cargo clean`.
- Ask before each commit.

## Risks

| Risk | Handling |
|---|---|
| The wall blocker costs too much (outline and nav rebuild per change) | One rebuild when a wall appears and one when it ends. Measure in W5; a moving wall rebuilds every frame, so measure that before choosing it. |
| A wall appears on the player or a ghost | The kit lets a body that starts inside a blocked area walk out (`collision.md`). Test it in W5. |
| Effect files do not reload while the game runs | Accepted. Restart the game to see a change. |
