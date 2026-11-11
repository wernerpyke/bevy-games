# Area effects

## Summary

- Plain geometry for effects that cover an area: a ring that grows, a band between two circles, and a push away from a centre.
- All rules are functions on `Vec2` and `Rect`, so they are unit tested without a running app.
- A temporary blocked area (a wall made by a spell) is in the collision world (see [collision.md](collision.md), "Temporary blocked areas").

## Status

| Part | Code today | Shared? |
|---|---|---|
| `ease_out`, `ring_radius`, `circle_hits_rect`, `rect_overlaps_band`, `push_delta` | `crates/game-kit/src/area.rs` | Shared. No game values. |
| Temporary blocked area | `crates/game-kit/src/collision.rs` | Shared. |
| Weapons that use it | `games/long-way/src/weapons/` (planned, `PLAN-weapons.md`) | No. Game specific. |

## Terms

- **Ring:** a circle around a centre whose radius grows with time. A body is hit when its rectangle touches the ring band.
- **Band:** the area between an inner and an outer radius around a centre.
- **Body rectangle:** the `HitBox` of a body in world coordinates (`hitbox.rs`).

## Parameters the game supplies

The centre, the radii, the grow time, the speed of a push, and the body rectangles. The kit holds no values.

## Specification

1. `ease_out(u)` is 0 at 0 and 1 at 1, fast first. The input is clamped.
2. `ring_radius(age, grow_secs, max)` is `max * ease_out(age / grow_secs)`. A negative age gives 0. After `grow_secs` it stays at `max`. A `grow_secs` of 0 or less means no growth: the radius is `max` from age 0.
3. `circle_hits_rect` is true when the circle touches or overlaps the rectangle. The nearest point of the rectangle is the test, so a corner counts as a distance.
4. `rect_overlaps_band(centre, inner, outer, rect)` is true when the nearest point of the rectangle is within `outer` and its farthest corner is at least `inner` away. It is exact for any rectangle, large or small.
5. A moving ring hit test uses a band that reaches back in time: the inner radius is the radius `trail` seconds ago. Consecutive frames must overlap (`ring_radius(now - trail) <= ring_radius(previous frame)`), or a body can be skipped between two frames. The test in `area.rs` checks this down to 20 frames per second.
6. `push_delta(centre, radius, rect, speed, dt)` is the move of one frame straight away from the centre, at `speed`, while any part of the body is inside `radius`. A body exactly on the centre is pushed along +X. The game moves the body with `collision::move_feet`, so blocked areas stop the push.

## Usage

```rust
let radius = area::ring_radius(age, GROW, MAX);
let inner = area::ring_radius(age - TRAIL, GROW, MAX);
if area::rect_overlaps_band(centre, inner, radius, hitbox.rect_at(ground)) { /* hit */ }
```

## Limits

- Circles and axis-aligned rectangles only.
- No line of sight: a body behind a tree is hit like any other. A game that wants shielding tests it itself against the collision edges.
