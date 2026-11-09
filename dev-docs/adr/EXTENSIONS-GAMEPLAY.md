# Extensions: gameplay decisions

Enemy strategies, spawners, random numbers, touch and collectables. Index of all extension decisions: `EXTENSIONS.md`.

## adr-x24. Strategies are components that write a goal (2026-11-04)

**Decision.** A movement strategy is a component with a system in `StrategySet` that writes `MoveGoal`. One kit system (`movement.rs`) turns the goal into a path and steps. The kit provides `RandomPatrol` and `Pursue`. A game adds its own strategy the same way.

**Rationale.** It is the normal ECS style and needs no trait objects or boxed state. A strategy stays small and its pure parts are unit tested. The kit does not need to know the strategies of a game. A strategy can only say "go there": a special movement needs its own mover.

**How to apply.** See `dev-docs/extensions/enemy-movement.md`.

## adr-x25. Spawner settings are legend parameters (2026-11-04)

**Decision.** A legend entry of a map file may have `key=value` parameters after its kind, for example `"S": spawn_skeleton every=10 strategy=patrol radius=120`. The parser keeps them in `MapObject.params` and the line number in `Params::line`. The game decides what they mean and reports errors with the file and the line. `Spawner` (`spawner.rs`) reads `every` and `max`.

**Rationale.** The grid keeps one character per cell, and each legend character is one configured spawner. Different intervals use different characters. The change is small and backward compatible. It is a first step into `rm4` (objects with settings).

**How to apply.** See `dev-docs/extensions/spawners.md` and `map-files.md`.

## adr-x26. A seedable random number generator in the kit (2026-11-04)

**Decision.** `crates/game-kit/src/rng.rs` has `Rng` (SplitMix64) and `RngPlugin`, which adds `GameRng`. The seed is `<PREFIX>_SEED` when set, else the clock. The web build has no clock at start, so it starts from a constant and mixes in the time of the first frame.

**Rationale.** The `rand` crate needs `getrandom` and extra setup for the web build. About 60 lines with no dependency are enough. A seed that we control makes scripted runs repeatable.

**How to apply.** Take `ResMut<GameRng>` and call `f32`, `range` or `point_in_disc`. Set `R_TOWN_SEED=7` for a repeatable run.

## adr-x38. The kit has a touch trigger for collectable items, and the game keeps the inventory (2026-11-08)

**Decision.** The kit owns `Collectable { area, kind }` (with a `MapId`), the `Collected { kind }` message and `CollectablePlugin`. When the feet of the `Controlled` entity touch the area of an item of the current map, the plugin sends `Collected` and despawns the item. The game owns the art, the item kinds, the touch area size and the player state (the inventory). The kit has no inventory type.

**Rationale.** The touch test is the same for every game that has pickups, and it follows `ExitTrigger`. The shape of player state is not known yet: it will probably grow past counts (health, keys), so a kit inventory would be built on a guess. Kinds are strings from the legend, so the kit has no game types (`adr-x13`). Placed in the kit at the developer's request, before a second game needs it (`adr-x12`).

**How to apply.** `dev-docs/extensions/collectables.md`. Long Way: `adr-lw27`.

## adr-x40. `Sentry` is one strategy that patrols until it sees the player, then pursues for good (2026-11-09)

**Decision.** The kit has `strategies::Sentry { sight, .. }` and `can_see(from, to, range, edges)`. A sentry patrols like `RandomPatrol`, but within `radius` of its home (the feet in its first frame, so it keeps to a small area and does not drift), until the `Controlled` entity is within `sight` and no blocked edge of `CollisionWorld` crosses the straight line between the feet (`nav::segment_blocked`). It then walks to that entity and never goes back to patrolling. The legend parameters are `strategy=sentry radius=<px> sight=<px>`. Everything that blocks walking blocks sight.

**Rationale.** Long Way's ghost needs a sight rule and R-Town's `Pursue` has none (`enemy-movement.md`). A second game wants it, so it belongs in the kit (`adr-x12`). One composite component keeps the one-way switch in one place. The alternative is a sight system that adds and removes `RandomPatrol` and `Pursue`: it spreads one behaviour over several components and systems, and the state would live in which components exist. Patrol logic is shared with `RandomPatrol` through one function, not copied. Giving up the chase is not a rule yet: the developer wants the chase to last.

**Also in the kit (same day).** `strategies::Strategy` (`Patrol`, `Pursue`, `Sentry`) with `from_params` (reads `strategy=` and the parameters of the strategy it names, errors without the file name) and `insert` (adds the component). Both games parse the same legend entries, so R-Town's own copy was removed. The kit does not take enemy markers out of the map object list: a game that keeps all objects in `map.objects` (Long Way) picks its markers by kind, as it does for rocks and pickups.

**How to apply.** `dev-docs/extensions/enemy-movement.md`. Add `StrategiesPlugin` (it needs `CollisionPlugin`), give the entity `Mover` and a `Strategy` from `Strategy::from_params(&object.params)` (or `Sentry::new(radius, sight)`). Read `Sentry::alerted()` for a cue.

## adr-x41. The kit reports touches on the controlled entity and plays facing tags (2026-11-09)

**Decision.** `game_kit::touch` has `Touchable` (on an entity with a `HitBox`), the message `ControlledTouched { by }` and `TouchPlugin`. The message is sent in every frame in which the hit box of the `Controlled` entity overlaps that of a touchable entity. A game reads it `.after(TouchSet)` and chooses the result. `game_kit::animation` has `FacingTags { down, side, up, speed }` for the sprite child of an entity with `MoveState`, and `view_of(facing)`: a sprite with the tags `Down`, `Left` (the side view, looking left) and `Up` plays the tag for the way the entity faces, flipped for the right. It plays all the time, and removing the component hands the animation back to the game.

**Rationale.** R-Town killed the player on overlap and Long Way's ghost will cost HP on overlap: the test is the same, the result is not (`adr-x13`: no game types in the kit). Sending every frame keeps the kit free of state. A game that wants one hit removes `Touchable`. The facing rule matches the Long Way player (sideways wins a tie). R-Town's walk flip and bob stays in R-Town (`walk_animation.rs`): it is a different look, with no tags.

**How to apply.** `dev-docs/extensions/enemy-movement.md`. R-Town uses the touch (`death.rs`). The ghost of Long Way uses both.

