//! Spawns the lanterns and markers of the current map (`map.rs`). A lantern or marker is 70% of the player's body height. Its art is drawn at its own size (scale 1),
//! centred in the cell whose bottom left corner is the character's cell. It blocks the middle 80% of its width and the bottom half of its
//! height (`adr-lw20`).

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;

use crate::map::Kind;
use crate::rocks::{SORT_BIAS, footprint_corner};
use game_kit::collision::Blocker;
use game_kit::depth::DepthSort;
use game_kit::map::{Map, MapObject};
use game_kit::map_transition::{ExitTrigger, MapId, MapLoad};

/// Size of `Markers.aseprite` in pixels. The height is 70% of `PLAYER_BODY_HEIGHT` (50), and the width keeps the aspect of the source.
const MARKER_WIDTH: f32 = 28.0;
const MARKER_HEIGHT: f32 = 35.0;
/// Part of the lantern height, from the bottom, that blocks the player.
const BLOCKED_FRACTION: f32 = 0.5;
/// Part of the lantern width, centred, that blocks the player.
const BLOCKED_WIDTH_FRACTION: f32 = 0.8;
/// How far the touch area of an exit marker reaches beyond the blocked area. The feet cannot enter the blocked area, so
/// "touching" means reaching its edge.
const TOUCH_MARGIN: f32 = 2.0;

fn tag(kind: Kind) -> Option<&'static str> {
    match kind {
        Kind::Lantern => Some("lantern"),
        Kind::LanternRed => Some("lantern_red"),
        Kind::Marker => Some("marker"),
        Kind::MarkerRed => Some("marker_red"),
        _ => None,
    }
}

/// The area the player's feet may not enter for a lantern whose cell corner is `corner`: the middle 80% of the lantern's width
/// and the bottom half of its height, both rounded to whole pixels. It is not widened by the player, so the drawn box is what blocks.
fn blocked_area(corner: Vec2, cell: f32) -> Rect {
    let width = (BLOCKED_WIDTH_FRACTION * MARKER_WIDTH).round();
    let left = (corner.x + (cell - width) / 2.0).round();
    Rect::new(
        left,
        corner.y,
        left + width,
        corner.y + (BLOCKED_FRACTION * MARKER_HEIGHT).round(),
    )
}

/// Spawns the lanterns and markers of one map. Their areas are blockers, which `rebuild_collision` adds to the collision world.
pub fn spawn_markers(commands: &mut Commands, server: &AssetServer, map: &Map, load: &MapLoad) {
    for object in &map.objects {
        let Some(tag) = tag(Kind::of(object)) else {
            continue;
        };
        spawn_marker(commands, server, tag, object, map, load);
    }
}

fn spawn_marker(
    commands: &mut Commands,
    server: &AssetServer,
    tag: &'static str,
    object: &MapObject,
    map: &Map,
    load: &MapLoad,
) {
    let corner = footprint_corner(object, map, load.origin);
    // Whole pixel left edge, so the art pixels line up with the screen pixels.
    let left = (corner.x + (map.cell_size - MARKER_WIDTH) / 2.0).round();
    let centre = Vec2::new(
        left + MARKER_WIDTH / 2.0,
        corner.y.round() + MARKER_HEIGHT / 2.0,
    );
    let blocked = blocked_area(corner, map.cell_size);
    let mut entity = commands.spawn((
        MapId(load.id.clone()),
        Blocker(blocked),
        DepthSort {
            feet_offset: MARKER_HEIGHT / 2.0 - SORT_BIAS,
        },
        AseAnimation {
            aseprite: server.load("art/features/Markers.aseprite"),
            animation: Animation::tag(tag),
        },
        Sprite::default(),
        Transform::from_translation(centre.extend(0.0)),
    ));
    // `exit_to` in the legend makes the marker an exit (`adr-x33`).
    if let Some(to) = object.exit_to() {
        entity.insert(ExitTrigger {
            area: Rect::from_corners(
                blocked.min - Vec2::splat(TOUCH_MARGIN),
                blocked.max + Vec2::splat(TOUCH_MARGIN),
            ),
            to: to.to_string(),
        });
    }
}
