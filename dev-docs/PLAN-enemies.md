# PLAN: enemies in R-Town

Status: draft. Waiting for the answers in sections 2 and 3.

## 1. Goal

Enemies appear at spawn points on the map, walk by a movement strategy, kill the player on touch and die when a projectile hits them.

- Sprites: `input/medieval-fantasy-1/monsters/skeleton.png` (13x11) and `goblin.png` (13x10). Each is one static pose, like the player.
- Spawn points are set in the map file (`test.txt`). Each spawns an enemy at its own interval, for example every 10 seconds.
- Enemies move and animate like the player: flip left and right, a one pixel bob while walking (`adr-rt3`). The player has no up or down art, and the monsters have none either.
- A strategy chooses where the enemy goes. The kit has a common set. A game can add its own.
  - **Random patrol:** pick a random place within X px of the current place, walk there, pick a new one.
  - **Pursue:** walk towards the player. When the player moves, follow.
  - Later: evade and others.
- Obstacles need path finding: a straight line to the player often runs into a tree or a pond.
- Enemy touches the player: the player dies.
- A projectile hits an enemy: the enemy dies and the projectile is removed.

## 2. Decisions needed (recommendation first)

| # | Question | Recommendation | Trade-off |
|---|---|---|---|
| D1 | Path finding | **A grid of walk points built from the collision edges, with A\*.** Default step 4 px (a parameter). Two neighbours are linked when `collision::resolve` lets a point move between them. Use the `pathfinding` crate for A\* if it compiles for `wasm32-unknown-unknown` (step 1). If not, write A\* ourselves (about 100 lines). | A grid reuses the exact outlines (thin tree bands too) and is simple to test. A navigation mesh crate is heavier and is tied to a Bevy version. The grid takes memory and build time: 60x60 cells at 4 px is 57,600 points, built once per map load. Roadmap item 3 plans this. This plan builds the first part. |
| D2 | How strategies are built | **A strategy is a component plus a system that writes a `MoveGoal` component** (a point to walk to, or none). One kit system turns the goal into movement (path, collision, speed). A game adds a strategy with its own component and system. | No trait objects and no boxed state, and it is the normal ECS style. A strategy can only say "go there". A strategy that needs a special movement (jump, teleport) needs its own mover. |
| D3 | Spawner settings in the map | **Parameters after the kind name in the legend**, for example `"S": spawn_skeleton every=10 max=3`. Each legend character is one configured spawner. Different intervals use different characters. The kit parser reads the parameters, and the game reads them by name. | The grid keeps one character per cell. A small, backward compatible parser change. It is a first step into roadmap item 4 (objects with settings). The alternative is fixed values per kind in code: simpler, but no "various intervals" in the map file. |
| D4 | Random numbers | **A small seedable generator in the kit** (about 15 lines, no dependency). The seed comes from the clock, or from `R_TOWN_SEED` for a repeatable run. | The `rand` crate needs `getrandom` and extra setup for the web build. A seed that we control makes scripted runs repeatable. |
| D5 | Hit boxes | **One kit component `HitBox { offset, half_size }`** (relative to the ground point). The projectile tests it (`ProjectileTarget`), and the game tests it for enemy against player. It is not the feet box. | Feet are tiny (10x4) and sit at the bottom. A touch or a hit on the body needs a bigger box. Boxes are rectangles, not the sprite shape. |
| D6 | Enemy collision with the world | **Enemies use the same blocked areas as the player** (`adr-rt5`, `adr-rt9`) and the same feet width. | One set of areas, and the path grid fits both. A bigger monster later needs its own areas. |
| D7 | Enemy against enemy | **No collision between enemies in this plan.** They may overlap. | Simple and cheap. Pursuers will stack on the same line. Add separation later if it looks bad. |
| D8 | Map reload (F1) | **Enemies and spawners are removed, and the path grid is built again.** | An old path can run through a place that is now blocked. This differs from D6 of the projectile plan (shots in flight are kept), because paths depend on the map. |
| D9 | Walk animation | **Move the flip and bob code from `player.rs` into one R-Town module used by the player and the enemies.** | No copy of the code. It stays in the game until a second game needs it. |

## 3. Questions for the developer

The game design is not defined for these points. Defaults are in brackets. Please answer or accept.

1. **Player death.** What happens? [Enemies stop, the player sprite is hidden, the text "You died, press R" shows, R restarts: player at the start, enemies removed.] Hearts or lives later?
2. **Which strategy for which enemy?** [Skeleton: random patrol, radius 96 px. Goblin: pursue.] Should a patrolling enemy start to pursue when the player comes close, and give up when the player is far? [Not in this plan.]
3. **Speeds.** The player walks at 80 px/s and the projectile flies at 160. [Skeleton 40, goblin 56.]
4. **Spawn rules.**
   - First spawn at the start, or after one interval? [After one interval.]
   - A fixed interval, or a range such as `every=8..12`? [Both: `every=10` or `every=8..12`.]
   - A limit of live enemies per spawner (`max=`)? [Default 3.]
   - Skip a spawn when the player is closer than 128 px? [Yes.]
   - Only spawn out of the camera view? [No.]
