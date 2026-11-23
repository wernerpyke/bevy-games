//! The volcano: an enemy that stands still where a `spawn_volcano` marker of a map puts it (legend entry such as
//! `"^": spawn_volcano`) and attacks until it is killed. For now it only loops its `Attack` tag
//! (`characters/volcano.aseprite`, durations set in the file). Its base blocks the player, who can walk up to it and
//! behind it: only its projectiles will hurt. It cannot be hit and launches nothing yet: the projectile and hit points
//! are still to come.

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;
use game_kit::collision::Blocker;
use game_kit::depth::DepthSort;
use game_kit::map::Map;
use game_kit::map_transition::{MapId, MapLoad};

use crate::collision::PLAYER_HALF_WIDTH;
use crate::map::Kind;
use crate::rocks::SORT_BIAS;

const ART: &str = "art/characters/volcano.aseprite";
/// The layers drawn in the game. The file also has `test_bg`, a backdrop for drawing that is never drawn in the game,
/// even when it is saved visible. A new art layer must be added here.
const LAYERS: [&str; 3] = ["base", "rim", "lava"];
/// The volcano is drawn at this many times the art size. The file stays at the size it is drawn at (`adr-a11`).
const SCALE: f32 = 2.0;
/// The art canvas is 64 by 64 and the lowest row of the body is row 62 of 64, so the bottom of the body is 31 art
/// pixels below the centre.
const FEET_BELOW_CENTRE: f32 = 31.0 * SCALE;
/// The base of the cone in art pixels from the left and top of the canvas: x 11 to 53 is the main cone (the thin
/// skirt reaches x 5 to 62 at row 50), and rows 50 to 62 are the bottom, where the skirt is widest. The feet may not
/// enter it, so the player stops at the bottom and the sides and walks behind the cone above row 50.
const BASE_LEFT_PX: f32 = 11.0;
const BASE_RIGHT_PX: f32 = 54.0;
const BASE_TOP_PX: f32 = 50.0;
const BASE_BOTTOM_PX: f32 = 63.0;
const CANVAS_HALF_PX: f32 = 32.0;

/// The root of a volcano. Its `Transform` is the centre of the sprite.
#[derive(Component)]
pub struct Volcano;

/// The area the feet may not enter for a volcano whose bottom is at `feet`: the base of the cone, scaled, and widened
/// on each side by half the player's width, as a tree trunk is.
fn blocked_area(feet: Vec2) -> Rect {
    let x = |px: f32| feet.x + (px - CANVAS_HALF_PX) * SCALE;
    Rect::new(
        x(BASE_LEFT_PX) - PLAYER_HALF_WIDTH,
        feet.y,
        x(BASE_RIGHT_PX) + PLAYER_HALF_WIDTH,
        feet.y + (BASE_BOTTOM_PX - BASE_TOP_PX) * SCALE,
    )
}

/// Spawns the volcanoes of one map with the bottom of the body on the bottom middle of their cells. They carry the map
/// id, so the kit removes them with the map. Their bases are blockers, which `rebuild_collision` adds to the collision
/// world.
pub fn spawn_volcanoes(commands: &mut Commands, server: &AssetServer, map: &Map, load: &MapLoad) {
    for object in &map.objects {
        if Kind::of(object) != Kind::SpawnVolcano {
            continue;
        }
        // Whole pixels, so the art pixels line up with the screen pixels.
        let feet =
            (load.origin + (object.cell.as_vec2() + Vec2::new(0.5, 0.0)) * map.cell_size).round();
        let centre = feet + Vec2::Y * FEET_BELOW_CENTRE;
        commands.spawn((
            Volcano,
            MapId(load.id.clone()),
            Blocker(blocked_area(feet)),
            // The player stops on the bottom of the blocked area, so the volcano sorts just above it, like a rock.
            DepthSort {
                feet_offset: FEET_BELOW_CENTRE - SORT_BIAS,
            },
            AseAnimation {
                aseprite: server
                    .load_builder()
                    .with_settings::<AsepriteLoaderSettings>(|settings| {
                        settings.layer_selection =
                            LayerSelectionSetting::Mask(LAYERS.map(String::from).to_vec());
                    })
                    .load(ART),
                animation: Animation::tag("Attack").with_repeat(AnimationRepeat::Loop),
            },
            Sprite::default(),
            Transform::from_translation(centre.extend(0.0))
                .with_scale(Vec3::new(SCALE, SCALE, 1.0)),
        ));
        debug!("volcano: placed at {centre:?} in map {}", load.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{asset_app, map, object, spawn_with};

    #[test]
    fn a_marker_puts_a_volcano_on_the_bottom_of_its_cell_on_whole_pixels_with_its_base_blocked_and_sorted_above_it()
     {
        let mut app = asset_app(16);
        let map = map(
            48.0,
            vec![
                object("spawn_volcano", 2, 3, &[]),
                object("rock_0", 0, 0, &[]),
            ],
        );
        // The feet are at x 10.4 + 2.5 * 48 = 130.4, so 130, and y 20 + 3 * 48 = 164.
        let load = MapLoad {
            id: "m".to_string(),
            origin: Vec2::new(10.4, 20.0),
        };
        spawn_with(&mut app, move |commands, server| {
            spawn_volcanoes(commands, server, &map, &load);
        });
        let volcanoes: Vec<(Vec2, Rect, f32, String)> = app
            .world_mut()
            .query_filtered::<(&Transform, &Blocker, &DepthSort, &MapId), With<Volcano>>()
            .iter(app.world())
            .map(|(transform, blocker, sort, id)| {
                (
                    transform.translation.truncate(),
                    blocker.0,
                    sort.feet_y(transform.translation.y),
                    id.0.clone(),
                )
            })
            .collect();
        // Centre 31 art px x 2 above the feet. Base x 11 and 54 of the 64 px canvas, so -21 and +22 art px x 2 from the
        // feet, widened by 14 on each side: 130 - 42 - 14 = 74 and 130 + 44 + 14 = 188. Rows 50 to 63 are 13 art px x 2.
        // The player stops on the bottom of the blocked area, so the volcano sorts `SORT_BIAS` above it, at 165: a player
        // there is in front whatever its x, not tied with the volcano.
        assert_eq!(
            volcanoes,
            vec![(
                Vec2::new(130.0, 226.0),
                Rect::new(74.0, 164.0, 188.0, 190.0),
                165.0,
                "m".to_string()
            )]
        );
    }
}
