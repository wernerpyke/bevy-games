use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;

use crate::camera::MAP_ORIGIN;
use crate::collision::{PLAYER_BODY_HEIGHT, PLAYER_FEET_OFFSET, PLAYER_HALF_WIDTH};
use crate::hp::{Hp, PlayerHurt};
use game_kit::animation::play_if_changed;
use game_kit::collision::SlideAssist;
use game_kit::depth::DepthSort;
use game_kit::hitbox::HitBox;
use game_kit::map::{Facing as EntryFacing, start_feet};
use game_kit::map_transition::{ActiveMaps, Arrival, MapRegistry, MapTransition, clamp_to_union};
use game_kit::movement::{FollowSet, MoveGoal, MoveState, Mover, StrategySet};
use game_kit::nav::{NavGrid, NavInset};
use game_kit::pointer::WorldClick;

/// Walk speed in art pixels per second.
pub const WALK_SPEED: f32 = 300.0;
/// Playback speed multiplier for the walking animations. 1.0 is the speed set in Aseprite.
const WALK_ANIMATION_SPEED: f32 = 2.5;
/// The `Run` tag in `Hunter.aseprite` is the sideways walk. Set to match the way the art faces.
const WALK_ART_FACES_RIGHT: bool = true;
/// The body that enemies touch (`game_kit::touch`). It is relative to the `Transform`, which is the centre of the canvas, so it
/// runs from the feet (`PLAYER_FEET_OFFSET` below) up by the body height.
const HIT_BOX: HitBox = HitBox {
    offset: Vec2::new(0.0, PLAYER_BODY_HEIGHT / 2.0 - PLAYER_FEET_OFFSET),
    half_size: Vec2::new(PLAYER_HALF_WIDTH, PLAYER_BODY_HEIGHT / 2.0),
};
/// Draw the `Weapon` layer of `Hunter.aseprite`. The `background` layer is never drawn.
const SHOW_WEAPON: bool = false;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        // Paths stay where the feet can stand (`limit_player`): half a body width from the sides, a body height from the top.
        app.insert_resource(NavInset {
            min: Vec2::new(PLAYER_HALF_WIDTH, 0.0),
            max: Vec2::new(PLAYER_HALF_WIDTH, PLAYER_BODY_HEIGHT),
        })
        .add_systems(Startup, spawn_player)
        .add_systems(Update, set_goal.in_set(StrategySet))
        .add_systems(
            Update,
            (limit_player, hurt_player, animate_player)
                .chain()
                .after(FollowSet),
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Facing {
    Up,
    Down,
    Side,
}

#[derive(Component)]
pub struct Player {
    facing: Facing,
    walking: bool,
    /// The `Hurt` animation is playing. It plays once and takes over from walking and idle.
    hurt: bool,
}

/// Layers of `Hunter.aseprite` that the game draws, by name.
fn hunter_layers() -> Vec<String> {
    let mut layers = vec!["Body".to_string()];
    if SHOW_WEAPON {
        layers.push("Weapon".to_string());
    }
    layers
}

/// Spawns the player with the feet at the centre of the `player_start` cell of the map. A map without the marker
/// is reported and the player starts at the bottom middle.
pub fn spawn_player(
    mut commands: Commands,
    server: Res<AssetServer>,
    registry: Res<MapRegistry>,
    active: Res<ActiveMaps>,
) {
    let Some(registered) = registry.get(&active.current) else {
        error!(
            "player: the start map `{}` is not registered",
            active.current
        );
        return;
    };
    let map = &registered.map;
    let feet = start_feet(map, MAP_ORIGIN).unwrap_or_else(|| {
        error!(
            "map {}: no player_start marker, starting at the bottom middle",
            map.name
        );
        let width = map.size.x as f32 * map.cell_size;
        Vec2::new(MAP_ORIGIN.x + width / 2.0, MAP_ORIGIN.y)
    });
    let start = feet + Vec2::new(0.0, PLAYER_FEET_OFFSET);
    commands.spawn((
        game_kit::Controlled,
        Hp::default(),
        HIT_BOX,
        Player {
            facing: Facing::Down,
            walking: false,
            hurt: false,
        },
        AseAnimation {
            aseprite: server
                .load_builder()
                .with_settings::<AsepriteLoaderSettings>(|settings| {
                    settings.layer_selection = LayerSelectionSetting::Mask(hunter_layers());
                })
                .load("art/characters/Hunter.aseprite"),
            animation: Animation::tag("Idle").with_repeat(AnimationRepeat::Loop),
        },
        Sprite::default(),
        DepthSort {
            feet_offset: PLAYER_FEET_OFFSET,
        },
        Mover::new(WALK_SPEED).with_feet_offset(PLAYER_FEET_OFFSET),
        Transform::from_translation(start.extend(0.0)),
    ));
}

/// A left click sets the goal. The player walks there by the nav grid (`game_kit::movement`), around blocked cells. A
/// click on a blocked place, or outside the walkable area, goes to the nearest place the player can reach.
fn set_goal(
    mut clicks: MessageReader<WorldClick>,
    active: Res<ActiveMaps>,
    transition: Res<MapTransition>,
    nav: Res<NavGrid>,
    mut query: Query<(&Transform, &mut MoveGoal), With<Player>>,
) {
    let click = clicks
        .read()
        .filter(|click| click.button == MouseButton::Left)
        .last();
    let Some(click) = click else {
        return;
    };
    // The player stands still while the screen fades, and the click is lost.
    if transition.locks_input() {
        return;
    }
    let walkable = walkable_areas(&active);
    for (transform, mut goal) in &mut query {
        let feet = feet_of(transform);
        let wanted = clamp_to_union(&walkable, feet, click.position);
        goal.0 = nav.nearest_reachable(feet, wanted);
    }
}

/// The animation facing for a direction of walking. Sideways wins when it is as strong as the vertical part, so the
/// sideways walk plays on a diagonal.
fn facing_of(direction: Vec2) -> Facing {
    if direction.x.abs() >= direction.y.abs() {
        Facing::Side
    } else if direction.y > 0.0 {
        Facing::Up
    } else {
        Facing::Down
    }
}

pub fn feet_of(transform: &Transform) -> Vec2 {
    transform.translation.truncate() - Vec2::new(0.0, PLAYER_FEET_OFFSET)
}

/// The areas the feet may stand in. They stay inside a map, and the body stays below its top edge. They may cross from one
/// map to another only through a doorway of a seamless exit.
fn walkable_areas(active: &ActiveMaps) -> Vec<Rect> {
    active.walkable(
        Vec2::new(PLAYER_HALF_WIDTH, 0.0),
        Vec2::new(PLAYER_HALF_WIDTH, PLAYER_BODY_HEIGHT),
    )
}

/// Keeps the feet in the walkable areas after `follow_goals` moved them (the mover only knows the rectangle of the
/// camera), and turns what the mover did into the animation state. The player stands still while the screen fades.
fn limit_player(
    active: Res<ActiveMaps>,
    transition: Res<MapTransition>,
    mut query: Query<(
        &mut Transform,
        &mut Player,
        &mut MoveGoal,
        &MoveState,
        &mut SlideAssist,
    )>,
) {
    let walkable = walkable_areas(&active);
    let locked = transition.locks_input();
    for (mut transform, mut player, mut goal, state, mut assist) in &mut query {
        if locked {
            goal.0 = None;
            assist.reset();
            player.walking = false;
            continue;
        }
        player.walking = state.walking;
        if !state.walking {
            continue;
        }
        let feet = feet_of(&transform);
        let limited = clamp_to_union(&walkable, feet, feet);
        transform.translation.x = limited.x;
        transform.translation.y = limited.y + PLAYER_FEET_OFFSET;

        player.facing = facing_of(state.facing);
        if player.facing == Facing::Side {
            let faces_right = state.facing.x > 0.0;
            transform.scale.x = if faces_right == WALK_ART_FACES_RIGHT {
                1.0
            } else {
                -1.0
            };
        }
    }
}

fn animate_player(mut query: Query<(&mut AseAnimation, &Player)>) {
    for (mut ase, player) in &mut query {
        if player.hurt {
            continue;
        }
        let tag = match (player.walking, player.facing) {
            (false, _) => "Idle",
            (true, Facing::Side) => "Run",
            (true, Facing::Up) => "Up",
            (true, Facing::Down) => "Down",
        };
        let speed = if player.walking {
            WALK_ANIMATION_SPEED
        } else {
            1.0
        };
        play_if_changed(&mut ase, tag, speed);
    }
}

/// Plays `Hurt` once when the player loses HP, and hands back to walking and idle when it has played.
fn hurt_player(
    mut hurt: MessageReader<PlayerHurt>,
    mut finished: MessageReader<AnimationEvents>,
    mut query: Query<(Entity, &mut AseAnimation, &mut Player)>,
) {
    let hits = hurt.read().count();
    let ended: Vec<Entity> = finished
        .read()
        .filter_map(|event| match event {
            AnimationEvents::Finished(entity) => Some(*entity),
            _ => None,
        })
        .collect();
    for (entity, mut ase, mut player) in &mut query {
        if hits > 0 {
            player.hurt = true;
            ase.animation.play("Hurt", AnimationRepeat::Count(0));
            ase.animation.speed = 1.0;
        } else if player.hurt && ended.contains(&entity) {
            player.hurt = false;
        }
    }
}

/// Puts the player in the entry after a map transition (a hook of `game_kit::map_transition`): the feet go to the
/// arrival position and the player looks the way the entry says.
pub fn arrive_player(
    In(arrival): In<Arrival>,
    active: Res<ActiveMaps>,
    mut query: Query<(&mut Transform, &mut Player, &mut SlideAssist, &mut MoveGoal)>,
) {
    // An entry at the edge of a map can put the feet where the body would be cut off by the edge of the view (the top
    // of the map) or outside the walkable area. Move them to the nearest place the player can stand, as `move_player` does.
    let walkable = active.walkable(
        Vec2::new(PLAYER_HALF_WIDTH, 0.0),
        Vec2::new(PLAYER_HALF_WIDTH, PLAYER_BODY_HEIGHT),
    );
    let feet = clamp_to_union(&walkable, arrival.feet, arrival.feet);
    for (mut transform, mut player, mut assist, mut goal) in &mut query {
        goal.0 = None;
        transform.translation.x = feet.x;
        transform.translation.y = feet.y + PLAYER_FEET_OFFSET;
        assist.reset();
        player.walking = false;
        let (facing, faces_right) = match arrival.facing {
            None => continue,
            Some(EntryFacing::Up) => (Facing::Up, None),
            Some(EntryFacing::Down) => (Facing::Down, None),
            Some(EntryFacing::Left) => (Facing::Side, Some(false)),
            Some(EntryFacing::Right) => (Facing::Side, Some(true)),
        };
        player.facing = facing;
        if let Some(right) = faces_right {
            transform.scale.x = if right == WALK_ART_FACES_RIGHT {
                1.0
            } else {
                -1.0
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> (App, Entity) {
        let mut app = App::new();
        app.add_message::<PlayerHurt>()
            .add_message::<AnimationEvents>()
            .add_systems(Update, (hurt_player, animate_player).chain());
        let player = app
            .world_mut()
            .spawn((
                Player {
                    facing: Facing::Down,
                    walking: false,
                    hurt: false,
                },
                AseAnimation::default(),
            ))
            .id();
        (app, player)
    }

    fn tag(app: &App, player: Entity) -> Option<String> {
        app.world()
            .get::<AseAnimation>(player)
            .unwrap()
            .animation
            .tag
            .clone()
    }

    #[test]
    fn the_facing_follows_the_direction_of_walking() {
        assert!(facing_of(Vec2::new(1.0, 0.2)) == Facing::Side);
        assert!(facing_of(Vec2::new(-0.7, 0.7)) == Facing::Side);
        assert!(facing_of(Vec2::new(0.2, 1.0)) == Facing::Up);
        assert!(facing_of(Vec2::new(0.2, -1.0)) == Facing::Down);
    }

    #[test]
    fn a_hit_plays_hurt_once_and_idle_comes_back_when_it_ends() {
        let (mut app, player) = app();
        app.update();
        assert_eq!(tag(&app, player).as_deref(), Some("Idle"));
        app.world_mut().write_message(PlayerHurt);
        app.update();
        assert_eq!(tag(&app, player).as_deref(), Some("Hurt"));
        let ase = app.world().get::<AseAnimation>(player).unwrap();
        assert!(matches!(ase.animation.repeat, AnimationRepeat::Count(0)));
        // Walking does not cut `Hurt` short.
        app.world_mut().get_mut::<Player>(player).unwrap().walking = true;
        app.update();
        assert_eq!(tag(&app, player).as_deref(), Some("Hurt"));
        app.world_mut()
            .write_message(AnimationEvents::Finished(player));
        app.update();
        assert_eq!(tag(&app, player).as_deref(), Some("Down"));
    }
}
