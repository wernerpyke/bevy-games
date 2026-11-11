//! Spawns the rocks of the current map (`map.rs`). Each map character is one rock whose footprint is
//! 2x2 cells with its bottom left corner at the character's cell. The art is the native 32x32 pixel art,
//! scaled by a whole number so every art pixel stays the same size on screen.

use bevy::prelude::*;
use game_kit::animation::tag_sprite;

use crate::collision::PLAYER_HALF_WIDTH;
use crate::map::{Group, Kind, blocking_groups};
use game_kit::collision::{Blocker, CollisionWorld};
use game_kit::depth::DepthSort;
use game_kit::map::{Map, MapObject};
use game_kit::map_transition::{MapId, MapLoad};

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

/// Marker for a rock. The collision code queries it.
#[derive(Component)]
pub struct Rock;

/// The area a rock blocks. `rebuild_collision` adds it to the collision world.
#[derive(Component)]
pub struct RockArea(Rect);

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

fn placement(object: &MapObject, map: &Map, origin: Vec2) -> Placement {
    let scale = (draw_size(map) / ART_PIXELS).round().max(1.0);
    let draw = scale * ART_PIXELS;
    let footprint = FOOTPRINT_CELLS * map.cell_size;
    let corner = footprint_corner(object, map, origin);
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
pub fn footprint_corner(object: &MapObject, map: &Map, origin: Vec2) -> Vec2 {
    origin + object.cell.as_vec2() * map.cell_size + jitter_offset(object, map.jitter)
}

/// The area the player's feet may not enter for an object in a group: the footprint inset by `INSET_FRACTION` on the left, right
/// and top and by `BOTTOM_INSET_FRACTION` at the bottom, then grown sideways by the player's half width. With `open_top` there
/// is no inset at the top, so it merges with the object above (`Group::neighbour_above`). There is no vertical margin.
pub fn group_area(object: &MapObject, map: &Map, origin: Vec2, open_top: bool) -> Rect {
    let corner = footprint_corner(object, map, origin);
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
fn blocked_area(object: &MapObject, map: &Map, origin: Vec2, group: Group) -> Rect {
    let full = group_area(object, map, origin, group.neighbour_above);
    if group.is_group() {
        return full;
    }
    let corner = footprint_corner(object, map, origin);
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

/// Spawns the rocks of one map. The blocked areas are added to the collision world by `rebuild_collision`, once the
/// trees exist too.
pub fn spawn_rocks(commands: &mut Commands, server: &AssetServer, map: &Map, load: &MapLoad) {
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

    for (object, group) in rocks {
        let tag = match Kind::of(object) {
            Kind::Rock0 => "rock_0",
            Kind::Rock1 => "rock_1",
            _ => unreachable!("only rocks are spawned here"),
        };
        let placement = placement(object, map, load.origin);
        commands.spawn((
            Rock,
            MapId(load.id.clone()),
            RockArea(blocked_area(object, map, load.origin, group)),
            tag_sprite(server, "art/features/Rocks.aseprite", tag),
            DepthSort {
                feet_offset: placement.feet_offset,
            },
            Transform::from_translation(placement.centre.extend(0.0))
                .with_scale(Vec3::splat(placement.scale)),
        ));
    }
}

/// Fills the collision world from the entities that exist: the rock areas first, then the tree trunks, so a trunk that
/// overlaps a rock joins its cluster. Then it marks the rocks that touch no empty space. The kit runs it after every
/// load and unload.
pub fn rebuild_collision(
    mut commands: Commands,
    mut world: ResMut<CollisionWorld>,
    rocks: Query<(Entity, &RockArea)>,
    blockers: Query<&Blocker>,
) {
    world.clear();
    let rocks: Vec<(Entity, Rect)> = rocks
        .iter()
        .map(|(entity, area)| (entity, area.0))
        .collect();
    let areas: Vec<Rect> = rocks.iter().map(|(_, area)| *area).collect();
    let first_rock = world.add_blocked(&areas);
    let trunks: Vec<Rect> = blockers.iter().map(|blocker| blocker.0).collect();
    world.add_blocked(&trunks);

    let mut interior = 0;
    for (index, (entity, _)) in rocks.iter().enumerate() {
        if world.is_interior(first_rock + index) {
            commands.entity(*entity).insert(InteriorRock);
            interior += 1;
        } else {
            commands.entity(*entity).remove::<InteriorRock>();
        }
    }
    info!("rocks: {} of {} rocks are interior", interior, rocks.len());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{map, object};

    /// Cell 40, so the 2x2 footprint (80) is not the drawn size, and cell (0, 0) at (10, 20).
    const CELL: f32 = 40.0;
    const ORIGIN: Vec2 = Vec2::new(10.0, 20.0);

    fn rock_map(draw: Option<f32>) -> (Map, MapObject) {
        let rock = object("rock_0", 2, 1, &[]);
        let mut map = map(CELL, vec![rock.clone()]);
        map.draw_size = draw;
        (map, rock)
    }

    #[test]
    fn a_rock_is_drawn_at_a_whole_scale_with_its_visible_bottom_on_the_footprint() {
        // The footprint corner is (10 + 2 * 40, 20 + 1 * 40) = (90, 60). The canvas centre is 3/8 of the drawn size
        // above it, and the sort line 1 pixel above the bottom of the footprint.
        for (draw, scale) in [
            (Some(100.0), 3.0),
            (Some(112.0), 4.0),
            (Some(10.0), 1.0),
            (None, 4.0),
        ] {
            let (map, rock) = rock_map(draw);
            let above = 0.375 * scale * ART_PIXELS;
            assert_eq!(
                placement(&rock, &map, ORIGIN),
                Placement {
                    centre: Vec2::new(90.0 + 40.0, 60.0 + above),
                    scale,
                    feet_offset: above - SORT_BIAS,
                },
                "{draw:?}"
            );
        }
    }

    #[test]
    fn a_rock_in_a_group_blocks_its_inset_footprint_and_reaches_the_rock_above_with_an_open_top() {
        // Inset 8 (a tenth of 80) on the left, right and top, then widened by the player's half width.
        let (map, rock) = rock_map(None);
        let (left, right) = (
            90.0 + 8.0 - PLAYER_HALF_WIDTH,
            170.0 - 8.0 + PLAYER_HALF_WIDTH,
        );
        let in_group = |neighbour_above| Group {
            size: 2,
            neighbour_above,
        };
        assert_eq!(
            blocked_area(&rock, &map, ORIGIN, in_group(false)),
            Rect::new(left, 60.0, right, 140.0 - 8.0)
        );
        assert_eq!(
            blocked_area(&rock, &map, ORIGIN, in_group(true)),
            Rect::new(left, 60.0, right, 140.0)
        );
    }

    #[test]
    fn a_rock_that_stands_alone_blocks_only_a_band_at_its_bottom() {
        let (map, rock) = rock_map(None);
        let alone = Group {
            size: 1,
            neighbour_above: false,
        };
        let area = blocked_area(&rock, &map, ORIGIN, alone);
        assert_eq!(
            area,
            Rect::new(
                90.0 + 8.0 - PLAYER_HALF_WIDTH,
                60.0,
                170.0 - 8.0 + PLAYER_HALF_WIDTH,
                60.0 + 8.0
            )
        );
    }

    #[test]
    fn the_jitter_stays_in_its_range_and_differs_between_neighbour_cells() {
        let jitter = 6.0;
        let offset = |x, y| jitter_offset(&object("rock_0", x, y, &[]), jitter);
        let mut seen = Vec::new();
        for x in -3..12 {
            for y in -3..12 {
                let at = offset(x, y);
                assert!(at.abs().max_element() <= jitter, "({x}, {y}): {at}");
                seen.push(at);
            }
        }
        // Both signs on both axes, so the range is used on each side of 0.
        assert!(
            seen.iter().any(|at| at.x < -jitter / 2.0) && seen.iter().any(|at| at.x > jitter / 2.0)
        );
        assert!(
            seen.iter().any(|at| at.y < -jitter / 2.0) && seen.iter().any(|at| at.y > jitter / 2.0)
        );
        assert_ne!(offset(3, 5), offset(4, 5));
        assert_ne!(offset(3, 5), offset(3, 6));
        assert_ne!(offset(3, 5).x, offset(3, 5).y);
        assert_eq!(jitter_offset(&object("rock_0", 3, 5, &[]), 0.0), Vec2::ZERO);
    }
}
