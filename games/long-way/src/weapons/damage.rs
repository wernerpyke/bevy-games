//! What a `WeaponHit` does to an enemy: hit points through `DamageEnemy` and a push through the collision world.
//! Also the two small pieces every weapon uses to decide how much to hit and push (`Contact`).

use bevy::prelude::*;
use game_kit::camera::CameraBounds;
use game_kit::collision::{CollisionWorld, resolve};
use game_kit::movement::Mover;

use super::WeaponHit;
use super::values::{Damage, Pushback, WeaponValues};
use crate::enemy_hp::{DamageEnemy, EnemyHp};

/// What one cast has done to one target so far. A weapon keeps one per target it touches.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Contact {
    /// Seconds until the next damage tick. 0 means the next call hits at once.
    wait: f32,
    /// Pixels pushed so far.
    pushed: f32,
    /// Damage ticks dealt so far.
    ticks: u32,
}

impl Contact {
    /// Call every frame that the target is in the effect. Returns the damage to deal now, or `None` between ticks.
    /// The first call hits at once, then one tick every `values.tick` seconds, each of `dps * tick`, so the damage
    /// averages `dps`. A frame longer than the tick deals one tick and the rhythm starts again from it: missed ticks
    /// are not dealt later (`PLAN-weapons.md` D14). `Damage::Impact` weapons deal their damage themselves, so they get
    /// `None` here.
    pub fn damage(&mut self, values: &WeaponValues, dt: f32) -> Option<f32> {
        let Damage::PerSecond(dps) = values.damage else {
            return None;
        };
        self.wait -= dt;
        if self.wait > 0.0 {
            return None;
        }
        if dt > values.tick {
            self.wait = values.tick;
        } else {
            self.wait += values.tick;
        }
        self.ticks += 1;
        Some(dps * values.tick)
    }

    /// Pixels pushed so far, for the log.
    pub fn pushed(&self) -> f32 {
        self.pushed
    }

    /// Damage ticks dealt so far, for the log.
    pub fn ticks(&self) -> u32 {
        self.ticks
    }

    /// The part of `wanted` (a move away from the centre) that the weapon's pushback allows. `Pixels(n)` allows
    /// `n` pixels in all, across every call. `ToEdge` allows all of it: the weapon stops asking at its final radius.
    pub fn push(&mut self, pushback: Pushback, wanted: Vec2) -> Vec2 {
        let allowed = match pushback {
            Pushback::None => 0.0,
            // The tornado shapes its push itself (`tornado.rs`).
            Pushback::ToEdge | Pushback::Outward(_) => wanted.length(),
            Pushback::Pixels(limit) => wanted.length().min((limit - self.pushed).max(0.0)),
        };
        if allowed <= 0.0 {
            return Vec2::ZERO;
        }
        self.pushed += allowed;
        wanted.normalize_or_zero() * allowed
    }
}

/// Moves `feet` by `push` and stops at the first blocked edge and at the map rectangle (shrunk by `margin`).
pub fn pushed_feet(
    feet: Vec2,
    push: Vec2,
    world: &CollisionWorld,
    bounds: Option<Rect>,
    margin: f32,
) -> Vec2 {
    let moved = resolve(feet, feet + push, world.edges()).feet;
    match bounds {
        Some(bounds) => moved.clamp(bounds.min + margin, bounds.max - margin),
        None => moved,
    }
}

