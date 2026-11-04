//! A small seedable random number generator (SplitMix64). The same seed gives the same numbers, so a scripted run
//! can repeat exactly. It has no dependency, so it compiles for the web build without extra setup.
//!
//! `RngPlugin` adds a `GameRng` resource. The seed is the environment variable `<PREFIX>_SEED` when it is set (for
//! example `R_TOWN_SEED=7`), else the clock. The web build has no clock at start, so it mixes in the time of the
//! first frame instead.

use bevy::prelude::*;
use std::f32::consts::TAU;

#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number from 0 up to, not including, 1.
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// A number from `from` up to, not including, `to`. Returns `from` when `to` is not larger.
    pub fn range(&mut self, from: f32, to: f32) -> f32 {
        if to <= from {
            return from;
        }
        from + self.f32() * (to - from)
    }

    /// A point inside the disc, spread evenly over its area.
    pub fn point_in_disc(&mut self, centre: Vec2, radius: f32) -> Vec2 {
        let distance = radius * self.f32().sqrt();
        let angle = self.f32() * TAU;
        centre + Vec2::from_angle(angle) * distance
    }
}

/// The game's random numbers.
#[derive(Resource)]
pub struct GameRng {
    pub rng: Rng,
    /// True when the seed was set by the environment variable. The first-frame mixing is then skipped.
    fixed: bool,
}

impl GameRng {
    /// A generator with a fixed seed, for tests.
    pub fn seeded(seed: u64) -> Self {
        Self {
            rng: Rng::new(seed),
            fixed: true,
        }
    }
}

impl std::ops::Deref for GameRng {
    type Target = Rng;
    fn deref(&self) -> &Rng {
        &self.rng
    }
}

impl std::ops::DerefMut for GameRng {
    fn deref_mut(&mut self) -> &mut Rng {
        &mut self.rng
    }
}

pub struct RngPlugin {
    /// Prefix of the seed variable, for example "R_TOWN" for `R_TOWN_SEED`.
    pub env_prefix: &'static str,
}

impl Plugin for RngPlugin {
    fn build(&self, app: &mut App) {
        let variable = format!("{}_SEED", self.env_prefix);
        let fixed_seed = std::env::var(&variable)
            .ok()
            .and_then(|text| text.parse::<u64>().ok());
        let seed = fixed_seed.unwrap_or_else(clock_seed);
        info!(
            "rng: seed {seed}{}",
            if fixed_seed.is_some() {
                format!(" (from {variable})")
            } else {
                String::new()
            }
        );
        app.insert_resource(GameRng {
            rng: Rng::new(seed),
            fixed: fixed_seed.is_some(),
        })
        .add_systems(Update, mix_first_frame.run_if(run_once));
    }
}

/// Nanoseconds since 1970 on a computer. The web build has no clock at start: it returns a constant and
/// `mix_first_frame` adds the time of the first frame.
fn clock_seed() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1)
    }
    #[cfg(target_arch = "wasm32")]
    {
        0x1234_5678_9ABC_DEF0
    }
}

fn mix_first_frame(mut rng: ResMut<GameRng>, time: Res<Time<Real>>) {
    if rng.fixed {
        return;
    }
    let mixed = rng.rng.state ^ time.elapsed().as_nanos() as u64;
    rng.rng = Rng::new(mixed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_gives_the_same_numbers() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_give_different_numbers() {
        assert_ne!(Rng::new(1).next_u64(), Rng::new(2).next_u64());
    }

    #[test]
    fn f32_is_in_the_unit_range() {
        let mut rng = Rng::new(7);
        for _ in 0..10_000 {
            let value = rng.f32();
            assert!((0.0..1.0).contains(&value), "{value}");
        }
    }

    #[test]
    fn range_stays_inside_its_limits() {
        let mut rng = Rng::new(3);
        for _ in 0..10_000 {
            let value = rng.range(-5.0, 12.0);
            assert!((-5.0..12.0).contains(&value), "{value}");
        }
        assert_eq!(rng.range(4.0, 4.0), 4.0);
        assert_eq!(rng.range(4.0, 1.0), 4.0);
    }

    #[test]
    fn points_in_a_disc_are_inside_and_spread_over_the_area() {
        let mut rng = Rng::new(11);
        let centre = Vec2::new(100.0, -50.0);
        let radius = 120.0;
        let mut inside_half_radius = 0;
        let count = 20_000;
        for _ in 0..count {
            let point = rng.point_in_disc(centre, radius);
            let distance = point.distance(centre);
            assert!(distance <= radius + 1e-3, "{distance}");
            if distance <= radius / 2.0 {
                inside_half_radius += 1;
            }
        }
        // A quarter of the area lies within half the radius.
        let share = inside_half_radius as f32 / count as f32;
        assert!((share - 0.25).abs() < 0.02, "{share}");
    }
}
