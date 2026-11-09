# Extensions: collision and movement decisions

Collision, sliding, blocked groups, projectiles, hit boxes and path finding in `crates/game-kit`. Index of all extension decisions: `EXTENSIONS.md`.

## adr-x4. Forgiving collision: `allow_sliding` (2026-10-31)

**Decision.** `CollisionPlugin` takes an `allow_sliding` flag. Long Way sets it to `true`. When true:
1. The player slides along a trunk when walking diagonally into it.
2. Walking straight into a face stops the player, which shows the collision.
3. If the player keeps pushing for 0.2 seconds (first 0.5, shortened on 2026-10-31 after the developer tried it) and its feet are within 35% of the face length from an edge (capped at 64 pixels, see `adr-x10`, which also covers faces of merged outlines and notches), it is slid sideways at 120 units per second to that edge, then walks on past the trunk. This works from below (slide left or right), from above, and into the trunk sides (slide up or down).
Pushing into the middle of a face never slides the player, so it must walk around.

When false, any blocked move leaves the player where it was. There is no sliding of any kind.

**Rationale.** Hitting a trunk by a few pixels should not make the player back off and line up again. The pause makes the collision visible before the game helps. Pushing into the middle is a clear signal that the player means to reach the trunk, so no help is given there.

**How to apply.** `SLIDE_DELAY`, `SLIDE_SPEED` and `SLIDE_RANGE_FRACTION` in `collision.rs` set the numbers. Entities that move with `move_feet` need a `SlideAssist` component, and call `reset()` on it when they are not trying to move. The setting is a resource (`CollisionSettings`) set by the plugin. Assisted sliding only starts when the input is straight along one axis. The collision rule from `adr-lw1` is unchanged. The push direction is stored as signs (for example (0, 1)), not as a normalised float, because comparing floats made the timer reset every few frames (`adr-x5`).

## adr-x10. Sliding along merged outlines, into notches (2026-11-01)

**Decision.** Blocked rectangles that touch or overlap are merged into one outline per cluster (`outline.rs`). The player collides with the outline edges only, and `allow_sliding` (`adr-x4`) works on those edges. The slide rules for an outline are:
1. The range is the smaller of 35% of the face and 64 pixels (`SLIDE_RANGE_CAP`), measured from the nearer end.
2. An end at an inward (concave) corner never slides the player. Past it there is only solid ground.
3. At a convex end, the player always slides. If the way on is open for 192 pixels (`SLIDE_CLEARANCE`) it stops exactly on the end. If a face blocks it (a notch or an overhang), it stops 0.5 pixels past the end (`SLIDE_PAST_CORNER`) and walks on until that face stops it. Example: pushing up into the bottom face of the test cluster from world x 463, the player slides to x 440.3 and walks up to y 64.
4. All blocked areas (trunks and rocks) are merged by one `outline::build`, so a trunk touching a rock is one cluster. Two rectangles that touch only at a corner leave no gap: that vertex counts as an inward corner. A slide target is passed through `resolve`, so it never ends inside another shape.

The blocked area and sort line of a rock are game rules, in `adr-lw13` (`games/long-way/docs/adr.md`).

**Rationale.** The first version refused to slide when the way past the end was blocked within 192 pixels, to avoid pushing the player into pockets. The developer tried it at a notch under an overhang and wanted the player to slide to the corner and walk on, stopping at the back of the notch (2026-11-01). Sliding into an inward corner is still refused, because it only puts the player inside the next rock.

**How to apply.** The rules are in `slide_to_nearest_edge` in `collision.rs`, with tests for pockets, notches and long faces. The numbers were accepted by the developer after play testing (2026-11-02). The full rules are in `dev-docs/extensions/collision.md`, the Long Way values in `games/long-way/docs/collision.md`.

## adr-x16. Collision blockers are a kit component, slide values scale with the cell size (2026-11-02)

**Decision.** The kit has a `Blocker(Rect)` component. A game grows each blocked area by the actor's size and spawns it as a `Blocker`, or calls `CollisionWorld::add_blocked`. The slide values are `SlideTuning` fields, and `CollisionPlugin { allow_sliding, cell_size }` computes them with `SlideTuning::for_cell_size`, which scales speed, range cap, clearance and past-corner from the values tuned at a cell size of 64.

**Rationale.** The kit does not need to know a game's obstacle types (Long Way `Trunk`, R-Town houses). The actor size is already applied when a game builds the area, so `resolve` and `move_feet` need no size parameter. Long Way at cell size 64 keeps its exact values. R-Town at 16 gets a quarter, so the slide feels the same relative to the art.

**How to apply.** Keep game rules (which part of a tree blocks, actor size) in the game. Keep the kit free of game types.

Refinement (2026-11-02): `parse` takes the list of kind names the game knows and reports an unknown legend name itself, with the file line, as before. The game's conversion from name to enum then cannot fail. This keeps the parser error messages and their tests in the kit. The `draw` key is optional (`Option<f32>`) and the game gives the default.

