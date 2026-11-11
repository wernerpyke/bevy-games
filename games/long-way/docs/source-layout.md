# Source layout

One plugin per feature. Each plugin owns its components, resources and systems (`CLAUDE.md`, architecture defaults).

```
games/long-way/
├── Cargo.toml
├── assets/
│   ├── art/            .aseprite sources and the palette
│   └── maps/           text map files
├── docs/               this folder
└── src/
    ├── main.rs         builds the App, adds plugins
    ├── camera.rs       Window size, VIEW_HEIGHT, HUD bar ratio and colours, CLEAR_COLOR, CELL_SIZE, MAP_ORIGIN, map_rect (the camera is `game_kit::camera`)
    ├── hud.rs          HudPlugin: the content of the HUD bar (the title, one clickable icon with a count for each energy, and the row of HP icons). The bar is `game_kit::screen` and `game_kit::hud`
    ├── weapon_menu.rs  WeaponMenuPlugin: the weapon drawer that slides up over the game area from an energy icon, `WeaponKind`, `weapons_for` (energy to weapons), the messages `EnergyIconClicked` and `WeaponChosen` (`adr-lw31`)
    ├── player.rs       PlayerPlugin: player spawn, click to walk (goal, limits), animation
    ├── trees.rs        `spawn_trees`: trees and their trunks of one map
    ├── rocks.rs        `spawn_rocks`, `rebuild_collision`: rocks and the collision world of the loaded maps
    ├── markers.rs      `spawn_markers`: lanterns, markers and their blockers of one map
    ├── pickups.rs      `spawn_pickups`: energy pickups of one map, each with a `Collectable`
    ├── inventory.rs    `Inventory` (energy counts), `InventoryPlugin`: fills it from `Collected`
    ├── hp.rs           `Hp` (the player's hit points), `DamagePlayer` message, `HpPlugin` (`adr-lw28`)
    ├── enemy_hp.rs     `EnemyHp`, the `DamageEnemy` and `EnemyDied` messages, `EnemyHpPlugin`: damage, the hurt flash (`adr-lw32`)
    ├── weapons/        `WeaponPlugin`: `CastWeapon` (from `WeaponChosen`) and `WeaponHit` messages, `values.rs` (every weapon number), `damage.rs` (hit points and push), `ring.rs` (the fire ring, `adr-lw33`) (`adr-lw32`)
    ├── ghost.rs        `spawn_ghosts` (from `spawn_ghost` markers, called by the load hook), `GhostPlugin`: the touch (1 HP, then dies) and the removal after `Die` (`adr-lw29`)
    ├── background.rs   `spawn_background`, `check_tiles`: draws the ground colour and the tile images of a map's `background:` block from `ground_tiles.aseprite` (`adr-x36`, `adr-lw26`)
    ├── scenery.rs      `spawn_scenery`: the Zen Garden trees and shrubs of one map, blocking like `green_tree` (`adr-lw23`)
    ├── map.rs          MapPlugin, `Kind`: registers the three maps and the game hooks. Parser and lifecycle: `game_kit::map`, `game_kit::map_transition`
    ├── collision.rs    Trunk and player constants. The engine is `game_kit::collision`
```

## Which extension each file implements

| Extension | Files |
|---|---|
| [Collision](../../../dev-docs/extensions/collision.md) | `collision.rs`, `outline.rs` in `crates/game-kit` (plus the game specific `Trunk`, `PLAYER_*` and `rocks.rs`) |
| [Draw order](../../../dev-docs/extensions/draw-order.md) | `depth.rs`, `layers.rs` in `crates/game-kit` |
| [Map files](../../../dev-docs/extensions/map-files.md) | `map.rs` (plus the parser in `crates/game-kit`) |
| [Collectables](../../../dev-docs/extensions/collectables.md) | `pickups.rs`, `inventory.rs` (plus `collectable.rs` in `crates/game-kit`) |
| [Debug tools](../../../dev-docs/extensions/debug-tools.md) | `debug.rs`, `collision_debug.rs` in `crates/game-kit` |
| [Pointer](../../../dev-docs/extensions/pointer.md) | `player.rs` (`set_goal`) (plus `pointer.rs`, `nav.rs`, `movement.rs` in `crates/game-kit`) |
| [Development feedback](../../../dev-docs/extensions/dev-feedback.md) | `script.rs` in `crates/game-kit` |
| [Plugin configuration](../../../dev-docs/extensions/plugin-configuration.md) | `collision.rs`, `debug.rs` in `crates/game-kit` |
| [Screen layout](../../../dev-docs/extensions/screen-layout.md) | `camera.rs`, `hud.rs`, `main.rs` (plus `screen.rs`, `hud.rs`, `app_state.rs` in `crates/game-kit`) |
| [Pixel art](../../../dev-docs/extensions/pixel-art.md) | `camera.rs`, `player.rs` (plus `camera.rs` in `crates/game-kit`) |

Game specific files: `main.rs`, `player.rs`, `trees.rs`, `rocks.rs`, `scenery.rs`, `background.rs`, `markers.rs`, `pickups.rs`, `inventory.rs`, `hp.rs`, `enemy_hp.rs`, `weapons/`, `ghost.rs`.

## Long Way values for the pixel art extension

- `VIEW_HEIGHT` is 512 world units (8 cells) and the game region of the window is square (the window is 576 by 756 with the HUD bar below, `adr-lw24`, `adr-lw31`), so the view is 8 by 8 cells. The Hunter is about 50 pixels tall, so it is about one 10th of the screen height. The camera follows the player and stops at the map edge (`adr-lw16`, `adr-lw17`). The player starts at the `player_start` marker (`adr-lw19`).
- The player is the Hunter. Walking animation speed is `WALK_ANIMATION_SPEED` 2.5. Idle is 1.0. Walk speed is 300 units per second.
- The player's tags (`Hunter.aseprite`): `Idle`, `Run`, `Up`, `Down`. `Up` and `Down` are placeholders. See [art-tags.md](art-tags.md).
