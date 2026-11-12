//! Helpers that several test modules of the game share (`dev-docs/testing.md` rule 11).

use std::time::Duration;

use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_aseprite_ultra::prelude::Aseprite;
use game_kit::map::{Map, MapObject, Origin, Params};

/// An app with `MinimalPlugins` whose clock moves by `ms` milliseconds in every update.
pub fn timed_app(ms: u64) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            ms,
        )));
    app
}

/// `timed_app` that can also load art and fonts, for code that takes the `AssetServer`. No file is read in a test.
pub fn asset_app(ms: u64) -> App {
    let mut app = timed_app(ms);
    app.add_plugins(AssetPlugin::default())
        .init_asset::<Aseprite>()
        .init_asset::<Font>();
    app
}

/// A map of 8 by 8 cells of `cell_size` pixels with these objects, no `draw` key and no jitter.
pub fn map(cell_size: f32, objects: Vec<MapObject>) -> Map {
    Map {
        name: "test".to_string(),
        cell_size,
        origin: Origin::BottomLeft,
        size: IVec2::splat(8),
        draw_size: None,
        jitter: 0.0,
        background: None,
        objects,
        exits: Vec::new(),
        entries: Vec::new(),
    }
}

/// An object of the legend name `kind` at cell (`x`, `y`) with these parameters.
pub fn object(kind: &str, x: i32, y: i32, params: &[(&str, &str)]) -> MapObject {
    MapObject {
        kind: kind.to_string(),
        cell: IVec2::new(x, y),
        params: Params::from_pairs(params),
    }
}

/// Runs `spawn` once with `Commands` and the `AssetServer`, as a map load does, and applies its commands.
pub fn spawn_with(app: &mut App, spawn: impl FnOnce(&mut Commands, &AssetServer)) {
    let server = app.world().resource::<AssetServer>().clone();
    let mut queue = CommandQueue::default();
    let mut commands = Commands::new(&mut queue, app.world());
    spawn(&mut commands, &server);
    queue.apply(app.world_mut());
}

/// The messages of type `M` written in the last update (or since it, outside of an update). With `TimePlugin` the
/// messages are only swapped in an update that runs a fixed step, so use an app without it or `timed_app` with a step
/// longer than the fixed timestep (15.6 ms).
pub fn messages<M: Message + Clone>(app: &App) -> Vec<M> {
    app.world()
        .resource::<Messages<M>>()
        .iter_current_update_messages()
        .cloned()
        .collect()
}

/// The mouse pointer, for picking events triggered in a test.
fn pointer() -> bevy::picking::events::Pointer {
    let location = bevy::picking::pointer::Location {
        target: bevy::camera::NormalizedRenderTarget::None {
            width: 1,
            height: 1,
        },
        position: Vec2::ZERO,
    };
    bevy::picking::events::Pointer::new(bevy::picking::pointer::PointerId::Mouse, location)
}

/// Triggers a completed click with `button` on `entity`, as the picking backend does.
pub fn click(app: &mut App, entity: Entity, button: PointerButton) {
    app.world_mut().trigger(PointerClick {
        entity,
        pointer: pointer(),
        button,
        hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        duration: std::time::Duration::ZERO,
        count: 1,
    });
}
