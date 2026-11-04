# Path finding

## Summary

A grid of walk points covers the map. Two neighbouring points are linked when a straight move between them crosses no collision outline edge from its free side. A* searches the links, and the path is smoothed to the fewest straight legs that cross no edge. The grid is built from the collision edges, so it follows the same blocked areas as the player, thin tree bands included.

## Status

`crates/game-kit/src/nav.rs`, used by R-Town (`adr-rt12`). Decision: `adr-x23`. Ready to share.

## Terms

- **Grid point.** A point every `step` units over the map rectangle. `step` is a parameter of `NavPlugin`. R-Town uses 4.
- **Link.** A move from a point to one of its 8 neighbours that is allowed. Links go one way: an edge only stops a move from its free side, so a start inside a blocked area can walk out, and a goal inside one has no way in.
- **Leg.** One straight piece of a path.

## Parameters the game supplies

| Parameter | R-Town |
|---|---|
| Grid step | 4 px (a 60x60 map of 16 px cells has 58,081 points) |
| Blocked areas | the ones in `CollisionWorld` (the player's, `adr-rt5`, `adr-rt9`) |
| Map rectangle | `CameraBounds` |

## Specification

1. `NavPlugin { step }` builds `NavGrid` from `CollisionWorld::edges()` and `CameraBounds`. It builds it again when either changes, for example after a map reload. Systems that read it run `.after(NavSet)`.
2. A straight link of the 4 main directions is allowed when `segment_blocked` is false. A diagonal link needs both ways around its corner to be open, so a path never cuts a corner.
3. `segment_blocked(a, b, edges)` is the rule of `collision::resolve` for a straight line: an edge stops a segment that crosses it from the free side, and its ends at convex corners are not part of it.
4. `find_path(from, to)` returns the points to walk to, without `from`. A clear straight line gives `[to]`. No path gives `None` (a goal inside a blocked area, an enclosed goal, or an empty grid). The start and goal use the up to four grid points around them that a straight move reaches.
5. Costs are 10 for a straight link and 14 for a diagonal one. The search uses the `pathfinding` crate (`astar`).
6. A search for an unreachable goal explores the whole grid. Callers must not do it every frame (`enemy-movement.md` has a budget).

## Usage

- Add `NavPlugin` after `CollisionPlugin`. Use `NavGrid` as a resource.
- `find_path` needs no ECS state, so a strategy can ask whether a place is reachable.

## Limits

- The grid does not follow moving obstacles. It is built again only when the collision areas change.
- A line that touches a convex corner of a blocked area exactly passes it. This matches walking (`collision.md`). Real paths do not meet it, because the grid points and the corners do not line up.
- The grid is as fine as `step`. A gap narrower than the step may be missed.
- Roadmap item 3 covers a larger design (many maps, dynamic obstacles).

## Tests

`crates/game-kit/src/nav.rs`: open ground, a thin band, a grove, a 6 unit gap, a goal inside a blocked area, a start inside one, an enclosed goal, and 300 random pairs whose paths cross no edge. Measured for R-Town's `test.txt`: the grid builds in about 9 ms. On a random town sized map with 536 edges, a long path takes about 1.5 ms. Run `./dev-scripts/rust.sh test game-kit`.
