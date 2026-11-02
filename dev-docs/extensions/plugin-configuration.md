# Plugin configuration

How a game configures shared behaviour: plugin fields stored as a resource.

## Summary

A plugin is a struct with public fields. Its `build` stores the settings as a resource. Systems and helper functions read the resource. The same code then behaves differently in each game, without copies or feature flags.

## Status

In use for `CollisionPlugin` in `games/long-way/src/collision.rs`. A pattern, not a module, so it needs no extraction.

## Specification

1. The plugin struct holds only configuration, as public fields.
2. `build` calls `insert_resource` with a settings struct built from those fields.
3. Systems take `Res<Settings>`. Plain helper functions take `&Settings`, so they stay testable without an `App`.
4. Tests build the settings struct directly (`CollisionSettings`), without a plugin.

## Usage

```rust
// main.rs
app.add_plugins(CollisionPlugin { allow_sliding: true });

// collision.rs
pub struct CollisionPlugin { pub allow_sliding: bool }

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CollisionSettings { allow_sliding: self.allow_sliding });
    }
}
```

`DebugPlugin { cell_size, grid_color, grid_origin }` ([debug-tools.md](debug-tools.md)) follows the same pattern.

## Limits

Settings are fixed at startup. A setting that changes while playing should be a resource that systems write, not a plugin field.
