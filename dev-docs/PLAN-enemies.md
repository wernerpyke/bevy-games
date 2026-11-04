# PLAN: enemies in R-Town

Status: in progress (steps 1 to 15 done). The developer answered the design questions (section 3). The recommendations in section 2 are accepted unless changed below. Waiting for the go-ahead for step 1.

## 1. Goal

Enemies appear at spawn points on the map, walk by a movement strategy, kill the player on touch and die when a projectile hits them.

- Sprites: `input/medieval-fantasy-1/monsters/skeleton.png` (13x11) and `goblin.png` (13x10). Each is one static pose, like the player.
- Spawn points are set in the map file (`test.txt`). Each spawns an enemy at its own interval.
- Enemies move and animate like the player: flip left and right, a one pixel bob while walking (`adr-rt3`). The player has no up or down art, and the monsters have none either.
- A strategy chooses where the enemy goes. The kit has a common set. A game can add its own.
  - **Random patrol:** pick a random place within X px of the current place, walk there, pick a new one.
  - **Pursue:** walk towards the player. When the player moves, follow.
  - Later: evade and others.
- Obstacles need path finding: a straight line to the player often runs into a tree or a pond.
- Enemy touches the player: the player dies. For now "dies" means the player entity and its sprite are removed (hit points come later).
- Spawn points are in `test.txt`: a skeleton spawner every 10 seconds and a goblin spawner every 15 seconds, first spawn at game start.
- A projectile hits an enemy: the enemy dies and the projectile is removed.

## 2. Decisions needed (recommendation first)

| # | Question | Recommendation | Trade-off |
|---|---|---|---|
| D1 | Path finding | **A grid of walk points built from the collision edges, with A\*.** Default step 4 px (a parameter). Two neighbours are linked when `collision::resolve` lets a point move between them. Use the `pathfinding` crate for A\* if it compiles for `wasm32-unknown-unknown` (step 1). If not, write A\* ourselves (about 100 lines). | A grid reuses the exact outlines (thin tree bands too) and is simple to test. A navigation mesh crate is heavier and is tied to a Bevy version. The grid takes memory and build time: 60x60 cells at 4 px is 57,600 points, built once per map load. Roadmap item 3 plans this. This plan builds the first part. |
| D2 | How strategies are built | **A strategy is a component plus a system that writes a `MoveGoal` component** (a point to walk to, or none). One kit system turns the goal into movement (path, collision, speed). A game adds a strategy with its own component and system. | No trait objects and no boxed state, and it is the normal ECS style. A strategy can only say "go there". A strategy that needs a special movement (jump, teleport) needs its own mover. |
| D3 | Spawner settings in the map | **Parameters after the kind name in the legend**, for example `"S": spawn_skeleton every=10 strategy=patrol radius=120`. `strategy=<name>` names the movement strategy of the enemies of that spawner. The kit names are `patrol` (needs `radius=<px>`) and `pursue` (no parameters). A game can add names of its own. `max=<n>` (live enemies per spawner) is an optional parameter, and without it there is no limit. Each legend character is one configured spawner. Different intervals use different characters. The kit parser reads the parameters, and the game reads them by name. | The grid keeps one character per cell. A small, backward compatible parser change. It is a first step into roadmap item 4 (objects with settings). The alternative is fixed values per kind in code: simpler, but no "various intervals" in the map file. |
| D4 | Random numbers | **A small seedable generator in the kit** (about 15 lines, no dependency). The seed comes from the clock, or from `R_TOWN_SEED` for a repeatable run. | The `rand` crate needs `getrandom` and extra setup for the web build. A seed that we control makes scripted runs repeatable. |
| D5 | Hit boxes | **One kit component `HitBox { offset, half_size }`** (relative to the ground point). The projectile tests it (`ProjectileTarget`), and the game tests it for enemy against player. It is not the feet box. | Feet are tiny (10x4) and sit at the bottom. A touch or a hit on the body needs a bigger box. Boxes are rectangles, not the sprite shape. |
| D6 | Enemy collision with the world | **Enemies use the same blocked areas as the player** (`adr-rt5`, `adr-rt9`) and the same feet width. | One set of areas, and the path grid fits both. A bigger monster later needs its own areas. |
| D7 | Enemy against enemy | **No collision between enemies in this plan.** They may overlap. | Simple and cheap. Pursuers will stack on the same line. Add separation later if it looks bad. |
| D8 | F1 (map reload) | **F1 resets the whole game state**: the map is read again, and the player, the enemies, the spawners and the projectiles in flight are removed. The player and the spawners are created again, the player at the start cell, and the path grid is built again. | Developer decision. A full reset is simple and safe: no old path, shot or enemy outlives its map. It reverses two earlier rules: F1 used to leave a living player where it was (`adr-rt8`), and projectiles in flight were kept (D6 of the projectile plan, `adr-rt11`). |
| D9 | Walk animation | **Move the flip and bob code from `player.rs` into one R-Town module used by the player and the enemies.** | No copy of the code. It stays in the game until a second game needs it. |
| D10 | Player death | **The player entity and its sprite are removed** (developer decision). No text, no state. Systems that need the player (`Single<...>`) skip their work when it is gone, and enemies with `Pursue` have no goal and stand still. Hit points and a death screen come later. | The simplest start. F1 puts the player back (section 3, answer 8). |

