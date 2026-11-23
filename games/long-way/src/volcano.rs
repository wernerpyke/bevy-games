//! The volcano: an enemy that stands still where a `spawn_volcano` marker of a map puts it (legend entry such as
//! `"^": spawn_volcano`) and attacks until it is killed. For now it only loops its `Attack` tag
//! (`characters/volcano.aseprite`, durations set in the file). It does not block, cannot be hit and launches nothing
//! yet: the projectile and hit points are still to come.

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;
use game_kit::depth::DepthSort;
use game_kit::map::Map;
use game_kit::map_transition::{MapId, MapLoad};

use crate::map::Kind;

const ART: &str = "art/characters/volcano.aseprite";
/// The layers drawn in the game. The file also has `test_bg`, a backdrop for drawing that is never drawn in the game,
/// even when it is saved visible. A new art layer must be added here.
const LAYERS: [&str; 3] = ["base", "rim", "lava"];
/// The volcano is drawn at this many times the art size. The file stays at the size it is drawn at (`adr-a11`).
const SCALE: f32 = 2.0;
/// The art canvas is 64 by 64 and the lowest row of the body is row 62 of 64, so the bottom of the body is 31 art
/// pixels below the centre.
const FEET_BELOW_CENTRE: f32 = 31.0 * SCALE;
/// The bottom of the body stands this far below the middle of the cell, whatever the scale: 31 px, 1 px above the
/// bottom of a 64 px cell.
const FEET_BELOW_CELL_MIDDLE: f32 = 31.0;

/// The root of a volcano. Its `Transform` is the centre of the sprite.
#[derive(Component)]
pub struct Volcano;

/// Spawns the volcanoes of one map at the middle of their cells. They carry the map id, so the kit removes them with
/// the map.
pub fn spawn_volcanoes(commands: &mut Commands, server: &AssetServer, map: &Map, load: &MapLoad) {
    for object in &map.objects {
        if Kind::of(object) != Kind::SpawnVolcano {
            continue;
        }
        // Whole pixels, so the art pixels line up with the screen pixels.
        let cell_middle =
            (load.origin + (object.cell.as_vec2() + Vec2::splat(0.5)) * map.cell_size).round();
        let centre = cell_middle + Vec2::Y * (FEET_BELOW_CENTRE - FEET_BELOW_CELL_MIDDLE);
        commands.spawn((
            Volcano,
            MapId(load.id.clone()),
            DepthSort {
                feet_offset: FEET_BELOW_CENTRE,
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
