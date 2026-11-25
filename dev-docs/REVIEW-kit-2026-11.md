# Review: game-kit and Long Way (2026-11-23)

A review of `game-kit` and `long-way` for duplicated code, Bevy 0.20 practice, and how easily Claude finds kit solutions. R-Town was included in the "game against kit" check.

- 58 findings confirmed, 3 rejected by the verifier.
- 17 agents, read-only, no builds.
- Each finding was checked by a second agent that tried to refute it by reading the code. No finding has been reproduced on screen yet.

## Blind benchmark: passed

One agent reviewed the code as it was before commit `d0b31fa` (`SortAtBlocker`). It was not told about the bug and could not read that commit or `adr-x53`. Its first finding was: "Every blocking object sets its depth-sort line by hand with SORT_BIAS, apart from its Blocker rectangle." It named rocks, trees, scenery, markers, the volcano, pickups and R-Town trees.

This is the bug that came back three times. So the "game against kit" method can find this class of problem without help.

## The main lesson

The repeated bug had one cause: **the kit asked each spawn site to remember a rule** (add a bias, set an offset, add a tag). `SortAtBlocker` fixed one case. The review found more of the same shape. Each one is a possible repeat bug.

Recommended principle, for an ADR:

> When a kit feature needs a value or a companion component at the spawn site, the kit derives it, requires it with `#[require]`, or warns in debug builds. A game never subtracts a kit constant by hand.

### Rules that each spawn site must still remember

**1. Rocks and pickups still set their sort line by hand.** High.
- Where: `long-way` `rocks.rs:31-34`, `rocks.rs:74`, `pickups.rs:71-73`; kit `depth.rs:58`, `depth.rs:88`.
- `SortAtBlocker` only reads a `Blocker`. Rocks keep their area in `RockArea`, so they calculate the line again with `SORT_BIAS`. Pickups borrow the same constant. The debug warning cannot see either of them. Six of the seven review tracks found this independently.
- Fix: give rocks a `Blocker` plus `SortAtBlocker` and delete `RockArea`. Or add a kit `SortAtArea(Rect)` that rocks and pickups both use. Pickups need an explicit bottom rect, not their touch box. Then make the bias constant private.

**2. The feet position has two sources, and some kit systems use neither.** High.
- Where: kit `movement.rs:78-80`, `strategies.rs:197`, `strategies.rs:249-254`, `strategies.rs:291`, `hitbox.rs:7`; `long-way` `player.rs:127-129`, `player.rs:272`.
- `Mover::feet_offset` and `DepthSort::feet_offset` must be equal, and the game sets both by hand. Some kit systems read one and some read the other. **Patrol and sentry read neither.** They find paths and check line of sight from the sprite centre. This is a latent bug for any enemy that has a feet offset.
- Fix: keep one source, for example a kit `feet_of(&Transform, Option<&DepthSort>)`. Use it in movement, strategies, collectables, script, hit boxes and debug.
- **Decision needed:** which component owns the feet offset.

**3. Every map entity needs `MapId(load.id.clone())` by hand.** Medium.
- Where: `long-way`, about 12 sites in 10 files; kit `map_transition.rs:732-754`.
- If a site leaves it out, the object stays after the map unloads, and nothing warns.
- Fix, option A: `MapLoad::tag()`. This is a small change but still manual.
- Fix, option B: the kit spawns a map root entity and map objects become its children. This is safe by construction.
- **Decision needed:** A or B. B changes the hook contract.

**4. Fx emitters and projectiles need `DepthSort` added by hand.** Medium.
- Where: kit `fx.rs:171-226`, `projectile.rs:38-41`; `long-way` fireball, ring and tornado (6 sites), `volcano.rs:166-168`.
- Fix: add `#[require(DepthSort)]` to `Projectile` and the emitter components. Set the path offset in an `on_add` hook.

**5. `SortAtBlocker` without a `Blocker` fails with no message.** Medium.
- Where: kit `depth.rs:49-65`.
- Fix: extend the debug warning to `(With<SortAtBlocker>, Without<Blocker>)`.

