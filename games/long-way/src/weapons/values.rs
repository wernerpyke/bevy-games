//! Every number that tunes a weapon, in one place. Each row names its source. All of them are placeholders until the
//! weapon's step in `PLAN-weapons.md` has been tuned with the user: the spike gave the looks and sizes, not damage.

use crate::camera::CELL_SIZE;
use crate::weapon_menu::WeaponKind;

/// How a weapon takes hit points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Damage {
    /// Hit points per second while a target is in the effect. It is dealt in ticks of `WeaponValues::tick` seconds.
    PerSecond(f32),
    /// Hit points dealt once, on impact (the fireball).
    Impact(f32),
}

/// How a weapon moves a target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pushback {
    /// No push.
    None,
    /// Pushed away from the centre by at most this many pixels in all, then left alone.
    Pixels(f32),
    /// Carried outward until the final radius or distance of the weapon.
    ToEdge,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeaponValues {
    pub damage: Damage,
    pub pushback: Pushback,
    /// Seconds between two damage ticks on one target. 0.5 in the spike.
    pub tick: f32,
    /// Particles are drawn with additive blending (the spike result: fire and the fireball on, water off).
    pub glow: bool,
}

/// The shape and timing of a ring (`weapons/ring.rs`). Spike defaults, except `max_radius`: 2 cells, so the ring is
/// 4 cells across (the user, 2026-11-13, after a manual test; it was 6 cells across).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RingShape {
    /// The ring grows to this radius, in pixels.
    pub max_radius: f32,
    /// Seconds to grow to `max_radius`, eased out: fast at the start, slowing to a stop.
    pub grow_secs: f32,
    /// Seconds the ring burns once it is at `max_radius`.
    pub burn_secs: f32,
    /// The burners start this many pixels before `max_radius`, so the growing front hands over to them.
    pub burn_lead: f32,
    /// Emitters on the growing front, and burners on the full ring.
    pub front_emitters: usize,
    pub burners: usize,
    /// While the ring burns, the damaging band is this wide on each side of `max_radius`.
    pub band_half: f32,
    /// While the ring grows, the damaging band reaches back this far in time (the trail of the moving emitters).
    pub trail_secs: f32,
}

pub const RING: RingShape = RingShape {
    max_radius: 2.0 * CELL_SIZE,
    grow_secs: 0.767,
    burn_secs: 2.0,
    burn_lead: 40.0,
    front_emitters: 40,
    burners: 40,
    band_half: 24.0,
    trail_secs: 0.12,
};

const _: () = assert!(RING.burn_lead < RING.max_radius && RING.band_half < RING.max_radius);
// The ring has a radius of 2 cells (the user, 2026-11-13).
const _: () = assert!(RING.max_radius == 2.0 * CELL_SIZE);

/// The values of a weapon. Placeholders (P4): damage and pushback are guesses to be tuned.
pub fn values(weapon: WeaponKind) -> WeaponValues {
    match weapon {
        // Balance rule b1 (`docs/balance.md`, the user, 2026-11-13): the first tick kills a ghost, so 100 per tick.
        // A ghost dies in the frame it is first touched, so a push would never show: none.
        WeaponKind::FireRing => WeaponValues {
            damage: Damage::PerSecond(200.0),
            pushback: Pushback::None,
            tick: 0.5,
            glow: true,
        },
        WeaponKind::IceRing => WeaponValues {
            damage: Damage::PerSecond(20.0),
            pushback: Pushback::ToEdge,
            tick: 0.5,
            glow: false,
        },
        WeaponKind::FireWall => WeaponValues {
            damage: Damage::PerSecond(40.0),
            pushback: Pushback::None,
            tick: 0.5,
            glow: true,
        },
        WeaponKind::IceWall => WeaponValues {
            damage: Damage::PerSecond(20.0),
            pushback: Pushback::None,
            tick: 0.5,
            glow: false,
        },
        WeaponKind::Whirlpool => WeaponValues {
            damage: Damage::PerSecond(20.0),
            pushback: Pushback::Pixels(96.0),
            tick: 0.5,
            glow: false,
        },
        WeaponKind::Fireball => WeaponValues {
            damage: Damage::Impact(40.0),
            pushback: Pushback::None,
            tick: 0.5,
            glow: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_weapon_has_values_that_make_sense() {
        for weapon in WeaponKind::ALL {
            let v = values(weapon);
            assert!(v.tick > 0.0, "{weapon:?}");
            match v.damage {
                Damage::PerSecond(n) | Damage::Impact(n) => assert!(n > 0.0, "{weapon:?}"),
            }
            if let Pushback::Pixels(n) = v.pushback {
                assert!(n > 0.0, "{weapon:?}");
            }
        }
    }

    #[test]
    fn only_the_fireball_deals_impact_damage_and_it_does_not_push() {
        for weapon in WeaponKind::ALL {
            let impact = matches!(values(weapon).damage, Damage::Impact(_));
            assert_eq!(impact, weapon == WeaponKind::Fireball, "{weapon:?}");
        }
        assert_eq!(values(WeaponKind::Fireball).pushback, Pushback::None);
    }

    /// Balance rule b1 (`docs/balance.md`): the fire ring kills a ghost with its first tick, so a ghost it touches dies
    /// wherever it stands and however far the ring would have pushed it.
    #[test]
    fn b1_the_first_tick_of_the_fire_ring_kills_a_ghost() {
        let v = values(WeaponKind::FireRing);
        let Damage::PerSecond(dps) = v.damage else {
            panic!(
                "the fire ring should deal damage per second, not {:?}",
                v.damage
            );
        };
        let first_tick = dps * v.tick;
        assert!(
            first_tick >= crate::ghost::HIT_POINTS,
            "the first tick deals {first_tick}, a ghost has {}",
            crate::ghost::HIT_POINTS
        );
    }
}
