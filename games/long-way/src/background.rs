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

    /// A map of 3 by 2 cells of 48, every cell with an image at scale 2.
    fn map_3x2(tiles: &str) -> Map {
        let source = format!(
            "---\nmap: t\ncell: 48\norigin: bottom-left\nsize: 3x2\nbackground:\n  color: \"#336699\"\n  mode: random\n  fill: 1\n  scale: 2\n  tiles: {tiles}\nlegend:\n  \".\": empty\n---\n   |0 1 2|\n 1 |. . .|\n 0 |. . .|\n"
        );
        parse("t.txt", &source, &[]).unwrap_or_else(|e| panic!("{e}"))
    }

    /// The colour rectangles (size, position) and the images (tag, position, scale) that the map spawns, the images
    /// in drawing order.
    #[allow(clippy::type_complexity)]
    fn spawn(map: Map) -> (Vec<(Option<Vec2>, Vec3)>, Vec<(String, Vec3, Vec3)>) {
        let mut app = crate::test_util::asset_app(16);
        let load = MapLoad {
            id: "m".to_string(),
            origin: Vec2::new(10.0, 20.0),
        };
        crate::test_util::spawn_with(&mut app, move |commands, server| {
            spawn_background(commands, server, &map, &load);
        });
        let world = app.world_mut();
        let colours = world
            .query_filtered::<(&Sprite, &Transform), (With<MapId>, Without<AseAnimation>)>()
            .iter(world)
            .map(|(sprite, transform)| (sprite.custom_size, transform.translation))
            .collect();
        let mut images: Vec<(String, Vec3, Vec3)> = world
            .query_filtered::<(&AseAnimation, &Transform), With<MapId>>()
            .iter(world)
            .map(|(ase, transform)| {
                let tag = ase.animation.tag.clone().unwrap_or_default();
                (tag, transform.translation, transform.scale)
            })
            .collect();
        images.sort_by(|a, b| a.1.z.total_cmp(&b.1.z));
        (colours, images)
    }

    #[test]
    fn the_ground_is_a_colour_the_size_of_the_map_with_the_placed_images_over_it() {
        let map = map_3x2("grass_0 flowers_1");
        let background = map.background.clone().unwrap();
        let placed = place_tiles("t", map.size, 48.0, &background, Vec2::splat(64.0));
        assert_eq!(placed.len(), 6);
        let (colours, images) = spawn(map);
        assert_eq!(
            colours,
            vec![(
                Some(Vec2::new(144.0, 96.0)),
                Vec3::new(82.0, 68.0, BACKGROUND_Z)
            )]
        );
        let expected: Vec<(String, Vec3, Vec3)> = placed
            .iter()
            .enumerate()
            .map(|(index, tile)| {
                let z = BACKGROUND_TILES_Z + index as f32 * BACKGROUND_TILE_Z_STEP;
                (
                    background.tiles[tile.tile].clone(),
                    (Vec2::new(10.0, 20.0) + tile.centre).extend(z),
                    Vec3::new(2.0, 2.0, 1.0),
                )
            })
            .collect();
        assert_eq!(images, expected);
    }

    #[test]
    fn a_map_with_an_unknown_tile_gets_its_colour_and_no_images() {
        let (colours, images) = spawn(map_3x2("grass_0 moss"));
        assert_eq!(colours.len(), 1);
        assert_eq!(images, vec![]);
    }
}
