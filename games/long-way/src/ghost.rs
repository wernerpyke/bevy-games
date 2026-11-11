//! The ghost (`adr-lw29`): the first enemy. A `spawn_ghost` marker of a map (legend entry such as
//! `"!": spawn_ghost strategy=sentry radius=40 sight=500`) puts one ghost at the middle of its cell when the map
//! loads, once: it does not respawn while the map stays loaded, and comes back when the map is loaded again. The kit
//! does the walking and the sight (`game_kit::strategies`, `movement`, `nav`, `touch`). Here are its values and its
//! art, `characters/ghost.aseprite` (tags `Down`, `Left`, `Up`, `Attack` and `Die`, durations set in the file).
//!
//! When a ghost touches the player it stops and plays `Attack` once. When that ends the player loses 1 HP (`hp.rs`)
//! and the ghost dies: it plays `Die` once and is removed when that ends. A ghost also has hit points
//! (`enemy_hp.rs`): when weapons take the last one it dies the same way, without hurting the player. A hit it
//! survives plays the `Hurt` tag once (the ghost has its own hurt frames, so there is no colour flash).

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;
use game_kit::animation::{FacingSet, FacingTags, tag_sprite};
use game_kit::depth::DepthSort;
use game_kit::hitbox::HitBox;
use game_kit::map::Map;
use game_kit::map_transition::{MapId, MapLoad};
use game_kit::movement::Mover;
use game_kit::strategies::{Pursue, RandomPatrol, Sentry, Strategy};
use game_kit::touch::{ControlledTouched, TouchSet, Touchable};

use crate::enemy_hp::{EnemyDied, EnemyHp, EnemyHurt, HurtFrames};
use crate::hp::DamagePlayer;
use crate::map::Kind;
use crate::player::WALK_SPEED;

const ART: &str = "art/characters/ghost.aseprite";
/// Walk speed as a share of the player's. The ghost is slow.
const SPEED_SHARE: f32 = 0.3;
/// The ghost is drawn at this share of the art size.
const SCALE: f32 = 0.6;
/// The art canvas is 60 by 60 and the feet are about 22 art pixels below the centre (the lowest rows of the body
/// are at y 49 to 53 of 60).
const FEET_BELOW_CENTRE: f32 = 22.0 * SCALE;
/// The feet stay this far inside the map edge.
const EDGE_MARGIN: f32 = 12.0;
/// The part of the body that hurts: 20 wide and 36 high in art pixels (the body is about 27 by 43), standing on the
/// feet, scaled with the sprite.
const HIT_BOX: HitBox = HitBox {
    offset: Vec2::new(0.0, 22.0 * SCALE),
    half_size: Vec2::new(10.0 * SCALE, 18.0 * SCALE),
};
/// Damage to the player on touch.
const DAMAGE: u32 = 1;
/// Hit points of a ghost. A placeholder until the weapons are tuned (`PLAN-weapons.md`).
const HIT_POINTS: f32 = 100.0;

pub struct GhostPlugin;

impl Plugin for GhostPlugin {
    fn build(&self, app: &mut App) {
        // These systems take over the sprite's animation and remove `FacingTags`, so they run before the kit
        // plays the facing tags (`FacingSet`), in a fixed order: attack, hurt, death, then the finished ones.
        app.add_systems(
            Update,
            (
                start_attack.after(TouchSet),
                play_hurt,
                die_when_killed,
                finish_animations,
            )
                .chain()
                .before(FacingSet),
        );
    }
}

/// The root of a ghost. Its `Transform` x and y are its feet.
#[derive(Component)]
pub struct Ghost;

/// On the sprite child of a ghost that plays `Attack`. The damage lands when it ends.
#[derive(Component)]
struct Attacking;

/// On the sprite child of a ghost that plays `Die`.
#[derive(Component)]
struct Dying;

/// On the sprite child of a ghost that plays `Hurt`. The walking tags come back when it ends.
#[derive(Component)]
struct Hurting;

