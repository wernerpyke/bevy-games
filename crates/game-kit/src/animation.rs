//! Helpers for Aseprite animations. `RandomStartFrame` starts a looping animation at a frame chosen from a seed,
//! so many copies of one animation (trees, pickups) do not move in step. See `dev-docs/extensions/pixel-art.md`.

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;

use crate::map_background::{cell_rng, hash_name};
use crate::movement::{FollowSet, MoveState};
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

/// The three views of a character drawn with a `Down`, a `Left` and an `Up` tag. `Right` is the `Left` tag flipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Down,
    Side,
    Up,
}

/// The view for a facing direction (y up) and whether the sprite is flipped, for art in which the side view looks
/// left. The larger part of the direction wins, and sideways wins a tie, like the player of Long Way.
pub fn view_of(facing: Vec2) -> (View, bool) {
    if facing.x.abs() >= facing.y.abs() {
        (View::Side, facing.x > 0.0)
    } else if facing.y > 0.0 {
        (View::Up, false)
    } else {
        (View::Down, false)
    }
}

/// On the sprite child of an entity with `MoveState`: plays the tag for the way the entity faces and flips the sprite
/// for the right side. It plays all the time, also while the entity stands (a character with no idle tag). Remove the
/// component to play something else, for example a death.
#[derive(Component, Clone, Copy, Debug)]
pub struct FacingTags {
    pub down: &'static str,
    pub side: &'static str,
    pub up: &'static str,
    /// Playback speed multiplier. 1.0 is the speed set in the Aseprite file (`adr-a6`).
    pub speed: f32,
}

impl FacingTags {
    pub fn new(down: &'static str, side: &'static str, up: &'static str) -> Self {
        Self {
            down,
            side,
            up,
            speed: 1.0,
        }
    }

    fn tag(&self, view: View) -> &'static str {
        match view {
            View::Down => self.down,
            View::Side => self.side,
            View::Up => self.up,
        }
    }
}

fn animate_facing(
    movers: Query<&MoveState>,
    mut sprites: Query<(&ChildOf, &FacingTags, &mut AseAnimation, &mut Sprite)>,
) {
    for (parent, tags, mut ase, mut sprite) in &mut sprites {
        let Ok(state) = movers.get(parent.parent()) else {
            continue;
        };
        let (view, flip) = view_of(state.facing);
        play_if_changed(&mut ase, tags.tag(view), tags.speed);
        sprite.flip_x = flip;
    }
}

/// Shows one frame and never plays: put it on an entity with an `AseAnimation` (for an icon). `relative` counts from the first
/// frame of the tag, or of the file when there is no tag, and is kept inside the range. Once the Aseprite file has
/// loaded, the plugin sets the frame and stops the animation, then removes this component.
#[derive(Component, Clone, Copy, Debug)]
pub struct StaticFrame {
    pub relative: u16,
}

/// The frames of the tag of `animation`, or of the whole file without a tag. None when the tag does not exist.
fn frame_range(
    aseprite: &Aseprite,
    animation: &AseAnimation,
) -> Option<std::ops::RangeInclusive<u16>> {
    match animation.animation.tag.as_ref() {
        Some(tag) => aseprite.tags.get(tag).map(|meta| meta.range.clone()),
        None => Some(0..=(aseprite.frame_durations.len().max(1) as u16 - 1)),
    }
}

/// The set of the system that plays the `FacingTags` of a sprite. A game system that takes over an animation (a death, an
/// attack) runs `.before(FacingSet)`: it plays its tag and removes `FacingTags` with a command, and the command is
/// applied before this set runs. Without that order the facing tag can overwrite the new tag in the same frame.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct FacingSet;

pub struct AnimationHelpPlugin;

impl Plugin for AnimationHelpPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (apply_random_start, apply_static_frame))
            .add_systems(Update, animate_facing.in_set(FacingSet).after(FollowSet));
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
        let Some(range) = frame_range(aseprite, animation) else {
            commands.entity(entity).remove::<RandomStartFrame>();
            continue;
        };
        let frames = range.end() - range.start() + 1;
        let offset = start_offset(start.seed, frames);
        state.relative_frame = offset;
        state.current_frame = range.start() + offset;
        state.elapsed = std::time::Duration::ZERO;
        commands.entity(entity).remove::<RandomStartFrame>();
    }
}

