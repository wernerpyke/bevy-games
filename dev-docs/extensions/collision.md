# Collision

Feet-point collision against merged outlines, with forgiving sliding. Top-down games where a character walks behind tall objects and around clusters of obstacles.

## Summary

- Only the feet of a character (one point) are tested. Obstacles become rectangles the feet may not enter.
- Rectangles that touch or overlap merge into clusters. Each cluster has one outline of straight edges.
- `resolve` moves the point one axis at a time and stops it at the first edge it crosses.
- `move_feet` adds the optional forgiving behaviour (`allow_sliding`): after a short push, the character is slid past a nearby edge.
- All rules are plain functions that use only `Vec2` and `Rect`, so they are unit tested without a running app.

## Status

| Part | Code today | Shared? |
|---|---|---|
| Outline: `build`, `Edge`, `Cluster` | `crates/game-kit/src/outline.rs` | Standalone, in the shared kit. |
| `resolve`, `move_feet`, sliding, `CollisionPlugin`, `CollisionWorld`, `SlideAssist`, `Blocker`, `SlideTuning` | `crates/game-kit/src/collision.rs` | Shared. The actor size is not in the code: the game grows each blocked area before it adds it. |
| `Trunk`, rock blocked areas | `games/long-way/src/collision.rs`, `rocks.rs` | No. Game specific (see the Long Way collision doc). |

The shared code is in the `game-kit` crate (adr-x12). A game adds `Blocker(Rect)` components or calls `CollisionWorld::add_blocked`.

Decisions: `adr-x4`, `adr-x10`. Long Way rules: `adr-lw1`, `adr-lw13`. Background and alternatives: [collision-research.md](collision-research.md).

## Terms

- **Feet:** the point (x, y) used for all collision. The game defines where it is on the sprite.
- **Blocked area:** a rectangle the feet may not enter.
- **Cluster:** blocked areas that touch or overlap, merged into one shape.
- **Outline:** the straight edges around a cluster. An **edge** is also called a **face**.
- **Blocked side:** the side of an edge that is solid.
- **Convex end:** an end of a face where the shape turns away (an outside corner).
- **Concave end:** an end of a face where the shape turns inward (an inside corner).

## Parameters the game supplies

| Parameter | Meaning | Long Way |
|---|---|---|
| Actor half width | The game grows each blocked area sideways by this, so the body does not overlap it | 20 |
| Obstacle rules | How a game object becomes a blocked area (`Blocker`) | Trunk and rock rules |
| `cell_size` | `CollisionPlugin` field. The slide values below scale with it (`SlideTuning::for_cell_size`) | 64 |
| `SlideTuning::delay` | Seconds of pushing before help starts | 0.2 |
| `speed` | Slide speed in units per second | 120 |
| `range_fraction` | Share of the face length, from the nearer end, where help is given | 0.35 |
| `range_cap` | Upper limit of that range in units | 64 |
| `clearance` | Free distance beyond an end that lets the slide stop on the end | 192 |
| `past_corner` | How far past an end a slide stops when the way on is blocked | 0.5 |
| `TOLERANCE` (in `outline.rs`) | Distance within which a position counts as being on an edge | 0.01 |

Values are for a cell size of 64. `SlideTuning::for_cell_size` scales speed, range cap, clearance and past-corner with the game cell size (R-Town: 16).

## Specification

### A. Outline

1. All blocked areas go through one `outline::build` inside `CollisionWorld::add_blocked`. It merges overlapping rectangles into clusters with coordinate compression. Areas added in separate calls still merge.
2. Each cluster has edges. An edge knows its side (`blocked_positive`) and whether each end is concave.
3. A cluster member is interior when no cell it covers has an empty 4-neighbour. The flag is information only. Interior members stay in the input of `build`, because one can fill a hole. Collision uses edges only, so interior members cost nothing per frame.
4. A convex end is excluded from the face (`covers`), so feet on a corner line can walk along the outside. A concave end is included, because past it the feet would be inside the shape.
5. A vertex where two rectangles touch only at a corner counts as concave for all four edge ends there, so there is no zero width gap to walk through.
6. Positions within `TOLERANCE` of an end count as at the end.

### B. Moving

7. `resolve(old, new, edges)` moves x first, then y. Each axis stops at the first edge it crosses from the free side. A large step cannot jump over a shape, because the test is a crossing and not a position check.
8. A start within `TOLERANCE` on the blocked side counts as free. This absorbs rounding noise after stopping on an edge.
9. With `allow_sliding` off, any blocked move leaves the character where it was.
10. With it on, a diagonal move into an obstacle slides along it, because each axis is resolved alone.

### C. Assisted sliding (`allow_sliding`)

11. Help starts only when the input is straight along one axis. Pushing is tracked as the signs of the step, not as floats. The timer resets when the direction changes or when the character does not push.
12. The character must push for `delay` before any help. The pause makes the collision visible first.
13. The face is the edge that blocked the pushed axis. The range is the smaller of `range_fraction` of the face and `range_cap`, measured from the nearer end. If the nearer end is outside the range there is no slide. The middle of a long face never slides.
14. A concave nearer end refuses the slide.
15. At a convex end, the character slides along the face at `speed`. The target is the end itself if the way beyond is open for `clearance`. If the way is blocked, the target is `past_corner` beyond the end, so the character is off the face and walks on until the next face stops it.
16. The slide target goes through `resolve`, so a slide never ends inside another shape.
17. After the slide, normal movement continues. The next stop may start a new slide.

## Usage

```rust
app.add_plugins(CollisionPlugin { allow_sliding: true });
```

- Register blocked areas with `CollisionWorld::add_blocked(&[Rect])` after the obstacles exist (Long Way does it in `Startup` and `PostStartup`).
- Give every entity that moves with `move_feet` a `SlideAssist` component.
- Move with `move_feet(&settings, &mut assist, old_feet, step, dt, world.edges())`.
- Call `assist.reset()` on frames where the entity does not try to move.
- Read the setting from `Res<CollisionSettings>`. See [plugin-configuration.md](plugin-configuration.md).

## Groups

A game may give an item a thin blocked area when it stands alone and a full footprint when it touches other items. The merge into one outline is the one described here. How groups are found is in `groups.md`.

## Pitfalls

- Never compare normalised float vectors for equality between frames. `Vec2::normalize` can return 0.99999994 or 1.0 for the same direction. Compare signs. Float comparison made the slide timer reset every few frames (`adr-x5`).
- Positions stored in one form and recomputed in another drift by millionths: feet at `translation.y - 30` can come back as 76.800003 instead of 76.8. Never test exact equality against an edge or corner coordinate. Use `TOLERANCE`. `collision::crosses` and `Edge::covers` already apply it.

## Limits

- The outline is built once per call to `add_blocked`. Doors or destroyed objects need a rebuild of one cluster (roadmap item 4).
- `outline::build` makes one grid over all rectangles, so large maps need chunking (roadmap item 4).
- Every moving entity tests every edge. A spatial hash is the later fix.
- A character placed inside a shape (for example by teleport) can walk freely inside it.
- A game that clamps the character to the view after collision can move the feet into a shape.
- Sliding works from all four sides of an obstacle, but starts only on straight input.

## Tests

Inline in `game-kit` `collision.rs` and `outline.rs`. Long Way keeps the trunk rule tests in its own `collision.rs`. Collision: stops, crossing, slide range, pockets, notches, long faces, float noise. Outline: shapes, concave flags, interior (including a random brute force check).
