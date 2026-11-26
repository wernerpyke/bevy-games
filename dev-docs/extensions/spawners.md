# Spawners

## Summary

A spawner is an entity that asks the game for new entities every so many seconds. The kit keeps the timer, the group size, the limit and the places, and sends one `SpawnRequest` per entity. The game spawns the entity. In R-Town a spawner is a legend entry of the map file, with its settings as parameters. Jet spawns one spawner in code, with groups and places on its field edges.

## Status

`crates/game-kit/src/spawner.rs` and the legend parameters in `crates/game-kit/src/map.rs`. Used by R-Town (`adr-rt12`) and Jet (`adr-jt18`). Decisions: `adr-x25`, `adr-x63`.

## Terms

- **Legend parameters.** `key=value` words after the kind in a legend entry. They are read into `MapObject.params` (`Params`).
- **Request.** `SpawnRequest { spawner, position, params }`, one per entity.
- **Gap.** The time between two spawns (`every`).
- **Group.** The entities of one spawn, sent as that many requests in one frame.
- **Places.** `SpawnPlaces { groups }`: groups of points where entities may start.

## Parameters the game supplies

| Parameter | Meaning | R-Town | Jet |
|---|---|---|---|
| Kinds | Legend kinds of spawners | `spawn_skeleton`, `spawn_goblin` | none (spawned in code) |
| `every` | Seconds between spawns, required | legend | drawn from the level's range, changed at each level-up |
| `max` | Most live entities of the spawner, optional | legend | none |
| `group` | Entities per spawn (`with_group`), default 1 | 1 | the level's group size |
| First spawn | At once (default) or after one gap (`first_after_gap`) | at once | after one gap |
| `places` | Where the entities start (`with_places`), default the spawner's position | the spawner's cell | one group per field edge |
| `strategy` | Game parameter in each request | `patrol` (needs `radius`) or `pursue` | none |

## Specification

1. A legend entry is `"S": spawn_skeleton every=10 strategy=patrol radius=120`. The first word is the kind. Each following word is `key=value`: the key has letters, digits and `_`, and a key is not given twice. A bad entry stops the load with the file and the legend line (`map-files.md`).
2. The parser does not know what a parameter means. The game checks the keys it needs when it loads the map (`Spawner::from_params`, `RandomPatrol::from_params`) and reports errors with the file and line.
3. `SpawnerPlugin` sends the first group at once, or one gap after the start with `first_after_gap`, and the next ones every `every` seconds. A long frame gives one group, not a burst.
4. A group is `group` requests in one frame. With `max`, the spawner counts the entities with its `SpawnedBy`. When the limit is reached the spawn stays due and happens as soon as one dies. When only part of a group fits under the limit, only that part is sent.
5. `set_every(seconds)` changes the gap while the game runs and starts it again: the next spawn is one new gap from now. The game may also change `group` at any time.
6. Without places, each entity starts at the spawner's position (`Transform` x and y; R-Town: the middle of its cell). With places, each entity picks a group of points at random, each group with the same chance however many points it has, then a point of that group at random, with the kit `GameRng` (`RngPlugin`).
7. A place inside a blocked area of the `CollisionWorld` (`is_blocked`, by more than `TOLERANCE`) is never picked, so places stay right when blocked areas change during play. A group with no free point is left out. When no place is free, no request is sent for that spawn, and the kit warns once.
8. The game reads `SpawnRequest`, spawns the entity at `position` and adds `SpawnedBy(spawner)`, which also gives it the spawner's `MapId` (`adr-x60`), so it goes with the spawner's map.
9. The legend parameters go in every request, so the game can choose the strategy and values per spawner.

## Usage

```rust
commands.spawn((
    Spawner::new(gap, None, Params::default())
        .first_after_gap()
        .with_group(2)
        .with_places(SpawnPlaces { groups: vec![top_edge, bottom_edge] }),
    Transform::default(),
));
// Later, at a level-up:
spawner.set_every(new_gap);
spawner.group = 3;
```

- In a map file, write one legend character per configured spawner. Different intervals use different characters.
- Spawners are removed with the map objects when the map is reloaded, and created again (`adr-rt13`). A spawner spawned in a map's `load` hook is adopted into the map root the same way.

## Limits

- The legend reads `every` and `max` only. Groups, places and the first-spawn option are set in code.
- The gap is fixed until the game changes it. A range such as `every=8..12` is not read from the legend: the game draws the gap and calls `set_every`.
- No rule against spawning close to the player.

## Tests

`crates/game-kit/src/spawner.rs`: the timer (first spawn, after a gap, restart, burst, hold), the parameters, the places (the same chance per group, blocked places, no free place), and headless apps for the first request, the interval, groups and `max`, a spawner that starts after a gap, and places checked against the collision world. `collision.rs`: `is_blocked`. `map.rs`: parameters. R-Town: `src/map.rs` and `src/enemy.rs` check the entries of `test.txt` and the errors. Run `./dev-scripts/rust.sh test`.
