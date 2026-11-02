# Source layout

One plugin per feature. Each plugin owns its components, resources and systems (`CLAUDE.md`, architecture guidance).

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
    ├── map.rs          MapPlugin: parses the markdown map file
    ├── outline.rs      merges blocked rectangles into clusters and outline edges
    ├── collision.rs    CollisionPlugin: CollisionWorld, resolve, sliding, Trunk, Guy constants
    ├── collision_debug.rs  collision overlay
    ├── debug.rs        DebugPlugin: grid and info box
    ├── layers.rs       draw order layers
    ├── depth.rs        DepthPlugin: draw order sorted by feet
    └── script.rs       scripted input and screenshots
```

## Which extension each file implements

| Extension | Files |
|---|---|
| [Collision](../../../dev-docs/extensions/collision.md) | `collision.rs`, `outline.rs` (plus the game specific `Trunk`, `GUY_*` and `rocks.rs`) |
| [Draw order](../../../dev-docs/extensions/draw-order.md) | `depth.rs`, `layers.rs` |
| [Map files](../../../dev-docs/extensions/map-files.md) | `map.rs` |
| [Debug tools](../../../dev-docs/extensions/debug-tools.md) | `debug.rs`, `collision_debug.rs` |
| [Development feedback](../../../dev-docs/extensions/dev-feedback.md) | `script.rs` |
| [Plugin configuration](../../../dev-docs/extensions/plugin-configuration.md) | `collision.rs`, `debug.rs` |
| [Pixel art](../../../dev-docs/extensions/pixel-art.md) | `camera.rs`, `player.rs` |

Game specific files: `main.rs`, `player.rs`, `trees.rs`, `rocks.rs`.

## Long Way values for the pixel art extension

- `VIEW_HEIGHT` is 1200 world units. Guy is 60 pixels tall, so he is one twentieth of the screen height.
- Walking animation speed is `WALK_ANIMATION_SPEED` 2.5. Idle is 1.0. Walk speed is 300 units per second.
- Guy's tags: `Idle`, `Walk`, `Up`, `Down`. See [art-tags.md](art-tags.md).
