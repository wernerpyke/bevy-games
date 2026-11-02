//! The collision debug overlay, drawn as sprites on the feature debug layers (`layers.rs`), not as gizmos, which
//! are always on top. It follows the F2 grid switch (`DebugSettings::show_grid`). It shows:
//! - yellow: every outline edge the actor collides with, with a tick on the blocked side
//! - cyan: the blocked area of every interior rock, which has no outline edge of its own
//! - green: the feet of every depth-sorted entity

use bevy::prelude::*;

use crate::collision::{CollisionWorld, register_blockers};
use crate::debug::DebugSettings;
use crate::depth::DepthSort;
use crate::layers::{FEATURE_DEBUG_FEET_Z, FEATURE_DEBUG_INTERIOR_Z, FEATURE_DEBUG_Z};
use crate::outline::{Edge, Orientation};

const OUTLINE_COLOR: Color = Color::srgb(1.0, 0.8, 0.1);
const INTERIOR_COLOR: Color = Color::srgb(0.2, 0.9, 1.0);
const FEET_COLOR: Color = Color::srgb(0.1, 1.0, 0.1);
/// Length of the ticks that point to the blocked side.
const TICK_LENGTH: f32 = 12.0;
/// Line thickness of the outline sprites, the same as the grid lines.
const LINE_THICKNESS: f32 = 2.0;
/// Side of the square that marks a pair of feet.
const FEET_MARKER_SIZE: f32 = 8.0;

/// The collision overlay. The draw order of its three sprite groups defaults to the kit layers (adr-x14).
pub struct CollisionDebugPlugin {
    /// Outline edges.
    pub outline_z: f32,
    /// Blocked areas of interior shapes.
    pub interior_z: f32,
    /// Feet markers.
    pub feet_z: f32,
}

impl Default for CollisionDebugPlugin {
    fn default() -> Self {
        CollisionDebugPlugin {
            outline_z: FEATURE_DEBUG_Z,
            interior_z: FEATURE_DEBUG_INTERIOR_Z,
            feet_z: FEATURE_DEBUG_FEET_Z,
        }
    }
}

#[derive(Resource)]
struct OverlayLayers {
    outline_z: f32,
    interior_z: f32,
    feet_z: f32,
}

impl Plugin for CollisionDebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(OverlayLayers {
            outline_z: self.outline_z,
            interior_z: self.interior_z,
            feet_z: self.feet_z,
        })
        .add_systems(PostStartup, draw_outline.after(register_blockers))
        .add_systems(
            Update,
            (add_feet_markers, move_feet_markers, show_overlay).chain(),
        );
    }
}

/// Parent of every overlay sprite, so they show and hide together.
#[derive(Component)]
struct OverlayRoot;

/// A green square that follows the feet of `0`.
#[derive(Component)]
struct FeetMarker(Entity);

/// Runs after every blocked area is registered, so the outline includes trunks and rocks.
fn draw_outline(mut commands: Commands, world: Res<CollisionWorld>, layers: Res<OverlayLayers>) {
    let clusters = world.clusters();
    info!(
        "collision: {} clusters with {} outline edges",
        clusters.len(),
        clusters.iter().map(|c| c.edges.len()).sum::<usize>(),
    );
    for cluster in clusters {
        debug!(
            "collision: cluster of {} areas, bounds {:?}, {} edges",
            cluster.members.len(),
            cluster.bounds,
            cluster.edges.len()
        );
    }
    commands
        .spawn((OverlayRoot, Transform::default(), Visibility::default()))
        .with_children(|parent| {
            for edge in clusters.iter().flat_map(|c| &c.edges) {
                for (centre, size) in edge_boxes(edge) {
                    parent.spawn(line(OUTLINE_COLOR, layers.outline_z, centre, size));
                }
            }
            for area in world.interior_areas() {
                for (centre, size) in rect_boxes(area) {
                    parent.spawn(line(INTERIOR_COLOR, layers.interior_z, centre, size));
                }
            }
        });
}

fn line(color: Color, z: f32, centre: Vec2, size: Vec2) -> (Sprite, Transform) {
    (
        Sprite::from_color(color, size),
        Transform::from_translation(centre.extend(z)),
    )
}

/// The boxes (centre and size) that draw an edge: the line, then the tick on its blocked side.
fn edge_boxes(edge: &Edge) -> [(Vec2, Vec2); 2] {
    let side = if edge.blocked_positive { 1.0 } else { -1.0 };
    let length = edge.to - edge.from;
    let middle = (edge.from + edge.to) / 2.0;
    match edge.orientation {
        Orientation::Vertical => [
            (
                Vec2::new(edge.at, middle),
                Vec2::new(LINE_THICKNESS, length),
            ),
            (
                Vec2::new(edge.at + side * TICK_LENGTH / 2.0, middle),
                Vec2::new(TICK_LENGTH, LINE_THICKNESS),
            ),
        ],
        Orientation::Horizontal => [
            (
                Vec2::new(middle, edge.at),
                Vec2::new(length, LINE_THICKNESS),
            ),
            (
                Vec2::new(middle, edge.at + side * TICK_LENGTH / 2.0),
                Vec2::new(LINE_THICKNESS, TICK_LENGTH),
            ),
        ],
    }
}

