# Source layout

One plugin per feature. Each plugin owns its components, resources and systems (`CLAUDE.md`, architecture defaults).

```
games/long-way/
├── Cargo.toml
├── assets/
│   ├── art/            .aseprite sources and the palette
│   └── maps/           markdown map files
├── docs/               this folder
└── src/
    ├── main.rs         builds the App, adds plugins
    ├── camera.rs       CameraPlugin: camera, background colour, VIEW_HEIGHT, MAP_ORIGIN
    ├── player.rs       PlayerPlugin: Guy spawn, movement, animation
    ├── trees.rs        TreesPlugin: trees and their trunks from the map
    ├── rocks.rs        RocksPlugin: rocks and their blocked areas from the map
    ├── map.rs          MapPlugin, `Kind`: loads the map file and names its kinds. The parser is `game_kit::map`
    ├── collision.rs    Trunk and Guy constants. The engine is `game_kit::collision`
```

## Which extension each file implements

| Extension | Files |
|---|---|
| [Collision](../../../dev-docs/extensions/collision.md) | `collision.rs`, `outline.rs` in `crates/game-kit` (plus the game specific `Trunk`, `GUY_*` and `rocks.rs`) |
| [Draw order](../../../dev-docs/extensions/draw-order.md) | `depth.rs`, `layers.rs` in `crates/game-kit` |
| [Map files](../../../dev-docs/extensions/map-files.md) | `map.rs` (plus the parser in `crates/game-kit`) |
| [Debug tools](../../../dev-docs/extensions/debug-tools.md) | `debug.rs`, `collision_debug.rs` in `crates/game-kit` |
| [Development feedback](../../../dev-docs/extensions/dev-feedback.md) | `script.rs` in `crates/game-kit` |
| [Plugin configuration](../../../dev-docs/extensions/plugin-configuration.md) | `collision.rs`, `debug.rs` in `crates/game-kit` |
| [Pixel art](../../../dev-docs/extensions/pixel-art.md) | `camera.rs`, `player.rs` |

Game specific files: `main.rs`, `player.rs`, `trees.rs`, `rocks.rs`.

## Long Way values for the pixel art extension

- `VIEW_HEIGHT` is 1200 world units. Guy is 60 pixels tall, so he is one twentieth of the screen height.
- Walking animation speed is `WALK_ANIMATION_SPEED` 2.5. Idle is 1.0. Walk speed is 300 units per second.
- Guy's tags: `Idle`, `Walk`, `Up`, `Down`. See [art-tags.md](art-tags.md).