**6. Each blocker grows its rectangle by the actor's half width by hand.** Low.
- Where: `long-way` trees, scenery, rocks, markers, volcano, edges; `r-town` trees, collision.
- The rule "alone: bottom band; in a group: whole cell" is written six times across both games.
- Fix: one kit function for the lone or grouped blocker rect.

## Bugs and likely bugs

**7. Projectiles can pass through the corner of a blocked area.** Medium.
- Where: kit `projectile.rs:417-442`, `collision.rs:216-291`, `nav.rs:20-49`.
- `resolve` checks the x move, then the y move. A diagonal step of about 6 px, one frame at 340 u/s, can cut through a box corner. The rule "an edge stops a move from its free side" also has three separate implementations.
- Fix: one `Edge::stops` rule and one segment query in `collision.rs`. Projectiles detect hits with the segment test.

**8. Depth sort runs in `Update` with no order against movement.** Medium.
- Where: kit `depth.rs:36`.
- On some frames z is one frame old. That is enough to cancel the 1 px sort bias, so a character can be drawn on the wrong side for a frame. New entities have z = 0 for their first frame.
- Fix: move to `PostUpdate` in a public `DepthSet`, before `TransformSystems::Propagate`, the same as the camera (`camera.rs:116-119`).

**9. The R-Town player spawns at the cell corner, not the centre.** Medium.
- Where: `r-town` `player.rs:62`, `map.rs:24`, `map.rs:107-113`; kit `map.rs:59-69`.
- A tree or water tile to the left reaches 5 px into the start cell, so the player can start inside a blocker.
- Fix: use the kit `start_feet` and `PLAYER_START`.

**10. A new ring weapon silently gets fire art, and a "built" weapon with no handler takes energy.** Low.
- Where: `long-way` `weapons/ring.rs:69-74`, `weapon_menu.rs:109-114`, `weapons/mod.rs:131-148`.
- Fix: remove the `_ =>` arm. Each weapon plugin registers its kinds, and "built" means that a handler is registered.

**11. `sort_by_feet` marks every `Transform` as changed each frame.** Low.
- Where: kit `depth.rs:121-135`.
- Every static tree and rock has its transform propagated again each frame.
- Fix: write z only when it changes. Filter `sort_at_blocker` by change.

## Code written twice across the games

**12. Collision rebuild after a map load is written three times.** Medium.
- Where: kit `collision.rs:58`, `collision.rs:163`; `long-way` `rocks.rs:182-208`; `r-town` `map.rs:204-208`.
- The kit registers blockers only at `PostStartup`, and its docs still describe that flow.
- Fix: a kit `rebuild_blockers` as the default rebuild hook. Fix the docs.

**13. R-Town has its own map lifecycle; `map_transition.rs` does six jobs in 2,669 lines.** Medium.
- Where: kit `map_transition.rs`; `r-town` `map.rs:164-201`.
- The plugin is all or nothing, so R-Town (one map, no exits) wrote its own reload. That reload has a second spawn list that each new kind must be added to.
- Fix: split into `map/registry`, `map/lifecycle`, `map/transition` and `map/seamless`. Offer a single-map lifecycle that R-Town can use.
- **Decision needed:** this is architectural (plugin boundaries).

**14. Cell-to-world maths in about 15 places; four rules for `player_start`.** Low.
- Where: kit `map.rs`, `map_areas.rs`; both games.
- Fix: world-space methods on `Map` and `Area`.

## Code written twice inside Long Way

**15. The "living enemy body" query and its geometry are copied in every weapon.** Medium.
- Where: `weapons/ring.rs`, `ice_ring.rs`, `tornado.rs`, `fireball.rs`.
- Fix: a `#[derive(QueryData)] EnemyBody` with `living_rect()`. Make the kit's private `closest_point` and away vector in `area.rs` public.

**16. Ring and tornado copy the kit's `Follows` lifetime.** Medium.
- Where: `ring.rs:370-410`, `tornado.rs:286-332`.
- Fix: give `Tornado` and `Ring` a `Transform` and use `fx.following`.

**17. Three hand-written hash functions next to the kit `Rng`.** Medium.
- Where: `fireball.rs:140-148`, `ring.rs:278-287`.
- The fireball's randomness cannot be seeded, so scripted runs differ.

