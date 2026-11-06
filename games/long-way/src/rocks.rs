//! Spawns the rocks of the current map (`map.rs`). Each map character is one rock whose footprint is
//! 2x2 cells with its bottom left corner at the character's cell. The art is the native 32x32 pixel art,
//! scaled by a whole number so every art pixel stays the same size on screen.

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;

use crate::camera::MAP_ORIGIN;
use crate::collision::PLAYER_HALF_WIDTH;
use crate::map::{CurrentMap, Group, Kind, MapEntity, blocking_groups};
use game_kit::collision::{CollisionWorld, register_blockers};
use game_kit::depth::DepthSort;
use game_kit::map::{Map, MapObject};

/// Footprint of a rock in cells, in x and y.
pub const FOOTPRINT_CELLS: f32 = 2.0;
/// Size of `Rocks.aseprite` in art pixels. Drawn size must be a whole multiple of this.
const ART_PIXELS: f32 = 32.0;
/// Empty rows at the bottom of the rock art, as a fraction of its height (4 of 32 art pixels). The rock is
/// placed so that its visible bottom, not the canvas bottom, sits on the bottom of the footprint.
const BOTTOM_MARGIN: f32 = 0.125;
/// Part of the footprint, on the left, right and top, that the player may overlap. Gives the rock a softer edge than the full 128 box.
const INSET_FRACTION: f32 = 0.1;
/// Part of the footprint inset at the bottom. 0 puts the bottom line on the lowest pixel of the art, not inside it.
const BOTTOM_INSET_FRACTION: f32 = 0.0;
/// Part of the footprint height a rock blocks when it stands alone: its bottom, as a trunk blocks the bottom 10% of a tree
/// (`adr-lw14`). The player walks behind the rest.
const ALONE_BAND_FRACTION: f32 = 0.1;
/// Drawn size of a rock in pixels when the map file has no `draw` key.
const DEFAULT_DRAW_SIZE: f32 = 128.0;
/// How far above the bottom of the blocked area a rock sorts. The player stops exactly on that line, so without a bias the
/// draw order of the two would be a tie. With it the player is drawn in front when it stands at the rock or below it.
pub const SORT_BIAS: f32 = 1.0;

pub struct RocksPlugin;

impl Plugin for RocksPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_rocks)
            .add_systems(PostStartup, mark_interior.after(register_blockers));
    }
}

/// Marker for a rock. The collision code queries it.
#[derive(Component)]
pub struct Rock;

/// Marks a rock that touches no empty space, because rocks around it fill every side (`outline::Cluster::interior`).
/// Collision uses only the outline edges, so this costs nothing there. The debug overlay colours these rocks.
#[derive(Component)]
pub struct InteriorRock;

/// Where and how large to draw one rock.
#[derive(Debug, PartialEq)]
struct Placement {
    /// World position of the centre of the sprite canvas.
    centre: Vec2,
    /// Whole number scale applied to the 32x32 art.
    scale: f32,
    /// Distance from the sprite centre down to the sort line, for depth sorting.
    feet_offset: f32,
}

fn placement(object: &MapObject, map: &Map) -> Placement {
    let scale = (draw_size(map) / ART_PIXELS).round().max(1.0);
    let draw = scale * ART_PIXELS;
    let footprint = FOOTPRINT_CELLS * map.cell_size;
    let corner =
        MAP_ORIGIN + object.cell.as_vec2() * map.cell_size + jitter_offset(object, map.jitter);
    // The canvas centre is above the footprint bottom by this much, which puts the visible rock bottom on it.
    let centre_above_footprint = (0.5 - BOTTOM_MARGIN) * draw;
    Placement {
        centre: Vec2::new(
            corner.x + footprint / 2.0,
            corner.y + centre_above_footprint,
        ),
        scale,
        // The sort line is the bottom of the blocked area plus a small bias, not the bottom of the footprint.
        feet_offset: centre_above_footprint - BOTTOM_INSET_FRACTION * footprint - SORT_BIAS,
    }
}

