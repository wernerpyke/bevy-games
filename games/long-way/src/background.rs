//! The ground of a map (`background:` block, `adr-x36`, `adr-lw26`): a colour rectangle the size of the map, with tile
//! images over it. The kit reads the block and works out where the images go (`game_kit::map_background`). This file
//! draws them from `art/backgrounds/ground_tiles.aseprite`, one tag per image.

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;
use game_kit::layers::{
    BACKGROUND_TILE_Z_STEP, BACKGROUND_TILES_MAX, BACKGROUND_TILES_Z, BACKGROUND_Z,
};
use game_kit::map::Map;
use game_kit::map_background::place_tiles;
use game_kit::map_transition::{MapId, MapLoad, MapRegistry};

/// The tile images in `ground_tiles.aseprite`, by tag. A map file may list only these names.
pub const TILE_NAMES: &[&str] = &[
    "grass_0",
    "grass_1",
    "grass_2",
    "grass_3",
    "flowers_0",
    "flowers_1",
];

const TILE_FILE: &str = "art/backgrounds/ground_tiles.aseprite";

/// Size of one tile image in the file, in art pixels. The drawn size is this times the `scale` of the map.
const TILE_ART_SIZE: f32 = 32.0;

/// The first tile name in a map's background that the art does not have.
pub fn unknown_tile(map: &Map) -> Option<&str> {
    map.background
        .as_ref()?
        .tiles
        .iter()
        .map(String::as_str)
        .find(|name| !TILE_NAMES.contains(name))
}

/// Stops the game at start when a map lists a tile that does not exist, like a bad map link does.
pub fn check_tiles(registry: Res<MapRegistry>) {
    for id in registry.ids() {
        let Some(registered) = registry.get(id) else {
            continue;
        };
        if let Some(name) = unknown_tile(&registered.map) {
            panic!(
                "{}: line {}: unknown background tile `{name}` (known: {})",
                registered.file,
                registered
                    .map
                    .background
                    .as_ref()
                    .map_or(0, |background| background.line),
                TILE_NAMES.join(", ")
            );
        }
    }
}

/// Spawns the ground of one map. Everything is tagged with the map id, so the kit removes it with the map.
pub fn spawn_background(commands: &mut Commands, server: &AssetServer, map: &Map, load: &MapLoad) {
    let Some(background) = &map.background else {
        return;
    };
    let extent = map.size.as_vec2() * map.cell_size;
    commands.spawn((
        MapId(load.id.clone()),
        Sprite::from_color(Color::from(background.color), extent),
        Transform::from_translation((load.origin + extent / 2.0).extend(BACKGROUND_Z)),
    ));
    // A file reloaded with F1 may name a tile that does not exist. Report it and draw no images.
    if let Some(name) = unknown_tile(map) {
        error!(
            "map: `{}`: unknown background tile `{name}`, no images drawn",
            map.name
        );
        return;
    }
    let image = Vec2::splat(TILE_ART_SIZE * background.scale);
    let aseprite: Handle<Aseprite> = server.load(TILE_FILE);
    for (index, tile) in place_tiles(&map.name, map.size, map.cell_size, background, image)
        .into_iter()
        .take(BACKGROUND_TILES_MAX)
        .enumerate()
    {
        let z = BACKGROUND_TILES_Z + index as f32 * BACKGROUND_TILE_Z_STEP;
        commands.spawn((
            MapId(load.id.clone()),
            AseAnimation {
                aseprite: aseprite.clone(),
                animation: Animation::tag(&background.tiles[tile.tile]),
            },
            Sprite::default(),
            Transform::from_translation((load.origin + tile.centre).extend(z))
                .with_scale(Vec3::new(background.scale, background.scale, 1.0)),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_kit::map::parse;

    fn map_with_tiles(tiles: &str) -> Map {
        let source = format!(
            "---\nmap: t\ncell: 64\norigin: bottom-left\nsize: 1x1\nbackground:\n  color: \"#000000\"\n  mode: random\n  fill: 1\n  tiles: {tiles}\nlegend:\n  \".\": empty\n---\n |0|\n 0 |.|\n"
        );
        parse("t.txt", &source, &[]).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn known_tiles_pass_and_unknown_tiles_are_found() {
        assert_eq!(unknown_tile(&map_with_tiles("grass_0 flowers_1")), None);
        assert_eq!(unknown_tile(&map_with_tiles("grass_0 moss")), Some("moss"));
    }
}
