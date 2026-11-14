//! Every number that tunes a weapon, in one place. Each row names its source. A weapon's numbers are placeholders until
//! the user has tuned and accepted it (`weapons.md`, "Values"): the spike gave the looks and sizes, not damage.

use crate::camera::CELL_SIZE;
use crate::weapon_menu::WeaponKind;

/// How a weapon takes hit points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Damage {
    /// Hit points per second while a target is in the effect. It is dealt in ticks of `WeaponValues::tick` seconds.
    PerSecond(f32),
    /// Hit points dealt once, on impact (the fireball): `direct` to the enemy it hits, and `blast` to every other enemy
    /// in the explosion (`FireballShape::blast_radius`).
    Impact { direct: f32, blast: f32 },
    /// Hit points dealt once to a target the weapon pushes (`push`), and once per cast to a target that walks into it
    /// (`touch`). The ice ring deals them itself, so `Contact::damage` gives `None` for it.
    PushAndTouch { push: f32, touch: f32 },
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
    /// Pushed outward at this many pixels per second while inside, and only the target's own steps take it deeper,
    /// so a target that walks in faster still gains (the tornado, b7 and b8).
    Outward(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeaponValues {
    pub damage: Damage,
    pub pushback: Pushback,
    /// Seconds between two damage ticks on one target. 0.5 in the spike.
    pub tick: f32,
    /// Particles are drawn with additive blending (the spike result: fire and the fireball on, water off).
    pub glow: bool,
    /// Energy taken from the weapon's own energy (its drawer) by one cast (the user, 2026-11-14).
    pub cost: u32,
}

/// The shape and timing of a ring (`weapons/ring.rs`).
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

/// Spike defaults, except `max_radius`: 2 cells, so the ring is 4 cells across (the user, 2026-11-13, after a manual
/// test; it was 6 cells across).
pub const FIRE_RING: RingShape = RingShape {
    max_radius: 2.0 * CELL_SIZE,
    grow_secs: 0.767,
    burn_secs: 2.0,
    burn_lead: 40.0,
    front_emitters: 40,
    burners: 40,
    band_half: 24.0,
    trail_secs: 0.12,
};

const _: () = assert!(
    FIRE_RING.burn_lead < FIRE_RING.max_radius && FIRE_RING.band_half < FIRE_RING.max_radius
);
// The ring has a radius of 2 cells (the user, 2026-11-13).
const _: () = assert!(FIRE_RING.max_radius == 2.0 * CELL_SIZE);

/// Spike defaults, except `max_radius`: 3 cells, so the ring is 6 cells across (the user, 2026-11-13). `burn_secs` is
/// how long the full ring holds as a barrier. `band_half` and `trail_secs` are not used: the ice ring pushes every
/// body that its front has passed (`weapons/ice_ring.rs`).
pub const ICE_RING: RingShape = RingShape {
    max_radius: 3.0 * CELL_SIZE,
    grow_secs: 0.767,
    burn_secs: 2.0,
    burn_lead: 40.0,
    front_emitters: 40,
    burners: 40,
    band_half: 24.0,
    trail_secs: 0.12,
};

// The ice ring has a radius of 3 cells (the user, 2026-11-13).
const _: () = assert!(ICE_RING.max_radius == 3.0 * CELL_SIZE);

/// Rays cast from the centre of an ice ring at the cast. Each ray stops at the first blocking feature or at the map
/// edge, and the ring stops with it (`weapons/ice_ring.rs`).
pub const ICE_RAYS: usize = 64;

/// The shape of the tornado (`weapons/tornado.rs`): a disc on the middle of the player's body that follows the player.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TornadoShape {
    /// The edge of the disc (where it pushes and touches) starts at this radius at the cast...
    pub start_radius: f32,
    /// ...and grows at an even speed to this radius in `grow_secs`, then holds.
    pub radius: f32,
    pub grow_secs: f32,
    /// Seconds it lasts.
    pub duration: f32,
    /// On every `bump_every`-th damage tick of an enemy, the enemy is also shoved `bump` pixels outward over
    /// `bump_secs` (b9).
    pub bump: f32,
    pub bump_every: u32,
    pub bump_secs: f32,
}

impl TornadoShape {
    /// The radius of the edge `age` seconds after the cast.
    pub fn edge_at(&self, age: f32) -> f32 {
        let grown = (age / self.grow_secs).clamp(0.0, 1.0);
        self.start_radius + (self.radius - self.start_radius) * grown
    }
}

