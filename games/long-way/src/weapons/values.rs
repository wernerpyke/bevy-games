//! Every number that tunes a weapon, in one place. Each row names its source. All of them are placeholders until the
//! weapon's step in `PLAN-weapons.md` has been tuned with the user: the spike gave the looks and sizes, not damage.

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

/// The values of a weapon. Placeholders (P4): damage and pushback are guesses to be tuned.
pub fn values(weapon: WeaponKind) -> WeaponValues {
    match weapon {
        WeaponKind::FireRing => WeaponValues {
            damage: Damage::PerSecond(40.0),
            pushback: Pushback::Pixels(64.0),
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
}
