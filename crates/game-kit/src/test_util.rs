//! Helpers that several test modules of the kit share (`dev-docs/testing.md`, rule 11).

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

use crate::camera::CameraBounds;
use crate::collision::{CollisionSettings, CollisionWorld, SlideTuning};
use crate::map::{Map, parse};
use crate::movement::MovementPlugin;
use crate::nav::NavPlugin;
use crate::outline::{self, Edge};

/// One frame at 60 frames per second.
pub(crate) const DT: f32 = 1.0 / 60.0;

/// A 200 by 200 map from (0, 0).
pub(crate) fn map_rect() -> Rect {
    Rect::new(0.0, 0.0, 200.0, 200.0)
}

/// The outline edges of the blocked areas `rects`.
pub(crate) fn edges_of(rects: &[Rect]) -> Vec<Edge> {
    outline::build(rects)
        .iter()
        .flat_map(|c| c.edges.iter().copied())
        .collect()
}

/// An app with `MinimalPlugins` in which each update advances time by `frame`.
pub(crate) fn timed_app(frame: Duration) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(TimeUpdateStrategy::ManualDuration(frame));
    app
}

/// A `timed_app` with 16 ms frames, the `blocked` areas, sliding on, the map rectangle `bounds`, navigation (step 4)
/// and movement.
pub(crate) fn movement_app(blocked: &[Rect], bounds: Rect) -> App {
    let mut app = timed_app(Duration::from_millis(16));
    let mut world = CollisionWorld::default();
    world.add_blocked(blocked);
    app.insert_resource(world)
        .insert_resource(CollisionSettings {
            allow_sliding: true,
            slide: SlideTuning::for_cell_size(16.0),
        })
        .insert_resource(CameraBounds(bounds))
        .add_plugins((NavPlugin { step: 4.0 }, MovementPlugin::default()));
    app
}

/// Every message of type `M` that `collect::<M>` has read.
#[derive(Resource)]
pub(crate) struct Collected<M>(pub Vec<M>);

impl<M> Default for Collected<M> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

/// Adds every message of type `M` to `Collected<M>`.
pub(crate) fn collect<M: Message + Clone>(
    mut reader: MessageReader<M>,
    mut seen: ResMut<Collected<M>>,
) {
    seen.0.extend(reader.read().cloned());
}

/// The text of an empty map file (bottom-left origin) of `w` by `h` cells. `blocks` goes after the legend.
pub(crate) fn empty_map_text(name: &str, cell: u32, w: i32, h: i32, blocks: &str) -> String {
    let head = format!(
        "---\nmap: {name}\ncell: {cell}\norigin: bottom-left\nsize: {w}x{h}\nlegend:\n  \".\": empty\n{blocks}---\n"
    );
    let row = vec!["."; w as usize].join(" ");
    let rows: String = (0..h).rev().map(|y| format!("{y:>2} |{row}|\n")).collect();
    format!("{head}{rows}")
}

/// The parsed `empty_map_text` of a map named `t`.
pub(crate) fn empty_map(cell: u32, w: i32, h: i32, blocks: &str) -> Map {
    parse("t.txt", &empty_map_text("t", cell, w, h, blocks), &[]).unwrap_or_else(|e| panic!("{e}"))
}
