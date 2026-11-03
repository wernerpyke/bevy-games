//! Spawns the trees of the current map (`map.rs`). A tree stands on its cell: the bottom centre of its art is the
//! bottom centre of the cell. Only the base blocks the hero (`adr-rt5`), so the hero walks behind the rest.

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;
use game_kit::collision::Blocker;
use game_kit::depth::DepthSort;
use game_kit::map::MapObject;

use crate::camera::CELL_SIZE;
use crate::collision::{BASE_BAND, FEET_HALF_WIDTH};
use crate::map::{CurrentMap, Kind, MapEntity};

/// Both art files are packed with the same canvas: 30 wide and 39 high, each tree at the bottom centre
/// (`dev-scripts/aseprite/pack_pngs.lua`).
const CANVAS_HEIGHT: f32 = 39.0;
/// A tree sorts this far above its base, so the hero standing on the base line is drawn in front (as `adr-lw13`).
const SORT_BIAS: f32 = 1.0;

pub struct TreesPlugin;

impl Plugin for TreesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_trees);
    }
}

/// The art and the base of a tree kind.
struct TreeArt {
    /// Aseprite tag in `art/features/background-elements.aseprite`.
    tag: &'static str,
    /// The blocking base: its left and right edge in pixels from the centre of the tree, measured in the art.
    base: (f32, f32),
}

fn art(kind: Kind) -> TreeArt {
    match kind {
        // `bg_18`: the base of the big tree is columns 6 to 22 of 30.
        Kind::TreeBig => TreeArt {
            tag: "bg_18",
            base: (-9.0, 8.0),
        },
        // `bg_19`: the trunk of the dead tree is columns 3 to 9 of 14.
        Kind::TreeDead => TreeArt {
            tag: "bg_19",
            base: (-4.0, 3.0),
        },
    }
}

/// The world position of the bottom centre of the cell an object stands on.
fn base_of(object: &MapObject) -> Vec2 {
    object.cell.as_vec2() * CELL_SIZE + Vec2::new(CELL_SIZE / 2.0, 0.0)
}

/// The area the hero's feet may not enter: the base of the tree grown sideways by the feet half width, 4 px deep.
fn blocked_area(base: Vec2, edges: (f32, f32)) -> Rect {
    Rect::new(
        base.x + edges.0 - FEET_HALF_WIDTH,
        base.y,
        base.x + edges.1 + FEET_HALF_WIDTH,
        base.y + BASE_BAND,
    )
}

pub fn spawn_trees(mut commands: Commands, server: Res<AssetServer>, map: Res<CurrentMap>) {
    for object in &map.0.objects {
        let art = art(Kind::of(object));
        let base = base_of(object);
        commands
            .spawn((
                MapEntity,
                Blocker(blocked_area(base, art.base)),
                DepthSort {
                    feet_offset: -SORT_BIAS,
                },
                Transform::from_translation(base.extend(0.0)),
                Visibility::default(),
            ))
            .with_child((
                AseAnimation {
                    aseprite: server.load("art/features/background-elements.aseprite"),
                    animation: Animation::tag(art.tag),
                },
                Sprite::default(),
                Transform::from_xyz(0.0, CANVAS_HEIGHT / 2.0, 0.0),
            ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tree_stands_on_the_bottom_centre_of_its_cell() {
        let object = MapObject {
            kind: "tree_big".to_string(),
            cell: IVec2::new(2, 3),
        };
        assert_eq!(base_of(&object), Vec2::new(40.0, 48.0));
    }

    #[test]
    fn the_blocked_area_is_the_base_grown_by_the_feet_half_width() {
        let area = blocked_area(Vec2::new(40.0, 48.0), (-9.0, 8.0));
        assert_eq!(area, Rect::new(26.0, 48.0, 53.0, 52.0));
    }
}
