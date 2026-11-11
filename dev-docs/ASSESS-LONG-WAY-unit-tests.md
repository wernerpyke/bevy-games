# Assessment: Long Way unit tests

Date: 2026-11-11. Scope: every `#[cfg(test)]` module in `games/long-way/src/`. Nothing was changed in the code. Same method as `ASSESS-KIT-unit-tests.md`; the rules are in `testing.md`.

## Answers in short

| Question | Answer |
|---|---|
| Rating | **Moderate to good: 6 out of 10.** The newest code (weapons, ghost, enemy hit points) has strong tests of game mechanics. But 9 of 68 tests (13%) do not check what their names say; 3 of these were proved with a mutation check. The rock, tree and marker geometry has no tests. The kit scored 7 because only 2% of its tests had this problem. |
| Is the coverage good? | **Good for weapons and the ghost, missing for the map objects.** `rocks.rs`, `trees.rs`, `markers.rs` and the player's movement limits have no tests. The cheapest high-value gap: nothing checks that every name in `KIND_NAMES` has a `Kind`, so a missing entry panics at run time. |
| Are there overlapping or unnecessary tests? | **Few duplicates, some constant tests.** One near duplicate. Five tests check a constant against its own comment. Two of those can be compile-time asserts. The games follow `adr-w7`: no test repeats kit behaviour. |
| Is it worth optimising for build or test time? | **No.** The 68 tests run in 0.10 s. After a one-line edit the test binary builds in 8.1 s, and the game itself in 9.8 s. |

## How this was measured

- All test modules and the code they test were read in full (1,623 lines of test code in about 5,100 lines).
- **Mutation check** (`testing.md` rule 1). Three guards were removed, the matching tests were run, and the edits were undone. All three tests still passed:

| Guard removed | Test that still passed |
|---|---|
| `With<Touchable>` in `ghost.rs` `start_attack` | `a_ghost_that_is_already_attacking_is_not_touched_again` |
| `if hp.is_empty() { continue; }` in `hp.rs` `apply_damage` | `a_damage_message_takes_points_and_stops_at_zero` |
| `With<EnemyHp>` in `weapons/ring.rs` `update_rings` | `a_body_that_is_already_dead_or_has_no_hit_box_is_not_hit` |

- Times come from `cargo build` and `cargo test --no-run` after a one-line edit to `hp.rs`, which was then removed.

## Numbers

68 tests, none ignored. They run in 0.10 s.

| File | Tests | Files with no tests |
|---|---|---|
| `weapons/damage.rs` | 11 | `rocks.rs` (207 lines) |
| `ghost.rs` | 10 | `markers.rs` (102) |
| `enemy_hp.rs` | 9 | `trees.rs` (92) |
| `weapons/ring.rs` | 8 | `pickups.rs` (82) |
| `scenery.rs`, `weapon_menu.rs` | 6 each | `collision.rs` (54, tested through `scenery.rs`) |
| `map.rs` | 4 | `camera.rs`, `main.rs` (no logic) |
| `hp.rs`, `weapons/values.rs` | 3 each | |
| `player.rs`, `inventory.rs`, `weapons/mod.rs` | 2 each | |
| `hud.rs`, `background.rs` | 1 each | |

## 1. Tests that do not check what their names say

Fix these first.

