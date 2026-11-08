//! Where the tile images of a map background go. The map file says the colour, the images, the `fill` chance, the
//! `jitter` and the `scale` (`map::Background`). This works out which cells get an image, which image, and where.
//! The game draws them (the kit has no art). See `dev-docs/extensions/map-files.md` and `dev-docs/adr/EXTENSIONS.md`
//! adr-x36.
//!
//! Each cell rolls its own dice, from a hash of the map name, the `seed` and the cell number. So the result is the
//! same on every load, on F1 reload, in a scripted run and on the web, it does not depend on the order in which the
//! cells are visited, and a map that grows keeps the images of the cells it already had. The hash is written here
//! (FNV-1a and SplitMix64) because `DefaultHasher` does not promise the same numbers on every platform.

use bevy::math::{IVec2, Vec2};

use crate::map::{Background, BackgroundMode};
use crate::rng::Rng;

/// One tile image on the map.
#[derive(Clone, Debug, PartialEq)]
pub struct TilePlacement {
    /// The cell that rolled the image.
    pub cell: IVec2,
    /// Index into `Background::tiles`.
    pub tile: usize,
    /// Centre of the image, in world units from the bottom left corner of cell (0, 0).
    pub centre: Vec2,
}

/// FNV-1a over the bytes of the map name.
fn hash_name(name: &str) -> u64 {
    let mut hash = 0xCBF2_9CE4_8422_2325u64;
    for byte in name.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

/// The dice of one cell.
fn cell_rng(name_hash: u64, seed: u64, cell: IVec2) -> Rng {
    let packed = ((cell.x as u32 as u64) << 32) | cell.y as u32 as u64;
    let base = Rng::new(name_hash ^ seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)).next_u64();
    Rng::new(Rng::new(base ^ packed).next_u64())
}

/// Moves the centre so the whole image, of size `image`, stays inside `0..extent` on one axis. An image that is
/// larger than the map is centred on it.
fn keep_inside(centre: f32, image: f32, extent: f32) -> f32 {
    if image >= extent {
        return extent / 2.0;
    }
    centre.clamp(image / 2.0, extent - image / 2.0)
}

