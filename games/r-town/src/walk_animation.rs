//! How a walking character looks (`adr-rt3`): the sprite flips to face left or right, and while it walks it lifts one
//! pixel, a few times a second. The player and the enemies use it. A character has a `WalkAnimation` on its root
//! entity (at the feet) and a sprite child with `WalkSprite`, so the bob moves the sprite and not the feet that
//! collision uses. Systems that decide whether the character walks run in `WalkStateSet`.

use bevy::prelude::*;

/// Walking bob: the sprite lifts one pixel, this many times a second.
const BOB_RATE: f32 = 6.0;

pub struct WalkAnimationPlugin;

impl Plugin for WalkAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (advance_walk_time, animate_sprites)
                .chain()
                .after(WalkStateSet),
        );
    }
}

/// Systems that set `WalkAnimation::walking` and `facing_left` run in this set.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct WalkStateSet;

#[derive(Component, Default)]
pub struct WalkAnimation {
    pub walking: bool,
    pub facing_left: bool,
    /// Seconds spent walking, for the bob.
    walk_time: f32,
}

impl WalkAnimation {
    /// Looks left when the direction goes left, right when it goes right, and keeps the old side for a direction
    /// that goes straight up or down.
    pub fn face(&mut self, direction: Vec2) {
        if let Some(left) = faces_left(direction) {
            self.facing_left = left;
        }
    }
}

/// On the sprite child. `canvas_height` is the height of the art canvas, so the sprite stands on the feet.
#[derive(Component)]
pub struct WalkSprite {
    pub canvas_height: f32,
}

/// Some(true) for a direction to the left, Some(false) to the right, None when it has no sideways part.
fn faces_left(direction: Vec2) -> Option<bool> {
    (direction.x != 0.0).then_some(direction.x < 0.0)
}

/// How many pixels the sprite lifts after `walk_time` seconds of walking: 0 or 1.
fn bob_lift(walking: bool, walk_time: f32) -> f32 {
    if walking {
        ((walk_time * BOB_RATE) as i32 % 2) as f32
    } else {
        0.0
    }
}

fn advance_walk_time(time: Res<Time>, mut walkers: Query<&mut WalkAnimation>) {
    for mut walker in &mut walkers {
        if walker.walking {
            walker.walk_time += time.delta_secs();
        } else {
            walker.walk_time = 0.0;
        }
    }
}

fn animate_sprites(
    walkers: Query<&WalkAnimation>,
    mut sprites: Query<(&ChildOf, &WalkSprite, &mut Sprite, &mut Transform)>,
) {
    for (parent, walk_sprite, mut sprite, mut transform) in &mut sprites {
        let Ok(walker) = walkers.get(parent.parent()) else {
            continue;
        };
        sprite.flip_x = walker.facing_left;
        transform.translation.y =
            walk_sprite.canvas_height / 2.0 + bob_lift(walker.walking, walker.walk_time);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sprite_does_not_lift_while_standing() {
        assert_eq!(bob_lift(false, 0.0), 0.0);
        assert_eq!(bob_lift(false, 0.4), 0.0);
    }

    #[test]
    fn the_sprite_lifts_every_other_sixth_of_a_second_while_walking() {
        assert_eq!(bob_lift(true, 0.0), 0.0);
        assert_eq!(bob_lift(true, 0.2), 1.0);
        assert_eq!(bob_lift(true, 0.4), 0.0);
        assert_eq!(bob_lift(true, 0.5), 1.0);
    }

    #[test]
    fn facing_follows_sideways_movement_only() {
        assert_eq!(faces_left(Vec2::new(-0.5, 1.0)), Some(true));
        assert_eq!(faces_left(Vec2::new(0.1, -1.0)), Some(false));
        assert_eq!(faces_left(Vec2::new(0.0, 1.0)), None);
        let mut walker = WalkAnimation::default();
        walker.face(Vec2::NEG_X);
        walker.face(Vec2::Y);
        assert!(walker.facing_left);
        walker.face(Vec2::X);
        assert!(!walker.facing_left);
    }
}
