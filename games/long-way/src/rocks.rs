//! Spawns the rocks of the current map (`map.rs`). Each map character is one rock whose footprint is
//! 2x2 cells with its bottom left corner at the character's cell. The art is the native 32x32 pixel art,
//! scaled by a whole number so every art pixel stays the same size on screen.

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;

use crate::camera::MAP_ORIGIN;
use crate::collision::GUY_HALF_WIDTH;
use crate::debug::DebugSettings;
use crate::depth::DepthSort;
use crate::map::{CurrentMap, Kind, Map, MapObject};
use crate::outline::{self, Cluster, Orientation};

/// Footprint of a rock in cells, in x and y.
const FOOTPRINT_CELLS: f32 = 2.0;
/// Size of `Rocks.aseprite` in art pixels. Drawn size must be a whole multiple of this.
const ART_PIXELS: f32 = 32.0;
/// Empty rows at the bottom of the rock art, as a fraction of its height (4 of 32 art pixels). The rock is
/// placed so that its visible bottom, not the canvas bottom, sits on the bottom of the footprint.
const BOTTOM_MARGIN: f32 = 0.125;
/// Part of the footprint, on each side, that Guy may overlap. Gives the rock a softer edge than the full 128 box.
const INSET_FRACTION: f32 = 0.1;
/// Outline colour and the length of the ticks that point to the blocked side.
const OUTLINE_COLOR: Color = Color::srgb(1.0, 0.8, 0.1);
const TICK_LENGTH: f32 = 12.0;

pub struct RocksPlugin;

impl Plugin for RocksPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_rocks, build_outline).chain())
            .add_systems(Update, draw_outline);
    }
}

/// The merged outline of all rock clusters. Plan steps 5 and 6 make Guy collide with it.
#[derive(Resource)]
pub struct RockOutline(pub Vec<Cluster>);

/// Marker for a rock. The collision work (plan step 3 onward) queries it.
#[derive(Component)]
pub struct Rock;

/// Where and how large to draw one rock.
#[derive(Debug, PartialEq)]
struct Placement {
    /// World position of the centre of the sprite canvas.
    centre: Vec2,
    /// Whole number scale applied to the 32x32 art.
    scale: f32,
    /// Distance from the sprite centre down to the bottom of the footprint, for depth sorting.
    feet_offset: f32,
}

fn placement(object: &MapObject, map: &Map) -> Placement {
    let scale = (map.draw_size / ART_PIXELS).round().max(1.0);
    let draw = scale * ART_PIXELS;
    let footprint = FOOTPRINT_CELLS * map.cell_size;
    let corner =
        MAP_ORIGIN + object.cell.as_vec2() * map.cell_size + jitter_offset(object, map.jitter);
    let feet_offset = (0.5 - BOTTOM_MARGIN) * draw;
    Placement {
        centre: Vec2::new(corner.x + footprint / 2.0, corner.y + feet_offset),
        scale,
        feet_offset,
    }
}

/// World position of the bottom left corner of the footprint.
fn footprint_corner(object: &MapObject, map: &Map) -> Vec2 {
    MAP_ORIGIN + object.cell.as_vec2() * map.cell_size + jitter_offset(object, map.jitter)
}

/// The area Guy's feet may not enter: the footprint inset by `INSET_FRACTION` on each side, then grown sideways
/// by Guy's half width. There is no vertical margin.
fn blocked_area(object: &MapObject, map: &Map) -> Rect {
    let corner = footprint_corner(object, map);
    let footprint = FOOTPRINT_CELLS * map.cell_size;
    let inset = INSET_FRACTION * footprint;
    Rect::new(
        corner.x + inset - GUY_HALF_WIDTH,
        corner.y + inset,
        corner.x + footprint - inset + GUY_HALF_WIDTH,
        corner.y + footprint - inset,
    )
}

