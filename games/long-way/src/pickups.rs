//! Spawns the energy pickups of the current map (`map.rs`). A pickup does not block. The kit removes it and sends
//! `Collected` when the player's feet touch the cell it stands in (`game_kit::collectable`). The art is
//! `pickups/pickups.aseprite` (one tag per colour, animation speed set in the file).

use bevy::prelude::*;
use game_kit::animation::{RandomStartFrame, tag_animation};

use crate::map::Kind;
use crate::rocks::{SORT_BIAS, footprint_corner};
use game_kit::collectable::Collectable;
use game_kit::depth::DepthSort;
use game_kit::map::{Map, MapObject};
use game_kit::map_transition::{MapId, MapLoad};

/// Size of the canvas of `pickups.aseprite` in art pixels.
const ART_WIDTH: f32 = 64.0;
const ART_HEIGHT: f32 = 96.0;
/// Drawn size as a multiple of the art size.
const PICKUP_SCALE: f32 = 0.25;
/// The drawn size of a pickup in art pixels. The HUD icons use the same size.
pub const DRAWN_SIZE: Vec2 = Vec2::new(ART_WIDTH * PICKUP_SCALE, ART_HEIGHT * PICKUP_SCALE);
/// The touch box: 52 wide (the touch width of an exit marker, `markers.rs`) and one cell high, centred on the bottom of the cell. It reaches 32 pixels below the base of the pickup, so a player walking past in front of it collects it.
const TOUCH_SIZE: Vec2 = Vec2::new(52.0, 64.0);
const TOUCH_CENTRE_Y: f32 = 0.0;

/// The animation tag and the legend name of a pickup kind.
fn art(kind: Kind) -> Option<(&'static str, &'static str)> {
    match kind {
        Kind::EnergyBlue => Some(("pickup_blue", "energy_blue")),
        Kind::EnergyRed => Some(("pickup_red", "energy_red")),
        Kind::EnergyViolet => Some(("pickup_violet", "energy_violet")),
        Kind::EnergyBlack => Some(("pickup_black", "energy_black")),
        _ => None,
    }
}

/// Spawns the pickups of one map. They carry the map id, so the kit removes them with the map.
pub fn spawn_pickups(commands: &mut Commands, server: &AssetServer, map: &Map, load: &MapLoad) {
    for object in &map.objects {
        let Some((tag, name)) = art(Kind::of(object)) else {
            continue;
        };
        spawn_pickup(commands, server, tag, name, object, map, load);
    }
}

fn spawn_pickup(
    commands: &mut Commands,
    server: &AssetServer,
    tag: &'static str,
    name: &'static str,
    object: &MapObject,
    map: &Map,
    load: &MapLoad,
) {
    let corner = footprint_corner(object, map, load.origin);
    let (width, height) = (DRAWN_SIZE.x, DRAWN_SIZE.y);
    let centre = Vec2::new(
        corner.x + map.cell_size / 2.0,
        corner.y.round() + height / 2.0,
    );
    commands.spawn((
        MapId(load.id.clone()),
        Collectable {
            area: Rect::from_center_size(
                Vec2::new(corner.x + map.cell_size / 2.0, corner.y + TOUCH_CENTRE_Y),
                TOUCH_SIZE,
            ),
            kind: name.to_string(),
        },
        DepthSort {
            feet_offset: height / 2.0 - SORT_BIAS,
        },
        tag_animation(server, "art/pickups/pickups.aseprite", tag),
        RandomStartFrame::for_cell(&map.name, object.cell),
        Sprite {
            custom_size: Some(Vec2::new(width, height)),
            ..default()
        },
        Transform::from_translation(centre.extend(0.0)),
    ));
}