/// The four boxes (centre and size) that draw the border of a rectangle.
fn rect_boxes(rect: Rect) -> [(Vec2, Vec2); 4] {
    let (centre, size) = (rect.center(), rect.size());
    [
        (
            Vec2::new(centre.x, rect.min.y),
            Vec2::new(size.x, LINE_THICKNESS),
        ),
        (
            Vec2::new(centre.x, rect.max.y),
            Vec2::new(size.x, LINE_THICKNESS),
        ),
        (
            Vec2::new(rect.min.x, centre.y),
            Vec2::new(LINE_THICKNESS, size.y),
        ),
        (
            Vec2::new(rect.max.x, centre.y),
            Vec2::new(LINE_THICKNESS, size.y),
        ),
    ]
}

fn add_feet_markers(
    mut commands: Commands,
    layers: Res<OverlayLayers>,
    new: Query<Entity, Added<DepthSort>>,
    root: Single<Entity, With<OverlayRoot>>,
) {
    for target in &new {
        commands.entity(*root).with_child((
            FeetMarker(target),
            Sprite::from_color(FEET_COLOR, Vec2::splat(FEET_MARKER_SIZE)),
            Transform::from_xyz(0.0, 0.0, layers.feet_z),
        ));
    }
}

fn move_feet_markers(
    mut commands: Commands,
    mut markers: Query<(Entity, &FeetMarker, &mut Transform)>,
    targets: Query<(&Transform, &DepthSort), Without<FeetMarker>>,
) {
    for (marker, FeetMarker(target), mut transform) in &mut markers {
        match targets.get(*target) {
            Ok((target, sort)) => {
                transform.translation.x = target.translation.x;
                transform.translation.y = sort.feet_y(target.translation.y);
            }
            Err(_) => commands.entity(marker).despawn(),
        }
    }
}

fn show_overlay(
    settings: Res<DebugSettings>,
    mut root: Single<&mut Visibility, With<OverlayRoot>>,
) {
    let wanted = if settings.show_grid {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if **root != wanted {
        **root = wanted;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_edge_is_drawn_as_a_line_and_a_tick_on_its_blocked_side() {
        let edge = |orientation, blocked_positive| Edge {
            orientation,
            at: 10.0,
            from: 20.0,
            to: 60.0,
            blocked_positive,
            from_concave: false,
            to_concave: false,
        };
        let [line, tick] = edge_boxes(&edge(Orientation::Vertical, true));
        assert_eq!(
            line,
            (Vec2::new(10.0, 40.0), Vec2::new(LINE_THICKNESS, 40.0))
        );
        assert_eq!(tick.0, Vec2::new(10.0 + TICK_LENGTH / 2.0, 40.0));
        let [_, tick] = edge_boxes(&edge(Orientation::Vertical, false));
        assert_eq!(tick.0, Vec2::new(10.0 - TICK_LENGTH / 2.0, 40.0));
        let [line, tick] = edge_boxes(&edge(Orientation::Horizontal, true));
        assert_eq!(
            line,
            (Vec2::new(40.0, 10.0), Vec2::new(40.0, LINE_THICKNESS))
        );
        assert_eq!(tick.0, Vec2::new(40.0, 10.0 + TICK_LENGTH / 2.0));
        let [_, tick] = edge_boxes(&edge(Orientation::Horizontal, false));
        assert_eq!(tick.0, Vec2::new(40.0, 10.0 - TICK_LENGTH / 2.0));
    }

    #[test]
    fn a_rectangle_is_drawn_as_four_lines_on_its_border() {
        let boxes = rect_boxes(Rect::new(0.0, 10.0, 40.0, 30.0));
        assert!(boxes.contains(&(Vec2::new(20.0, 10.0), Vec2::new(40.0, LINE_THICKNESS))));
        assert!(boxes.contains(&(Vec2::new(20.0, 30.0), Vec2::new(40.0, LINE_THICKNESS))));
        assert!(boxes.contains(&(Vec2::new(0.0, 20.0), Vec2::new(LINE_THICKNESS, 20.0))));
        assert!(boxes.contains(&(Vec2::new(40.0, 20.0), Vec2::new(LINE_THICKNESS, 20.0))));
    }
}