/// Start values (the user's design, 2026-11-13), to tune. Balance rule b4: it lasts twice as long as the ice ring's
/// growth and hold. The edge starts at the inner edge of the dense droplets (1 cell, the emitters' circle in
/// `tornado.rs`) and grows to 1.5 cells in a droplet's life (the user, 2026-11-13, after a play test). The bump:
/// option B of the same day, 40 px on every second tick.
pub const TORNADO: TornadoShape = TornadoShape {
    start_radius: CELL_SIZE,
    radius: 1.5 * CELL_SIZE,
    grow_secs: 1.3,
    duration: 2.0 * (ICE_RING.grow_secs + ICE_RING.burn_secs),
    bump: 40.0,
    bump_every: 2,
    bump_secs: 0.1,
};

const _: () = assert!(TORNADO.start_radius < TORNADO.radius && TORNADO.bump_every > 0);

/// The flight and explosion of the fireball (`weapons/fireball.rs`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FireballShape {
    /// Pixels per second. It steers to its target every frame, so it does not miss a target that moves.
    pub speed: f32,
    /// A shot with no target (no enemy on the map) explodes after this many pixels.
    pub range: f32,
    /// The blast damages every other enemy whose body is within this radius of the explosion.
    pub blast_radius: f32,
    /// The shot starts this far from the player's feet, towards its target.
    pub muzzle: f32,
    /// How high above its ground point the ball flies (`Projectile::height`): about the middle of a ghost's body, so
    /// the ground point heads for the ghost's feet.
    pub height: f32,
}

/// Start values (the user's design, 2026-11-13). Speed, range and muzzle from the spike. The blast radius is 1.25
/// cells, a little more than 1 cell (the user: "may have to be slightly larger than 1 cell"), where the spike's
/// shockwave ends (the test in `fireball.rs`).
pub const FIREBALL: FireballShape = FireballShape {
    speed: 420.0,
    range: 1100.0,
    blast_radius: 1.25 * CELL_SIZE,
    muzzle: 34.0,
    height: 14.0,
};

