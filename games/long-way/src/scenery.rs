//! Spawns the Zen Garden trees and shrubs of the current map (`map.rs`). Each map character is one object whose cell
//! space has its bottom left corner at the character's cell. The art is drawn at its own size (scaled 2x in the file).
//! The medium green tree has its canvas bottom left on that corner, like `green_tree`. The other kinds have the bottom
//! middle of the canvas on the bottom middle of the cell space.
//!
//! All of them block like `green_tree` (`Trunk`, `trees.rs`). Alone, an object blocks a foot and a band at its bottom. In a
//! group (`map::blocking_groups`, `adr-lw23`) it blocks exactly its anchor cell, 64 by 64, which holds its trunk or base,
//! so the boxes of neighbours touch with no gap.

use bevy::prelude::*;
use bevy::sprite::Anchor;
use game_kit::animation::all_frames_animation;

use crate::collision::Trunk;
use crate::map::{Group, Kind, blocking_groups};
use crate::rocks::{SORT_BIAS, footprint_corner};
use game_kit::collision::Blocker;
use game_kit::depth::{DepthSort, TieBreakX};
use game_kit::map::Map;
use game_kit::map_transition::{MapId, MapLoad};

/// The foot of the object that blocks the player, as pixels from the left edge of the canvas, and the height the blocked
/// band is taken from. A tree measures the lowest 12 rows of its trunk and the height of its canvas. A shrub has a foot
/// as wide as the one of the red tree of its size, centred on its art, and the height of its drawn box. Both block like
/// `green_tree` (`Trunk`, `trees.rs`).
#[derive(Clone, Copy)]
struct Foot {
    left: f32,
    right: f32,
    height: f32,
}

/// Where the canvas hangs from the base point.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Hang {
    /// The base point is the bottom left corner of the cell space.
    BottomLeft,
    /// The base point is the bottom middle of the cell space, `cells_wide` cells wide.
    BottomMiddle { cells_wide: f32 },
}

struct Art {
    path: &'static str,
    /// Canvas size in pixels.
    size: Vec2,
    hang: Hang,
    foot: Foot,
}

fn art(kind: Kind) -> Option<Art> {
    let art = match kind {
        Kind::GreenTreeMedium => Art {
            path: "art/features/green_tree_medium.aseprite",
            size: Vec2::new(128.0, 128.0),
            hang: Hang::BottomLeft,
            foot: Foot {
                left: 48.0,
                right: 64.0,
                height: 128.0,
            },
        },
        Kind::RedTreeMedium => Art {
            path: "art/features/red_tree_medium.aseprite",
            size: Vec2::new(84.0, 128.0),
            hang: Hang::BottomMiddle { cells_wide: 1.0 },
            foot: Foot {
                left: 34.0,
                right: 48.0,
                height: 128.0,
            },
        },
        Kind::RedTreeSmall => Art {
            path: "art/features/red_tree_small.aseprite",
            size: Vec2::new(64.0, 64.0),
            hang: Hang::BottomMiddle { cells_wide: 1.0 },
            foot: Foot {
                left: 36.0,
                right: 44.0,
                height: 64.0,
            },
        },
        Kind::GreenShrub => Art {
            path: "art/features/green_shrub.aseprite",
            size: Vec2::new(64.0, 64.0),
            hang: Hang::BottomMiddle { cells_wide: 1.0 },
            foot: Foot {
                left: 25.0,
                right: 39.0,
                height: 48.0,
            },
        },
        Kind::GreenShrubSmall => Art {
            path: "art/features/green_shrub_small.aseprite",
            size: Vec2::new(64.0, 64.0),
            hang: Hang::BottomMiddle { cells_wide: 1.0 },
            foot: Foot {
                left: 28.0,
                right: 36.0,
                height: 36.0,
            },
        },
        _ => return None,
    };
    Some(art)
}

/// The base point of the canvas and the world x of its left edge, for an object whose cell space has its bottom left
/// corner at `corner`.
fn placement(art: &Art, corner: Vec2, cell: f32) -> (Vec2, f32) {
    match art.hang {
        Hang::BottomLeft => (corner, corner.x),
        Hang::BottomMiddle { cells_wide } => {
            let base = Vec2::new(
                (corner.x + cells_wide * cell / 2.0).round(),
                corner.y.round(),
            );
            (base, base.x - art.size.x / 2.0)
        }
    }
}

/// The area the player's feet may not enter: the anchor cell when the object is in a group, else its foot and band.
fn blocker(
    art: &Art,
    base: Vec2,
    canvas_left: f32,
    corner: Vec2,
    cell: f32,
    group: Group,
) -> Blocker {
    if group.is_group() {
        return Blocker(Rect::new(
            corner.x,
            corner.y,
            corner.x + cell,
            corner.y + cell,
        ));
    }
    Trunk {
        left: canvas_left + art.foot.left,
        right: canvas_left + art.foot.right,
        base_y: base.y,
        height: art.foot.height,
    }
    .blocker()
}

