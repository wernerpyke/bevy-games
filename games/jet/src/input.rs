//! The player's key rules (`SPEC.md` section 4): a press starts or switches direction, one fallback direction is
//! remembered, and releasing the current key falls back or stops. Plain Rust, so every rule is unit tested.

use bevy::prelude::*;

/// One of the four directions. There is no diagonal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    /// The order in which presses of the same frame are taken: the last one wins.
    pub const ALL: [Dir; 4] = [Dir::Up, Dir::Down, Dir::Left, Dir::Right];

    /// The unit vector, y up.
    pub fn vec(self) -> Vec2 {
        match self {
            Dir::Up => Vec2::Y,
            Dir::Down => Vec2::NEG_Y,
            Dir::Left => Vec2::NEG_X,
            Dir::Right => Vec2::X,
        }
    }

    pub fn key(self) -> KeyCode {
        match self {
            Dir::Up => KeyCode::ArrowUp,
            Dir::Down => KeyCode::ArrowDown,
            Dir::Left => KeyCode::ArrowLeft,
            Dir::Right => KeyCode::ArrowRight,
        }
    }

    /// The rotation of art that faces up: right is 90° clockwise, down 180°, left 270° clockwise.
    pub fn rotation(self) -> Quat {
        let clockwise_quarters = match self {
            Dir::Up => 0.0,
            Dir::Right => 1.0,
            Dir::Down => 2.0,
            Dir::Left => 3.0,
        };
        Quat::from_rotation_z(-clockwise_quarters * std::f32::consts::FRAC_PI_2)
    }
}

/// Where the player wants to go: the current direction, and the one fallback.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Steering {
    pub current: Option<Dir>,
    pub fallback: Option<Dir>,
}

impl Steering {
    /// Applies one frame of key changes. Releases are taken before presses. `held` tells whether a key is down at
    /// the end of the frame.
    pub fn update(&mut self, pressed: &[Dir], released: &[Dir], held: impl Fn(Dir) -> bool) {
        for &dir in released {
            if self.current == Some(dir) {
                self.fall_back(&held);
            }
        }
        for &dir in pressed {
            if self.current == Some(dir) {
                continue;
            }
            if self.current.is_some() {
                self.fallback = self.current;
            }
            self.current = Some(dir);
        }
        // A key pressed and released within one frame is not held: do not keep going that way.
        if self.fallback.is_some_and(|dir| !held(dir)) {
            self.fallback = None;
        }
        if self.current.is_some_and(|dir| !held(dir)) {
            self.fall_back(&held);
        }
    }

    /// Blocked: stop, and forget the held directions, so a key held from before does not start movement again.
    pub fn stop(&mut self) {
        *self = Self::default();
    }

    fn fall_back(&mut self, held: &impl Fn(Dir) -> bool) {
        self.current = self.fallback.take().filter(|dir| held(*dir));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Dir::*;

    /// Plays frames of (pressed, released) into a still player, keeping track of the held keys.
    fn play(frames: &[(&[Dir], &[Dir])]) -> Steering {
        play_from(Steering::default(), &[], frames)
    }

    fn play_from(mut steering: Steering, held: &[Dir], frames: &[(&[Dir], &[Dir])]) -> Steering {
        let mut held = held.to_vec();
        for (pressed, released) in frames {
            held.retain(|dir| !released.contains(dir));
            held.extend(pressed.iter().filter(|dir| !released.contains(dir)));
            steering.update(pressed, released, |dir| held.contains(&dir));
        }
        steering
    }

    fn moving(current: Dir, fallback: Option<Dir>) -> Steering {
        Steering {
            current: Some(current),
            fallback,
        }
    }

    #[test]
    fn a_press_starts_movement_that_way() {
        for dir in Dir::ALL {
            assert_eq!(play(&[(&[dir], &[])]), moving(dir, None), "{dir:?}");
        }
    }

    #[test]
    fn a_different_press_switches_at_once_and_keeps_the_old_direction_as_fallback() {
        assert_eq!(
            play(&[(&[Up], &[]), (&[Left], &[])]),
            moving(Left, Some(Up))
        );
        // A third press keeps only the last direction as fallback.
        assert_eq!(
            play(&[(&[Up], &[]), (&[Left], &[]), (&[Down], &[])]),
            moving(Down, Some(Left))
        );
    }

    #[test]
    fn releasing_the_current_key_falls_back_while_the_fallback_key_is_held() {
        assert_eq!(
            play(&[(&[Up], &[]), (&[Right], &[]), (&[], &[Right])]),
            moving(Up, None)
        );
    }

    #[test]
    fn releasing_the_current_key_stops_without_a_fallback() {
        assert_eq!(play(&[(&[Up], &[]), (&[], &[Up])]), Steering::default());
    }

    #[test]
    fn a_released_fallback_key_is_dropped() {
        // Up, then Right: Up is the fallback. Up is let go, then Right: the player stops.
        let after_release = play(&[(&[Up], &[]), (&[Right], &[]), (&[], &[Up])]);
        assert_eq!(after_release, moving(Right, None));
        assert_eq!(
            play(&[(&[Up], &[]), (&[Right], &[]), (&[], &[Up]), (&[], &[Right])]),
            Steering::default()
        );
    }

    #[test]
    fn only_one_fallback_is_remembered() {
        // Up, Left, Down held. Down and then Left are let go: Left was the fallback, Up is forgotten.
        let steering = play(&[(&[Up], &[]), (&[Left], &[]), (&[Down], &[]), (&[], &[Down])]);
        assert_eq!(steering, moving(Left, None));
        assert_eq!(
            play_from(steering, &[Up, Left], &[(&[], &[Left])]),
            Steering::default()
        );
    }

    #[test]
    fn after_a_stop_a_held_key_does_not_start_movement_and_a_new_press_does() {
        let mut steering = play(&[(&[Up], &[]), (&[Left], &[])]);
        steering.stop();
        assert_eq!(steering, Steering::default());
        // Up and Left are still held: frames with no press keep the player still.
        let still = play_from(steering, &[Up, Left], &[(&[], &[]), (&[], &[])]);
        assert_eq!(still, Steering::default());
        // Releasing a held key changes nothing; a new press starts.
        assert_eq!(
            play_from(still, &[Up, Left], &[(&[], &[Left]), (&[Right], &[])]),
            moving(Right, None)
        );
    }

    #[test]
    fn a_switch_and_a_release_in_one_frame_take_the_release_first() {
        // Moving up with Left as fallback. In one frame Up is let go (fall back to Left) and Down is pressed.
        let steering = play(&[(&[Left], &[]), (&[Up], &[]), (&[Down], &[Up])]);
        assert_eq!(steering, moving(Down, Some(Left)));
    }

    #[test]
    fn presses_in_one_frame_take_the_last_in_order_and_keep_one_fallback() {
        assert_eq!(play(&[(&[Up, Right], &[])]), moving(Right, Some(Up)));
    }

    #[test]
    fn a_tap_shorter_than_a_frame_does_not_leave_the_player_moving() {
        assert_eq!(play(&[(&[Up], &[Up])]), Steering::default());
        // While moving left: a tap of Up switches for no frame, and Left goes on.
        assert_eq!(play(&[(&[Left], &[]), (&[Up], &[Up])]), moving(Left, None));
    }

    #[test]
    fn the_rotation_turns_art_that_faces_up_clockwise() {
        for dir in Dir::ALL {
            let turned = dir.rotation() * Vec3::Y;
            assert!(
                turned.truncate().distance(dir.vec()) < 1e-6,
                "{dir:?}: {turned}"
            );
        }
    }
}