/// The drawn size of a rock: the `draw` key of the map file, or the default.
fn draw_size(map: &Map) -> f32 {
    map.draw_size.unwrap_or(DEFAULT_DRAW_SIZE)
}

/// World position of the bottom left corner of the footprint.
pub fn footprint_corner(object: &MapObject, map: &Map) -> Vec2 {
    MAP_ORIGIN + object.cell.as_vec2() * map.cell_size + jitter_offset(object, map.jitter)
}

/// The area the player's feet may not enter for an object in a group: the footprint inset by `INSET_FRACTION` on the left, right
/// and top and by `BOTTOM_INSET_FRACTION` at the bottom, then grown sideways by the player's half width. With `open_top` there
/// is no inset at the top, so it merges with the object above (`Group::neighbour_above`). There is no vertical margin.
pub fn group_area(object: &MapObject, map: &Map, open_top: bool) -> Rect {
    let corner = footprint_corner(object, map);
    let footprint = FOOTPRINT_CELLS * map.cell_size;
    let inset = INSET_FRACTION * footprint;
    let top_inset = if open_top { 0.0 } else { inset };
    Rect::new(
        corner.x + inset - PLAYER_HALF_WIDTH,
        corner.y + BOTTOM_INSET_FRACTION * footprint,
        corner.x + footprint - inset + PLAYER_HALF_WIDTH,
        corner.y + footprint - top_inset,
    )
}

/// The blocked area of a rock: the band at its bottom when it stands alone, the group area when it is in a group.
fn blocked_area(object: &MapObject, map: &Map, group: Group) -> Rect {
    let full = group_area(object, map, group.neighbour_above);
    if group.is_group() {
        return full;
    }
    let corner = footprint_corner(object, map);
    let footprint = FOOTPRINT_CELLS * map.cell_size;
    Rect::new(
        full.min.x,
        full.min.y,
        full.max.x,
        corner.y + ALONE_BAND_FRACTION * footprint,
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

pub fn spawn_rocks(
    mut commands: Commands,
    server: Res<AssetServer>,
    map: Res<CurrentMap>,
    mut world: ResMut<CollisionWorld>,
) {
    let map = &map.0;
    if (draw_size(map) / ART_PIXELS).fract() != 0.0 {
        warn!(
            "map {}: draw {} is not a multiple of {}, using {}",
            map.name,
            draw_size(map),
            ART_PIXELS,
            (draw_size(map) / ART_PIXELS).round().max(1.0) * ART_PIXELS
        );
    }
    let rocks: Vec<(&MapObject, Group)> = map
        .objects
        .iter()
        .zip(blocking_groups(map))
        .filter(|(o, _)| Kind::of(o).is_rock())
        .collect();
    let areas: Vec<Rect> = rocks
        .iter()
        .map(|(o, group)| blocked_area(o, map, *group))
        .collect();
    let first_area = world.add_blocked(&areas);

    for (index, (object, _)) in rocks.into_iter().enumerate() {
        let tag = match Kind::of(object) {
            Kind::Rock0 => "rock_0",
            Kind::Rock1 => "rock_1",
            _ => unreachable!("only rocks are spawned here"),
        };
        let placement = placement(object, map);
        commands.spawn((
            Rock,
            MapEntity,
            BlockedArea(first_area + index),
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

/// Number of the rock's blocked area in `CollisionWorld`.
#[derive(Component)]
pub struct BlockedArea(usize);

/// Runs after every blocked area is registered, so a rock next to a trunk is judged with the trunk in place.
pub fn mark_interior(
    mut commands: Commands,
    world: Res<CollisionWorld>,
    rocks: Query<(Entity, &BlockedArea), With<Rock>>,
) {
    let mut interior = 0;
    for (entity, area) in &rocks {
        if world.is_interior(area.0) {
            commands.entity(entity).insert(InteriorRock);
            interior += 1;
        }
    }
    info!(
        "rocks: {} of {} rocks are interior",
        interior,
        rocks.iter().count()
    );
}