/// How far the middle of the footprint is from the base point, in x: ties between equal feet break there, as for the
/// big trees, whose `Transform` is the middle of their footprint (`TieBreakX`, `adr-x2`).
fn tie_break(kind: Kind, base: Vec2, corner: Vec2, cell: f32) -> f32 {
    corner.x + kind.footprint().x as f32 * cell / 2.0 - base.x
}

/// Spawns the scenery of one map. The blocked areas are added to the collision world by `rebuild_collision`.
pub fn spawn_scenery(commands: &mut Commands, server: &AssetServer, map: &Map, load: &MapLoad) {
    for (object, group) in map.objects.iter().zip(blocking_groups(map)) {
        let Some(art) = art(Kind::of(object)) else {
            continue;
        };
        let corner = footprint_corner(object, map, load.origin);
        let (base, canvas_left) = placement(&art, corner, map.cell_size);
        let anchor = match art.hang {
            Hang::BottomLeft => Anchor::BOTTOM_LEFT,
            Hang::BottomMiddle { .. } => Anchor::BOTTOM_CENTER,
        };
        commands.spawn((
            MapId(load.id.clone()),
            blocker(&art, base, canvas_left, corner, map.cell_size, group),
            // The feet are the bottom of the canvas, one pixel up like the other objects.
            DepthSort {
                feet_offset: -SORT_BIAS,
            },
            TieBreakX(tie_break(Kind::of(object), base, corner, map.cell_size)),
            all_frames_animation(server, art.path),
            Sprite::default(),
            anchor,
            Transform::from_translation(base.extend(0.0)),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORNER: Vec2 = Vec2::new(128.0, 64.0);
    const ALONE: Group = Group {
        size: 1,
        neighbour_above: false,
    };

    #[test]
    fn medium_green_tree_hangs_from_the_bottom_left_corner() {
        let art = art(Kind::GreenTreeMedium).unwrap();
        assert_eq!(placement(&art, CORNER, 64.0), (CORNER, 128.0));
    }

    #[test]
    fn other_kinds_hang_from_the_bottom_middle_of_their_cell_space() {
        let art = art(Kind::RedTreeMedium).unwrap();
        let (base, left) = placement(&art, CORNER, 64.0);
        assert_eq!(base, Vec2::new(160.0, 64.0));
        assert_eq!(left, 160.0 - 42.0);
    }

    #[test]
    fn an_object_in_a_group_blocks_its_anchor_cell() {
        let grouped = Group {
            size: 3,
            neighbour_above: false,
        };
        for kind in [
            Kind::GreenTreeMedium,
            Kind::RedTreeMedium,
            Kind::GreenShrubSmall,
        ] {
            let art = art(kind).unwrap();
            let (base, left) = placement(&art, CORNER, 64.0);
            let area = blocker(&art, base, left, CORNER, 64.0, grouped).0;
            assert_eq!(area, Rect::new(128.0, 64.0, 192.0, 128.0));
        }
    }

    #[test]
    fn feet_ties_break_at_the_middle_of_the_footprint() {
        // The medium green tree hangs from its left corner over a 2 cell footprint; the others are centred on 1 cell.
        for cell in [64.0, 48.0] {
            for (kind, cells) in [
                (Kind::GreenTreeMedium, 2.0),
                (Kind::RedTreeMedium, 1.0),
                (Kind::GreenShrub, 1.0),
            ] {
                let (base, _) = placement(&art(kind).unwrap(), CORNER, cell);
                let tie_x = base.x + tie_break(kind, base, CORNER, cell);
                assert_eq!(
                    tie_x,
                    CORNER.x + cells * cell / 2.0,
                    "{kind:?} at cell {cell}"
                );
            }
        }
    }

    #[test]
    fn every_scenery_kind_has_art() {
        for kind in [
            Kind::GreenTreeMedium,
            Kind::RedTreeMedium,
            Kind::RedTreeSmall,
            Kind::GreenShrub,
            Kind::GreenShrubSmall,
        ] {
            assert!(art(kind).is_some());
        }
        assert!(art(Kind::GreenTree).is_none());
    }

    #[test]
    fn a_tree_blocks_the_bottom_tenth_of_its_height_around_the_trunk() {
        let art = art(Kind::GreenTreeMedium).unwrap();
        let (base, left) = placement(&art, CORNER, 64.0);
        let area = blocker(&art, base, left, CORNER, 64.0, ALONE).0;
        assert_eq!(area.min, Vec2::new(128.0 + 48.0 - 14.0, 64.0));
        assert_eq!(area.max, Vec2::new(128.0 + 64.0 + 14.0, 64.0 + 12.8));
    }

    #[test]
    fn a_shrub_is_as_wide_as_the_red_tree_of_its_size_and_a_tenth_as_high_as_its_art() {
        let width = |kind| {
            let art = art(kind).unwrap();
            let (base, left) = placement(&art, CORNER, 64.0);
            let area = blocker(&art, base, left, CORNER, 64.0, ALONE).0;
            (area.width(), area.height())
        };
        assert_eq!(width(Kind::GreenShrub).0, width(Kind::RedTreeMedium).0);
        assert_eq!(width(Kind::GreenShrubSmall).0, width(Kind::RedTreeSmall).0);
        assert!((width(Kind::GreenShrubSmall).1 - 3.6).abs() < 1e-3);
    }
}