/// A fixed offset in `-jitter..=jitter` on each axis, taken from the cell number, so a map always looks
/// the same and every copy of a cluster at a different place looks slightly different.
fn jitter_offset(object: &MapObject, jitter: f32) -> Vec2 {
    if jitter == 0.0 {
        return Vec2::ZERO;
    }
    let unit = |salt: u32| {
        let mut h = (object.cell.x as u32)
            .wrapping_mul(0x9E37_79B1)
            .wrapping_add((object.cell.y as u32).wrapping_mul(0x85EB_CA77))
            .wrapping_add(salt.wrapping_mul(0xC2B2_AE3D));
        h ^= h >> 15;
        h = h.wrapping_mul(0x2C1B_3C6D);
        h ^= h >> 12;
        h as f32 / u32::MAX as f32 * 2.0 - 1.0
    };
    Vec2::new(unit(1), unit(2)) * jitter
}

fn spawn_rocks(mut commands: Commands, server: Res<AssetServer>, map: Res<CurrentMap>) {
    let map = &map.0;
    if (map.draw_size / ART_PIXELS).fract() != 0.0 {
        warn!(
            "map {}: draw {} is not a multiple of {}, using {}",
            map.name,
            map.draw_size,
            ART_PIXELS,
            (map.draw_size / ART_PIXELS).round().max(1.0) * ART_PIXELS
        );
    }
    for object in &map.objects {
        let tag = match object.kind {
            Kind::Rock0 => "rock_0",
            Kind::Rock1 => "rock_1",
        };
        let placement = placement(object, map);
        commands.spawn((
            Rock,
            AseAnimation {
                aseprite: server.load("art/features/Rocks.aseprite"),
                animation: Animation::tag(tag),
            },
            Sprite::default(),
            DepthSort {
                feet_offset: placement.feet_offset,
            },
            Transform::from_translation(placement.centre.extend(0.0))
                .with_scale(Vec3::splat(placement.scale)),
        ));
    }
}

fn build_outline(mut commands: Commands, map: Res<CurrentMap>) {
    let map = &map.0;
    let areas: Vec<Rect> = map.objects.iter().map(|o| blocked_area(o, map)).collect();
    let clusters = outline::build(&areas);
    info!(
        "rocks: {} rocks form {} clusters with {} outline edges",
        areas.len(),
        clusters.len(),
        clusters.iter().map(|c| c.edges.len()).sum::<usize>()
    );
    for cluster in &clusters {
        debug!(
            "rocks: cluster of {} rocks, bounds {:?}, {} edges",
            cluster.members.len(),
            cluster.bounds,
            cluster.edges.len()
        );
    }
    commands.insert_resource(RockOutline(clusters));
}

