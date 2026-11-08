//! Helpers for Aseprite animations. `RandomStartFrame` starts a looping animation at a frame chosen from a seed,
//! so many copies of one animation (trees, pickups) do not move in step. See `dev-docs/extensions/pixel-art.md`.

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;

use crate::map_background::{cell_rng, hash_name};
use crate::rng::Rng;

/// Put this on an entity with an `AseAnimation`. Once the Aseprite file has loaded, the plugin moves the animation
/// to a frame of its tag (or of the whole file when there is no tag) chosen from `seed`, then removes this component.
/// The same seed gives the same frame.
#[derive(Component, Clone, Copy, Debug)]
pub struct RandomStartFrame {
    pub seed: u64,
}

impl RandomStartFrame {
    /// A seed from the map name and the cell of the object, so the frame is the same on every load, on F1 reload and
    /// when the map is entered again.
    pub fn for_cell(map_name: &str, cell: IVec2) -> Self {
        Self {
            seed: cell_rng(hash_name(map_name), 0, cell).next_u64(),
        }
    }
}

/// The frame, counted from the first frame of the range, that `seed` picks out of `frames` frames. 0 when there are none.
pub fn start_offset(seed: u64, frames: u16) -> u16 {
    if frames == 0 {
        return 0;
    }
    (Rng::new(seed).next_u64() % frames as u64) as u16
}

/// An animation of the tag `tag` in the Aseprite file at `path`. A tag that does not exist is reported by the animation system.
pub fn tag_animation(
    server: &AssetServer,
    path: &'static str,
    tag: impl AsRef<str>,
) -> AseAnimation {
    AseAnimation {
        aseprite: server.load(path),
        animation: Animation::tag(tag.as_ref()),
    }
}

/// An animation that loops over every frame of the file at `path` (a file without tags).
pub fn all_frames_animation(server: &AssetServer, path: &'static str) -> AseAnimation {
    AseAnimation {
        aseprite: server.load(path),
        animation: Animation::default(),
    }
}

/// `tag_animation` with a default `Sprite`, the usual bundle of a world object.
pub fn tag_sprite(
    server: &AssetServer,
    path: &'static str,
    tag: impl AsRef<str>,
) -> (AseAnimation, Sprite) {
    (tag_animation(server, path, tag), Sprite::default())
}

/// `all_frames_animation` with a default `Sprite`.
pub fn all_frames_sprite(server: &AssetServer, path: &'static str) -> (AseAnimation, Sprite) {
    (all_frames_animation(server, path), Sprite::default())
}

/// Switches to `tag`, looping at `speed`, only when `tag` is not the one already playing, so the animation does not
/// restart every frame. Call it every frame with the tag the state asks for. Returns whether it switched.
pub fn play_if_changed(ase: &mut AseAnimation, tag: &str, speed: f32) -> bool {
    if ase.animation.tag.as_deref() == Some(tag) {
        return false;
    }
    ase.animation.play_loop(tag);
    ase.animation.speed = speed;
    true
}

pub struct AnimationHelpPlugin;

impl Plugin for AnimationHelpPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, apply_random_start);
    }
}

/// Waits for the Aseprite asset, then sets the frame. A tag that does not exist is left to the animation system, which
/// reports it.
fn apply_random_start(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &AseAnimation,
        &mut AnimationState,
        &RandomStartFrame,
    )>,
    asprites: Res<Assets<Aseprite>>,
) {
    for (entity, animation, mut state, start) in &mut query {
        let Some(aseprite) = asprites.get(&animation.aseprite) else {
            continue;
        };
        let range = match animation.animation.tag.as_ref() {
            Some(tag) => match aseprite.tags.get(tag) {
                Some(meta) => meta.range.clone(),
                None => {
                    commands.entity(entity).remove::<RandomStartFrame>();
                    continue;
                }
            },
            None => 0..=(aseprite.frame_durations.len().max(1) as u16 - 1),
        };
        let frames = range.end() - range.start() + 1;
        let offset = start_offset(start.seed, frames);
        state.relative_frame = offset;
        state.current_frame = range.start() + offset;
        state.elapsed = std::time::Duration::ZERO;
        commands.entity(entity).remove::<RandomStartFrame>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_offset_is_inside_the_range_and_repeatable() {
        for seed in 0..200 {
            assert!(start_offset(seed, 8) < 8);
            assert_eq!(start_offset(seed, 8), start_offset(seed, 8));
        }
        assert_eq!(start_offset(5, 0), 0);
        assert_eq!(start_offset(5, 1), 0);
    }

    #[test]
    fn seeds_spread_over_all_frames() {
        let mut seen = [false; 8];
        for seed in 0..200 {
            seen[start_offset(seed, 8) as usize] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn a_tag_is_played_once_until_it_changes() {
        let mut ase = AseAnimation {
            animation: Animation::tag("Idle"),
            ..default()
        };
        assert!(!play_if_changed(&mut ase, "Idle", 1.0));
        assert!(play_if_changed(&mut ase, "Run", 2.5));
        assert_eq!(ase.animation.tag.as_deref(), Some("Run"));
        assert_eq!(ase.animation.speed, 2.5);
        assert!(!play_if_changed(&mut ase, "Run", 9.0));
        assert_eq!(ase.animation.speed, 2.5);
    }

    #[test]
    fn a_cell_seed_depends_on_map_and_cell() {
        let a = RandomStartFrame::for_cell("1", IVec2::new(3, 4)).seed;
        assert_eq!(a, RandomStartFrame::for_cell("1", IVec2::new(3, 4)).seed);
        assert_ne!(a, RandomStartFrame::for_cell("2", IVec2::new(3, 4)).seed);
        assert_ne!(a, RandomStartFrame::for_cell("1", IVec2::new(4, 3)).seed);
    }
}
