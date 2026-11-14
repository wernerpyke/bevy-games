//! Weapons (`games/long-way/docs/weapons.md`). Two messages are the whole public surface: `CastWeapon` asks for a weapon and
//! `WeaponHit` reports an effect on one enemy. A weapon's gameplay owns its state, sends `WeaponHit` and holds no
//! particles. Its visuals read that state. `damage.rs` turns a hit into hit points (`enemy_hp.rs`) and a push.
//! `values.rs` holds every number.

#[allow(
    dead_code,
    reason = "Contact and pushed_feet are used by the weapon steps, W1 onwards"
)]
pub mod damage;
pub mod fireball;
pub mod ice_ring;
pub mod ring;
pub mod tornado;
#[allow(dead_code, reason = "used by the weapon steps, W1 onwards")]
pub mod values;

use bevy::prelude::*;
use game_kit::Controlled;
use game_kit::fx::ParticleEffectHandle;
use game_kit::hitbox::HitBox;
use game_kit::map_transition::{ActiveMaps, MapId};
use game_kit::movement::{FollowSet, MoveState};

use crate::inventory::Inventory;
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
            .add_plugins((
                ring::RingPlugin,
                tornado::TornadoPlugin,
                fireball::FireballPlugin,
            ))
            .add_plugins(MapBindingPlugin)
            .add_systems(Update, cast_active.in_set(WeaponSystems::Cast))
            .add_systems(Update, damage::apply_hits.in_set(WeaponSystems::Apply));
    }
}

/// Gives every weapon entity the `MapId` of the map the player is in when it appears, so the kit removes it with
/// that map: a weapon cast before a map change does not go on in the next map (the user, 2026-11-14). It covers the
/// gameplay roots (`Ring`, `Tornado`, `Fireball`), the edge dots and every particle emitter (`ParticleEffectHandle`,
/// on all `game_kit::fx` emitters). A child, such as the fireball head, goes with its parent.
struct MapBindingPlugin;

impl Plugin for MapBindingPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(bind_to_map::<ring::Ring>)
            .add_observer(bind_to_map::<tornado::Tornado>)
            .add_observer(bind_to_map::<tornado::EdgeDot>)
            .add_observer(bind_to_map::<fireball::Fireball>)
            .add_observer(bind_to_map::<ParticleEffectHandle>);
    }
}