/// Spawns the ghosts of one map. They carry the map id, so the kit removes them with the map. A marker with a bad
/// strategy is reported and skipped.
pub fn spawn_ghosts(commands: &mut Commands, server: &AssetServer, map: &Map, load: &MapLoad) {
    for object in &map.objects {
        if Kind::of(object) != Kind::SpawnGhost {
            continue;
        }
        let strategy = match Strategy::from_params(&object.params) {
            Ok(strategy) => strategy,
            Err(message) => {
                error!("ghost: {}:{}: {message}", map.name, object.params.line());
                continue;
            }
        };
        let feet = load.origin + (object.cell.as_vec2() + Vec2::splat(0.5)) * map.cell_size;
        let mut ghost = commands.spawn((
            Ghost,
            MapId(load.id.clone()),
            Mover::new(WALK_SPEED * SPEED_SHARE).with_edge_margin(EDGE_MARGIN),
            HIT_BOX,
            EnemyHp::full(HIT_POINTS),
            HurtFrames,
            Touchable,
            DepthSort { feet_offset: 0.0 },
            Transform::from_translation(feet.extend(0.0)),
            Visibility::default(),
        ));
        strategy.insert(&mut ghost);
        ghost.with_child((
            FacingTags::new("Down", "Left", "Up"),
            tag_sprite(server, ART, "Down"),
            Transform::from_xyz(0.0, FEET_BELOW_CENTRE, 0.0)
                .with_scale(Vec3::new(SCALE, SCALE, 1.0)),
        ));
        debug!("ghost: placed at {feet:?} in map {}", load.id);
    }
}

/// A ghost that touches the player stops walking and cannot touch again, and its sprite plays `Attack` once.
fn start_attack(
    mut commands: Commands,
    mut touched: MessageReader<ControlledTouched>,
    ghosts: Query<&Children, (With<Ghost>, With<Touchable>)>,
    mut animations: Query<&mut AseAnimation>,
) {
    for touch in touched.read() {
        let Ok(children) = ghosts.get(touch.by) else {
            continue;
        };
        commands
            .entity(touch.by)
            .remove::<(Touchable, Mover, RandomPatrol, Pursue, Sentry)>();
        for child in children {
            commands
                .entity(*child)
                .remove::<(FacingTags, Hurting)>()
                .insert(Attacking);
            if let Ok(mut ase) = animations.get_mut(*child) {
                ase.animation.play("Attack", AnimationRepeat::Count(0));
            }
        }
        info!("ghost: {:?} touched the player and attacks", touch.by);
    }
}

/// A ghost that survives a hit plays `Hurt` once. It keeps walking. A ghost that attacks or dies does not play it.
fn play_hurt(
    mut commands: Commands,
    mut hurt: MessageReader<EnemyHurt>,
    ghosts: Query<&Children, With<Ghost>>,
    mut sprites: Query<&mut AseAnimation, (Without<Attacking>, Without<Dying>)>,
) {
    for EnemyHurt(entity) in hurt.read() {
        let Ok(children) = ghosts.get(*entity) else {
            continue;
        };
        for child in children {
            let Ok(mut ase) = sprites.get_mut(*child) else {
                continue;
            };
            info!("ghost: {entity:?} hurt, plays Hurt");
            commands
                .entity(*child)
                .remove::<FacingTags>()
                .insert(Hurting);
            ase.animation.play("Hurt", AnimationRepeat::Count(0));
        }
    }
}

/// A ghost whose hit points reached 0 stops, cannot be hit or touch again, and plays `Die`. It does not hurt the player.
fn die_when_killed(
    mut commands: Commands,
    mut died: MessageReader<EnemyDied>,
    ghosts: Query<&Children, With<Ghost>>,
    mut animations: Query<&mut AseAnimation>,
) {
    for EnemyDied(entity) in died.read() {
        let Ok(children) = ghosts.get(*entity) else {
            continue;
        };
        commands
            .entity(*entity)
            .remove::<(Touchable, Mover, RandomPatrol, Pursue, Sentry, HitBox)>();
        for child in children {
            commands
                .entity(*child)
                .remove::<(FacingTags, Attacking, Hurting)>()
                .insert(Dying);
            if let Ok(mut ase) = animations.get_mut(*child) {
                ase.animation.play("Die", AnimationRepeat::Count(0));
            }
        }
        info!("ghost: {entity:?} was killed, plays Die");
    }
}

/// A ghost's sprite child with the animation states it can be in.
type GhostSprite = (
    &'static ChildOf,
    &'static mut AseAnimation,
    Has<Attacking>,
    Has<Dying>,
    Has<Hurting>,
);

