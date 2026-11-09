//! The player's hit points (`adr-lw28`). Other systems ask for damage with the `DamagePlayer` message and do not
//! touch `Hp` themselves. At 0 the player stays alive: there is no game over screen yet (roadmap item 13), so the
//! plugin only logs it and damage stops.

use bevy::prelude::*;

use game_kit::Controlled;

/// Hit points at the start of the game.
pub const START_HP: u32 = 3;

pub struct HpPlugin;

impl Plugin for HpPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DamagePlayer>()
            .add_systems(Update, apply_damage);
    }
}

/// Current and maximum hit points. The HUD shows one icon per point of `max`, the lost points as empty.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hp {
    pub current: u32,
    pub max: u32,
}

impl Default for Hp {
    fn default() -> Self {
        Self::full(START_HP)
    }
}

impl Hp {
    pub fn full(max: u32) -> Self {
        Self { current: max, max }
    }

    /// Removes up to `amount` points. Never goes below 0.
    pub fn lose(&mut self, amount: u32) {
        self.current = self.current.saturating_sub(amount);
    }

    pub fn is_empty(&self) -> bool {
        self.current == 0
    }
}

/// Asks to take hit points from the player.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct DamagePlayer {
    pub amount: u32,
}

fn apply_damage(
    mut damage: MessageReader<DamagePlayer>,
    mut players: Query<&mut Hp, With<Controlled>>,
) {
    for hit in damage.read() {
        let Ok(mut hp) = players.single_mut() else {
            continue;
        };
        if hp.is_empty() {
            continue;
        }
        hp.lose(hit.amount);
        info!("hp: {} of {} left", hp.current, hp.max);
        if hp.is_empty() {
            info!("hp: no points left (no game over screen yet, the player stays)");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_player_starts_with_three_full_points() {
        let hp = Hp::default();
        assert_eq!((hp.current, hp.max), (3, 3));
    }

    #[test]
    fn points_cannot_go_below_zero() {
        let mut hp = Hp::full(3);
        hp.lose(1);
        assert_eq!((hp.current, hp.is_empty()), (2, false));
        hp.lose(5);
        assert_eq!((hp.current, hp.is_empty()), (0, true));
        hp.lose(1);
        assert_eq!(hp.current, 0);
    }

    fn app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(HpPlugin);
        let player = app.world_mut().spawn((Controlled, Hp::default())).id();
        (app, player)
    }

    #[test]
    fn a_damage_message_takes_points_and_stops_at_zero() {
        let (mut app, player) = app();
        for _ in 0..5 {
            app.world_mut().write_message(DamagePlayer { amount: 1 });
            app.update();
        }
        assert_eq!(app.world().get::<Hp>(player).unwrap().current, 0);
    }
}
