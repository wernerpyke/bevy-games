# Enemy movement: goals, strategies and hit boxes

## Summary

An entity with a `Mover` walks to its `MoveGoal`. A strategy is a component and a system that only chooses the goal. The kit has `RandomPatrol`, `Pursue` and `Sentry`. A game adds a strategy of its own the same way, and it needs no change in the kit. `HitBox` is the body rectangle used for touching and for projectile hits.

## Status

`crates/game-kit/src/movement.rs`, `strategies.rs`, `touch.rs`, `hitbox.rs`, `rng.rs`, `movement_debug.rs`. Used by R-Town (`adr-rt12`). Decisions: `adr-x23` (path finding), `adr-x24` (strategies), `adr-x26` (random numbers), `adr-x27` (hit boxes), `adr-x40` (sentry), `adr-x41` (touch, facing tags).

## Terms

- **Goal.** `MoveGoal(Option<Vec2>)`: the point to walk to. `None` means stand still.
- **Strategy.** A component and a system in `StrategySet` that writes the goal.
- **Mover.** The component that makes an entity follow its goal: speed, edge margin, arrival distance.
- **Feet.** The `Transform` x and y of the entity. It is the point that collides (`DepthSort { feet_offset: 0.0 }`).

## Parameters the game supplies

| Parameter | R-Town |
|---|---|
| Speed | skeleton 0.5 x and goblin 0.2 x the player's walk speed (`enemy.rs`) |
| Edge margin | 5 px, the player's feet half width |
| Strategy per spawner | the legend parameter `strategy=` (`spawners.md`) |
| Patrol radius | the legend parameter `radius=` (120 px for the skeleton) |
| Hit box | skeleton 13x9, goblin 13x8 (the sprite without its bottom 2 px), player 8x12 |

## Specification

1. `MovementPlugin` runs `follow_goals` in `FollowSet`, after `StrategySet`, after `NavSet`. `Mover` adds `MoveGoal`, `MoveState`, `PathFollower` and `SlideAssist` as required components.
2. The follower searches a path with `NavGrid::find_path` when it has a new goal, when the goal moved more than 4 units and 0.3 s have passed, or when it has made no progress for 0.5 s. At most `searches_per_frame` (4) searches run in a frame. An entity that has to wait keeps its old path.
3. It walks the path with `collision::move_feet`, so the sliding rules of the player apply, and keeps the feet inside the map rectangle by `edge_margin`.
4. `MoveState` reports `walking`, `facing` (the last direction as a unit vector), `arrived` (within `arrive_distance` of the goal) and `unreachable` (the last search found no path).
5. `RandomPatrol { radius }` picks a random point within `radius` of the current place, keeps it only when it is on the map and `find_path` finds a way, and picks again on arrival. It tries 8 times and then waits 1 s. It never needs a "no progress" rule, because the path is checked first.
6. `Pursue { target }` sets the goal to the position of the `Controlled` entity, or of `target`. Without a target entity the goal is `None`.
6a. `Sentry { sight }` patrols like `RandomPatrol`, but within `radius` of the place where it started (its home), until `can_see` is true for the `Controlled` entity: within `sight` and no blocked edge on the straight line between the feet. Then it pursues for good (`Sentry::alerted()`). Legend: `strategy=sentry radius=<px> sight=<px>`. Everything in `CollisionWorld` blocks sight. `adr-x40`.
7. A game strategy is a component and a system `.in_set(StrategySet)` that sets `MoveGoal`. Evade, for example, would choose a point away from the player. The headless test `a_custom_strategy_can_drive_the_mover` shows the pattern.
7a. `Touchable` plus a `HitBox` on an enemy makes `TouchPlugin` send `ControlledTouched { by }` in every frame in which its box overlaps the box of the `Controlled` entity. Read it `.after(TouchSet)`. Remove `Touchable` for one hit. The player needs a `HitBox` too, relative to its `Transform`. `adr-x41`.
7b. `FacingTags { down, side, up, speed }` on the sprite child of an entity with `MoveState` plays `Down`, `Left` or `Up` for the way it faces and flips the sprite for the right (`view_of`). `adr-x41`.
8. `HitBox { offset, half_size }` is relative to the ground point. `HitBox::rect_at(ground)` gives the world rectangle and `overlaps(a, b)` tests two rectangles (touching edges do not overlap).
9. `Rng` and `GameRng` (`rng.rs`) give seedable random numbers: the environment variable `<PREFIX>_SEED` sets the seed for a repeatable run.
10. `MovementDebugPlugin` shows paths, goals and spawners with the F3 grid switch (`debug-tools.md`).

## Usage

- Add `NavPlugin`, `MovementPlugin`, `StrategiesPlugin` (needs `CollisionPlugin`) and `RngPlugin`. Give an entity `Mover::new(speed)`, a strategy (`Strategy::from_params` reads `strategy=` from a legend entry, `.insert(&mut entity)` adds it), `WalkAnimation`-style data of the game and a `DepthSort`.
- Read `MoveState` for animation. R-Town copies it into `WalkAnimation` (`src/enemy.rs`).

## Limits

- Strategies only choose a goal. A movement that is not a walk (a jump, a teleport) needs its own mover.
- Enemies do not collide with each other. They may overlap (`adr-rt12`).
- A pursuer (`Pursue`) always knows where the player is. Only `Sentry` has a sight rule, and it never gives up the chase.
- A hit box is a rectangle in the ground plane, shown from the side: the sprite body is above the feet. Projectiles test it at their flight height (`projectiles.md`).

## Tests

Pure parts and headless Bevy apps in `movement.rs` (a walk around a band, speed, an unreachable goal, a custom strategy), `strategies.rs` (parameters, picks, a patrol that keeps choosing, a pursuer that follows and stops) and `hitbox.rs`. Run `./dev-scripts/rust.sh test game-kit`.
