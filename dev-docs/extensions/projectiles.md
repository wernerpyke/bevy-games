# Projectiles

## Summary

A projectile flies in a straight line over the ground, or along a thrown path with arcs, bounces and a roll, and disappears when it reaches a blocked area or the edge of the map. It collides as a point, with the same outline edges as a character's feet (`collision.md`): a straight projectile at its ground point, a thrown one where it is drawn. The kit moves it, removes it and sends a message. The game spawns it, draws it and chooses its speed.

## Status

`crates/game-kit/src/projectile.rs`, used by R-Town (`adr-rt11`) and Long Way (the fireball, and the volcano's thrown shots). Decisions: `adr-x21`, `adr-x27`, `adr-x54` (thrown path), `adr-x55` (hit message, no targets). Ready to share. R-Town reads `ProjectileHit` to remove enemies.

## Terms

- **Ground point.** The `Transform` x and y of the projectile entity. It is the point that collides, like the feet of a character.
- **Sprite child.** The drawn sprite, a child of the projectile entity, lifted above the ground point. It carries `ProjectileSprite`.
- **Hit.** The projectile reached an outline edge of a blocked area (`HitCause::Blocked`) or the edge of the map (`HitCause::MapEdge`).

## Parameters the game supplies

| Parameter | R-Town |
|---|---|
| Speed | 160 art px/s, twice the walk speed |
| Direction | the way the player faces, 8 directions (`adr-rt11`) |
| Start point | the player's feet |
| Sprite height above the ground, also the height at which a shot hits a body | 8 px (`Projectile::at_height`) |
| Art | `proj_8` in `features/projectiles.aseprite`, 7x6, drawn pointing to the right |
| Collision areas | the ones the `CollisionWorld` holds (the player's, see Limits) |
| Map rectangle | `CameraBounds` |

## Specification

1. `ProjectilePlugin` moves every `Projectile` each frame by `velocity * dt`. It needs `CollisionPlugin`. The map rectangle is the `CameraBounds` resource. Without it a projectile never leaves the map.
2. The move uses `collision::resolve`, which stops a move when it crosses an edge. A long step cannot jump over a thin blocked band. A diagonal move that touches an edge on either axis is a hit.
3. A hit wins over leaving the map. The hit point is on the edge, or on the map boundary for `MapEdge`.
4. On a hit the kit sends `ProjectileHit { projectile, position, cause }` and logs a `debug!` line. The projectile stops moving and is despawned in `PostUpdate`, with its sprite child, so a reader `.after(ProjectileSet)` can still query it (`adr-x55`). A game that despawns a projectile itself uses `try_despawn`.
5. The game spawns the entity with `Projectile::new(direction, speed)`, `DepthSort { feet_offset: 0.0 }`, a `Transform` at the ground point and `Visibility`. It adds the sprite as a child with `ProjectileSprite`. The projectile is in the world layer, so it passes behind trees like a character does (`draw-order.md`).
6. The kit turns the `ProjectileSprite` child to point along the velocity. The art must point along +x. Only the child turns, so its lift above the ground does not change.
7. A `ProjectileTarget` entity with a `HitBox` is hit when the projectile hits targets (not `without_targets()`) and the path of the shot, lifted by `Projectile::height`, enters the box. The path is cut at the point where a blocked area stops the shot, so a target behind a tree is safe. The first target on the path is hit. The hit is `HitCause::Target(entity)` and the game removes the target (`adr-x27`). A path that only touches an edge or a corner of the box (a graze) is not a hit, the same rule as `hitbox::overlaps`.
8. Spawn the projectile at the shooter's feet, which are never inside a blocked area. A spawn point inside a blocked area is not a hit, because only crossing an edge counts: a projectile leaves a blocked area freely.
9. `Projectile::ignoring(rect)`: an edge that would stop the move at a point of `rect` (its border included) does not stop it, and the move is resolved again without it (`step_ignoring`). Use it for the base of what throws the projectile.

## Thrown projectiles

A `ProjectilePath` next to the `Projectile` makes a thrown projectile (`adr-x54`).

1. `PathShape { distance, duration, start_height, peak, bounces, bounce_ratio, roll_share }` is plain Rust. `ground_distance(t)` eases out (quadratic) from 0 to `distance` at `duration`. `height(t)` starts at `start_height`, rises to `peak` and falls to 0, then makes `bounces` arcs, each `bounce_ratio` as high as the one before, all under one gravity chosen so the arcs fill the time before the roll. The last `roll_share` of `duration` is on the ground.
2. `ProjectilePath::new(shape, direction, rest_height)`. Each frame the kit sets `velocity` to the move from `t` to `t + dt` (the ground distance along `direction` plus the change of height up the screen) divided by `dt`. So the entity's point is where the projectile is drawn, and blocked areas, the map edge and hit boxes are tested there. The sprite is a plain child with no offset. Do not add `ProjectileSprite` to art that must not turn.
3. With `DepthSort` on the entity, the kit sets `feet_offset` to `rest_height + height`, so the projectile sorts by the ground under it. `rest_height` is how far the point is above the ground when the height is 0 (half the art height for a ball). The F3 green dot is this sort point.
4. Spawn the entity at the drawn start point with `DepthSort { feet_offset: rest_height + start_height }`. `Projectile::new(direction, 0.0)`: the path sets the speed.
5. After `duration` the kit removes it with `HitCause::PathEnd` at its last point.

## Usage

- Add `game_kit::projectile::ProjectilePlugin` after the collision plugin.
- Spawn as in rule 5. R-Town: `games/r-town/src/weapon.rs`.
- Read `ProjectileHit` with a `MessageReader` for impact effects or damage. To tell kinds of projectile apart, run the reader `.after(ProjectileSet)` and check the game marker on `projectile` (Long Way: `With<Fireball>` in `weapons/fireball.rs`).
- Thrown: Long Way's volcano (`games/long-way/src/volcano.rs`): `ProjectilePath`, `without_targets()`, `ignoring(base)`, and the kit `touch` (`Touchable` and a `HitBox`) to hit the player.
- Scripted test: `hold:x:0.1` fires one shot (`dev-feedback.md`). Set `LOG_FILTER="projectile:|script:"` to see the removal lines.

## Limits

- The edges are the ones built for the shooter's feet, grown sideways by the feet half width. A projectile stops up to that far before the side of a blocked area. A lone tree blocks only its base band, so a shot hits it only where its ground line crosses that band (`adr-x21`).
- No range limit for a straight projectile (a thrown one ends with its path), no cooldown, no hit points, no hit effects. A game that wants a range limit despawns the projectile itself (Long Way does it for the fireball, and steers it by setting `velocity` every frame: `games/long-way/src/weapons/fireball.rs`). R-Town removes projectiles in flight when the map reloads (`adr-rt13`).
- A thrown path is a straight line over the ground. There is no homing, and a bounce does not change the direction. No shadow.

## Tests

`crates/game-kit/src/projectile.rs` has unit tests for `step` (open ground, faces on both axes, diagonal, free paths, all four map sides, start on an edge, hit before map edge), for `target_hit` (each side, a long step, a graze, the nearest of several), for the system (a target behind a blocked area, the map edge, the hit height, a shot without targets, a reader after `ProjectileSet` that still sees the removed shot), for leaving a blocked area through each side, for `step_ignoring`, for `PathShape` (the eased distance; the arc, each bounce and the flat roll for 0, 2 and 3 bounces), for a path shot (its point, its sort, its end) and for the sprite rotation. A step longer than a band is tested on `collision::resolve`. Run `./dev-scripts/rust.sh test game-kit`. Checks in the running game: `./dev-scripts/rust.sh script r-town "teleport:340:882;hold:x:0.1;wait:1;quit"` fires at a lone tree.
