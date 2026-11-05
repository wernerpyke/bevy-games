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

#[cfg(test)]
mod tests {
    use super::*;
    use game_kit::outline;

    fn map(draw_size: f32, jitter: f32) -> Map {
        Map {
            name: "t".into(),
            cell_size: 64.0,
            origin: game_kit::map::Origin::BottomLeft,
            size: bevy::math::IVec2::ZERO,
            draw_size: Some(draw_size),
            jitter,
            objects: Vec::new(),
        }
    }

    fn rock(x: i32, y: i32) -> MapObject {
        MapObject {
            kind: "rock_0".to_string(),
            cell: IVec2::new(x, y),
            params: Default::default(),
        }
    }

    #[test]
    fn cell_zero_zero_is_at_the_map_origin() {
        let p = placement(&rock(0, 0), &map(128.0, 0.0));
        // Footprint 128 wide: centre is 64 right of the corner. The 128 canvas is 48 above the footprint bottom.
        assert_eq!(p.centre, MAP_ORIGIN + Vec2::new(64.0, 48.0));
        assert_eq!(p.scale, 4.0);
        assert_eq!(p.feet_offset, (48.0 - SORT_BIAS));
    }

    #[test]
    fn rocks_sort_just_above_the_bottom_of_their_blocked_area() {
        // The player is stopped at the bottom of the blocked area, or walks below it. Both must draw in front of the rock.
        for draw in [128.0, 160.0] {
            let m = map(draw, 0.0);
            let o = rock(4, 2);
            let p = placement(&o, &m);
            let sort_y = p.centre.y - p.feet_offset;
            let bottom = blocked_area(&o, &m, Group::ALONE).min.y;
            assert!(
                (sort_y - (bottom + SORT_BIAS)).abs() < 0.001,
                "draw {draw}: sort {sort_y}, blocked bottom {bottom}"
            );
        }
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
    fn the_blocked_area_is_the_inset_footprint_grown_sideways_by_players_half_width() {
        let a = group_area(&rock(0, 0), &map(128.0, 0.0), false);
        // Footprint 128, inset 12.8 left, right and top, none at the bottom, then `PLAYER_HALF_WIDTH` wider on each side.
        assert_eq!(a.min, MAP_ORIGIN + Vec2::new(12.8 - PLAYER_HALF_WIDTH, 0.0));
        assert_eq!(
            a.max,
            MAP_ORIGIN + Vec2::new(115.2 + PLAYER_HALF_WIDTH, 115.2)
        );
    }

    #[test]
    fn the_blocked_area_follows_the_cell_and_the_jitter() {
        let plain = group_area(&rock(3, 2), &map(128.0, 0.0), false);
        assert_eq!(
            plain.min,
            MAP_ORIGIN + Vec2::new(192.0 + 12.8 - PLAYER_HALF_WIDTH, 128.0)
        );
        let jittered = group_area(&rock(3, 2), &map(128.0, 8.0), false);
        assert_eq!(jittered.size(), plain.size());
        assert!((jittered.min - plain.min).abs().max_element() <= 8.0);
    }

    #[test]
    fn rocks_in_the_middle_of_a_pile_are_interior_and_the_rim_is_not() {
        let m = map(128.0, 0.0);
        let rocks: Vec<MapObject> = (0..5)
            .flat_map(|y| (0..5).map(move |x| rock(x, y)))
            .collect();
        let areas: Vec<Rect> = rocks.iter().map(|o| group_area(o, &m, false)).collect();
        let clusters = outline::build(&areas);
        assert_eq!(clusters.len(), 1);
        let interior = &clusters[0].interior;
        assert!(interior.contains(&12), "the centre rock (2, 2) is interior");
        for rim in [0, 4, 20, 24, 2, 10, 14, 22] {
            assert!(!interior.contains(&rim), "rock {rim} is on the rim");
        }
    }

    #[test]
    fn neighbouring_cells_merge_and_distant_rocks_do_not() {
        let m = map(128.0, 0.0);
        let pile: Vec<Rect> = [rock(0, 0), rock(1, 0), rock(0, 1), rock(1, 1)]
            .iter()
            .map(|o| group_area(o, &m, false))
            .collect();
        assert_eq!(outline::build(&pile).len(), 1);
        // Rocks 3 cells apart leave a gap the player can walk through, 4 cells apart certainly.
        let apart: Vec<Rect> = [rock(0, 0), rock(3, 0)]
            .iter()
            .map(|o| group_area(o, &m, false))
            .collect();
        assert_eq!(outline::build(&apart).len(), 2);
    }
    #[test]
    fn a_rock_alone_blocks_only_its_bottom_tenth() {
        let m = map(128.0, 0.0);
        let a = blocked_area(&rock(0, 0), &m, Group::ALONE);
        let full = group_area(&rock(0, 0), &m, false);
        assert_eq!((a.min, a.max.x), (full.min, full.max.x));
        assert_eq!(a.max.y, MAP_ORIGIN.y + 12.8);
    }

    #[test]
    fn a_rock_with_a_neighbour_above_reaches_the_top_of_its_footprint() {
        let m = map(128.0, 0.0);
        let closed = group_area(&rock(0, 0), &m, false);
        let open = group_area(&rock(0, 0), &m, true);
        assert_eq!(closed.max.y, MAP_ORIGIN.y + 115.2);
        assert_eq!(open.max.y, MAP_ORIGIN.y + 128.0);
    }

    #[test]
    fn rocks_two_cells_apart_form_one_cluster_in_every_direction_and_three_apart_do_not() {
        let m = Map {
            objects: vec![rock(0, 0), rock(2, 0), rock(0, 2), rock(2, 2)],
            ..map(128.0, 0.0)
        };
        let areas: Vec<Rect> = m
            .objects
            .iter()
            .zip(blocking_groups(&m))
            .map(|(o, g)| blocked_area(o, &m, g))
            .collect();
        assert_eq!(outline::build(&areas).len(), 1);
        let lone = Map {
            objects: vec![rock(0, 0), rock(3, 0)],
            ..map(128.0, 0.0)
        };
        let areas: Vec<Rect> = lone
            .objects
            .iter()
            .zip(blocking_groups(&lone))
            .map(|(o, g)| blocked_area(o, &lone, g))
            .collect();
        assert_eq!(outline::build(&areas).len(), 2);
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