/// Draws the outline with the F2 grid. Each tick points to the blocked side.
fn draw_outline(settings: Res<DebugSettings>, outline: Res<RockOutline>, mut gizmos: Gizmos) {
    if !settings.show_grid {
        return;
    }
    for edge in outline.0.iter().flat_map(|c| &c.edges) {
        let (a, b, normal) = match edge.orientation {
            Orientation::Vertical => (
                Vec2::new(edge.at, edge.from),
                Vec2::new(edge.at, edge.to),
                Vec2::X,
            ),
            Orientation::Horizontal => (
                Vec2::new(edge.from, edge.at),
                Vec2::new(edge.to, edge.at),
                Vec2::Y,
            ),
        };
        gizmos.line_2d(a, b, OUTLINE_COLOR);
        let middle = (a + b) / 2.0;
        let inward = if edge.blocked_positive {
            normal
        } else {
            -normal
        };
        gizmos.line_2d(middle, middle + inward * TICK_LENGTH, OUTLINE_COLOR);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(draw_size: f32, jitter: f32) -> Map {
        Map {
            name: "t".into(),
            cell_size: 64.0,
            pitch: 1,
            draw_size,
            jitter,
            objects: Vec::new(),
        }
    }

    fn rock(x: i32, y: i32) -> MapObject {
        MapObject {
            kind: Kind::Rock0,
            cell: IVec2::new(x, y),
        }
    }

    #[test]
    fn cell_zero_zero_is_at_the_map_origin() {
        let p = placement(&rock(0, 0), &map(128.0, 0.0));
        // Footprint 128 wide: centre is 64 right of the corner. The 128 canvas is 48 above the footprint bottom.
        assert_eq!(p.centre, MAP_ORIGIN + Vec2::new(64.0, 48.0));
        assert_eq!(p.scale, 4.0);
        assert_eq!(p.feet_offset, 48.0);
    }

    #[test]
    fn cells_step_by_the_cell_size() {
        let a = placement(&rock(0, 0), &map(128.0, 0.0)).centre;
        let b = placement(&rock(3, 2), &map(128.0, 0.0)).centre;
        assert_eq!(b - a, Vec2::new(192.0, 128.0));
    }

    #[test]
    fn a_larger_draw_size_keeps_the_footprint_bottom_and_centre() {
        let a = placement(&rock(5, 5), &map(128.0, 0.0));
        let b = placement(&rock(5, 5), &map(160.0, 0.0));
        assert_eq!(b.scale, 5.0);
        assert_eq!(a.centre.x, b.centre.x);
        // The canvas bottom edge is 12.5% of the canvas below the footprint bottom in both.
        let bottom = |p: &Placement, draw: f32| p.centre.y - draw / 2.0 + BOTTOM_MARGIN * draw;
        assert_eq!(bottom(&a, 128.0), bottom(&b, 160.0));
    }

    #[test]
    fn draw_size_rounds_to_a_whole_scale() {
        assert_eq!(placement(&rock(0, 0), &map(150.0, 0.0)).scale, 5.0);
        assert_eq!(placement(&rock(0, 0), &map(10.0, 0.0)).scale, 1.0);
    }

    #[test]
    fn the_blocked_area_is_the_inset_footprint_grown_sideways_by_guys_half_width() {
        let a = blocked_area(&rock(0, 0), &map(128.0, 0.0));
        // Footprint 128, inset 12.8 each side, so 12.8 to 115.2, then 20 wider on each side.
        assert_eq!(a.min, MAP_ORIGIN + Vec2::new(12.8 - 20.0, 12.8));
        assert_eq!(a.max, MAP_ORIGIN + Vec2::new(115.2 + 20.0, 115.2));
    }

    #[test]
    fn the_blocked_area_follows_the_cell_and_the_jitter() {
        let plain = blocked_area(&rock(3, 2), &map(128.0, 0.0));
        assert_eq!(
            plain.min,
            MAP_ORIGIN + Vec2::new(192.0 + 12.8 - 20.0, 128.0 + 12.8)
        );
        let jittered = blocked_area(&rock(3, 2), &map(128.0, 8.0));
        assert_eq!(jittered.size(), plain.size());
        assert!((jittered.min - plain.min).abs().max_element() <= 8.0);
    }

    #[test]
    fn neighbouring_cells_merge_and_distant_rocks_do_not() {
        let m = map(128.0, 0.0);
        let pile: Vec<Rect> = [rock(0, 0), rock(1, 0), rock(0, 1), rock(1, 1)]
            .iter()
            .map(|o| blocked_area(o, &m))
            .collect();
        assert_eq!(outline::build(&pile).len(), 1);
        // Rocks 3 cells apart leave a gap Guy can walk through, 4 cells apart certainly.
        let apart: Vec<Rect> = [rock(0, 0), rock(3, 0)]
            .iter()
            .map(|o| blocked_area(o, &m))
            .collect();
        assert_eq!(outline::build(&apart).len(), 2);
    }

    #[test]
    fn jitter_is_zero_when_off() {
        assert_eq!(jitter_offset(&rock(7, 3), 0.0), Vec2::ZERO);
    }

    #[test]
    fn jitter_is_fixed_bounded_and_differs_between_cells() {
        let mut seen = Vec::new();
        for x in -20..20 {
            for y in -20..20 {
                let a = jitter_offset(&rock(x, y), 8.0);
                assert_eq!(a, jitter_offset(&rock(x, y), 8.0));
                assert!(a.x.abs() <= 8.0 && a.y.abs() <= 8.0, "{a}");
                seen.push(a);
            }
        }
        let mean = seen.iter().copied().sum::<Vec2>() / seen.len() as f32;
        assert!(mean.length() < 1.0, "offsets are centred on 0, mean {mean}");
        assert!(seen.iter().any(|a| a.x > 4.0) && seen.iter().any(|a| a.x < -4.0));
        assert!(seen.iter().any(|a| a.y > 4.0) && seen.iter().any(|a| a.y < -4.0));
    }
}