/// The shape of a ring weapon, `None` for the other weapons.
pub fn ring_shape(weapon: WeaponKind) -> Option<RingShape> {
    match weapon {
        WeaponKind::FireRing => Some(FIRE_RING),
        WeaponKind::IceRing => Some(ICE_RING),
        _ => None,
    }
}

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
            cost: 1,
        },
        // A shield (the user, 2026-11-13): a medium amount once on the push, a small amount once per cast on a touch.
        // Start values, to tune. Balance rule b3 (`docs/balance.md`): push + touch < ghost HP.
        WeaponKind::IceRing => WeaponValues {
            damage: Damage::PushAndTouch {
                push: 50.0,
                touch: 10.0,
            },
            pushback: Pushback::ToEdge,
            tick: 0.5,
            glow: false,
            cost: 2,
        },
        WeaponKind::FireWall => WeaponValues {
            damage: Damage::PerSecond(40.0),
            pushback: Pushback::None,
            tick: 0.5,
            glow: true,
            cost: 1, // A placeholder: the cost is set when the wall is built.
        },
        WeaponKind::IceWall => WeaponValues {
            damage: Damage::PerSecond(20.0),
            pushback: Pushback::None,
            tick: 0.5,
            glow: false,
            cost: 1, // A placeholder: the cost is set when the wall is built.
        },
        // The user's design (2026-11-13), start values to tune. 13 per tick of contact: a ghost dies on the 8th tick,
        // at 3.5 s of contact, past 60% of the duration (b5). A steady push of 20 px/s, with the bump in `TORNADO` (b7,
        // b9).
        WeaponKind::Tornado => WeaponValues {
            damage: Damage::PerSecond(26.0),
            pushback: Pushback::Outward(20.0),
            tick: 0.5,
            glow: false,
            cost: 1,
        },
        // The user's design (2026-11-13), start values to tune. A direct hit and the blast each kill a ghost (b10,
        // b11), and the blast is the smaller (b12).
        WeaponKind::Fireball => WeaponValues {
            damage: Damage::Impact {
                direct: 150.0,
                blast: 100.0,
            },
            pushback: Pushback::None,
            tick: 0.5,
            glow: true,
            cost: 3,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::MAX_ENERGY;

    #[test]
    fn every_weapon_has_values_that_make_sense() {
        for weapon in WeaponKind::ALL {
            let v = values(weapon);
            assert!(v.tick > 0.0, "{weapon:?}");
            match v.damage {
                Damage::PerSecond(n) => assert!(n > 0.0, "{weapon:?}"),
                Damage::PushAndTouch { push, touch } => {
                    assert!(push > 0.0 && touch > 0.0, "{weapon:?}")
                }
                Damage::Impact { direct, blast } => {
                    assert!(direct > 0.0 && blast > 0.0, "{weapon:?}")
                }
            }
            if let Pushback::Pixels(n) | Pushback::Outward(n) = v.pushback {
                assert!(n > 0.0, "{weapon:?}");
            }
            // A full gauge can pay for every weapon.
            assert!((1..=MAX_ENERGY).contains(&v.cost), "{weapon:?}");
        }
    }

    #[test]
    fn each_built_weapon_costs_the_users_value() {
        // The user's costs (2026-11-14).
        for (weapon, cost) in [
            (WeaponKind::FireRing, 1),
            (WeaponKind::Fireball, 3),
            (WeaponKind::Tornado, 1),
            (WeaponKind::IceRing, 2),
        ] {
            assert!(weapon.is_built(), "{weapon:?}");
            assert_eq!(values(weapon).cost, cost, "{weapon:?}");
        }
    }

    #[test]
    fn only_the_fireball_deals_impact_damage_and_it_does_not_push() {
        for weapon in WeaponKind::ALL {
            let impact = matches!(values(weapon).damage, Damage::Impact { .. });
            assert_eq!(impact, weapon == WeaponKind::Fireball, "{weapon:?}");
        }
        assert_eq!(values(WeaponKind::Fireball).pushback, Pushback::None);
    }

    /// The fireball's direct hit and blast.
    fn fireball_damage() -> (f32, f32) {
        let Damage::Impact { direct, blast } = values(WeaponKind::Fireball).damage else {
            panic!("the fireball should deal impact damage");
        };
        (direct, blast)
    }

    /// Balance rule b10 (`docs/balance.md`).
    #[test]
    fn b10_a_direct_fireball_hit_kills_a_ghost() {
        let (direct, _) = fireball_damage();
        assert!(direct >= crate::ghost::HIT_POINTS, "{direct}");
    }

    /// Balance rule b11 (`docs/balance.md`).
    #[test]
    fn b11_the_fireball_blast_alone_kills_a_ghost() {
        let (_, blast) = fireball_damage();
        assert!(blast >= crate::ghost::HIT_POINTS, "{blast}");
    }

    /// Balance rule b12 (`docs/balance.md`).
    #[test]
    fn b12_the_fireball_blast_deals_less_than_a_direct_hit() {
        let (direct, blast) = fireball_damage();
        assert!(blast < direct, "blast {blast}, direct {direct}");
    }

    /// Balance rule b3 (`docs/balance.md`): one ice ring deals at most its push and its touch to a ghost, and that
    /// must not kill it. A second ring may.
    #[test]
    fn b3_one_ice_ring_never_kills_a_ghost() {
        let Damage::PushAndTouch { push, touch } = values(WeaponKind::IceRing).damage else {
            panic!("the ice ring should deal push and touch damage");
        };
        let ghost = crate::ghost::HIT_POINTS;
        assert!(
            push + touch < ghost,
            "one ring deals {}, a ghost has {ghost}",
            push + touch
        );
    }

    #[test]
    fn only_the_rings_have_a_ring_shape() {
        for weapon in WeaponKind::ALL {
            let ring = matches!(weapon, WeaponKind::FireRing | WeaponKind::IceRing);
            assert_eq!(ring_shape(weapon).is_some(), ring, "{weapon:?}");
        }
        assert_eq!(ring_shape(WeaponKind::FireRing), Some(FIRE_RING));
        assert_eq!(ring_shape(WeaponKind::IceRing), Some(ICE_RING));
    }

    /// The tornado's damage per tick and its tick.
    fn tornado_tick() -> (f32, f32) {
        let v = values(WeaponKind::Tornado);
        let Damage::PerSecond(dps) = v.damage else {
            panic!("the tornado should deal damage per second");
        };
        (dps * v.tick, v.tick)
    }

    /// Seconds of contact at which the tick that kills a ghost lands (the first tick is at 0).
    fn tornado_kill_secs() -> f32 {
        let (per_tick, tick) = tornado_tick();
        ((crate::ghost::HIT_POINTS / per_tick).ceil() - 1.0) * tick
    }

    #[test]
    fn the_tornado_edge_grows_evenly_from_its_start_to_its_radius_and_holds() {
        let middle = (TORNADO.start_radius + TORNADO.radius) / 2.0;
        assert_eq!(TORNADO.edge_at(0.0), TORNADO.start_radius);
        assert!((TORNADO.edge_at(TORNADO.grow_secs / 2.0) - middle).abs() < 1e-3);
        assert_eq!(TORNADO.edge_at(TORNADO.grow_secs), TORNADO.radius);
        assert_eq!(TORNADO.edge_at(TORNADO.duration), TORNADO.radius);
        // The user's start and end (2026-11-13): the inner edge of the dense droplets, then 1.5 cells.
        assert_eq!(TORNADO.start_radius, CELL_SIZE);
        assert_eq!(TORNADO.radius, 1.5 * CELL_SIZE);
    }

    /// Balance rule b4 (`docs/balance.md`).
    #[test]
    fn b4_the_tornado_lasts_twice_as_long_as_the_ice_ring() {
        assert_eq!(
            TORNADO.duration,
            2.0 * (ICE_RING.grow_secs + ICE_RING.burn_secs)
        );
    }

    /// Balance rule b5 (`docs/balance.md`): the tick that kills lands after 60% of the duration, and the tick before
    /// it does not kill.
    #[test]
    fn b5_a_ghost_dies_from_one_tornado_only_after_60_percent_contact() {
        let (_, tick) = tornado_tick();
        let threshold = 0.6 * TORNADO.duration;
        let kill = tornado_kill_secs();
        assert!(kill > threshold, "kills at {kill} s, 60% is {threshold} s");
        assert!(
            kill - tick <= threshold,
            "the tick at {} s is already past 60% ({threshold} s)",
            kill - tick
        );
        assert!(kill <= TORNADO.duration, "never kills: {kill} s");
    }

    /// Balance rule b6 (`docs/balance.md`): the most one tornado can deal is more than the most one ice ring can.
    #[test]
    fn b6_one_tornado_can_deal_more_than_one_ice_ring() {
        let (per_tick, tick) = tornado_tick();
        let tornado = ((TORNADO.duration / tick).floor() + 1.0) * per_tick;
        let Damage::PushAndTouch { push, touch } = values(WeaponKind::IceRing).damage else {
            panic!("the ice ring should deal push and touch damage");
        };
        assert!(tornado > push + touch, "{tornado} against {}", push + touch);
    }

    /// A ghost walking straight at a standing player through the full tornado, counted in contact time: it gains at
    /// its speed less the steady push, and loses `bump` on every `bump_every`-th tick (not below the edge). Returns
    /// the seconds to cross the whole radius, the ticks dealt and the bumps taken on the way. The real crossing is
    /// shorter: it ends at the player's body, not at the centre.
    fn tornado_walk_in() -> (f32, u32, u32) {
        let Pushback::Outward(push) = values(WeaponKind::Tornado).pushback else {
            panic!("the tornado should push outward at a speed");
        };
        let (_, tick) = tornado_tick();
        let gain = crate::ghost::SPEED - push;
        assert!(gain > 0.0, "the push {push} is not slower than a ghost");
        let dt = 1.0 / 240.0;
        let (mut secs, mut depth, mut next_tick, mut ticks, mut bumps) = (0.0, 0.0, 0.0, 0, 0);
        while depth < TORNADO.radius {
            if secs >= next_tick {
                ticks += 1;
                next_tick += tick;
                if ticks % TORNADO.bump_every == 0 {
                    bumps += 1;
                    depth = (depth - TORNADO.bump).max(0.0);
                }
            }
            depth += gain * dt;
            secs += dt;
        }
        (secs, ticks, bumps)
    }

    /// Balance rule b7 (`docs/balance.md`): a ghost walking in gains on the push and the bumps, and crosses the whole
    /// radius before the tornado kills it and before it ends.
    #[test]
    fn b7_a_ghost_walking_at_a_standing_player_gets_through() {
        let (secs, ticks, _) = tornado_walk_in();
        let (per_tick, _) = tornado_tick();
        assert!(
            ticks as f32 * per_tick < crate::ghost::HIT_POINTS,
            "{ticks} ticks kill it on the way in"
        );
        assert!(secs < TORNADO.duration, "crosses in {secs} s");
    }

    /// Balance rule b9 (`docs/balance.md`): that ghost is bumped back at least twice on the way in.
    #[test]
    fn b9_a_ghost_walking_at_a_standing_player_is_bumped_back_at_least_twice() {
        let (secs, _, bumps) = tornado_walk_in();
        assert!(bumps >= 2, "{bumps} bumps in {secs} s");
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
