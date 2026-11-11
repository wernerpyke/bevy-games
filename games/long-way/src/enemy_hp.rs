//! Enemy hit points (`PLAN-weapons.md` D7). Other systems ask for damage with the `DamageEnemy` message and do not
//! touch `EnemyHp` themselves, like `hp.rs` does for the player. At 0 the enemy sends `EnemyDied` once and the
//! enemy's own plugin removes it (the ghost plays `Die`, `ghost.rs`).
//!
//! A hurt enemy shows it in one of two ways. An enemy that has its own hurt frames carries `HurtFrames` and gets an
//! `EnemyHurt` message, which its own plugin turns into its `Hurt` animation (the ghost, `ghost.rs`). Any other enemy
//! flashes `HURT_COLOUR` for `HURT_FLASH_SECS`: the colour is set on the `Sprite` of each child of the enemy.

use bevy::prelude::*;

/// The tint of a hurt enemy, `#993D41`.
pub const HURT_COLOUR: Color = Color::srgb(0.6, 61.0 / 255.0, 65.0 / 255.0);
/// Seconds that the tint stays after a hit.
pub const HURT_FLASH_SECS: f32 = 0.12;

pub struct EnemyHpPlugin;

impl Plugin for EnemyHpPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DamageEnemy>()
            .add_message::<EnemyHurt>()
            .add_message::<EnemyDied>()
            .add_systems(Update, (apply_damage, flash_hurt).chain());
    }
}

/// Current and maximum hit points. A float, because weapons deal damage per second.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct EnemyHp {
    pub current: f32,
    pub max: f32,
}

impl EnemyHp {
    pub fn full(max: f32) -> Self {
        Self { current: max, max }
    }

    /// Removes up to `amount` points. Never goes below 0. Returns true when this call took the last points, so the
    /// caller reports the death once.
    pub fn lose(&mut self, amount: f32) -> bool {
        if self.is_dead() || amount <= 0.0 {
            return false;
        }
        self.current = (self.current - amount).max(0.0);
        self.is_dead()
    }

    pub fn is_dead(&self) -> bool {
        self.current <= 0.0
    }
}

/// Asks to take hit points from an enemy. A target without `EnemyHp`, or one that is dead, is ignored.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct DamageEnemy {
    pub target: Entity,
    pub amount: f32,
}

/// Sent for every hit that an enemy survives, so an enemy with `HurtFrames` can play its hurt animation.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnemyHurt(pub Entity);

/// Marks an enemy that has its own hurt frames. It plays them on `EnemyHurt` and does not get the colour flash.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct HurtFrames;

/// Sent once when an enemy's hit points reach 0.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnemyDied(pub Entity);

/// On an enemy that was just hurt. The tint goes when the timer ends.
#[derive(Component)]
struct HitFlash(Timer);

fn apply_damage(
    mut damage: MessageReader<DamageEnemy>,
    mut enemies: Query<(&mut EnemyHp, Has<HurtFrames>, Option<&Transform>)>,
    mut hurt: MessageWriter<EnemyHurt>,
    mut died: MessageWriter<EnemyDied>,
    mut commands: Commands,
) {
    for hit in damage.read() {
        let Ok((mut hp, has_hurt_frames, transform)) = enemies.get_mut(hit.target) else {
            continue;
        };
        if hp.is_dead() {
            continue;
        }
        let killed = hp.lose(hit.amount);
        let at = transform.map_or(Vec2::ZERO, |t| t.translation.truncate());
        info!(
            "enemy: {:?} lost {:.1}, {:.1} of {:.1} left, at ({:.0}, {:.0})",
            hit.target, hit.amount, hp.current, hp.max, at.x, at.y
        );
        if !has_hurt_frames {
            commands
                .entity(hit.target)
                .insert(HitFlash(Timer::from_seconds(
                    HURT_FLASH_SECS,
                    TimerMode::Once,
                )));
        }
        if killed {
            info!("enemy: {:?} died", hit.target);
            died.write(EnemyDied(hit.target));
        } else {
            hurt.write(EnemyHurt(hit.target));
        }
    }
}