/// Waits for the Aseprite asset, then shows the frame and stops the animation.
fn apply_static_frame(
    mut commands: Commands,
    mut query: Query<(Entity, &mut AseAnimation, &mut AnimationState, &StaticFrame)>,
    asprites: Res<Assets<Aseprite>>,
) {
    for (entity, mut animation, mut state, frame) in &mut query {
        let Some(aseprite) = asprites.get(&animation.aseprite) else {
            continue;
        };
        if let Some(range) = frame_range(aseprite, &animation) {
            let relative = frame.relative.min(range.end() - range.start());
            state.relative_frame = relative;
            state.current_frame = range.start() + relative;
            state.elapsed = std::time::Duration::ZERO;
            animation.animation.playing = false;
        }
        commands.entity(entity).remove::<StaticFrame>();
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
    fn the_larger_part_of_the_direction_picks_the_view() {
        assert_eq!(view_of(Vec2::new(-1.0, 0.0)), (View::Side, false));
        assert_eq!(view_of(Vec2::new(1.0, 0.0)), (View::Side, true));
        assert_eq!(view_of(Vec2::new(0.0, 1.0)), (View::Up, false));
        assert_eq!(view_of(Vec2::new(0.0, -1.0)), (View::Down, false));
        assert_eq!(view_of(Vec2::new(0.3, 1.0)), (View::Up, false));
        assert_eq!(view_of(Vec2::new(-1.0, -0.3)), (View::Side, false));
        // A tie goes sideways.
        assert_eq!(view_of(Vec2::new(1.0, 1.0)), (View::Side, true));
    }

    fn facing_app() -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_systems(Update, animate_facing);
        let mover = app.world_mut().spawn(MoveState::default()).id();
        let sprite = app
            .world_mut()
            .spawn((
                ChildOf(mover),
                FacingTags::new("Down", "Left", "Up"),
                AseAnimation::default(),
                Sprite::default(),
            ))
            .id();
        (app, mover, sprite)
    }

    fn face(app: &mut App, mover: Entity, facing: Vec2) {
        app.world_mut().get_mut::<MoveState>(mover).unwrap().facing = facing;
        app.update();
    }

    #[test]
    fn the_sprite_plays_the_tag_of_the_facing_and_flips_for_the_right() {
        let (mut app, mover, sprite) = facing_app();
        for (facing, tag, flip) in [
            (Vec2::X, "Left", true),
            (Vec2::NEG_X, "Left", false),
            (Vec2::Y, "Up", false),
            (Vec2::NEG_Y, "Down", false),
        ] {
            face(&mut app, mover, facing);
            let ase = app.world().get::<AseAnimation>(sprite).unwrap();
            assert_eq!(ase.animation.tag.as_deref(), Some(tag), "{facing:?}");
            assert_eq!(app.world().get::<Sprite>(sprite).unwrap().flip_x, flip);
        }
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

    const ELAPSED: std::time::Duration = std::time::Duration::from_millis(300);

    /// The two frame systems, and a handle to an Aseprite file that is not loaded yet.
    fn frame_app() -> (App, Handle<Aseprite>) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Aseprite>()
            .add_systems(Update, (apply_random_start, apply_static_frame));
        let file = app.world().resource::<Assets<Aseprite>>().reserve_handle();
        (app, file)
    }

    /// Loads the file: `frames` frames and no tags.
    fn load(app: &mut App, file: &Handle<Aseprite>, frames: usize) {
        let mut aseprite = Aseprite::default();
        aseprite.frame_durations = vec![std::time::Duration::from_millis(100); frames];
        app.world_mut()
            .resource_mut::<Assets<Aseprite>>()
            .insert(file, aseprite)
            .unwrap();
    }

    fn animated(file: &Handle<Aseprite>, tag: Option<&str>) -> (AseAnimation, AnimationState) {
        let animation = tag.map_or_else(Animation::default, Animation::tag);
        (
            AseAnimation {
                animation,
                aseprite: file.clone(),
            },
            AnimationState {
                relative_frame: 1,
                current_frame: 1,
                elapsed: ELAPSED,
                ..default()
            },
        )
    }

    fn frame_of(app: &App, entity: Entity) -> (u16, u16, std::time::Duration) {
        let state = app.world().get::<AnimationState>(entity).unwrap();
        (state.relative_frame, state.current_frame, state.elapsed)
    }

    #[test]
    fn a_random_start_waits_for_the_file_then_sets_the_frame_from_the_seed() {
        let (mut app, file) = frame_app();
        let seed = 3;
        let expected = start_offset(seed, 5);
        assert!(
            expected > 1,
            "a seed whose frame differs from the one set: {expected}"
        );
        let entity = app
            .world_mut()
            .spawn((animated(&file, None), RandomStartFrame { seed }))
            .id();
        app.update();
        assert!(app.world().get::<RandomStartFrame>(entity).is_some());
        assert_eq!(frame_of(&app, entity), (1, 1, ELAPSED));
        load(&mut app, &file, 5);
        app.update();
        assert!(app.world().get::<RandomStartFrame>(entity).is_none());
        assert_eq!(
            frame_of(&app, entity),
            (expected, expected, std::time::Duration::ZERO)
        );
    }

    #[test]
    fn a_random_start_with_a_missing_tag_is_dropped_and_changes_nothing() {
        let (mut app, file) = frame_app();
        load(&mut app, &file, 5);
        let entity = app
            .world_mut()
            .spawn((animated(&file, Some("Run")), RandomStartFrame { seed: 3 }))
            .id();
        app.update();
        assert!(app.world().get::<RandomStartFrame>(entity).is_none());
        assert_eq!(frame_of(&app, entity), (1, 1, ELAPSED));
    }

    #[test]
    fn a_static_frame_waits_for_the_file_then_shows_the_frame_kept_in_range_and_stops() {
        let (mut app, file) = frame_app();
        let entity = app
            .world_mut()
            .spawn((animated(&file, None), StaticFrame { relative: 9 }))
            .id();
        app.update();
        assert!(app.world().get::<StaticFrame>(entity).is_some());
        load(&mut app, &file, 4);
        app.update();
        assert!(app.world().get::<StaticFrame>(entity).is_none());
        assert_eq!(frame_of(&app, entity), (3, 3, std::time::Duration::ZERO));
        let ase = app.world().get::<AseAnimation>(entity).unwrap();
        assert!(!ase.animation.playing);
    }

    #[test]
    fn a_static_frame_with_a_missing_tag_is_dropped_and_keeps_playing() {
        let (mut app, file) = frame_app();
        load(&mut app, &file, 4);
        let entity = app
            .world_mut()
            .spawn((animated(&file, Some("Run")), StaticFrame { relative: 2 }))
            .id();
        app.update();
        assert!(app.world().get::<StaticFrame>(entity).is_none());
        assert_eq!(frame_of(&app, entity), (1, 1, ELAPSED));
        let ase = app.world().get::<AseAnimation>(entity).unwrap();
        assert!(ase.animation.playing);
    }
}
