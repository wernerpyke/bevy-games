# Spawners

## Summary

A spawner is an entity that asks the game for a new entity every so many seconds. The kit keeps the timer and the limit and sends `SpawnRequest`. The game spawns the entity. In R-Town a spawner is a legend entry of the map file, with its settings as parameters.

## Status

`crates/game-kit/src/spawner.rs` and the legend parameters in `crates/game-kit/src/map.rs`. Used by R-Town (`adr-rt12`). Decision: `adr-x25`.

## Terms

- **Legend parameters.** `key=value` words after the kind in a legend entry. They are read into `MapObject.params` (`Params`).
- **Request.** `SpawnRequest { spawner, position, params }`.

## Parameters the game supplies

| Parameter | R-Town |
|---|---|
| Kinds | `spawn_skeleton` and `spawn_goblin` |
| `every` | seconds between spawns, required |
| `max` | most live entities of the spawner, optional, no limit without it |
| `strategy` | `patrol` (needs `radius`) or `pursue` |

## Specification

1. A legend entry is `"S": spawn_skeleton every=10 strategy=patrol radius=120`. The first word is the kind. Each following word is `key=value`: the key has letters, digits and `_`, and a key is not given twice. A bad entry stops the load with the file and the legend line (`map-files.md`).
2. The parser does not know what a parameter means. The game checks the keys it needs when it loads the map (`Spawner::from_params`, `RandomPatrol::from_params`) and reports errors with the file and line.
3. The spawner entity stands at the position of the spawn (R-Town: the middle of its cell). `SpawnerPlugin` sends the first request at once and the next ones every `every` seconds. A long frame gives one request, not a burst.
4. The game reads `SpawnRequest`, spawns the entity at `position` and adds `SpawnedBy(spawner)`. With `max`, the spawner counts the entities with its `SpawnedBy`. When the limit is reached the spawn stays due and happens as soon as one dies.
5. The legend parameters go in every request, so the game can choose the strategy and values per spawner.

## Usage

- Write one legend character per configured spawner. Different intervals use different characters.
- Spawners are removed with the map objects when the map is reloaded, and created again (`adr-rt13`).

## Limits

- A fixed interval only. A range such as `every=8..12` is not supported.
- No check that a spawner stands on free ground, and no rule against spawning close to the player.

## Tests

`crates/game-kit/src/spawner.rs` (timer, burst, hold, parameters, headless apps for the first request, the interval and `max`) and `map.rs` (parameters). R-Town: `src/map.rs` and `src/enemy.rs` check the entries of `test.txt` and the errors. Run `./dev-scripts/rust.sh test`.