fn bind_to_map<C: Component>(
    add: On<Add<C>>,
    maps: Option<Res<ActiveMaps>>,
    mut commands: Commands,
) {
    if let Some(map) = maps.as_deref().and_then(ActiveMaps::current_map) {
        commands.entity(add.entity).insert(MapId(map.id.clone()));
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

/// The player clicked an active weapon button: cast the weapon in that slot from where the player stands, and take its
/// cost from the energy of that slot. An empty slot, a weapon that is not built, or too little energy casts nothing.
/// The HUD also disables the button while the energy is too low, but this check is the rule.
fn cast_active(
    mut clicks: MessageReader<ActiveWeaponClicked>,
    active: Res<ActiveWeapons>,
    mut inventory: ResMut<Inventory>,
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
        let cost = values::values(weapon).cost;
        if !inventory.try_spend(click.0, cost) {
            info!(
                "weapons: {} needs {cost} {:?}, has {}",
                weapon.label(),
                click.0,
                inventory.count(click.0)
            );
            continue;
        }
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
            .init_resource::<Inventory>()
            .add_systems(Update, cast_active);
        app
    }

    fn spawn_player(app: &mut App) {
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
    }

    fn set_energy(app: &mut App, counts: [(Energy, u32); 4]) {
        let mut inventory = app.world_mut().resource_mut::<Inventory>();
        for (energy, count) in counts {
            inventory.set_for_test(energy, count);
        }
    }

    /// Sends one click per energy in the same frame, then runs it.
    fn click(app: &mut App, energies: &[Energy]) -> Vec<WeaponKind> {
        for energy in energies {
            app.world_mut().write_message(ActiveWeaponClicked(*energy));
        }
        app.update();
        crate::test_util::messages::<CastWeapon>(app)
            .iter()
            .map(|cast| cast.weapon)
            .collect()
    }

    fn summary(app: &App) -> String {
        app.world().resource::<Inventory>().summary()
    }

    #[test]
    fn the_active_weapon_is_cast_from_the_players_feet_with_its_facing_and_a_blank_slot_casts_nothing()
     {
        let mut app = app();
        spawn_player(&mut app);
        set_energy(
            &mut app,
            [
                (Energy::Blue, 0),
                (Energy::Red, 4),
                (Energy::Violet, 5),
                (Energy::Black, 6),
            ],
        );
        // Violet has no weapon and black holds one that is not built: only red casts, and they take nothing.
        app.world_mut()
            .resource_mut::<ActiveWeapons>()
            .set_for_test(Energy::Black, Some(WeaponKind::IceWall));
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
        assert_eq!(summary(&app), "B 0  R 3  V 5  K 6");
    }

    #[test]
    fn a_cast_takes_exactly_its_cost_from_its_own_energy() {
        let mut app = app();
        spawn_player(&mut app);
        set_energy(
            &mut app,
            [
                (Energy::Blue, 5),
                (Energy::Red, 5),
                (Energy::Violet, 5),
                (Energy::Black, 5),
            ],
        );
        app.world_mut()
            .resource_mut::<ActiveWeapons>()
            .set_for_test(Energy::Red, Some(WeaponKind::Fireball));
        assert_eq!(click(&mut app, &[Energy::Red]), [WeaponKind::Fireball]);
        assert_eq!(summary(&app), "B 5  R 2  V 5  K 5");
        assert_eq!(click(&mut app, &[Energy::Blue]), [WeaponKind::Tornado]);
        assert_eq!(summary(&app), "B 4  R 2  V 5  K 5");
        app.world_mut()
            .resource_mut::<ActiveWeapons>()
            .set_for_test(Energy::Blue, Some(WeaponKind::IceRing));
        assert_eq!(click(&mut app, &[Energy::Blue]), [WeaponKind::IceRing]);
        assert_eq!(summary(&app), "B 2  R 2  V 5  K 5");
    }

    #[test]
    fn with_too_little_energy_nothing_is_cast_and_nothing_is_taken() {
        let mut app = app();
        spawn_player(&mut app);
        set_energy(
            &mut app,
            [
                (Energy::Blue, 1),
                (Energy::Red, 2),
                (Energy::Violet, 0),
                (Energy::Black, 0),
            ],
        );
        app.world_mut()
            .resource_mut::<ActiveWeapons>()
            .set_for_test(Energy::Red, Some(WeaponKind::Fireball));
        app.world_mut()
            .resource_mut::<ActiveWeapons>()
            .set_for_test(Energy::Blue, Some(WeaponKind::IceRing));
        assert_eq!(click(&mut app, &[Energy::Red, Energy::Blue]), []);
        assert_eq!(summary(&app), "B 1  R 2  V 0  K 0");
    }

    #[test]
    fn two_casts_in_one_frame_with_energy_for_one_cast_once() {
        let mut app = app();
        spawn_player(&mut app);
        set_energy(
            &mut app,
            [
                (Energy::Blue, 0),
                (Energy::Red, 1),
                (Energy::Violet, 0),
                (Energy::Black, 0),
            ],
        );
        assert_eq!(
            click(&mut app, &[Energy::Red, Energy::Red]),
            [WeaponKind::FireRing]
        );
        assert_eq!(summary(&app), "B 0  R 0  V 0  K 0");
    }

    #[test]
    fn without_a_player_nothing_is_cast_and_nothing_is_taken() {
        let mut app = app();
        set_energy(
            &mut app,
            [
                (Energy::Blue, 3),
                (Energy::Red, 3),
                (Energy::Violet, 0),
                (Energy::Black, 0),
            ],
        );
        assert_eq!(click(&mut app, &[Energy::Red, Energy::Blue]), []);
        assert_eq!(summary(&app), "B 3  R 3  V 0  K 0");
    }
}

#[cfg(test)]
mod map_tests {
    use super::*;
    use game_kit::map_transition::ActiveMap;

    /// Two maps in the world, the player in `b`.
    fn app() -> App {
        let map = |id: &str| ActiveMap {
            id: id.to_string(),
            origin: Vec2::ZERO,
            rect: Rect::new(0.0, 0.0, 640.0, 640.0),
        };
        let mut app = crate::test_util::timed_app(20);
        app.add_message::<CastWeapon>()
            .init_resource::<game_kit::collision::CollisionWorld>()
            .insert_resource(ActiveMaps {
                current: "b".to_string(),
                maps: vec![map("a"), map("b")],
                doorways: Vec::new(),
            })
            .add_plugins(MapBindingPlugin)
            .add_systems(
                Update,
                (
                    ring::cast_rings,
                    tornado::cast_tornadoes,
                    tornado::spawn_edge_dots,
                    fireball::cast_fireballs,
                )
                    .chain(),
            );
        app.update();
        app
    }

    fn maps_of<C: Component>(app: &mut App) -> Vec<Option<String>> {
        app.world_mut()
            .query_filtered::<Option<&MapId>, With<C>>()
            .iter(app.world())
            .map(|map| map.map(|map| map.0.clone()))
            .collect()
    }

    #[test]
    fn every_weapon_entity_belongs_to_the_map_the_player_is_in() {
        let mut app = app();
        for weapon in [
            WeaponKind::FireRing,
            WeaponKind::IceRing,
            WeaponKind::Tornado,
            WeaponKind::Fireball,
        ] {
            app.world_mut().write_message(CastWeapon {
                weapon,
                origin: Vec2::splat(100.0),
                centre: Vec2::splat(125.0),
                facing: Vec2::X,
            });
        }
        app.world_mut()
            .spawn(ParticleEffectHandle(Handle::default()));
        app.update();
        app.update();
        let b = Some("b".to_string());
        assert_eq!(maps_of::<ring::Ring>(&mut app), vec![b.clone(), b.clone()]);
        assert_eq!(maps_of::<tornado::Tornado>(&mut app), vec![b.clone()]);
        assert_eq!(maps_of::<fireball::Fireball>(&mut app), vec![b.clone()]);
        let dots = maps_of::<tornado::EdgeDot>(&mut app);
        assert!(!dots.is_empty());
        assert!(dots.iter().all(|map| *map == b), "{dots:?}");
        assert_eq!(maps_of::<ParticleEffectHandle>(&mut app), vec![b]);
    }
}