## 3. Design answers from the developer

1. **Player death.** The player entity and its sprite are removed (D10). Hit points come later.
2. **Strategies and speeds.** The player walks at 80 px/s.
   - Skeleton: random patrol within 120 px, speed 0.8 x the player = 64 px/s.
   - Goblin: pursue, speed 0.2 x the player = 16 px/s (one cell per second).
   - The code states both as a factor of the player's `WALK_SPEED`, so they follow when it changes.
3. **Spawners.** One spawner per enemy kind, in `test.txt`: skeleton every 10 s, goblin every 15 s. Each legend entry also names the movement strategy: `strategy=patrol radius=120` for the skeleton, `strategy=pursue` for the goblin. Speed stays in the game code, per enemy kind (answer 2). The first enemy appears at game start. No limit of live enemies, no distance rule, no on-screen rule. A fixed interval only (`every=<seconds>`). The first test positions are in step 12.
4. **Projectile.** It disappears on the first enemy it hits. A dead enemy leaves nothing behind.
5. **Hit boxes.** Enemy: the sprite without its bottom 2 px. Player: 8x12 around the body. A touch is a box overlap.
6. **Patrol.** A new target when the enemy arrives. There is no "no progress" rule: path finding must make it unnecessary. A target is only chosen when a path to it exists.
7. **Test map.** No `enemy-test.txt`. The spawners go into `test.txt`.

8. **Restart.** F1 resets the whole game state: player (back at the start cell, dead or alive), enemies, spawners and projectiles in flight. This replaces the rules "F1 keeps a living player where it is" (`adr-rt8`) and "projectiles in flight are kept" (`adr-rt11`).

## 4. Design

### 4.1 Kit modules (`crates/game-kit/src/`)

- `rng.rs`: seedable generator (D4). `Rng::new(seed)`, `f32()`, `range(a, b)`, `point_in_disc(centre, radius)`.
- `map.rs`: legend parameters (D3). `MapObject` gets `params: Vec<(String, String)>`, with a helper to read a number. A legend entry `kind key=value ...` stays valid when it has no parameters. Errors keep the file and line.
- `nav.rs`: `NavGrid` resource (D1). Built from `CollisionWorld` and the map rectangle. Built again when `CollisionWorld` changes. `find_path(from, to) -> Option<Vec<Vec2>>` and a path smoother (skip points that the straight line between them can reach). A straight line check with `resolve` stays cheap, so an enemy with a clear line does not need a path.
- `movement.rs`:
  - `MoveGoal(Option<Vec2>)`: set by strategies.
  - `Mover { speed }` and `PathFollower` state (the path, the next point, a repath timer).
  - A system turns the goal into a step with `move_feet` (so sliding rules apply), clamps to the map, and writes `Facing`-style data that the animation reads (the walking flag and the direction).
  - Repath only when the goal moved to another grid point, or every 0.3 s at most. A path request budget per frame keeps many enemies cheap.
- `strategies.rs`: `RandomPatrol { radius, ... }` and `Pursue { target }` components with their systems. `Pursue` follows the `Controlled` entity when no target is set. `RandomPatrol::from_params` reads `radius` from the legend parameters and returns an error message when it is missing or not a number. `RandomPatrol` picks a point in the disc around the enemy, keeps it only when it is on the map and `NavGrid` has a path to it, tries a few times, and waits one second before it tries again when none works. It picks a new target on arrival. Pure functions (next patrol point, arrival) are unit tested.
- `spawner.rs`: `Spawner { every, max: Option<u32> }` component and a timer system. The first request is sent at once, the next after each interval. It sends `SpawnRequest { spawner, position, params }`. `params` are the legend parameters of the spawner, so the game can read `strategy` and the values that belong to it. The game spawns the entity (as for projectiles). The kit counts live children to enforce `max` when it is set.
- `hitbox.rs`: `HitBox` and `overlaps(a, b)` (D5).
- `projectile.rs`: `ProjectileTarget` marker. The step tests the path of the shot (a segment, not only the end point) against hit boxes. New `HitCause::Target(Entity)`. The projectile is removed and `ProjectileHit` is sent. The game removes the enemy when it reads that message.

