# Projectiles

## Summary

A projectile flies in a straight line over the ground and disappears when it reaches a blocked area or the edge of the map. It collides as a point on the ground, with the same outline edges as a character's feet (`collision.md`). The kit moves it, removes it and sends a message. The game spawns it, draws it and chooses its speed.

## Status

`crates/game-kit/src/projectile.rs`, used by R-Town (`adr-rt11`). Decision: `adr-x21`. Ready to share. R-Town reads `ProjectileHit` to remove enemies.

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
4. On a hit the kit sends `ProjectileHit { position, cause }`, logs a `debug!` line and despawns the entity. The sprite child goes with it.
5. The game spawns the entity with `Projectile::new(direction, speed)`, `DepthSort { feet_offset: 0.0 }`, a `Transform` at the ground point and `Visibility`. It adds the sprite as a child with `ProjectileSprite`. The projectile is in the world layer, so it passes behind trees like a character does (`draw-order.md`).
6. The kit turns the `ProjectileSprite` child to point along the velocity. The art must point along +x. Only the child turns, so its lift above the ground does not change.
7. A `ProjectileTarget` entity with a `HitBox` is hit when the path of the shot, lifted by `Projectile::height`, enters the box. The path is cut at the point where a blocked area stops the shot, so a target behind a tree is safe. The first target on the path is hit. The hit is `HitCause::Target(entity)` and the game removes the target (`adr-x27`). A path that only touches an edge or a corner of the box (a graze) is not a hit, the same rule as `hitbox::overlaps`.
8. Spawn the projectile at the shooter's feet, which are never inside a blocked area. A spawn point inside a blocked area is not a hit, because only crossing an edge counts.

## Usage

- Add `game_kit::projectile::ProjectilePlugin` after the collision plugin.
- Spawn as in rule 5. R-Town: `games/r-town/src/weapon.rs`.
- Read `ProjectileHit` with a `MessageReader` for impact effects or damage.
- Scripted test: `hold:x:0.1` fires one shot (`dev-feedback.md`). Set `LOG_FILTER="projectile:|script:"` to see the removal lines.

## Limits

- The edges are the ones built for the shooter's feet, grown sideways by the feet half width. A projectile stops up to that far before the side of a blocked area. A lone tree blocks only its base band, so a shot hits it only where its ground line crosses that band (`adr-x21`).
- No range limit, no cooldown, no hit points, no hit effects. A game that wants a range limit despawns the projectile itself (Long Way does it for the fireball, `PLAN-weapons.md`). R-Town removes projectiles in flight when the map reloads (`adr-rt13`).
- Straight flight only. There is no gravity, homing or bounce.

## Tests

`crates/game-kit/src/projectile.rs` has unit tests for `step` (open ground, faces on both axes, diagonal, free paths, all four map sides, start on an edge, hit before map edge), for `target_hit` (each side, a long step, a graze, the nearest of several), for the system (a target behind a blocked area, the map edge, the hit height) and for the sprite rotation. A step longer than a band is tested on `collision::resolve`. Run `./dev-scripts/rust.sh test game-kit`. Checks in the running game: `./dev-scripts/rust.sh script r-town "teleport:340:882;hold:x:0.1;wait:1;quit"` fires at a lone tree.
