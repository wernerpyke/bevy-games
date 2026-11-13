//! Weapons (`PLAN-weapons.md`). Two messages are the whole public surface: `CastWeapon` asks for a weapon and
//! `WeaponHit` reports an effect on one enemy. A weapon's gameplay owns its state, sends `WeaponHit` and holds no
//! particles. Its visuals read that state. `damage.rs` turns a hit into hit points (`enemy_hp.rs`) and a push.
//! `values.rs` holds every number.

#[allow(
    dead_code,
    reason = "Contact and pushed_feet are used by the weapon steps, W1 onwards"
)]
pub mod damage;
pub mod ice_ring;
pub mod ring;
pub mod tornado;
#[allow(dead_code, reason = "used by the weapon steps, W1 onwards")]
pub mod values;

use bevy::prelude::*;
use game_kit::Controlled;
use game_kit::hitbox::HitBox;
use game_kit::movement::{FollowSet, MoveState};

use crate::player::feet_of;
use crate::weapon_menu::{ActiveWeaponClicked, ActiveWeapons, WeaponKind};

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CastWeapon>()
            .add_message::<WeaponHit>()
            .configure_sets(
                Update,
                (
                    WeaponSystems::Cast,
                    WeaponSystems::Effect.after(FollowSet),
                    WeaponSystems::Apply,
                )
                    .chain(),
            )
            .add_plugins((ring::RingPlugin, tornado::TornadoPlugin))
            .add_systems(Update, cast_active.in_set(WeaponSystems::Cast))
            .add_systems(Update, damage::apply_hits.in_set(WeaponSystems::Apply));
    }
}

/// The order of the weapon systems in a frame. `Effect` (the weapons' gameplay) runs after the kit has moved the
/// enemies (`FollowSet`), so a barrier can undo a step in the same frame. `Apply` then turns the hits into damage and
/// pushes.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum WeaponSystems {
    Cast,
    Effect,
    Apply,
}

/// Asks for a weapon to be cast.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct CastWeapon {
    pub weapon: WeaponKind,
    /// The player's feet at the moment of the cast.
    pub origin: Vec2,
    /// The middle of the player's body at the moment of the cast (where the rings are centred).
    pub centre: Vec2,
    /// The direction the player last moved in, as a unit vector (the aim rule of the walls).
    pub facing: Vec2,
}

/// An effect of a weapon on one enemy. `amount` is the hit points to take now (already `dps * tick` or the impact
/// damage). `push` is the move for this frame, already limited by the weapon's pushback.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct WeaponHit {
    pub target: Entity,
    pub weapon: WeaponKind,
    pub amount: f32,
    pub push: Vec2,
}

/// The player clicked an active weapon button: cast the weapon in that slot from where the player stands. An empty
/// slot, or one whose weapon is not built, casts nothing.
fn cast_active(
    mut clicks: MessageReader<ActiveWeaponClicked>,
    active: Res<ActiveWeapons>,
    player: Query<(&Transform, &MoveState, &HitBox), With<Controlled>>,
    mut cast: MessageWriter<CastWeapon>,
) {
    for click in clicks.read() {
        let Some(weapon) = active.get(click.0) else {
            continue;
        };
        let Ok((transform, state, hit_box)) = player.single() else {
            warn!("weapons: {} clicked but there is no player", weapon.label());
            continue;
        };
        let origin = feet_of(transform);
        info!(
            "weapons: cast {} at {origin:?}, facing ({:.2}, {:.2})",
            weapon.label(),
            state.facing.x,
            state.facing.y
        );
        cast.write(CastWeapon {
            weapon,
            origin,
            // The player's hit box is relative to its `Transform` (the middle of the canvas), not to the feet.
            centre: hit_box.rect_at(transform.translation.truncate()).center(),
            facing: state.facing,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::Energy;

    fn app() -> App {
        let mut app = App::new();
        app.add_message::<ActiveWeaponClicked>()
            .add_message::<CastWeapon>()
            .init_resource::<ActiveWeapons>()
            .add_systems(Update, cast_active);
        app
    }

    #[test]
    fn the_active_weapon_is_cast_from_the_players_feet_with_its_facing_and_a_blank_slot_casts_nothing()
     {
        let mut app = app();
        app.world_mut().spawn((
            Controlled,
            Transform::from_xyz(100.0, 200.0, 0.0),
            // Like the player's: 50 high, standing on the feet that are 56 below the `Transform`.
            HitBox {
                offset: Vec2::new(0.0, 25.0 - 56.0),
                half_size: Vec2::new(14.0, 25.0),
            },
            MoveState {
                facing: Vec2::NEG_X,
                ..default()
            },
        ));
        // Violet has no weapon and black holds one that is not built: only red casts.
        app.world_mut()
            .resource_mut::<ActiveWeapons>()
            .set_for_test(Energy::Black, Some(WeaponKind::Fireball));
        for energy in [Energy::Violet, Energy::Red, Energy::Black] {
            app.world_mut().write_message(ActiveWeaponClicked(energy));
        }
        app.update();
        let sent: Vec<CastWeapon> = crate::test_util::messages(&app);
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].weapon, WeaponKind::FireRing);
        assert_eq!(
            sent[0].origin,
            feet_of(&Transform::from_xyz(100.0, 200.0, 0.0))
        );
        assert_eq!(sent[0].centre, sent[0].origin + Vec2::new(0.0, 25.0));
        assert_eq!(sent[0].facing, Vec2::NEG_X);
    }

    #[test]
    fn without_a_player_nothing_is_cast() {
        let mut app = app();
        app.world_mut()
            .write_message(ActiveWeaponClicked(Energy::Red));
        app.update();
        assert_eq!(app.world().resource::<Messages<CastWeapon>>().len(), 0);
    }
}