**18. Smaller repeats.** Low.
- The weapon map binding is a hand-kept list of types.
- Pushes are resolved twice.
- "Touched" reports appear three times.
- The player `Facing` copies the kit's `view_of`.
- Ghost lists the strategy components twice.
- The HUD and the weapon menu share easing code.

## Kit structure

**19. Helpers live in modules that do other jobs.** Low.
- Rect helpers are written again in several modules.
- `debug.rs` imports geometry from `map_transition`.
- The cell seed is private in `map_background`, so games write their own.
- Easing is in `area.rs`.
- `map.rs` and `map_edge.rs` depend on each other. `hud.rs` and `screen.rs` depend on each other.
- `animation.rs` has three nearly identical "wait for the asset" systems.
- Fix: a small `geom` module, and move each helper next to its job.

Large file sizes are partly tests. For example, 760 of the 1,219 lines in `collision.rs` are tests.

## Discoverability for Claude

**20. CLAUDE.md names 12 kit areas; `lib.rs` has 35 modules.** Medium.
- Projectiles, map transitions, path finding, spawners, movement, fx and others are missing. CLAUDE.md is the only file that every session reads.
- Fix: replace the list with a pointer: "Before you write game code for movement, draw order, collision, maps, projectiles, particles or HUD, read the symptom index." Add a module table to the `lib.rs` `//!` doc.

**21. `depth.rs` and `collision.rs` have no module doc.** Medium. These are the two modules behind the repeated bug.

**22. Stale docs.** Low. `draw-order.md` was written before `adr-x53` and the edge layers. The extensions README index is missing 4 kit modules.

### Draft symptom index

Proposed for `dev-docs/extensions/README.md`:

| If you see | Use |
|---|---|
| A character is drawn behind an object it stands in front of, or the order changes with x | `SortAtBlocker` + `Blocker` (`depth.rs`, `adr-x53`) |
| Neighbouring objects in a row flicker or swap order | `TieBreakX` |
| A character is not drawn behind a tree it walks behind | `DepthSort` with the real feet offset |
| A sprite is under the ground or above the HUD | `layers.rs` constants, never a bare z |
| A debug drawing covers the HUD | A sprite on `FEATURE_DEBUG_Z` or `MOVEMENT_DEBUG_Z` |
| The map edge hides the character | `EDGE_GROUND_Z` or `EDGE_CANOPY_Z` |
| The player walks through an object, or hits an invisible wall after a map change | `Blocker` + rebuild in `MapHooks.rebuild` |
| The player sticks on corners | `allow_sliding`, `SlideTuning`, `SlideAssist` |
| Gaps or wrong blocked areas between neighbours | `groups.rs` |
| An enemy walks into walls | `Mover` + `NavGrid` |
| An enemy should patrol, chase or guard | `strategies.rs` |
| Contact damage does not fire, or fires too often | `Touchable` + `HitBox`, `ControlledTouched` |
| A shot passes through trees or never leaves the map | `Projectile` + `CameraBounds` |
| A thrown shot sorts wrongly | `ProjectilePath` + `DepthSort` |
| A `ProjectileHit` handler cannot find the projectile | Order it after `ProjectileSet` |
| A pickup is not collected, or is collected twice | `Collectable` + `MapId` |
| Enemies do not appear again | `Spawner`, `SpawnRequest`, `SpawnedBy` |
| Many sprites animate in step | `RandomStartFrame::for_cell` |
| The animation speed is wrong | Frame durations in the `.aseprite` file |
| Pixels are blurred | The kit camera's view height |
| The fade or entry position is wrong at an exit | `map_transition` and `MapHooks` |
| An exit leads nowhere | `map_links` |
| Ground tiles change on reload | `map_background::place_tiles` |
| A HUD click moves the player | `BlocksWorldClick` |
| The HUD overlaps the game | Screen regions (`screen.rs`) |
| The game runs while paused | `AppState` and `PlayState` |
| F1 does nothing | `ReloadPlugin` |
| Scripted runs differ | Kit `rng` with a seed |
| You need a repeatable bug | Script runner with `--debug`, `shot.sh` |
| Particles do not follow their owner | Fx follow |

## Bevy 0.20 practice