## adr-x20. Neighbour groups in the kit (2026-11-03)

**Decision.** `crates/game-kit/src/groups.rs` finds groups of touching items on a cell grid (`neighbour_groups`, `group_sizes`, 8 neighbours or 4). A game uses the group size to choose the blocked area of an item: a thin one when it stands alone, a full one in a group. The function was moved into the kit at once, although the rule "move code to the kit when two games use it" (`adr-x12`) was not met yet, because R-Town and Long Way adopted it together.

**Rationale.** The grouping is a plain grid algorithm with no game types, so the kit can unit test it alone. Two games with the same logic would copy it. The game keeps the values: footprint sizes, bands and kinds.

**How to apply.** See `dev-docs/extensions/groups.md`.

## adr-x21. Projectiles are in the kit and collide as a ground point (2026-11-04)

**Decision.** `crates/game-kit/src/projectile.rs` has the projectile component, the plugin that moves it, the hit message and the turning of the sprite. It was written in the kit from the start, although the rule "move code to the kit when two games use it" (`adr-x12`) was not met yet, because it holds no game types and no assets and the developer asked for it. A projectile is a point on the ground that uses the same outline edges as a character's feet (`collision::resolve`). It does not have its own, exact blocked shapes.

**Rationale.** Weapons are a general need for the games in this workspace, and the movement and removal rules are plain geometry that the kit can unit test alone. Reusing the edges needs no second set of areas. The cost is that the areas are grown sideways by the shooter's feet half width, so a shot stops up to 5 px early at the side of a tree, and a lone tree stops a shot only at its base band. This matches how the player meets trees. Exact shapes can be added if play testing shows a problem.

**How to apply.** See `dev-docs/extensions/projectiles.md`.

## adr-x23. Path finding on a grid built from the collision edges (2026-11-04)

**Decision.** `crates/game-kit/src/nav.rs` builds a grid of walk points (R-Town: every 4 px) and links two neighbours when a straight move crosses no outline edge from its free side. A* from the `pathfinding` crate (4.16.0, compiles for `wasm32-unknown-unknown`) searches it, and the path is smoothed to straight legs. The grid is built again when the collision areas or the map rectangle change.

**Rationale.** The grid reuses the exact outlines, thin tree bands included, so there is no second description of the obstacles and paths follow the rule the walker follows. It is easy to test without a running world. A navigation mesh crate (`bevy_northstar` and others in `adr-e4`) is heavier and tied to a Bevy version. The cost is memory and a build time of about 9 ms for a 60x60 map, once per map load.

**How to apply.** See `dev-docs/extensions/nav.md`. It is the first part of `rm3`.

## adr-x27. Hit boxes, and projectiles hit at their flight height (2026-11-04)

**Decision.** `HitBox { offset, half_size }` (`hitbox.rs`) is a body rectangle relative to the ground point, and it is not the feet box. A projectile hits an entity with `ProjectileTarget` and a `HitBox`, tested along its path (not only at its end point) and at its flight height (`Projectile::height`), and it stops at a blocked area first. The hit is `ProjectileHit` with `HitCause::Target(entity)`, and the game removes the target.

**Rationale.** A body sits above the feet. A shot that flies at hand height along the ground line of the shooter hits a body that stands on the same line. Testing the ground line would miss it, as the first scripted run showed. Testing the path stops a long step from jumping over a thin target.

**How to apply.** See `dev-docs/extensions/projectiles.md` and `enemy-movement.md`.

## adr-x29. A corner contact between blocked areas is closed with a filler (2026-11-06)

**Decision.** `outline::build` adds a filler rectangle (0.5 wide, 1.0 high, `FILLER_HALF`) at every vertex where two blocked areas touch only at a corner and the other two quadrants are empty. This replaces the rule that the four edge ends at such a vertex count as concave (`adr-x4`, rule 5 of `extensions/collision.md`).

**Rationale.** With the old rule the feet could stop exactly on the vertex, where all four edges cover them and each blocks one direction, so the player could not move at all (Long Way, one-cell tree boxes on the grid). Letting the feet leave along the free boundary instead would let them squeeze through the gap in two steps. A filler makes one solid shape: the feet stop just short of the vertex and can only back away. The filler is taller than wide so that a 45 degree line (a nav diagonal) through the vertex enters it through the middle of an edge, not exactly through its corners, where the edge test passes it. R-Town widens its blocked cells sideways, so its areas overlap at corners and the filler never applies there (same clusters and edges on `test.txt`).

**How to apply.** Nothing in a game. `resolve`, `move_feet`, projectiles and nav see ordinary edges. Tests: `outline.rs`, `collision.rs`, `projectile.rs` and `nav.rs` (names with "corner contact"). A line of another exact slope through a filler corner can still pass, the same limit as at any convex corner.