/// The tile images of a map, row by row from the bottom. `image_size` is the drawn size of one image in world
/// units (the art size times `Background::scale`). Empty for a background without `fill` or `tiles`. The position of
/// an image in the list is its stacking order (`layers::BACKGROUND_TILES_Z`).
pub fn place_tiles(
    map_name: &str,
    size: IVec2,
    cell_size: f32,
    background: &Background,
    image_size: Vec2,
) -> Vec<TilePlacement> {
    if background.mode != BackgroundMode::Random
        || background.fill <= 0.0
        || background.tiles.is_empty()
    {
        return Vec::new();
    }
    let name_hash = hash_name(map_name);
    let extent = size.as_vec2() * cell_size;
    let mut placed = Vec::new();
    for y in 0..size.y {
        for x in 0..size.x {
            let cell = IVec2::new(x, y);
            let mut rng = cell_rng(name_hash, background.seed, cell);
            if rng.f32() >= background.fill {
                continue;
            }
            let tile = (rng.next_u64() % background.tiles.len() as u64) as usize;
            let shift = Vec2::new(
                rng.range(-background.jitter, background.jitter),
                rng.range(-background.jitter, background.jitter),
            );
            let centre = (cell.as_vec2() + Vec2::splat(0.5)) * cell_size + shift;
            placed.push(TilePlacement {
                cell,
                tile,
                centre: Vec2::new(
                    keep_inside(centre.x, image_size.x, extent.x),
                    keep_inside(centre.y, image_size.y, extent.y),
                )
                .round(),
            });
        }
    }
    placed
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::color::Srgba;

    fn background(fill: f32, jitter: f32, tiles: &[&str]) -> Background {
        Background {
            color: Srgba::BLACK,
            mode: BackgroundMode::Random,
            fill,
            jitter,
            scale: 2.0,
            tiles: tiles.iter().map(|t| t.to_string()).collect(),
            seed: 0,
            line: 6,
        }
    }

    const IMAGE: Vec2 = Vec2::new(64.0, 64.0);

    fn place(name: &str, size: (i32, i32), background: &Background) -> Vec<TilePlacement> {
        place_tiles(name, IVec2::new(size.0, size.1), 64.0, background, IMAGE)
    }

    #[test]
    fn fill_zero_gives_no_images_and_fill_one_gives_every_cell() {
        let none = place("m", (10, 10), &background(0.0, 0.0, &["a"]));
        assert!(none.is_empty());
        let all = place("m", (10, 10), &background(1.0, 0.0, &["a"]));
        assert_eq!(all.len(), 100);
        assert_eq!(all[0].cell, IVec2::new(0, 0));
        assert_eq!(all[99].cell, IVec2::new(9, 9));
    }

    #[test]
    fn a_background_without_tiles_or_in_pattern_mode_gives_no_images() {
        assert!(place("m", (10, 10), &background(1.0, 0.0, &[])).is_empty());
        let mut pattern = background(1.0, 0.0, &["a"]);
        pattern.mode = BackgroundMode::Pattern;
        assert!(place("m", (10, 10), &pattern).is_empty());
    }

    #[test]
    fn the_same_input_gives_the_same_result() {
        let b = background(0.3, 16.0, &["a", "b", "c"]);
        assert_eq!(place("m", (20, 20), &b), place("m", (20, 20), &b));
    }

    #[test]
    fn a_different_seed_or_map_name_gives_a_different_ground() {
        let mut b = background(0.3, 16.0, &["a", "b", "c"]);
        let first = place("m", (20, 20), &b);
        b.seed = 1;
        assert_ne!(first, place("m", (20, 20), &b));
        b.seed = 0;
        assert_ne!(first, place("n", (20, 20), &b));
    }

    #[test]
    fn the_share_of_cells_is_close_to_the_fill() {
        let placed = place("m", (100, 100), &background(0.2, 0.0, &["a"]));
        let share = placed.len() as f32 / 10_000.0;
        assert!((share - 0.2).abs() < 0.02, "share {share}");
    }

    #[test]
    fn every_image_is_chosen_and_every_index_is_in_range() {
        let placed = place("m", (30, 30), &background(1.0, 0.0, &["a", "b", "c", "d"]));
        let mut seen = [0; 4];
        for tile in &placed {
            seen[tile.tile] += 1;
        }
        assert!(seen.iter().all(|n| *n > 150), "{seen:?}");
    }

    #[test]
    fn a_shift_stays_within_the_jitter() {
        let placed = place("m", (30, 30), &background(1.0, 16.0, &["a"]));
        let mut moved = false;
        for tile in &placed {
            let home = (tile.cell.as_vec2() + Vec2::splat(0.5)) * 64.0;
            // Away from the map edge the clamp does nothing, so the shift is within the jitter (plus rounding).
            if tile.cell.x > 0 && tile.cell.x < 29 && tile.cell.y > 0 && tile.cell.y < 29 {
                let shift = (tile.centre - home).abs();
                assert!(shift.x <= 16.5 && shift.y <= 16.5, "{tile:?}");
                moved |= shift != Vec2::ZERO;
            }
        }
        assert!(moved);
    }

    #[test]
    fn no_jitter_puts_the_image_on_the_cell_centre() {
        let placed = place("m", (5, 5), &background(1.0, 0.0, &["a"]));
        for tile in placed {
            assert_eq!(tile.centre, (tile.cell.as_vec2() + Vec2::splat(0.5)) * 64.0);
        }
    }

    #[test]
    fn an_image_never_leaves_the_map() {
        for image in [
            Vec2::splat(64.0),
            Vec2::splat(128.0),
            Vec2::new(256.0, 96.0),
        ] {
            let placed = place_tiles(
                "m",
                IVec2::new(10, 6),
                64.0,
                &background(1.0, 64.0, &["a"]),
                image,
            );
            for tile in placed {
                assert!(tile.centre.x - image.x / 2.0 >= 0.0, "{tile:?} {image}");
                assert!(tile.centre.x + image.x / 2.0 <= 640.0, "{tile:?} {image}");
                assert!(tile.centre.y - image.y / 2.0 >= 0.0, "{tile:?} {image}");
                assert!(tile.centre.y + image.y / 2.0 <= 384.0, "{tile:?} {image}");
            }
        }
    }

    #[test]
    fn an_image_larger_than_the_map_is_centred_on_it() {
        let placed = place_tiles(
            "m",
            IVec2::new(1, 1),
            64.0,
            &background(1.0, 10.0, &["a"]),
            Vec2::splat(256.0),
        );
        assert_eq!(placed[0].centre, Vec2::splat(32.0));
    }

    #[test]
    fn a_map_that_grows_keeps_the_images_of_its_old_cells() {
        let b = background(0.4, 16.0, &["a", "b"]);
        let small = place("m", (8, 8), &b);
        let big = place("m", (12, 12), &b);
        // Cells away from the old map edge have the same image and the same place.
        for tile in small
            .iter()
            .filter(|t| t.cell.x > 0 && t.cell.x < 7 && t.cell.y > 0 && t.cell.y < 7)
        {
            assert!(big.contains(tile), "{tile:?} is missing in the bigger map");
        }
    }

    #[test]
    fn the_hash_never_changes_by_accident() {
        // The same numbers on native and on the web: if this fails, every map ground changes.
        let placed = place("1", (4, 4), &background(0.5, 16.0, &["a", "b", "c"]));
        let summary: Vec<(i32, i32, usize, i32, i32)> = placed
            .iter()
            .map(|t| {
                (
                    t.cell.x,
                    t.cell.y,
                    t.tile,
                    t.centre.x as i32,
                    t.centre.y as i32,
                )
            })
            .collect();
        assert_eq!(summary, EXPECTED);
    }

    const EXPECTED: [(i32, i32, usize, i32, i32); 7] = [
        (0, 0, 0, 32, 34),
        (2, 0, 2, 173, 34),
        (0, 1, 1, 36, 82),
        (3, 1, 1, 221, 101),
        (1, 3, 0, 105, 218),
        (2, 3, 2, 171, 224),
        (3, 3, 0, 211, 208),
    ];
}
