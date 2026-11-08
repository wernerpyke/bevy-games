# Collectables

Map items that do not block the player. The player collects one by touching it.

## Summary

- A game spawns an entity with `Collectable { area, kind }` and a `MapId`.
- When the feet of the `Controlled` entity are inside `area`, the kit sends a `Collected { kind }` message and despawns the entity.
- The kit knows no item kinds. `kind` is the legend name from the map file. What a kind means, how it looks and where the count is kept are the game's.

## Status

| Part | Code | Shared? |
|---|---|---|
| `Collectable`, `Collected`, `CollectablePlugin`, `touches` | `crates/game-kit/src/collectable.rs` | Shared (`adr-x38`) |
| Art, kinds, spawning, inventory | `games/<game>/src/` | Per game |

## Parameters the game supplies

- The entity: `Collectable`, `MapId`, a sprite and a `DepthSort`. It has no `Blocker`.
- The touch area in world units. Long Way uses a box 52 wide and 64 high, centred on the base of the pickup.
- A system that reads `MessageReader<Collected>`. Long Way: `inventory.rs`.

## Specification

1. Only items whose `MapId` is the current map are tested.
2. The edges of `area` count as inside (the same rule as `ExitTrigger`).
3. One `Collected` message per item. The item is gone at once, so it cannot be collected twice.
4. Items go with their map. A map that loads again has all its items again. Remembering collected items needs a save design and is not done.
5. Legend characters for pickups are digits (`adr-x37`).

## Usage

Add `CollectablePlugin` after `MapTransitionPlugin`. Spawn items in the map load hook. Long Way: `pickups.rs` (spawn), `inventory.rs` (counts per energy colour, `Inventory`), `hud.rs` (an icon and a count for each energy).

## Limits

- Touch only. No pull towards the player, no pickup by key press.
- No cap or use for the counts yet.

## Tests

`collectable.rs` tests the touch rule. `inventory.rs` in Long Way tests the counts.