/// Tints the sprites of a hurt enemy and clears the tint when the flash has run out.
fn flash_hurt(
    time: Res<Time>,
    mut flashing: Query<(Entity, &mut HitFlash, &Children)>,
    mut sprites: Query<&mut Sprite>,
    mut commands: Commands,
) {
    for (entity, mut flash, children) in &mut flashing {
        flash.0.tick(time.delta());
        let finished = flash.0.is_finished();
        let colour = if finished { Color::WHITE } else { HURT_COLOUR };
        for child in children {
            if let Ok(mut sprite) = sprites.get_mut(*child) {
                sprite.color = colour;
            }
        }
        if finished {
            commands.entity(entity).remove::<HitFlash>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_go_down_and_stop_at_zero() {
        let mut hp = EnemyHp::full(10.0);
        assert!(!hp.lose(4.0));
        assert_eq!(hp.current, 6.0);
        assert!(hp.lose(100.0));
        assert_eq!((hp.current, hp.is_dead()), (0.0, true));
    }

    #[test]
    fn the_last_points_are_reported_once() {
        let mut hp = EnemyHp::full(5.0);
        assert!(hp.lose(5.0));
        assert!(!hp.lose(1.0));
    }

    #[test]
    fn no_damage_or_negative_damage_changes_nothing() {
        let mut hp = EnemyHp::full(5.0);
        assert!(!hp.lose(0.0));
        assert!(!hp.lose(-3.0));
        assert_eq!(hp.current, 5.0);
    }

    #[test]
    fn the_hurt_colour_is_993d41() {
        let srgba = HURT_COLOUR.to_srgba();
        let bytes = [srgba.red, srgba.green, srgba.blue].map(|c| (c * 255.0).round() as u8);
        assert_eq!(bytes, [0x99, 0x3D, 0x41]);
    }

    fn app() -> App {
        let mut app = crate::test_util::timed_app(50);
        app.add_plugins(EnemyHpPlugin)
            .init_resource::<Deaths>()
            .add_systems(Update, count_deaths.after(apply_damage));
        app
    }

    fn enemy(app: &mut App, hp: f32) -> (Entity, Entity) {
        let sprite = app.world_mut().spawn(Sprite::default()).id();
        let root = app
            .world_mut()
            .spawn(EnemyHp::full(hp))
            .add_child(sprite)
            .id();
        (root, sprite)
    }

    /// Every `EnemyDied` seen so far, counted by a reader that runs after the plugin's systems.
    #[derive(Resource, Default)]
    struct Deaths(usize, usize);

    fn count_deaths(
        mut died: MessageReader<EnemyDied>,
        mut hurt: MessageReader<EnemyHurt>,
        mut deaths: ResMut<Deaths>,
    ) {
        deaths.0 += died.read().count();
        deaths.1 += hurt.read().count();
    }

    fn hurt_messages(app: &App) -> usize {
        app.world().resource::<Deaths>().1
    }

    fn died_messages(app: &App) -> usize {
        app.world().resource::<Deaths>().0
    }

    #[test]
    fn damage_lowers_hp_and_tints_the_sprite_then_clears_it() {
        let mut app = app();
        let (root, sprite) = enemy(&mut app, 100.0);
        app.update();
        app.world_mut().write_message(DamageEnemy {
            target: root,
            amount: 30.0,
        });
        app.update();
        assert_eq!(app.world().get::<EnemyHp>(root).unwrap().current, 70.0);
        assert_eq!(
            app.world().get::<Sprite>(sprite).unwrap().color,
            HURT_COLOUR
        );
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            app.world().get::<Sprite>(sprite).unwrap().color,
            Color::WHITE
        );
        assert!(app.world().get::<HitFlash>(root).is_none());
    }

    #[test]
    fn a_kill_sends_one_died_message_and_later_damage_is_ignored() {
        let mut app = app();
        let (root, _) = enemy(&mut app, 10.0);
        app.world_mut().write_message(DamageEnemy {
            target: root,
            amount: 99.0,
        });
        app.update();
        assert_eq!(died_messages(&app), 1);
        app.world_mut().write_message(DamageEnemy {
            target: root,
            amount: 99.0,
        });
        app.update();
        assert_eq!(died_messages(&app), 1, "still only the first death");
    }

    #[test]
    fn an_enemy_with_hurt_frames_gets_a_message_and_no_flash() {
        let mut app = app();
        let (root, sprite) = enemy(&mut app, 100.0);
        app.world_mut().entity_mut(root).insert(HurtFrames);
        app.update();
        app.world_mut().write_message(DamageEnemy {
            target: root,
            amount: 10.0,
        });
        app.update();
        assert_eq!(hurt_messages(&app), 1);
        assert_eq!(
            app.world().get::<Sprite>(sprite).unwrap().color,
            Color::WHITE
        );
        assert!(app.world().get::<HitFlash>(root).is_none());
    }

    #[test]
    fn an_enemy_without_hurt_frames_flashes_and_a_killing_blow_is_not_a_hurt() {
        let mut app = app();
        let (root, _) = enemy(&mut app, 10.0);
        app.update();
        app.world_mut().write_message(DamageEnemy {
            target: root,
            amount: 4.0,
        });
        app.update();
        assert_eq!(hurt_messages(&app), 1);
        assert!(app.world().get::<HitFlash>(root).is_some());
        app.world_mut().write_message(DamageEnemy {
            target: root,
            amount: 40.0,
        });
        app.update();
        assert_eq!(
            hurt_messages(&app),
            1,
            "the killing blow is a death, not a hurt"
        );
        assert_eq!(died_messages(&app), 1);
    }

    #[test]
    fn a_target_without_hp_is_not_tinted_and_gets_no_hurt() {
        let mut app = app();
        let sprite = app.world_mut().spawn(Sprite::default()).id();
        let other = app.world_mut().spawn_empty().add_child(sprite).id();
        app.world_mut().write_message(DamageEnemy {
            target: other,
            amount: 5.0,
        });
        app.update();
        assert!(app.world().get::<HitFlash>(other).is_none());
        assert_eq!(
            app.world().get::<Sprite>(sprite).unwrap().color,
            Color::WHITE
        );
        assert_eq!((hurt_messages(&app), died_messages(&app)), (0, 0));
    }
}
