//! The ghost (`adr-lw29`): the first enemy. A `spawn_ghost` marker of a map (legend entry such as
//! `"!": spawn_ghost strategy=sentry radius=40 sight=500`) puts one ghost at the middle of its cell when the map
//! loads, once: it does not respawn while the map stays loaded, and comes back when the map is loaded again. The kit
//! does the walking and the sight (`game_kit::strategies`, `movement`, `nav`, `touch`). Here are its values and its
//! art, `characters/ghost.aseprite` (tags `Down`, `Left`, `Up`, `Attack` and `Die`, durations set in the file).
//!
//! When a ghost touches the player it stops and plays `Attack` once. When that ends the player loses 1 HP (`hp.rs`)
//! and the ghost dies: it plays `Die` once and is removed when that ends.

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;
use game_kit::animation::{FacingTags, tag_sprite};
use game_kit::depth::DepthSort;
use game_kit::hitbox::HitBox;
use game_kit::map::Map;
use game_kit::map_transition::{MapId, MapLoad};
use game_kit::movement::Mover;
use game_kit::strategies::{Pursue, RandomPatrol, Sentry, Strategy};
use game_kit::touch::{ControlledTouched, TouchSet, Touchable};

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

pub struct GhostPlugin;

impl Plugin for GhostPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (start_attack.after(TouchSet), finish_animations));
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
                .remove::<FacingTags>()
                .insert(Attacking);
            if let Ok(mut ase) = animations.get_mut(*child) {
                ase.animation.play("Attack", AnimationRepeat::Count(0));
            }
        }
        debug!("ghost: {:?} touched the player and attacks", touch.by);
    }
}

/// When `Attack` has played the player loses HP and the ghost plays `Die`. When `Die` has played the ghost is removed.
fn finish_animations(
    mut commands: Commands,
    mut events: MessageReader<AnimationEvents>,
    mut damage: MessageWriter<DamagePlayer>,
    mut sprites: Query<(&ChildOf, &mut AseAnimation, Has<Attacking>, Has<Dying>)>,
) {
    for event in events.read() {
        let AnimationEvents::Finished(entity) = event else {
            continue;
        };
        let Ok((parent, mut ase, attacking, dying)) = sprites.get_mut(*entity) else {
            continue;
        };
        if attacking {
            damage.write(DamagePlayer { amount: DAMAGE });
            commands.entity(*entity).remove::<Attacking>().insert(Dying);
            ase.animation.play("Die", AnimationRepeat::Count(0));
        } else if dying {
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
            .add_systems(Update, (start_attack, finish_animations).chain());
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
        app.world()
            .resource::<Messages<DamagePlayer>>()
            .iter_current_update_messages()
            .count()
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
    fn a_ghost_that_is_already_attacking_is_not_touched_again() {
        let mut app = app();
        let (ghost, sprite) = spawn_ghost(&mut app);
        for _ in 0..2 {
            app.world_mut()
                .write_message(ControlledTouched { by: ghost });
            app.update();
        }
        app.world_mut()
            .write_message(AnimationEvents::Finished(sprite));
        app.update();
        assert_eq!(damage_messages(&app), 1, "one touch, one damage");
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
}