| Test | Problem |
|---|---|
| `ghost.rs:309` `a_ghost_that_is_already_attacking_is_not_touched_again` | **Proved.** Only one `Finished` message is sent, so the damage count is 1 with or without the guard. Assert instead that the second touch does not restart `Attack`. For example, set a frame or a marker after the first touch and check that it stays. |
| `hp.rs:110` `a_damage_message_takes_points_and_stops_at_zero` | **Proved.** `saturating_sub` already stops at 0. The guard's real effect is that no `PlayerHurt` is sent once HP is 0, and nothing asserts that. Count the `PlayerHurt` messages. |
| `weapons/ring.rs:446` `a_body_that_is_already_dead_or_has_no_hit_box_is_not_hit` | **Proved.** The body has neither `EnemyHp` nor `HitBox`, so the test cannot tell which filter works. It never tests a dead body. In the game a dead ghost loses its `HitBox` (`ghost.rs`), so the ring depends on each enemy plugin doing that. Split the test: one body with `HitBox` and no `EnemyHp`, and one dead enemy that still has its `HitBox`. |
| `ghost.rs:439` `a_ghost_that_is_attacking_or_dying_does_not_play_hurt` | Tests only the attacking case, not dying. |
| `ghost.rs:461` `starting_an_attack_or_dying_while_hurt_takes_over` | Tests only dying, not attacking. |
| `weapons/damage.rs:149` `a_long_frame_still_deals_one_tick_and_keeps_the_rhythm` | No frame in it is long. The frames are 0.0, 0.4 and 0.2 s, and the tick is 0.5 s. See "Possible defects". |
| `weapons/ring.rs:458` `the_ring_starts_to_burn_at_the_lead_radius_and_not_before` | Checks "not burning after 2 frames" and "burning after 1 more second". It never checks the radius when burning starts. Assert that the frame before `Burning` has a radius below `max_radius - burn_lead`, and the frame with it has a radius at or above it. |
| `player.rs:348` `a_hit_plays_hurt_once_and_idle_comes_back_when_it_ends` | Ends on `Down`, not `Idle`, because the test sets `walking = true`. The behaviour is right, but the name is wrong. |
| `enemy_hp.rs:302` `a_target_without_hp_is_ignored` | Asserts only "no death". A target without HP can never die, so this asserts nothing. The real risk is a `HitFlash` inserted on a non-enemy, so assert that it is absent and that no `EnemyHurt` is sent. |

## 2. Possible defects found while reading

- **`Contact::damage` catches up after a long frame.** A 2 s frame gives one tick at once, then one tick in each of the next 4 frames. The kit's spawner deliberately avoids such a burst (`a_long_frame_gives_one_spawn_not_a_burst`). It cannot happen in the game today, because Bevy limits a frame to 0.25 s, which is below the 0.5 s tick. Decide which rule is wanted, then name and test it.
- **The ring hits any `EnemyHp` that has a `HitBox`.** This includes a dead one, until its plugin removes the `HitBox`. That works for the ghost. A future enemy that keeps its `HitBox` while it plays `Die` would still be pushed.

## 3. Gaps

### Priority A (game rules that break without a visible error)

- **`KIND_NAMES` and `Kind::from_name` can drift.** `Kind::of` panics on a name that the parser accepts but `from_name` does not know. Add one test: every entry of `KIND_NAMES` gives `Some(_)`. Do the same for `Energy::from_kind` against the energy kinds.
- **`rocks.rs` has no tests.** The untested geometry:
  - `placement` (scale rounding, the sort line)
  - `group_area` with and without `open_top`
  - the band of a rock that stands alone in `blocked_area`
  - `jitter_offset`: it stays inside `-jitter..=jitter`, and two cells get different offsets

  This geometry decides how the player walks around rocks.
- **`trees.rs` and `markers.rs` have no tests.** The untested functions:
  - `trees.rs`: `placement` and `tree_blocker` (alone and in a group)
  - `markers.rs`: `blocked_area` and `touch_area`

  `scenery.rs` has the same kind of tests, which can serve as the pattern.
- **`blocking_groups` sets `neighbour_above`.** Only `touches_top` is tested, not that the flag reaches the right object.
- **`player.rs`:**
  - `limit_player`: no input while a transition locks it, and the sideways flip with `WALK_ART_FACES_RIGHT`
  - `arrive_player`: the facing and the flip for each `EntryFacing`
- **`ghost.rs` `spawn_ghosts`:** the feet at the cell centre (use a cell size other than 64), and a marker with a bad strategy is skipped.
- **`weapon_menu.rs` systems:**
  - The same icon closes the drawer and a click in the world closes it.
  - An energy with no weapons shows the text.
  - Only a primary click chooses a weapon.
  - The drawer is hidden when the slide is at 0.

  The pure helpers are tested; the systems are not.