### 4.2 Game (`games/r-town/src/`)

- `walk_animation.rs`: flip and bob (D9), used by `player.rs` and `enemy.rs`.
- `enemy.rs`: `EnemyKind` (skeleton, goblin) with speed, hit box and art tag. The strategy comes from the spawner parameters: a plain `match` on the name adds the kit component (`RandomPatrol::from_params`, `Pursue`) or a component of the game. An unknown name, or `patrol` without `radius`, is a map error with the file and line, found when the map loads. Spawns on `SpawnRequest`. Removes the enemy on `ProjectileHit` with a target. `MapEntity` so F1 removes them (D8).
- `map.rs`: legend kinds `spawn_skeleton` and `spawn_goblin` (markers with no art), read like `player_start`. Spawner entities with `MapEntity`.
- `player.rs`: a `HitBox`.
- `death.rs`: the touch check. On a touch it removes the player entity with its sprite (D10).
- `map.rs` (`reload_map`): the reload removes everything marked `MapEntity`, so the marker goes on the player, the projectiles (set in `weapon.rs`) and the enemies as well as on map objects. It then runs `spawn_player` (`pub(crate)`) and the spawner setup. The doc comment of `MapEntity` says it marks everything that a reload removes. The projectile change is small and lands in step 10.
- `assets/maps/test.txt`: Claude adds the two legend entries (`"S": spawn_skeleton every=10 strategy=patrol radius=120`, `"O": spawn_goblin every=15 strategy=pursue`) and one cell for each, and changes nothing else. Suggested cells (the developer may move them): skeleton at column 20, row 20 (open ground south west of the pond), goblin at column 40, row 12 (east of the pond, so the straight line to the player start crosses the pond and the goblin must go around it).
- Art: `art/characters/monsters.aseprite`, prefix `mon_`, tags `mon_skeleton` and `mon_goblin`, built with `pack_pngs.lua` (`adr-rt1`, `adr-rt6`). Update `art-tags.md`.

## 5. Steps

Each step ends with `./dev-scripts/rust.sh lint` and `test`. Ask before any commit.

