# Writing unit tests

Rules for every crate in the workspace: `game-kit` and each game. They come from the assessment of the kit tests (`ASSESS-KIT-unit-tests.md`, 2026-11-11). That assessment found tests that passed without checking what their names said, repeated tests, and branches no test reached. Each example below is a real case from that assessment.

Where tests live and which crate tests what: `adr/WORKSPACE.md` adr-w7 and `CLAUDE.md`, "Tests".

## The three main rules

### 1. Prove that each test can fail

After you write or change a test, break the line it targets and run the test:

1. Change the code under test: remove the guard, swap the two checks, change `<` to `<=`, or return early.
2. Run only that test: `./dev-scripts/rust.sh test <crate> <test-name>`.
3. Make sure it fails. Then undo the change with the Edit tool. Do not use `git checkout`: the file can have other changes that are not committed.

If the test still passes, the test is wrong, not the code. Fix the test before you go on.

*Examples:*
- `projectile.rs` `a_hit_wins_over_leaving_the_map` still passed with the two checks in `step` swapped.
- `map_transition.rs` `requests_during_a_transition_are_ignored` still passed with the idle guard removed.

### 2. Make the input cross the boundary that the name describes

A test name states a rule. The input must reach the exact case that the rule decides.

- For "X wins over Y", X and Y must both happen in the same step. *Example:* the shot that "hits before it leaves the map" ended at x 199.67, inside a map 200 wide, so it never left the map.
- For "ignored while Z", send the input while Z is active, in a later frame. Do not send it in the same call that starts Z. *Example:* two map requests in one frame were read in one loop, so the second only overwrote the first. The test could not tell whether the guard worked.
- For a limit, test the value just on each side of the limit, and not a value far from it.

### 3. Assert exact values

- Do not put asserts inside `if let` or a `match` arm that may never run. If a branch must run, assert that it ran. *Example:* `strategies.rs` asserted inside `if let Some(point)`, so it passed when `pick_target` always gave `None`.
- Do not stop at `is_some()`, `is_ok()` or `matches!(x, Variant(_))`. Check the value inside: `assert_eq!(result, Step::HitBlocked(Vec2::new(198.0, 52.0)))`.
- Do not use a count as a stand-in for what you mean. *Example:* counting all `GlobalZIndex` entities to find the dim layers breaks when any other UI entity is added.
- Assert what the name says. *Example:* a test named "patrols" asserted only "not alerted", and never checked that a goal was set.

## More rules

### Inputs and branches

4. **Vary every value that the code multiplies, divides or offsets by.** Give each map in a test a different cell size, a non-zero origin and a scale other than 1. *Example:* every map test used cell size 64, so the offset scaling `to.cell / from.cell` was never tested.
5. **Test every arm of code that has one arm per direction, per order or per error.** Use one table test over the four edges, both input orders and each error message. *Example:* the `Bottom` and `Left` arms of `walkable` swap two values, and no test reached them.
6. **Test the error paths.** Each error message is a branch. Check the message text and the line number. Check also that a failed load or reload leaves the old state unchanged.

### What not to test

7. **Do not test code that you do not own.** Examples: glam math, Bevy scheduling or `run_if`, query matching, the fields of a struct literal.
8. **Do not test constants or determinism alone.** A compile-time `const _: () = { assert!(...) };` is the right check for constant order (see `layers.rs`). "The same input gives the same result" adds nothing when a test already compares against fixed expected values.

### Duplicates and helpers

9. **Read the tests in the module before you add one.** List them with `./dev-scripts/rust.sh test <crate> <module>:: -- --list`.
   - If a test already covers the branch, make that test stronger. Do not add a new one.
   - Do not add a looser copy of a stricter test. *Example:* "3 cells apart do not touch" next to "2 cells apart do not touch".
10. **Keep one test layer per rule.** A pure function is tested in its own module. A system or plugin test then checks only the wiring: that the system calls the rule and acts on the result. It does not test every case of the rule again.
11. **Do not copy code.** Before you write a helper, search for one that exists. Put helpers that several test modules share in one `#[cfg(test)]` module in that crate. Never copy a production function into another file: make it public and call it. *Example:* `rects_overlap` was copied into `debug.rs` from `map_transition.rs`, and both copies had tests.

### Keeping tests honest over time

12. **Keep the name, the comment and the body in step.** When you change a test body, read its name and its comment again. *Example:* a comment said the feet move back into a trigger, but the code never moved them.
13. **Use neutral values in kit tests.** Game values and game layouts belong in the game's own tests. *Example:* kit tests used Long Way's player width and R-Town map layouts. They break when a game changes its tuning.
14. **Do not tie asserts to engine internals or incidental output:**
    - exact log text or hook lists
    - Bevy's maximum frame time
    - a fixed number of updates when a condition can be awaited
    - exact equality on computed floats

    Use a tolerance, and state why that tolerance is right.
15. **Keep tests deterministic.** Use `TimeUpdateStrategy::ManualDuration` for time and a seeded random source. Never use real time or sleeps.

## Checklist before reporting work as done

- [ ] Each new or changed test failed when its target line was broken (rule 1).
- [ ] Each test reaches the case that its name describes (rule 2).
- [ ] No asserts inside an optional branch. Values are checked, not only variants (rule 3).
- [ ] No new test repeats an existing one (rule 9).
- [ ] `./dev-scripts/rust.sh test <crate>` and `./dev-scripts/rust.sh lint` pass.