pub(crate) fn apply_hits(
    mut hits: MessageReader<WeaponHit>,
    mut damage: MessageWriter<DamageEnemy>,
    mut enemies: Query<(&mut Transform, Option<&Mover>), With<EnemyHp>>,
    world: Res<CollisionWorld>,
    bounds: Option<Res<CameraBounds>>,
) {
    for hit in hits.read() {
        let Ok((mut transform, mover)) = enemies.get_mut(hit.target) else {
            continue;
        };
        if hit.amount > 0.0 {
            damage.write(DamageEnemy {
                target: hit.target,
                amount: hit.amount,
            });
        }
        if hit.push != Vec2::ZERO {
            let feet = transform.translation.truncate();
            let margin = mover.map_or(0.0, |mover| mover.edge_margin);
            let to = pushed_feet(feet, hit.push, &world, bounds.as_ref().map(|b| b.0), margin);
            transform.translation.x = to.x;
            transform.translation.y = to.y;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon_menu::WeaponKind;

    fn values(damage: Damage, pushback: Pushback) -> WeaponValues {
        WeaponValues {
            damage,
            pushback,
            tick: 0.5,
            glow: false,
        }
    }

    #[test]
    fn damage_per_second_is_dealt_in_ticks_and_adds_up() {
        let v = values(Damage::PerSecond(40.0), Pushback::None);
        let mut contact = Contact::default();
        let dt = 0.1;
        let mut total = 0.0;
        let mut ticks = 0;
        for _ in 0..30 {
            // 3 seconds in the effect
            if let Some(amount) = contact.damage(&v, dt) {
                total += amount;
                ticks += 1;
            }
        }
        // A tick at once, then one every 0.5 s: 0, 0.5, ... 2.5 is 6 ticks of 20.
        assert_eq!(ticks, 6);
        assert!((total - 120.0).abs() < 1e-3, "{total}");
    }

    #[test]
    fn the_first_frame_in_the_effect_hits_at_once() {
        let v = values(Damage::PerSecond(10.0), Pushback::None);
        assert_eq!(Contact::default().damage(&v, 0.016), Some(5.0));
    }

    #[test]
    fn a_frame_no_longer_than_the_tick_keeps_the_rhythm() {
        // Ticks at 0, then due at 0.5 and 1.0. The frame that passes 0.5 ends at 0.75, so the next tick is 0.25 later.
        let v = values(Damage::PerSecond(10.0), Pushback::None);
        let mut contact = Contact::default();
        assert_eq!(contact.damage(&v, 0.0), Some(5.0));
        assert_eq!(contact.damage(&v, 0.375), None);
        assert_eq!(contact.damage(&v, 0.375), Some(5.0));
        assert_eq!(contact.damage(&v, 0.125), None);
        assert_eq!(contact.damage(&v, 0.125), Some(5.0));
    }

    #[test]
    fn a_long_frame_deals_one_tick_and_the_rhythm_starts_again_from_it() {
        // D14: no burst of missed ticks. Just over the tick and far over it give the same.
        let v = values(Damage::PerSecond(10.0), Pushback::None);
        for long in [0.625, 2.0] {
            let mut contact = Contact::default();
            assert_eq!(contact.damage(&v, 0.0), Some(5.0));
            assert_eq!(contact.damage(&v, long), Some(5.0), "{long}");
            assert_eq!(contact.damage(&v, 0.375), None, "{long}");
            assert_eq!(contact.damage(&v, 0.125), Some(5.0), "{long}");
            assert_eq!(contact.ticks(), 3);
        }
    }

    #[test]
    fn impact_weapons_get_no_ticks() {
        let v = values(Damage::Impact(40.0), Pushback::None);
        assert_eq!(Contact::default().damage(&v, 1.0), None);
    }

    #[test]
    fn pixels_pushback_never_moves_a_target_more_than_its_limit_in_all() {
        let mut contact = Contact::default();
        let limit = Pushback::Pixels(50.0);
        let mut total = Vec2::ZERO;
        for _ in 0..20 {
            total += contact.push(limit, Vec2::new(0.0, 7.0));
        }
        assert!((total.y - 50.0).abs() < 1e-4, "{total}");
        assert_eq!(total.x, 0.0);
        assert_eq!(contact.push(limit, Vec2::new(0.0, 7.0)), Vec2::ZERO);
    }

    #[test]
    fn pixels_pushback_takes_a_partial_step_at_the_end() {
        let mut contact = Contact::default();
        contact.push(Pushback::Pixels(10.0), Vec2::new(8.0, 0.0));
        assert_eq!(
            contact.push(Pushback::Pixels(10.0), Vec2::new(8.0, 0.0)),
            Vec2::new(2.0, 0.0)
        );
    }

    #[test]
    fn to_edge_pushback_passes_everything_and_none_passes_nothing() {
        let mut contact = Contact::default();
        let step = Vec2::new(3.0, 4.0);
        assert_eq!(contact.push(Pushback::ToEdge, step), step);
        assert_eq!(contact.push(Pushback::None, step), Vec2::ZERO);
    }

    #[test]
    fn a_push_stops_at_a_blocked_area_and_inside_the_map() {
        let mut world = CollisionWorld::default();
        world.add_blocked(&[Rect::new(100.0, 0.0, 110.0, 200.0)]);
        let from = Vec2::new(90.0, 50.0);
        let to = pushed_feet(from, Vec2::new(40.0, 0.0), &world, None, 0.0);
        assert_eq!(to.x, 100.0);
        let map = Rect::new(0.0, 0.0, 200.0, 200.0);
        let free = CollisionWorld::default();
        let to = pushed_feet(
            Vec2::new(190.0, 50.0),
            Vec2::new(40.0, 0.0),
            &free,
            Some(map),
            12.0,
        );
        assert_eq!(to.x, 188.0);
    }

    fn app() -> (App, Entity) {
        let mut app = App::new();
        app.add_message::<WeaponHit>()
            .add_message::<DamageEnemy>()
            .init_resource::<CollisionWorld>()
            .add_systems(Update, apply_hits);
        let enemy = app
            .world_mut()
            .spawn((EnemyHp::full(100.0), Transform::from_xyz(50.0, 50.0, 0.0)))
            .id();
        (app, enemy)
    }

    fn damage_messages(app: &App) -> Vec<DamageEnemy> {
        crate::test_util::messages(app)
    }

    #[test]
    fn a_hit_asks_for_damage_and_moves_the_enemy() {
        let (mut app, enemy) = app();
        app.world_mut().write_message(WeaponHit {
            target: enemy,
            weapon: WeaponKind::FireRing,
            amount: 20.0,
            push: Vec2::new(0.0, 6.0),
        });
        app.update();
        assert_eq!(
            damage_messages(&app),
            vec![DamageEnemy {
                target: enemy,
                amount: 20.0
            }]
        );
        let at = app.world().get::<Transform>(enemy).unwrap().translation;
        assert_eq!((at.x, at.y), (50.0, 56.0));
    }

    #[test]
    fn a_push_only_hit_asks_for_no_damage() {
        let (mut app, enemy) = app();
        app.world_mut().write_message(WeaponHit {
            target: enemy,
            weapon: WeaponKind::Tornado,
            amount: 0.0,
            push: Vec2::new(5.0, 0.0),
        });
        app.update();
        assert!(damage_messages(&app).is_empty());
        assert_eq!(
            app.world().get::<Transform>(enemy).unwrap().translation.x,
            55.0
        );
    }

    #[test]
    fn a_hit_on_something_that_is_not_an_enemy_does_nothing() {
        let (mut app, _) = app();
        let rock = app.world_mut().spawn(Transform::default()).id();
        app.world_mut().write_message(WeaponHit {
            target: rock,
            weapon: WeaponKind::FireRing,
            amount: 20.0,
            push: Vec2::X,
        });
        app.update();
        assert!(damage_messages(&app).is_empty());
        assert_eq!(
            app.world().get::<Transform>(rock).unwrap().translation,
            Vec3::ZERO
        );
    }
}