5. **Projectile.** Does it disappear on the first enemy it hits? [Yes.] Does a dead enemy leave anything behind (a puff, a drop)? [No, it just disappears.]
6. **Hit box sizes.** [Enemy: the sprite without the bottom 2 px. Player: 8x12 around the body. A touch is a box overlap.]
7. **Patrol details.** X is a parameter. [96 px, a new target when the enemy arrives or when it makes no progress for 2 seconds.]

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
- `strategies.rs`: `RandomPatrol { radius, ... }` and `Pursue { target }` components with their systems. `Pursue` follows the `Controlled` entity when no target is set. Pure functions (next patrol point, arrival, stuck check) are unit tested.
- `spawner.rs`: `Spawner { every, max, ... }` component and a timer system. It sends `SpawnRequest { spawner, position }`. The game spawns the entity (as for projectiles). The kit counts live children to enforce `max`.
- `hitbox.rs`: `HitBox` and `overlaps(a, b)` (D5).
- `projectile.rs`: `ProjectileTarget` marker. The step tests the path of the shot (a segment, not only the end point) against hit boxes. New `HitCause::Target(Entity)`. The projectile is removed and `ProjectileHit` is sent. The game removes the enemy when it reads that message.

### 4.2 Game (`games/r-town/src/`)

- `walk_animation.rs`: flip and bob (D9), used by `player.rs` and `enemy.rs`.
- `enemy.rs`: `EnemyKind` (skeleton, goblin) with speed, strategy, hit box and art tag. Spawns on `SpawnRequest`. Removes the enemy on `ProjectileHit` with a target. `MapEntity` so F1 removes them (D8).
- `map.rs`: legend kinds `spawn_skeleton` and `spawn_goblin` (markers with no art), read like `player_start`. Spawner entities with `MapEntity`.
- `player.rs`: a `HitBox`, a `Dead` state flag.
- `death.rs`: the touch check, the state, the text and the restart (question 1).
- Test map `assets/maps/enemy-test.txt`: small, with a lone tree, a grove, a pond and spawners with short intervals. Run with `./dev-scripts/rust.sh run r-town enemy-test.txt`. The developer's `test.txt` is not edited by Claude.
- Art: `art/characters/monsters.aseprite`, prefix `mon_`, tags `mon_skeleton` and `mon_goblin`, built with `pack_pngs.lua` (`adr-rt1`, `adr-rt6`). Update `art-tags.md`.

## 5. Steps

Each step ends with `./dev-scripts/rust.sh lint` and `test`. Ask before any commit.

1. **Checks.** Compile `pathfinding` for `wasm32-unknown-unknown`, and check its version and licence (workspace rule 2). Verify the Bevy 0.20.1 APIs: `States` and `run_if(in_state)`, `Timer` in a component, running a headless `App` in a test. Look at both PNGs for the direction the sprites face. Record in `dev-docs/bevy/notes.md`.
2. **Kit: `rng.rs`** and tests (same seed gives the same numbers, ranges, points inside the disc).
3. **Kit: legend parameters in `map.rs`** and tests (no parameters, one, several, a bad pair gives an error with its line). Update `map-files.md`.
4. **Kit: `nav.rs`.** Tests on built edges: open ground, around a thin band, around a grove, through a 6 px gap, a goal inside a blocked area (no path), a start inside one, smoothing keeps the path legal. Measure the build time and the path time for the 60x60 test map and write both numbers in the plan.
5. **Kit: `hitbox.rs`** and the projectile target step. Tests: a hit from each side, a diagonal, a long step through a box, a miss beside it.
6. **Kit: `movement.rs`** and a headless App test: an entity with a `MoveGoal` reaches a point behind a blocked band.
7. **Kit: `strategies.rs`.** Tests for the pure parts. A headless test that a custom strategy of the test (a component and a system that writes `MoveGoal`) moves an entity: this shows that a game can add its own.
8. **Kit: `spawner.rs`** and tests (timer, `max`, the player distance rule).
9. **Art:** pack the monsters and update `art-tags.md`.
10. **R-Town: `walk_animation.rs`.** Move the code out of `player.rs`. Walking must not change (a scripted walk before and after, as in the input step of the projectile plan).
11. **R-Town: `enemy.rs`, spawners in the map, the test map.** Scripted runs and screenshots: an enemy spawns after its interval, a patrol enemy walks to new places, a pursuer goes around a tree and a pond, a shot kills an enemy, F1 removes the enemies.
12. **R-Town: `death.rs`.** Touch kills, the text shows, R restarts. Scripted run: the player stands still and a pursuer arrives.
13. **Debug view.** F3 shows spawners, goals and paths as thin sprites on the feature debug layer (`adr-x9`).
14. **Load test.** 30 enemies at once: frame time stays near 60 fps. If not, tune the repath budget or the grid step.
15. **Lint, test, web** for the whole workspace.
16. **Commit.** Ask first. Do not push unless asked.
17. **Migrate lasting content, then delete this plan.**
    - New extension documents: `nav.md` (grid, A\*, parameters), `enemy-movement.md` (goal, patrol, pursue, how to write a strategy), `spawners.md`. Add them to the index in `dev-docs/extensions/README.md`. Update `projectiles.md` (targets) and `map-files.md`.
    - ADRs for D1 to D9 in `dev-docs/adr/EXTENSIONS.md` (`adr-x23` on) and `games/r-town/docs/adr.md` (`adr-rt12` on).
    - `games/r-town/docs/SPEC.md` (enemies, death), the R-Town `README.md` (source layout, controls, test map), the user guide `docs/README.md` (R restarts).
    - `dev-docs/ROADMAP.md`: completed item "Enemies". Add the open points of section 6. Update item 3 (the grid exists) and item 9.
    - `dev-docs/bevy/notes.md` and `concepts.md` (states, headless tests).
    - Delete `dev-docs/PLAN-enemies.md`.

## 6. Other things to think about now

- **Player death flow** (question 1). It decides the Bevy `States` layout for the whole game. Better to choose a simple one now.
- **Safe spawns.** An enemy that spawns on top of the player is an unfair death. The distance rule (question 4) and a short pause before a new spawner may act on a restart help.
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
