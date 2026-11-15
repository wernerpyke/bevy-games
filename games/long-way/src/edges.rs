//! The map edges (`adr-x49`): the kit chooses the pieces, the quarter pieces, the sprites and the blocked areas
//! (`game_kit::map_edge`). This file draws them from `art/features/map_edge_ground.aseprite`, one tag per piece with the
//! name of the piece, on the ground below the world, and blocks them for the player's feet.

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;
use game_kit::collision::Blocker;
use game_kit::layers::{EDGE_GROUND_Z, EDGE_HALF_Z_OFFSET};
use game_kit::map::Map;
use game_kit::map_edge::{EdgeFit, EdgeMode, edge_blocked_rects, edge_quarters, edge_sprites};
use game_kit::map_transition::{MapId, MapLoad};

use crate::collision::PLAYER_HALF_WIDTH;

/// Long Way draws smooth edges: the quarter rules round the corners with half-size pieces.
pub const EDGE_MODE: EdgeMode = EdgeMode::Smooth;

/// The stair step of a corner, in world pixels. Tuned by playing.
pub const EDGE_STEP: f32 = 16.0;

/// What the kit needs to place the player on an edge cell.
pub const EDGE_FIT: EdgeFit = EdgeFit {
    step: EDGE_STEP,
    half_width: PLAYER_HALF_WIDTH,
};

const EDGE_FILE: &str = "art/features/map_edge_ground.aseprite";

/// Size of one frame of the edge art, in art pixels.
const EDGE_ART_SIZE: f32 = 64.0;

/// Spawns the edge sprites and blockers of one map. Everything is tagged with the map id, so the kit removes it with
/// the map.
pub fn spawn_edges(commands: &mut Commands, server: &AssetServer, map: &Map, load: &MapLoad) {
    let quarters = edge_quarters(EDGE_MODE, map.size, &map.edges);
    let aseprite: Handle<Aseprite> = server.load(EDGE_FILE);
    for sprite in edge_sprites(&map.edges, &quarters) {
        let place = sprite.place(load.origin, map.cell_size, EDGE_ART_SIZE);
        let z = if sprite.half {
            EDGE_GROUND_Z + EDGE_HALF_Z_OFFSET
        } else {
            EDGE_GROUND_Z
        };
        commands.spawn((
            MapId(load.id.clone()),
            AseAnimation {
                aseprite: aseprite.clone(),
                animation: Animation::tag(sprite.piece.name()),
            },
            Sprite {
                rect: place.source,
                ..default()
            },
            Transform::from_translation(place.centre.extend(z)).with_scale(Vec3::new(
                place.scale,
                place.scale,
                1.0,
            )),
        ));
    }
    for rect in edge_blocked_rects(map, load.origin, &quarters, EDGE_STEP) {
        commands.spawn((
            MapId(load.id.clone()),
            Blocker(Rect::new(
                rect.min.x - PLAYER_HALF_WIDTH,
                rect.min.y,
                rect.max.x + PLAYER_HALF_WIDTH,
                rect.max.y,
            )),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_kit::map::parse;

    /// A ring with a bump on the left (a cut at its tip) and a step down on the top right (fills in the inside
    /// corners), drawn with cells of 32.
    fn bumpy_map() -> Map {
        let source = "---\nmap: t\ncell: 32\norigin: bottom-left\nsize: 6x6\nlegend:\n  \".\": edge\n---\n 5 |. . . . . .|\n 4 |.       . .|\n 3 |. .       .|\n 2 |. . .     .|\n 1 |. .       .|\n 0 |. . . . . .|\n";
        parse("t.txt", source, &[]).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn the_kit_sprites_are_spawned_on_the_ground_and_the_blocked_areas_grown_by_the_player_half_width()
     {
        let map = bumpy_map();
        let load = MapLoad {
            id: "m".to_string(),
            origin: Vec2::new(-40.0, 24.0),
        };
        let quarters = edge_quarters(EDGE_MODE, map.size, &map.edges);
        assert!(quarters.iter().any(|q| q.cut) && quarters.iter().any(|q| !q.cut));
        let mut expected: Vec<(String, Vec3, f32, Option<Rect>)> =
            edge_sprites(&map.edges, &quarters)
                .into_iter()
                .map(|s| {
                    let place = s.place(load.origin, 32.0, EDGE_ART_SIZE);
                    let z = if s.half {
                        EDGE_GROUND_Z + EDGE_HALF_Z_OFFSET
                    } else {
                        EDGE_GROUND_Z
                    };
                    (
                        s.piece.name().to_string(),
                        place.centre.extend(z),
                        place.scale,
                        place.source,
                    )
                })
                .collect();
        let expected_blocked: Vec<Rect> =
            edge_blocked_rects(&map, load.origin, &quarters, EDGE_STEP)
                .into_iter()
                .map(|r| {
                    Rect::new(
                        r.min.x - PLAYER_HALF_WIDTH,
                        r.min.y,
                        r.max.x + PLAYER_HALF_WIDTH,
                        r.max.y,
                    )
                })
                .collect();

        let mut app = crate::test_util::asset_app(16);
        let spawn_load = load.clone();
        crate::test_util::spawn_with(&mut app, move |commands, server| {
            spawn_edges(commands, server, &map, &spawn_load);
        });
        let world = app.world_mut();
        let mut sprites: Vec<(String, Vec3, f32, Option<Rect>)> = world
            .query_filtered::<(&AseAnimation, &Sprite, &Transform), With<MapId>>()
            .iter(world)
            .map(|(ase, sprite, transform)| {
                let tag = ase.animation.tag.clone().unwrap_or_default();
                (tag, transform.translation, transform.scale.x, sprite.rect)
            })
            .collect();
        let key = |s: &(String, Vec3, f32, Option<Rect>)| {
            (
                s.0.clone(),
                (s.1.x * 100.0) as i64,
                (s.1.y * 100.0) as i64,
                (s.1.z * 100.0) as i64,
            )
        };
        sprites.sort_by_key(key);
        expected.sort_by_key(key);
        assert_eq!(sprites, expected);
        let blocked: Vec<Rect> = world.query::<&Blocker>().iter(world).map(|b| b.0).collect();
        assert_eq!(blocked, expected_blocked);
    }
}
