//! Particle sprite sheets built at run time from a tag of an `.aseprite` file (`PLAN-weapons.md` D2).
//!
//! Enoki draws a sprite particle from one texture: a single row of equal cells, each drawn as a square quad. The
//! `Aseprite` asset holds something else: every frame at full canvas size, packed in a 2D atlas. `strip_from_atlas`
//! copies the frames of a tag into a strip, cropped to the area where any of them has a visible pixel and (when asked)
//! padded to square cells. `FxLooks` does that once when the art has loaded and keeps the result as a `Look`.
//! There are no exported PNG files, so a fresh clone needs no export step.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::image::TextureAtlasLayout;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_aseprite_ultra::prelude::*;
use bevy_enoki::prelude::SpriteParticle2dMaterial;

use crate::fx::{AdditiveSprite, Look, SpriteLook};

const BYTES: usize = 4;

/// A strip of `frames` cells, `cell` pixels each, side by side. Pixels are RGBA, 8 bits per channel.
#[derive(Debug, PartialEq, Eq)]
pub struct Strip {
    pub cell: UVec2,
    pub frames: u32,
    pub rgba: Vec<u8>,
}

/// One frame to cut from the atlas: the image bytes and the rectangle of the frame in it.
struct FrameView<'a> {
    rgba: &'a [u8],
    atlas_width: u32,
    rect: URect,
}

impl FrameView<'_> {
    fn pixel(&self, x: u32, y: u32) -> &[u8] {
        let at = ((self.rect.min.y + y) * self.atlas_width + self.rect.min.x + x) as usize * BYTES;
        &self.rgba[at..at + BYTES]
    }
}

/// The smallest rectangle (frame coordinates, `max` exclusive) that holds every pixel with alpha above 0, over all frames.
fn visible_bounds(frames: &[FrameView]) -> Option<URect> {
    let mut bounds: Option<URect> = None;
    for frame in frames {
        let size = frame.rect.size();
        for y in 0..size.y {
            for x in 0..size.x {
                if frame.pixel(x, y)[3] > 0 {
                    let pixel = URect::new(x, y, x + 1, y + 1);
                    bounds = Some(bounds.map_or(pixel, |b| b.union(pixel)));
                }
            }
        }
    }
    bounds
}

/// Copies the frames at `indices` (positions in `layout`) out of an atlas image into a strip. Every cell has the
/// size of the union of the visible pixels. With `square` the cells are padded to a square, because Enoki draws
/// every particle as a square quad and would stretch a tall cell. Returns `None` for no frames, a frame that is not
/// in the layout, or frames with no visible pixel.
pub fn strip_from_atlas(
    atlas: &[u8],
    atlas_width: u32,
    layout: &TextureAtlasLayout,
    indices: &[usize],
    square: bool,
) -> Option<Strip> {
    let frames: Vec<FrameView> = indices
        .iter()
        .map(|&index| {
            layout.textures.get(index).map(|rect| FrameView {
                rgba: atlas,
                atlas_width,
                rect: *rect,
            })
        })
        .collect::<Option<_>>()?;
    let bounds = visible_bounds(&frames)?;
    let size = bounds.size();
    let cell = if square {
        UVec2::splat(size.x.max(size.y))
    } else {
        size
    };
    let pad = (cell - size) / 2;
    let width = cell.x as usize * frames.len();
    let mut rgba = vec![0u8; width * cell.y as usize * BYTES];
    for (n, frame) in frames.iter().enumerate() {
        for y in 0..size.y {
            for x in 0..size.x {
                let from = frame.pixel(bounds.min.x + x, bounds.min.y + y);
                let column = n * cell.x as usize + (pad.x + x) as usize;
                let at = ((pad.y + y) as usize * width + column) * BYTES;
                rgba[at..at + BYTES].copy_from_slice(from);
            }
        }
    }
    Some(Strip {
        cell,
        frames: frames.len() as u32,
        rgba,
    })
}

/// Names of looks that are being built from `.aseprite` tags, and the looks that are ready.
#[derive(Resource, Default)]
pub struct FxLooks {
    pending: Vec<Pending>,
    ready: HashMap<String, Look>,
}

struct Pending {
    name: String,
    aseprite: Handle<Aseprite>,
    tag: String,
    square: bool,
}

impl FxLooks {
    /// Asks for a look called `name`, made from the frames of `tag` in `file`. It is ready a few frames later (when
    /// the file has loaded). Ask in `Startup`.
    pub fn request(
        &mut self,
        server: &AssetServer,
        name: &str,
        file: &str,
        tag: &str,
        square: bool,
    ) {
        self.pending.push(Pending {
            name: name.to_owned(),
            aseprite: server.load(file.to_owned()),
            tag: tag.to_owned(),
            square,
        });
    }