/// When `Attack` has played the player loses HP and the ghost plays `Die`. When `Die` has played the ghost is removed.
fn finish_animations(
    mut commands: Commands,
    mut events: MessageReader<AnimationEvents>,
    mut damage: MessageWriter<DamagePlayer>,
    mut sprites: Query<GhostSprite>,
) {
    for event in events.read() {
        let AnimationEvents::Finished(entity) = event else {
            continue;
        };
        let Ok((parent, mut ase, attacking, dying, hurting)) = sprites.get_mut(*entity) else {
            continue;
        };
        if hurting {
            commands
                .entity(*entity)
                .remove::<Hurting>()
                .insert(FacingTags::new("Down", "Left", "Up"));
        } else if attacking {
            info!(
                "ghost: {:?} attack landed, the player loses {DAMAGE}, the ghost dies",
                parent.parent()
            );
            damage.write(DamagePlayer { amount: DAMAGE });
            commands.entity(*entity).remove::<Attacking>().insert(Dying);
            commands
                .entity(parent.parent())
                .remove::<(HitBox, EnemyHp)>();
            ase.animation.play("Die", AnimationRepeat::Count(0));
        } else if dying {
            info!("ghost: {:?} removed after Die", parent.parent());
            commands.entity(parent.parent()).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.add_message::<ControlledTouched>()
            .add_message::<DamagePlayer>()
            .add_message::<AnimationEvents>()
            .add_message::<EnemyDied>()
            .add_message::<EnemyHurt>()
            .add_systems(
                Update,
                (start_attack, play_hurt, die_when_killed, finish_animations).chain(),
            );
        app
    }

    fn spawn_ghost(app: &mut App) -> (Entity, Entity) {
        let sprite = app
            .world_mut()
            .spawn((
                FacingTags::new("Down", "Left", "Up"),
                AseAnimation::default(),
            ))
            .id();
        let ghost = app
            .world_mut()
            .spawn((Ghost, Touchable, Mover::new(10.0), Sentry::new(40.0, 90.0)))
            .add_child(sprite)
            .id();
        (ghost, sprite)
    }

    fn damage_messages(app: &App) -> usize {
        crate::test_util::messages::<DamagePlayer>(app).len()
    }

    fn tag(app: &App, sprite: Entity) -> Option<&str> {
        app.world()
            .get::<AseAnimation>(sprite)
            .unwrap()
            .animation
            .tag
            .as_deref()
    }

    #[test]
    fn a_touching_ghost_stops_and_plays_attack_without_damage_yet() {
        let mut app = app();
        let (ghost, sprite) = spawn_ghost(&mut app);
        app.world_mut()
            .write_message(ControlledTouched { by: ghost });
        app.update();
        assert_eq!(damage_messages(&app), 0);
        let world = app.world();
        assert!(world.get::<Touchable>(ghost).is_none());
        assert!(world.get::<Mover>(ghost).is_none());
        assert!(world.get::<Sentry>(ghost).is_none());
        assert!(world.get::<FacingTags>(sprite).is_none());
        assert!(world.get::<Attacking>(sprite).is_some());
        let ase = world.get::<AseAnimation>(sprite).unwrap();
        assert_eq!(ase.animation.tag.as_deref(), Some("Attack"));
        assert!(matches!(ase.animation.repeat, AnimationRepeat::Count(0)));
    }

    #[test]
    fn a_ghost_that_has_attacked_is_not_touched_again_while_it_dies() {
        // Without the guard a touch after the attack would start `Attack` again over `Die`, and hurt the player twice.
        let mut app = app();
        let (ghost, sprite) = spawn_ghost(&mut app);
        let mut damage = 0;
        for _ in 0..2 {
            app.world_mut()
                .write_message(ControlledTouched { by: ghost });
            app.update();
            damage += damage_messages(&app);
        }
        app.world_mut()
            .write_message(AnimationEvents::Finished(sprite));
        app.update();
        damage += damage_messages(&app);
        assert_eq!(tag(&app, sprite), Some("Die"));
        app.world_mut()
            .write_message(ControlledTouched { by: ghost });
        app.update();
        damage += damage_messages(&app);
        assert_eq!(tag(&app, sprite), Some("Die"));
        assert!(app.world().get::<Attacking>(sprite).is_none());
        app.world_mut()
            .write_message(AnimationEvents::Finished(sprite));
        app.update();
        damage += damage_messages(&app);
        assert_eq!(damage, 1, "one attack, one damage");
        assert!(app.world().get_entity(ghost).is_err());
    }

    #[test]
    fn attack_ends_with_damage_and_die_and_die_ends_with_removal() {
        let mut app = app();
        let (ghost, sprite) = spawn_ghost(&mut app);
        app.world_mut()
            .write_message(ControlledTouched { by: ghost });
        app.update();
        // A finished message of another entity is ignored.
        let other = app.world_mut().spawn_empty().id();
        app.world_mut()
            .write_message(AnimationEvents::Finished(other));
        app.update();
        assert_eq!(damage_messages(&app), 0);
        app.world_mut()
            .write_message(AnimationEvents::Finished(sprite));
        app.update();
        assert_eq!(damage_messages(&app), 1);
        let world = app.world();
        assert!(world.get::<Attacking>(sprite).is_none());
        assert!(world.get::<Dying>(sprite).is_some());
        let ase = world.get::<AseAnimation>(sprite).unwrap();
        assert_eq!(ase.animation.tag.as_deref(), Some("Die"));
        assert!(app.world().get_entity(ghost).is_ok());
        app.world_mut()
            .write_message(AnimationEvents::Finished(sprite));
        app.update();
        assert!(app.world().get_entity(ghost).is_err());
    }

    #[test]
    fn a_killed_ghost_plays_die_without_hurting_the_player_and_is_removed() {
        let mut app = app();
        let (ghost, sprite) = spawn_ghost(&mut app);
        app.world_mut()
            .entity_mut(ghost)
            .insert((HIT_BOX, EnemyHp::full(10.0)));
        app.world_mut().write_message(EnemyDied(ghost));
        app.update();
        let world = app.world();
        assert!(world.get::<Touchable>(ghost).is_none());
        assert!(world.get::<Mover>(ghost).is_none());
        assert!(world.get::<HitBox>(ghost).is_none());
        assert!(world.get::<Dying>(sprite).is_some());
        assert_eq!(
            world
                .get::<AseAnimation>(sprite)
                .unwrap()
                .animation
                .tag
                .as_deref(),
            Some("Die")
        );
        app.world_mut()
            .write_message(AnimationEvents::Finished(sprite));
        app.update();
        assert_eq!(damage_messages(&app), 0);
        assert!(app.world().get_entity(ghost).is_err());
    }

    #[test]
    fn a_ghost_that_dies_while_attacking_never_lands_its_hit() {
        let mut app = app();
        let (ghost, sprite) = spawn_ghost(&mut app);
        app.world_mut()
            .write_message(ControlledTouched { by: ghost });
        app.update();
        app.world_mut().write_message(EnemyDied(ghost));
        app.update();
        assert!(app.world().get::<Attacking>(sprite).is_none());
        app.world_mut()
            .write_message(AnimationEvents::Finished(sprite));
        app.update();
        assert_eq!(damage_messages(&app), 0);
    }

    #[test]
    fn a_ghost_that_has_attacked_cannot_be_hurt_while_it_dies() {
        let mut app = app();
        let (ghost, sprite) = spawn_ghost(&mut app);
        app.world_mut()
            .entity_mut(ghost)
            .insert((HIT_BOX, EnemyHp::full(10.0)));
        app.world_mut()
            .write_message(ControlledTouched { by: ghost });
        app.update();
        app.world_mut()
            .write_message(AnimationEvents::Finished(sprite));
        app.update();
        let world = app.world();
        assert!(world.get::<EnemyHp>(ghost).is_none());
        assert!(world.get::<HitBox>(ghost).is_none());
    }

    #[test]
    fn a_hit_the_ghost_survives_plays_hurt_once_and_then_the_walking_tags_come_back() {
        let mut app = app();
        let (ghost, sprite) = spawn_ghost(&mut app);
        app.world_mut().write_message(EnemyHurt(ghost));
        app.update();
        let world = app.world();
        assert!(world.get::<FacingTags>(sprite).is_none());
        assert!(world.get::<Hurting>(sprite).is_some());
        let ase = world.get::<AseAnimation>(sprite).unwrap();
        assert_eq!(ase.animation.tag.as_deref(), Some("Hurt"));
        assert!(matches!(ase.animation.repeat, AnimationRepeat::Count(0)));
        app.world_mut()
            .write_message(AnimationEvents::Finished(sprite));
        app.update();
        let world = app.world();
        assert!(world.get::<Hurting>(sprite).is_none());
        assert!(world.get::<FacingTags>(sprite).is_some());
        assert!(app.world().get_entity(ghost).is_ok());
        assert_eq!(damage_messages(&app), 0);
    }

    /// Starts an attack (`touched`) or a death (`killed`) of the ghost in this update.
    fn start(app: &mut App, ghost: Entity, what: &str) {
        if what == "touched" {
            app.world_mut()
                .write_message(ControlledTouched { by: ghost });
        } else {
            app.world_mut().write_message(EnemyDied(ghost));
        }
        app.update();
    }

    #[test]
    fn a_ghost_that_is_attacking_or_dying_does_not_play_hurt() {
        for (what, playing) in [("touched", "Attack"), ("killed", "Die")] {
            let mut app = app();
            let (ghost, sprite) = spawn_ghost(&mut app);
            start(&mut app, ghost, what);
            app.world_mut().write_message(EnemyHurt(ghost));
            app.update();
            assert!(app.world().get::<Hurting>(sprite).is_none(), "{what}");
            assert_eq!(tag(&app, sprite), Some(playing), "{what}");
        }
    }

    #[test]
    fn starting_an_attack_or_dying_while_hurt_takes_over() {
        for (what, playing) in [("touched", "Attack"), ("killed", "Die")] {
            let mut app = app();
            let (ghost, sprite) = spawn_ghost(&mut app);
            app.world_mut().write_message(EnemyHurt(ghost));
            app.update();
            start(&mut app, ghost, what);
            let world = app.world();
            assert!(world.get::<Hurting>(sprite).is_none(), "{what}");
            assert!(world.get::<FacingTags>(sprite).is_none(), "{what}");
            assert_eq!(tag(&app, sprite), Some(playing), "{what}");
            let attacking = world.get::<Attacking>(sprite).is_some();
            let dying = world.get::<Dying>(sprite).is_some();
            assert_eq!((attacking, dying), (what == "touched", what == "killed"));
        }
    }

    /// The kit's `animate_facing` sets the walking tag of every sprite that still has `FacingTags`. A ghost that
    /// takes over the animation (`Die`, `Hurt`, `Attack`) must be ordered before it, or the walking tag can win in the
    /// same frame and the animation never plays (the ghost then never dies).
    fn real_facing_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Aseprite>()
            .add_plugins(game_kit::animation::AnimationHelpPlugin)
            .add_plugins(GhostPlugin)
            .add_message::<ControlledTouched>()
            .add_message::<DamagePlayer>()
            .add_message::<AnimationEvents>()
            .add_message::<EnemyDied>()
            .add_message::<EnemyHurt>();
        app
    }

    fn spawn_walking_ghost(app: &mut App) -> (Entity, Entity) {
        let sprite = app
            .world_mut()
            .spawn((
                FacingTags::new("Down", "Left", "Up"),
                AseAnimation::default(),
                Sprite::default(),
            ))
            .id();
        let ghost = app
            .world_mut()
            .spawn((
                Ghost,
                Touchable,
                Mover::new(10.0),
                game_kit::movement::MoveState::default(),
            ))
            .add_child(sprite)
            .id();
        (ghost, sprite)
    }

    #[test]
    fn dying_hurt_and_attacking_are_not_overwritten_by_the_walking_tags_in_the_same_frame() {
        for round in 0..20 {
            for (name, expected) in [("killed", "Die"), ("hurt", "Hurt"), ("touched", "Attack")] {
                let mut app = real_facing_app();
                let (ghost, sprite) = spawn_walking_ghost(&mut app);
                app.update();
                match name {
                    "killed" => {
                        app.world_mut().write_message(EnemyDied(ghost));
                    }
                    "hurt" => {
                        app.world_mut().write_message(EnemyHurt(ghost));
                    }
                    _ => {
                        app.world_mut()
                            .write_message(ControlledTouched { by: ghost });
                    }
                }
                app.update();
                app.update();
                assert_eq!(tag(&app, sprite), Some(expected), "{name}, round {round}");
            }
        }
    }

    #[test]
    fn a_marker_puts_a_ghost_with_its_feet_at_the_cell_centre_and_one_with_a_bad_strategy_is_skipped()
     {
        use crate::test_util::{asset_app, map, object, spawn_with};
        let mut app = asset_app(16);
        let map = map(
            48.0,
            vec![
                object("spawn_ghost", 2, 3, &[("strategy", "pursue")]),
                object("spawn_ghost", 5, 1, &[("strategy", "dance")]),
                object("rock_0", 0, 0, &[]),
            ],
        );
        let load = MapLoad {
            id: "m".to_string(),
            origin: Vec2::new(10.0, 20.0),
        };
        spawn_with(&mut app, move |commands, server| {
            spawn_ghosts(commands, server, &map, &load);
        });
        let ghosts: Vec<(Vec2, String, bool)> = app
            .world_mut()
            .query_filtered::<(&Transform, &MapId, Has<Pursue>), With<Ghost>>()
            .iter(app.world())
            .map(|(transform, id, pursue)| (transform.translation.truncate(), id.0.clone(), pursue))
            .collect();
        assert_eq!(
            ghosts,
            vec![(
                Vec2::new(10.0 + 2.5 * 48.0, 20.0 + 3.5 * 48.0),
                "m".to_string(),
                true
            )]
        );
    }
}
