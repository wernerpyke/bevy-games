# Assessment: game-kit unit tests

Date: 2026-11-11. Scope: every `#[cfg(test)]` module in `crates/game-kit/src/`. Nothing was changed in the code. The rules learned from it are in `testing.md`.

## Answers in short

| Question | Answer |
|---|---|
| Is the coverage good? | **Good for the rules, thin for the systems.** The pure rules (outline, collision, nav, projectile, map parser, map links) have good tests. Most Bevy systems in the dev tools have none, and some gameplay systems are missing branches. 8 tests pass but do not check what their names say. Fix these first. |
| Are there overlapping or unnecessary tests? | **A few.** About 25 of 364 tests (7%) repeat another test, and about 20 more check trivial code. This is low. Remove them to keep the tests easy to maintain, not for speed. |
| Is it worth optimising the tests for build or test time? | **No.** All tests run in 0.27 s. On a full `test`, the kit test binary builds at the same time as the kit library, so it does not make the wait longer. Deleting every duplicate saves less than 1 s. The build time comes from code generation of the kit itself (82%), not from the tests. See "Build and test time" for the levers that do help. |

## How this was measured

- The tests were run and timed. Compile times were taken from `cargo test --timings` after a one-line edit to `collision.rs`, which was then removed.
- Three review passes read all the test modules. I checked their main claims in the source: `projectile.rs:524`, `map_transition.rs:1776`, `outline.rs:576`, `app_state.rs:106-143`, `layers.rs:55`, and the two copies of `rects_overlap`.
- No coverage tool was used. `cargo-llvm-cov` is not installed, and it would build another copy of Bevy with instrumentation (about 2 GB, see `adr-e8`). The gaps below come from reading the code, so they are not a line count.

## Numbers

- 364 tests, 1 ignored (`collision.rs:1119`, a manual timing probe). There are no doc tests.
- About 6,900 lines of test code in 29,100 lines (24%).
- Files with the most tests:

| File | Tests | Test lines |
|---|---|---|
| `collision.rs` | 44 | 723 |
| `map.rs` | 35 | 522 |
| `map_transition.rs` | 34 | 1,080 |
| `outline.rs` | 32 | 542 |
| `map_areas.rs` | 20 | 292 |
| `projectile.rs` | 19 | 332 |
| `nav.rs` | 16 | 251 |
| `map_links.rs` | 15 | 238 |

- `hud.rs`, `layers.rs` and `lib.rs` have no tests. None is needed: `layers.rs` checks its order with a compile-time `const _: () = { assert!(...) }` (`layers.rs:55-71`), and `ScreenPlugin` tests already run `spawn_hud`.

## 1. Coverage

### Tests that pass but do not check what their names say

Fix these first. Each gives false confidence.

| Test | Problem |
|---|---|
| `projectile.rs:524` `a_hit_wins_over_leaving_the_map` | The shot ends at x 199.67. That is still inside the map (0 to 200), so the "blocked wins over leaving the map" order in `step` (`:216-221`) never runs. The test still passes if the two checks change order. Use a step that ends past x 200. |
| `map_transition.rs:1776` `requests_during_a_transition_are_ignored` | `accept_request` reads both requests in one call. Without the idle guard (`:629`), the second request would only overwrite the first with the same phase, so the load count is still 2. Send the second request in a later frame, during a fade that is longer than zero. |
| `map_transition.rs:1725-1729` | The comment says the feet move back into the trigger. The code never moves them, and its `armed` check repeats `:1752`. |
| `map_transition.rs:2090` | Asserts `!locks_input()` while idle, where it is always false. No test asserts `true` during `FadeOut`, `Switch` or `FadeIn`. |
| `outline.rs:619` "a filler at each inner vertex" | Asserts only the cluster and member counts. Spans that touch give the same counts with no filler. |
| `strategies.rs:357` | `if let Some(point)` passes even if `pick_target` always returns `None`. |
| `strategies.rs:636` "patrols" | Asserts only `!alerted`. It never checks that a goal was set. |
| `pointer.rs:171` "without a window" | Returns early at `camera.single()`, not because there is no window. |

### A possible defect found while reading

- **`app_state.rs:138` `skip_title_once`.** Its `true` branch never runs in a test. The test named for it (`:338`) uses the skip up in `finish_loading`. In the code: a game that starts in `Playing` with the skip switch on keeps `skip_left = true` (`:106`). It then skips its first title screen later, for example after game over. Impact is low today. Decide what was intended, then add a test.
- **`area.rs` `ring_radius` with `grow_secs == 0`** gives NaN at age 0. No test covers it.
- **`projectile.rs` `target_hit`** counts a touch on an edge as a hit, but `hitbox::overlaps` does not. No test grazes an edge, so it is not known which rule is wanted.