1. **Checks.** Compile `pathfinding` for `wasm32-unknown-unknown`, and check its version and licence (workspace rule 2). Verify the Bevy 0.20.1 APIs: `Timer` in a component, how `Single` skips a system when its entity is gone, running a headless `App` in a test. Look at both PNGs for the direction the sprites face. Record in `dev-docs/bevy/notes.md`.
2. **Kit: `rng.rs`** and tests (same seed gives the same numbers, ranges, points inside the disc).
3. **Kit: legend parameters in `map.rs`** and tests (no parameters, one, several, a bad pair gives an error with its line). Update `map-files.md`.
4. **Kit: `nav.rs`.** Tests on built edges: open ground, around a thin band, around a grove, through a 6 px gap, a goal inside a blocked area (no path), a start inside one, smoothing keeps the path legal. Measure the build time and the path time for the 60x60 test map and write both numbers in the plan. Done: the grid for `test.txt` (58,081 points at 4 px) builds in about 9 ms. On a random town-sized test map with 536 edges, 50 long paths took 64 to 91 ms in total (about 1.5 ms each), and a search for an unreachable goal explores the whole grid, so repath requests need a budget (step 6).
5. **Kit: `hitbox.rs`** and the projectile target step. Tests: a hit from each side, a diagonal, a long step through a box, a miss beside it.
6. **Kit: `movement.rs`** and a headless App test: an entity with a `MoveGoal` reaches a point behind a blocked band.
7. **Kit: `strategies.rs`.** Tests for the pure parts and for `from_params` (valid, missing radius, bad number). A headless test that a custom strategy of the test (a component and a system that writes `MoveGoal`) moves an entity: this shows that a game can add its own.
8. **Kit: `spawner.rs`** and tests (first spawn at once, the interval, `max` when set).
9. **Art:** pack the monsters and update `art-tags.md`.
10. **R-Town: F1 resets everything.** Put `MapEntity` on the player and on projectiles, make `reload_map` run `spawn_player` again, and amend the doc comments. Scripted run: fire, press F1 while a shot flies, and check that the shot is gone and the player is at the start. This step is independent of enemies.
11. **R-Town: `walk_animation.rs`.** Move the code out of `player.rs`. Walking must not change (a scripted walk before and after, as in the input step of the projectile plan).
12. **R-Town: `enemy.rs`, spawners in the map, `test.txt`.** Add the two legend entries and cells (section 4.2). Check that a wrong strategy name and a `patrol` without `radius` stop the load with the line number. Scripted runs and screenshots: an enemy spawns at the start and then at its interval, a skeleton walks to new places within 120 px and never stands still for long, the goblin goes around the pond, a shot kills an enemy, F1 removes the enemies and they spawn again.
13. **R-Town: `death.rs`.** A touch removes the player and its sprite. Scripted run: the player stands still and the goblin arrives (about 25 s at 16 px/s from 400 px, so use a `teleport` close to a spawner). After the death the log shows no error or warning from systems that need the player (camera, info box, weapon, enemy strategies). Then F1: the player is back at the start, the old enemies are gone, and the spawners start again.
14. **Debug view.** F3 shows spawners, goals and paths as thin sprites on the feature debug layer (`adr-x9`).
15. **Load test.** 30 enemies at once: frame time stays near 60 fps. If not, tune the repath budget or the grid step. Done: with both spawners at 0.3 s (a test copy of `test.txt`), 114 enemies were alive after 16 s (about 57 skeletons patrolling and 57 goblins pursuing the player). The frame time stayed at about 16.7 ms (60 fps, the display limit), apart from one 32 ms frame. No tuning was needed.
16. **Lint, test, web** for the whole workspace.
17. **Commit.** Ask first. Do not push unless asked.
18. **Migrate lasting content, then delete this plan.**
    - New extension documents: `nav.md` (grid, A\*, parameters), `enemy-movement.md` (goal, patrol, pursue, how to write a strategy), `spawners.md`. Add them to the index in `dev-docs/extensions/README.md`. Update `projectiles.md` (targets) and `map-files.md`.
    - ADRs for D1 to D10 in `dev-docs/adr/EXTENSIONS.md` (`adr-x23` on) and `games/r-town/docs/adr.md` (`adr-rt12` on).
    - `games/r-town/docs/SPEC.md` (enemies, death), the R-Town `README.md` (source layout, the spawners in `test.txt`).
    - Amend the decisions that F1 now changes: `adr-rt8` (F1 resets the whole game state, also a living player) and `adr-rt11` (projectiles in flight are removed on reload). Update the Limits of `projectiles.md`, the known issues of the completed item "Projectile weapons" in the roadmap, and the F1 text in `games/r-town/docs/README.md`, `docs/README.md` and `dev-docs/extensions/dev-feedback.md` if they say otherwise.
    - `dev-docs/ROADMAP.md`: completed item "Enemies". Add the open points of section 6. Update item 3 (the grid exists) and item 9.
    - `dev-docs/bevy/notes.md` and `concepts.md` (headless tests, systems that skip without their entity).
    - Delete `dev-docs/PLAN-enemies.md`.

## 6. Other things to think about now

- **Player death flow.** Removing the player is the first step. Hit points, a death screen and a restart will need a game state later (`States`).
- **Safe spawns.** An enemy that spawns on top of the player is an unfair death. A distance rule and a short pause after a restart are not in this plan.
- **Spawner on blocked ground.** A spawn point inside a tree area makes an enemy that cannot move. Check on load and log a warning with the cell.
- **Unreachable goals.** A patrol target or a player inside a pond or behind water has no path. The strategy must pick another target or wait. It must not search all points every frame.
- **Repeatable tests.** A seed (D4) and test maps make scripted runs repeatable. The script tool has no spawn command (`adr-x15`), so short spawn intervals in the test map do that job.
- **Performance and the web build.** Path search on a 57,600 point grid, many enemies, and no threads in the web build. The grid step, the repath rate and a budget per frame are parameters. Measure in steps 4 and 14.
- **Enemy stacking** (D7). Several pursuers will line up and overlap. Separation steering is a later step.
- **More than one hit point.** One hit kills now. Keep the death in one function so hit points can replace it.
- **Faction and friendly fire.** Player shots hit only enemies because only enemies have a target box. Enemy shots later need a rule for who they hit.
- **Mouse and input.** Roadmap item 8 is not part of this plan. Death and restart add a second key (R): route it through `PlayerInput` too.
- **Feedback.** A death puff, a hit flash and sound are not in this plan. `ProjectileHit` is where they would start.
- **Hit box and sprite size.** The projectile is a ground point with a height (8 px). A box test in the ground plane only is a simplification: a shot "passes over" nothing. Check on screen that hits look fair.
- **Pack art.** The pack has seven more monsters (bat, cyclop, dragon, king skeleton, leonard, slim, snake). They differ in size and need their own feet and hit boxes. The bat flies and may ignore obstacles. Not in this plan.
- **Strategy mixing.** "Patrol until the player is near, then pursue" needs a switch between strategies. With D2 it is a system that swaps the strategy component. Not in this plan, but D2 allows it.