### Priority B (seen on screen at once)

- The HUD update systems, the background tile spawn, the pickup art table, and the warning when `add_collected` sees an unknown kind.

## 4. Overlap and unnecessary tests

**Near duplicate:** `weapons/damage.rs:143` `the_first_frame_in_the_effect_hits_at_once` is mostly covered by `:124`, whose tick count includes the tick at time 0. Keep `:143`, because it states the rule plainly, but do not add more of this kind.

**Constant tests** (rule 8):
- **Turn into compile-time asserts:**
  - `weapon_menu.rs:387` `the_drawer_is_60_percent_of_the_bar`
  - `weapons/values.rs:132` `the_ring_is_six_cells_across`

  `values.rs:69` already has a `const _` assert. Put these beside it.
- **Delete or keep only as notes:** `hp.rs:86` `the_player_starts_with_three_full_points` and `enemy_hp.rs:166` `the_hurt_colour_is_993d41`. The second guards the conversion of a palette hex into floats, so it may be worth keeping.

**Data checks worth keeping:** `values.rs:118`, `values.rs:137`, `weapon_menu.rs:392`. Each guards a design rule across all weapons.

**Layered, not duplicates:** the pure `EnemyHp::lose` tests and the `apply_damage` system tests; `Contact::push` and `apply_hits`.

**Kit boundary (`adr-w7`): followed.**
- The map tests use the kit parser only to build a test map.
- `map.rs:349` `big_objects_group_when_two_cells_apart_and_not_when_three` partly repeats the kit's grouping rule. Its own value is Long Way's footprint sizes, so it is acceptable.

## 5. Other quality notes

- **Weak asserts:**
  - `weapons/ring.rs:481` uses a bound of `max - lead + 4 + 100`, where the 100 has no reason.
  - `weapons/ring.rs:395` checks `times.len() >= 4`, which is loose. The burn gives 4 or 5 ticks, plus one from the front.
- **Wrong constant:** `weapon_menu.rs:406` compares a row width with `VIEW_HEIGHT`. It works only because the view is square. It also checks the current largest drawer (3 weapons), not the 4 that the comment promises. Compare `4 * BUTTON_SIZE.x + 3 * BUTTON_GAP` with the drawer art width.
- **Style:** `player.rs:340` uses `assert!(a == b)`, so a failure does not print the values. Use `assert_eq!`.
- **Shared helpers:**
  - Collecting the current messages (`iter_current_update_messages`) is written 4 times.
  - The `MinimalPlugins` plus `ManualDuration` App builder is written twice.

  This is small. A shared helper is optional.
- **Flaky tests: none found.**
  - Time is manual in all App tests.
  - `ghost.rs:523` repeats 20 rounds to catch an unordered schedule, which is right for a past ordering bug.
- **Out-of-date doc:** `games/long-way/docs/testing.md` says Long Way has no unit tests, and lists only kit tests. Rewrite it to list the game's own tests and to point to `dev-docs/testing.md`.

## 6. Build and test time

| Step | Time |
|---|---|
| Run all 68 tests | 0.10 s |
| `cargo build -p long-way` after a one-line edit | 9.8 s |
| `cargo test -p long-way --no-run` after the same edit | 8.1 s |
| Long Way test binary in a full `test` after a kit edit | 8.7 s (from the kit assessment) |

The test code is 32% of the crate, but it adds no extra wait: the test binary builds in less time than the game binary. There is nothing to gain here. The kit assessment lists the levers that do help, all of them in the kit build.

## Suggested order of work

1. Fix the 9 tests in section 1. For each fix, run the mutation check.
2. Add the `KIND_NAMES` test and the `rocks.rs` geometry tests.
3. Decide the long-frame rule for `Contact::damage`, and whether the ring should skip dead enemies itself.
4. Add tests for `trees.rs`, `markers.rs`, `blocking_groups` with `neighbour_above`, and the `player.rs` limits.
5. Turn the two constant tests into `const` asserts, and rewrite `games/long-way/docs/testing.md`.