### Gaps in gameplay code (priority A)

Games depend on this behaviour, and a mistake here does not always show on screen.

- **`map_transition.rs`**
  - `plan_switch` scales the offset by `to.cell / from.cell` (`:701`). Every fixture uses cell 64, so the scaling is never tested.
  - The `Edge::Bottom` and `Edge::Left` arms of `walkable` (`:213`, `:215`) swap a and b, and no test reaches them.
  - The errors of `begin_seamless` ("not active", "no entry") and of `plan_switch` ("has no entry") are untested. Only the overlap clash (`:2209`) is tested.
  - The error paths of `MapRegistry::load`, `reload` and `read` are untested, including the check that a failed reload leaves the maps unchanged.
  - The `detect_exit` filter by current map is not tested with two seamless maps.
- **`projectile.rs`:** a target behind a blocked area must be safe (`:138-140`). The `Blocked` and `MapEdge` causes are not tested through the system.
- **`collision.rs`**
  - `interior_areas()` is untested. It maps indices into `built`, which also holds temporary areas.
  - The `to_concave` refusal in `slide_to_nearest_edge` is untested (`:844` covers only `from_concave`).
- **`outline.rs`:** the random-layout test (`:876`) runs all four `corner_fillers` branches, but only checks the interior flag. A direct test with the upper rectangle first would check the filler itself.
- **`movement.rs`:**
  - the `searches_per_frame` budget
  - a new search after a stall
  - a goal that becomes reachable again
- **`strategies.rs`:** `Pursue` with an explicit target, and the `NO_TARGET_WAIT` branch.
- **`spawner.rs`:** the `max` count filters by spawner (`:116`), but no test has two spawners.
- **`map.rs`, `map_areas.rs`**
  - `Params::f32` rejecting `inf` and `NaN`
  - 6 of the 7 "given twice" errors
  - an empty `tiles:` line
  - the `facing` values down, left and right
  - a vertical strip in a top-left file (the y swap at `map_areas.rs:240`)

### Gaps in dev tools and visual systems (priority B)

These run in every scripted or debug session, so a fault shows quickly. Add tests only when one of them breaks.

- `script.rs`: only `parse` is tested. The `run_script` runner has no tests (Hold, Wait, Teleport, the three UiClick phases, the Goto timeout, Quit). It can be tested with `TimeUpdateStrategy::ManualDuration`.
- `debug.rs`, `collision_debug.rs`, `movement_debug.rs`: the pure helpers are tested, but the systems are not.
- `fx.rs`, `fx_strip.rs`, `animation.rs`: the material branches, `build_looks`, `apply_random_start` and `apply_static_frame`.
- `camera.rs` follow smoothing, the `screen.rs` edge cases, and the success path of `pointer.rs` `send_clicks`.

## 2. Overlap and unnecessary tests

### Real duplicates

The second test cannot fail unless the first one fails too. Delete it or merge it.

| Remove or merge | Covered by |
|---|---|
| `outline.rs:576` | `outline.rs:583` (same setup, same asserts and more) |
| `outline.rs:808` | first assert of `outline.rs:838` |
| `outline.rs:651`, `:659`, `:644` | `outline.rs:467`, `:483`, `:676` |
| `area.rs:64` (`ease_out`) | `area.rs:85` reaches the same four branches |
| `depth.rs:80` | `depth.rs:101` |
| `groups.rs:220` (3 apart) | `groups.rs:143` (2 apart, the tighter limit) |
| `collision.rs:495` | `collision.rs:807`, same branch |
| `collision.rs:618`, `:508`, `:1064` | `collision.rs:667`, `:519`, `:1081` |
| `collision.rs:1032`, `:1050` | `collision.rs:1013`, the same check through projectile and nav. Keep one as a smoke test. |
| `projectile.rs:538`, `:445` | `collision.rs:955` with `projectile.rs:413`, and `projectile.rs:426` |
| `map_transition.rs:1331` | `map_transition.rs:1314` (`entry_feet` has no input for the exit side) |
| `map_transition.rs:1348` | `:1314`, `:1331`, `:1372`, `:1386` (the outer loop variable is used only in the message) |
| `map.rs:1051-1056`, `:1057-1062`, `:912-914`, `:1012` case `"."` | `map.rs:1234`, `:1121`, `:918`, `:1126` |
| `map_links.rs:241`, `:389-390` | `map_links.rs:303`, `:281` |
| `map_background.rs:142` (same input, same result) | `map_background.rs:248` (fixed expected values) |
| `app_state.rs:339-343` | `app_state.rs:317-320` |
| `camera.rs:313` | `camera.rs:207` |

### Duplicated code under test

- `rects_overlap` is copied: `debug.rs:220` is the same as `map_transition.rs:897`, and both are tested. Make `debug.rs` call the public one.