Few problems. State cleanup already uses `DespawnOnExit`, plugins are split by feature, and the camera is ordered correctly. The real items are the depth sort schedule (8) and the change detection (11), plus the missing `#[require]` on projectiles and emitters (4). A few `let Ok(..) = q.single() else { return }` could be `Single`. That is a low priority.

## Recommended order

Progress: step 1 done (2026-11-25). Step 2 done (2026-11-25): `DepthSet` in `PostUpdate` with change-only writes; one `collision::segment_hit` for projectiles, `ray_distance` and `nav`; patrol and sentry use `Mover::feet` (who owns the feet offset is still open, step 3); R-Town starts at `start_feet`; Long Way `BuiltWeapons` registry and exhaustive ring art. Step 3 done (2026-11-25), with item 5: `DepthSort` owns the feet (`adr-x57`, `depth::feet_of`); `Projectile` requires `DepthSort` and a `ProjectilePath` sets its start sort in a hook; `Fx` emitters get a `DepthSort`; rocks are `Blocker` + `SortAtBlocker`, pickups `SortAtArea`, and the bias is private; the kit adopts what a `load` hook spawns into a `MapRoot` (`adr-x58`); `grow_sideways` and `group_or_alone` in the kit; a debug warning for `SortAtBlocker` without a `Blocker`. Step 4 done (2026-11-25): one kit `collision::rebuild_blockers` (Long Way and R-Town copies removed); `map_transition` split into `registry`, `lifecycle`, `transition` and `seamless`, with `MapLifecyclePlugin` that R-Town now uses (`adr-x59`). Step 5 done (2026-11-25). "Pushes are resolved twice" (18) is kept on purpose, with a comment on `apply_hits`: the weapons only forecast, and the real move resolves hits one after another. Long Way `EnemyBody` query and kit `area::{closest_point, distance_to_rect, away_from}` (15); rings and tornado emitters use the kit `Follows`, ordered by the new `FxFollowSet` (16); fireball and ring looks use `GameRng`, the rock jitter uses the kit `map_background::cell_dice` (17); one `OnCurrentMap` marker instead of the type list, `log_ended` for the touched reports, the player facing is the kit `view_of`, kit `StrategyComponents`, the weapon menu uses `area::ease_out` (18).

1. **Discoverability (docs only, low risk).** Symptom index in the extensions README, a CLAUDE.md pointer, a `lib.rs` module table, `//!` docs on `depth.rs` and `collision.rs`, fix `draw-order.md`. An ADR for the "kit derives, requires or warns" principle.
2. **Bugs.** Depth sort in `PostUpdate` with change detection (8, 11); projectile corner (7); patrol and sentry feet (2); R-Town start (9); ring art arm (10). Reproduce each with a script first, as the working rules require.
3. **Spawn-site rules.** One feet source (2); rocks and pickups on the kit sort (1); `#[require(DepthSort)]` on projectiles and emitters (4); `MapId` by construction (3); one lone or grouped blocker rect (6).
4. **Cross-game kit.** Collision rebuild in the kit (12); split `map_transition` and give R-Town a single-map lifecycle (13).
5. **Long Way cleanup.** `EnemyBody` query, `Follows`, `Rng`, weapon registry, smaller repeats (15-18).

## Rejected findings

- Weapons bind themselves to the map with a game observer, and the kit should own this. First rejected: R-Town did not use `MapId`. Reconsidered and done on 2026-11-25 (`adr-x60`): since step 4 R-Town uses `MapId` and wrote it by hand at two spawn sites. The kit now owns `OnCurrentMap`, and `SpawnedBy` copies the spawner's map.
- Debug line drawing is written three times. Rejected: only one constant is shared; the drawing code differs.
- `SortAtBlocker` should derive `TieBreakX`. Rejected: the blocker centre is not the footprint middle for grouped or lone scenery.

## Coverage limits

- Only the benchmark was blind. The other reviewers knew about `d0b31fa`, which may have pulled their attention towards draw order.
- Test modules, `nav.rs` path-finding internals, the outline build, `fx_strip`, and parts of `hud.rs` and `weapon_menu.rs` were skimmed, not reviewed in depth.
- The Bevy track did not review weapons, hud, nav, strategies, script or fx in depth.
- Most reviewers used grep, not the LSP, so some line numbers may be a few lines off.
