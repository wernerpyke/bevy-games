# Games

A Cargo workspace of pixel art games built with Bevy. Each game lives in `games/<name>/`.

| Game | Folder |
|---|---|
| Long Way | `games/long-way/` |

Single entry point for players and users: quick start, controls, common commands and troubleshooting.

## Quick start

```
./dev-scripts/rust.sh run long-way
```

The first build takes several minutes.

## Controls

- Arrow keys: walk.

## Troubleshooting

- Asset not found when running the binary directly: run it through `./dev-scripts/rust.sh run <game>`, or set `BEVY_ASSET_ROOT` to the game folder (for example `games/long-way`).