### Low-value tests

These check Rust, glam or Bevy behaviour, a struct literal, or a constant:

- `fx.rs:279`, `animation.rs:304`, `reload.rs:38`, `debug.rs:521`, `movement_debug.rs:97`
- `strategies.rs:347`, `camera.rs:216`, `hitbox.rs:33`, most of `projectile.rs:251`
- `depth.rs:87` (a pure function gives the same result twice)
- `depth.rs:94` (repeats the compile-time assert at `depth.rs:13`)
- `map_transition.rs:1244`, `map_areas.rs:535`
- `map.rs:1248`, `:1021`, `:1157`, the `"4"` case of `:1269`
- `map_links.rs:248`

### Other quality notes

- **Game values in kit tests.** This is against the spirit of `adr-w7` and of "the kit has no game types":
  - `outline.rs:378` uses `ACTOR_HALF_WIDTH`, "Long Way's player".
  - `area.rs:95` checks game tuning (0.767, 230, 0.12).
  - `groups.rs:196-238` uses R-Town map layouts.

  These tests break when a game changes its values. Use neutral values.
- **Brittle tests:**
  - `map_transition.rs:2105` depends on Bevy's 0.25 s maximum delta and on fixed update counts.
  - Several tests compare exact log lengths and hook strings (`map_transition.rs:1558`, `:1586`, `:2083`).
  - Some collision tests compare computed floats exactly (`collision.rs:782-803`).
- **Shared test helpers.** The same helpers are written many times:
  - `edges_of` 5 times, `map()` 3 times, `DT` 3 times
  - the movement and strategies App builders
  - 4 message collectors
  - 5 empty-grid map builders
  - 3 random sources

  One `#[cfg(test)] mod test_util` in the kit would remove about 200 lines. It makes the tests easier to maintain, but it does not make the build faster.
- **Flaky tests: none found.** No test uses real time or sleeps, and all randomness is seeded.

## 3. Build and test time

### Measurements

All builds were run with the dependencies already built.

| Step | Time |
|---|---|
| Run all kit tests (parallel / one thread) | 0.27 s / 0.44 s |
| Slowest tests: `strategies.rs`, about 5,100 frames in total | within the 0.27 s |
| Kit test binary, with no incremental cache | 19.7 s |
| Kit library after a one-line edit (2 runs) | 12.8 s, 12.8 s |
| Kit test binary after a one-line edit (2 runs) | 11.4 s, 19.4 s |
| Full `./dev-scripts/rust.sh test` after a one-line kit edit | 42.5 s |
| Kit doc-test step (there are no doc tests) | about 3 s |

The full `test` breaks down like this (from `--timings`):
- **The wait follows the kit library.** It takes 32.8 s (5.8 s front end, **27.0 s code generation**), then the game test binaries follow (Long Way 8.7 s, R-Town 5.2 s).
- **The kit test binary does not add to the wait.** It takes 25.2 s, but it builds at the same time as the library.
- **Lint checks the test code too.** `lint` runs `clippy --all-targets`, so it does a front-end pass over the test code. The front end is only 18% of the build.

### Conclusion

The tests are not worth optimising for time.
- Run time is too small to matter.
- Test code adds compile time only when you run `test game-kit` alone (about 11 to 19 s after an edit), and test code is only part of that time.
- Removing the 25 duplicates would cut about 7% of the test code, which is less than 1 s.

### Levers that do change the wait (each needs a decision)

1. **Skip the empty doc-test step.** Add `doctest = false` to `[lib]` in `crates/game-kit/Cargo.toml`. This saves about 3 s on every `test`. Trade-off: a future code example in a doc comment would not be compiled.
2. **Lower the cost of code generation for the kit.** It is 82% of the kit build. `[profile.dev]` builds workspace crates at `opt-level = 1`. A lower level for the test profile could cut the test binary time, but the games would run slower in tests. Measure it first, and check `target/` size: a profile change can build an extra copy of the outputs (`CLAUDE.md`, "Disk use").
3. **Split `game-kit` into smaller crates.** For example: geometry (outline, collision, nav, projectile), maps (parser, areas, links), and the Bevy plugins. An edit to one part would then rebuild less. This is an architectural decision with a large change. Consider it only if the edit-and-test loop becomes a real problem.

## Suggested order of work

1. Fix the 8 tests that do not check what they say, and decide what `skip_title_once` should do.
2. Add the priority A gap tests, starting with `map_transition.rs` (cell scaling, `walkable` Bottom and Left, the error paths) and `projectile.rs` (a target behind a blocked area).
3. Delete or merge the duplicates and low-value tests. Remove the copied `rects_overlap`.
4. Replace the game values in kit tests with neutral values.
5. Optional: add `doctest = false`, and a shared test helper module.