    /// The look, or `None` while it is not ready (or the name is unknown).
    pub fn get(&self, name: &str) -> Option<&Look> {
        self.ready.get(name)
    }

    /// True when every requested look is ready or has failed.
    pub fn settled(&self) -> bool {
        self.pending.is_empty()
    }
}

pub(crate) fn build_looks(
    mut looks: ResMut<FxLooks>,
    aseprites: Res<Assets<Aseprite>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
    mut images: ResMut<Assets<Image>>,
    mut normals: ResMut<Assets<SpriteParticle2dMaterial>>,
    mut adds: ResMut<Assets<AdditiveSprite>>,
) {
    if looks.pending.is_empty() {
        return;
    }
    let pending = std::mem::take(&mut looks.pending);
    for item in pending {
        let Some(aseprite) = aseprites.get(&item.aseprite) else {
            looks.pending.push(item);
            continue;
        };
        let (Some(layout), Some(atlas)) = (
            layouts.get(&aseprite.atlas_layout),
            images.get(&aseprite.atlas_image),
        ) else {
            looks.pending.push(item);
            continue;
        };
        let Some(tag) = aseprite.tags.get(&item.tag) else {
            error!(
                "fx: {:?} has no tag {:?}, look {:?} not made",
                item.aseprite.path(),
                item.tag,
                item.name
            );
            continue;
        };
        let indices: Vec<usize> = tag
            .range
            .clone()
            .map(|frame| aseprite.get_atlas_index(frame as usize))
            .collect();
        let Some(data) = atlas.data.as_ref() else {
            error!(
                "fx: the atlas of look {:?} has no pixel data on the CPU",
                item.name
            );
            continue;
        };
        let Some(strip) = strip_from_atlas(data, atlas.width(), layout, &indices, item.square)
        else {
            error!(
                "fx: tag {:?} of look {:?} has no visible pixel",
                item.tag, item.name
            );
            continue;
        };
        let mut image = Image::new(
            Extent3d {
                width: strip.cell.x * strip.frames,
                height: strip.cell.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            strip.rgba,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.sampler = atlas.sampler.clone();
        let texture = images.add(image);
        let look = SpriteLook::new(&mut normals, &mut adds, texture, strip.frames);
        info!(
            "fx: look {:?}: {} frames of {}x{}",
            item.name, strip.frames, strip.cell.x, strip.cell.y
        );
        looks.ready.insert(item.name, Look::Sprites(look));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An atlas of `frames` frames of 4x4 pixels in a row, all transparent. Returns the bytes and the layout.
    fn atlas(frames: u32) -> (Vec<u8>, TextureAtlasLayout) {
        let mut layout = TextureAtlasLayout::new_empty(UVec2::new(4 * frames, 4));
        for n in 0..frames {
            layout.add_texture(URect::new(4 * n, 0, 4 * n + 4, 4));
        }
        (vec![0; (4 * frames * 4) as usize * BYTES], layout)
    }

    fn paint(atlas: &mut [u8], width: u32, x: u32, y: u32, value: u8) {
        let at = (y * width + x) as usize * BYTES;
        atlas[at..at + BYTES].copy_from_slice(&[value, value, value, 255]);
    }

    #[test]
    fn the_cell_is_the_union_of_the_visible_pixels_of_all_frames() {
        let (mut data, layout) = atlas(2);
        paint(&mut data, 8, 1, 1, 10); // frame 0: pixel (1,1)
        paint(&mut data, 8, 4 + 2, 3, 20); // frame 1: pixel (2,3)
        let strip = strip_from_atlas(&data, 8, &layout, &[0, 1], false).unwrap();
        assert_eq!((strip.cell, strip.frames), (UVec2::new(2, 3), 2));
        assert_eq!(strip.rgba.len(), 2 * 2 * 3 * BYTES);
    }

    #[test]
    fn each_frame_keeps_its_pixels_in_its_own_cell() {
        let (mut data, layout) = atlas(2);
        paint(&mut data, 8, 1, 1, 10);
        paint(&mut data, 8, 4 + 2, 3, 20);
        let strip = strip_from_atlas(&data, 8, &layout, &[0, 1], false).unwrap();
        let at = |x: usize, y: usize| strip.rgba[(y * 4 + x) * BYTES];
        // Frame 0 holds its pixel at the top left of the cell, frame 1 holds its pixel at the bottom right.
        assert_eq!((at(0, 0), strip.rgba[3]), (10, 255));
        assert_eq!(at(2 + 1, 2), 20);
        assert_eq!(at(1, 2), 0);
    }

    #[test]
    fn frames_follow_the_order_of_the_indices() {
        let (mut data, layout) = atlas(2);
        paint(&mut data, 8, 0, 0, 10);
        paint(&mut data, 8, 4, 0, 20);
        let strip = strip_from_atlas(&data, 8, &layout, &[1, 0], false).unwrap();
        assert_eq!(strip.cell, UVec2::new(1, 1));
        assert_eq!((strip.rgba[0], strip.rgba[BYTES]), (20, 10));
    }

    #[test]
    fn square_cells_are_padded_and_the_content_is_centred() {
        let (mut data, layout) = atlas(1);
        for y in 0..4 {
            paint(&mut data, 4, 1, y, 9); // a thin column, 1 wide and 4 tall
        }
        let strip = strip_from_atlas(&data, 4, &layout, &[0], true).unwrap();
        assert_eq!(strip.cell, UVec2::new(4, 4));
        // Padding is (4 - 1) / 2 = 1 on the left, so the column sits at x = 1.
        assert_eq!(strip.rgba[BYTES], 9);
        assert_eq!(strip.rgba[0], 0);
    }

    #[test]
    fn a_frame_in_a_lower_row_of_the_atlas_is_read_from_its_row() {
        // Two frames of 4x4 stacked in an atlas 4 wide and 8 high.
        let mut layout = TextureAtlasLayout::new_empty(UVec2::new(4, 8));
        layout.add_texture(URect::new(0, 0, 4, 4));
        layout.add_texture(URect::new(0, 4, 4, 8));
        let mut data = vec![0; 4 * 8 * BYTES];
        paint(&mut data, 4, 1, 0, 10); // frame 0
        paint(&mut data, 4, 2, 5, 20); // frame 1: pixel (2, 1)
        let strip = strip_from_atlas(&data, 4, &layout, &[1], false).unwrap();
        assert_eq!((strip.cell, strip.rgba[0]), (UVec2::new(1, 1), 20));
    }

    /// `build_looks` with one requested look, of tag "fly" in an `.aseprite` file that is not loaded yet.
    fn looks_app() -> (App, Handle<Aseprite>) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Aseprite>()
            .init_asset::<TextureAtlasLayout>()
            .init_asset::<Image>()
            .init_asset::<SpriteParticle2dMaterial>()
            .init_asset::<AdditiveSprite>()
            .init_resource::<FxLooks>()
            .add_systems(Update, build_looks);
        let file = app.world().resource::<Assets<Aseprite>>().reserve_handle();
        app.world_mut()
            .resource_mut::<FxLooks>()
            .pending
            .push(Pending {
                name: "spark".to_owned(),
                aseprite: file.clone(),
                tag: "fly".to_owned(),
                square: false,
            });
        (app, file)
    }

    /// Loads an `.aseprite` file with no tags, and its atlas (the default handles of `Aseprite::default()`).
    fn load(app: &mut App, file: &Handle<Aseprite>, with_atlas: bool) {
        let world = app.world_mut();
        world
            .resource_mut::<Assets<Aseprite>>()
            .insert(file, Aseprite::default())
            .unwrap();
        if with_atlas {
            let (data, layout) = atlas(1);
            world
                .resource_mut::<Assets<TextureAtlasLayout>>()
                .insert(&Handle::default(), layout)
                .unwrap();
            let image = Image::new(
                Extent3d {
                    width: 4,
                    height: 4,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                data,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            );
            world
                .resource_mut::<Assets<Image>>()
                .insert(&Handle::default(), image)
                .unwrap();
        }
    }

    #[test]
    fn a_look_waits_until_its_file_and_its_atlas_are_loaded() {
        let (mut app, file) = looks_app();
        app.update();
        assert!(!app.world().resource::<FxLooks>().settled());
        load(&mut app, &file, false);
        app.update();
        assert!(!app.world().resource::<FxLooks>().settled(), "no atlas yet");
    }

    #[test]
    fn a_look_whose_tag_is_missing_is_dropped() {
        let (mut app, file) = looks_app();
        load(&mut app, &file, true);
        app.update();
        let looks = app.world().resource::<FxLooks>();
        assert!(looks.settled());
        assert!(looks.get("spark").is_none());
    }

    #[test]
    fn no_frames_or_a_missing_frame_or_an_empty_tag_gives_nothing() {
        let (data, layout) = atlas(1);
        assert_eq!(strip_from_atlas(&data, 4, &layout, &[], false), None);
        assert_eq!(strip_from_atlas(&data, 4, &layout, &[3], false), None);
        assert_eq!(strip_from_atlas(&data, 4, &layout, &[0], false), None);
    }
}
