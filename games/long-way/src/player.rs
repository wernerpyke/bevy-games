use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;

use crate::camera::MAP_ORIGIN;
use crate::collision::{PLAYER_BODY_HEIGHT, PLAYER_FEET_OFFSET, PLAYER_HALF_WIDTH};
use crate::hp::{Hp, PlayerHurt};
use game_kit::animation::play_if_changed;
use game_kit::collision::{CollisionSettings, CollisionWorld, SlideAssist, move_feet};
use game_kit::depth::DepthSort;
use game_kit::hitbox::HitBox;
use game_kit::map::{Facing as EntryFacing, start_feet};
use game_kit::map_transition::{ActiveMaps, Arrival, MapRegistry, MapTransition, clamp_to_union};
use game_kit::movement::{FollowSet, MoveGoal, MoveState, Mover, StrategySet};
use game_kit::nav::{NavGrid, NavInset};
use game_kit::outline::Edge;
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
        .add_systems(
            Update,
            (set_goal, stop_goal_on_keys).chain().in_set(StrategySet),
        )
        .add_systems(
            Update,
            (walk_by_keys, limit_player, hurt_player, animate_player)
                .chain()
                .after(FollowSet),
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
        KeyAssist::default(),
        Transform::from_translation(start.extend(0.0)),
    ));
}

/// The slide assist of the walk by the arrow keys. The mover's `SlideAssist` cannot be used: `follow_goals` resets it in
/// every frame without a goal, so the time pushed into a trunk would never add up.
#[derive(Component, Default)]
struct KeyAssist(SlideAssist);

/// The way the arrow keys point, not normalised. Two opposite keys cancel each other.
fn key_direction(keys: &ButtonInput<KeyCode>) -> Vec2 {
    let mut direction = Vec2::ZERO;
    if keys.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }
    if keys.pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }
    direction
}

/// While an arrow key is held, the player has no goal: the keys win over a click in the same frame and stop a walk to a
/// clicked goal. Runs after `set_goal`.
fn stop_goal_on_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut MoveGoal, With<Player>>,
) {
    if key_direction(&keys) == Vec2::ZERO {
        return;
    }
    for mut goal in &mut query {
        goal.0 = None;
    }
}

/// Walks the player the way the arrow keys point, with the collision and sliding rules of `move_feet`. It writes
/// `MoveState` as the mover does, so `limit_player`, the animation and the aim of the weapons treat both walks the same.
fn walk_by_keys(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    transition: Res<MapTransition>,
    world: Res<CollisionWorld>,
    settings: Res<CollisionSettings>,
    mut query: Query<(&mut Transform, &mut MoveState, &mut KeyAssist), With<Player>>,
) {
    let direction = key_direction(&keys);
    let locked = transition.locks_input();
    for (mut transform, mut state, mut assist) in &mut query {
        walk_one(
            direction,
            locked,
            time.delta_secs(),
            &settings,
            world.edges(),
            &mut transform,
            &mut state,
            &mut assist.0,
        );
    }
}

/// `walk_by_keys` for one player. No direction, or a fade, leaves the player as it is.
#[allow(clippy::too_many_arguments)]
fn walk_one(
    direction: Vec2,
    locked: bool,
    dt: f32,
    settings: &CollisionSettings,
    edges: &[Edge],
    transform: &mut Transform,
    state: &mut MoveState,
    assist: &mut SlideAssist,
) {
    if direction == Vec2::ZERO || locked {
        assist.reset();
        return;
    }
    let direction = direction.normalize();
    let old = feet_of(transform);
    let feet = move_feet(
        settings,
        assist,
        old,
        direction * WALK_SPEED * dt,
        dt,
        edges,
    );
    transform.translation.x = feet.x;
    transform.translation.y = feet.y + PLAYER_FEET_OFFSET;
    // The walk plays also while a trunk blocks the feet, as it did before the mouse.
    state.walking = true;
    state.facing = direction;
}

/// A left click sets the goal. The player walks there by the nav grid (`game_kit::movement`), around blocked cells. A
/// click on a blocked place, or outside the walkable area, goes to the nearest place the player can reach. A held arrow
/// key clears the goal again (`stop_goal_on_keys`).
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
        limit_one(
            locked,
            &walkable,
            &mut transform,
            &mut player,
            &mut goal,
            state,
            &mut assist,
        );
    }
}

/// `limit_player` for one player.
fn limit_one(
    locked: bool,
    walkable: &[Rect],
    transform: &mut Transform,
    player: &mut Player,
    goal: &mut MoveGoal,
    state: &MoveState,
    assist: &mut SlideAssist,
) {
    if locked {
        goal.0 = None;
        assist.reset();
        player.walking = false;
        return;
    }
    player.walking = state.walking;
    if !state.walking {
        return;
    }
    let feet = feet_of(transform);
    let limited = clamp_to_union(walkable, feet, feet);
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
        assert_eq!(facing_of(Vec2::new(1.0, 0.2)), Facing::Side);
        assert_eq!(facing_of(Vec2::new(-0.7, 0.7)), Facing::Side);
        assert_eq!(facing_of(Vec2::new(0.2, 1.0)), Facing::Up);
        assert_eq!(facing_of(Vec2::new(0.2, -1.0)), Facing::Down);
    }

    /// The scale x of the sprite when the player looks right, and left.
    fn right_and_left() -> (f32, f32) {
        let right = if WALK_ART_FACES_RIGHT { 1.0 } else { -1.0 };
        (right, -right)
    }

    /// A player whose feet are at `feet`, looking down, with a scale x that no rule sets, so a change shows.
    fn player_at(feet: Vec2) -> (Transform, Player) {
        let transform = Transform::from_xyz(feet.x, feet.y + PLAYER_FEET_OFFSET, 0.0)
            .with_scale(Vec3::new(0.5, 1.0, 1.0));
        let player = Player {
            facing: Facing::Down,
            walking: false,
            hurt: false,
        };
        (transform, player)
    }

    const WALKABLE: [Rect; 1] = [Rect {
        min: Vec2::new(10.0, 20.0),
        max: Vec2::new(300.0, 200.0),
    }];

    #[test]
    fn a_walking_player_is_kept_in_the_walkable_area_and_faces_the_way_it_walks() {
        let (right, left) = right_and_left();
        for (direction, facing, scale) in [
            (Vec2::X, Facing::Side, right),
            (Vec2::NEG_X, Facing::Side, left),
            (Vec2::Y, Facing::Up, 0.5),
            (Vec2::NEG_Y, Facing::Down, 0.5),
        ] {
            let (mut transform, mut player) = player_at(Vec2::new(350.0, 100.0));
            let state = MoveState {
                walking: true,
                facing: direction,
                ..default()
            };
            limit_one(
                false,
                &WALKABLE,
                &mut transform,
                &mut player,
                &mut MoveGoal(None),
                &state,
                &mut SlideAssist::default(),
            );
            assert_eq!(
                transform.translation.truncate(),
                Vec2::new(300.0, 100.0 + PLAYER_FEET_OFFSET),
                "{direction}"
            );
            assert_eq!(
                (player.walking, player.facing, transform.scale.x),
                (true, facing, scale),
                "{direction}"
            );
        }
    }

    #[test]
    fn a_player_that_does_not_walk_or_is_locked_by_a_transition_stands_as_it_is() {
        for locked in [false, true] {
            let (mut transform, mut player) = player_at(Vec2::new(350.0, 100.0));
            player.walking = true;
            let before = transform;
            let mut goal = MoveGoal(Some(Vec2::new(50.0, 50.0)));
            // While locked the mover's state says walking, and it is ignored.
            let state = MoveState {
                walking: locked,
                facing: Vec2::NEG_X,
                ..default()
            };
            limit_one(
                locked,
                &WALKABLE,
                &mut transform,
                &mut player,
                &mut goal,
                &state,
                &mut SlideAssist::default(),
            );
            assert_eq!(transform, before, "locked {locked}");
            assert_eq!((player.walking, player.facing), (false, Facing::Down));
            let kept = if locked {
                None
            } else {
                Some(Vec2::new(50.0, 50.0))
            };
            assert_eq!(goal.0, kept, "locked {locked}");
        }
    }

    fn keys(held: &[KeyCode]) -> ButtonInput<KeyCode> {
        let mut keys = ButtonInput::default();
        for key in held {
            keys.press(*key);
        }
        keys
    }

    #[test]
    fn each_arrow_key_points_its_way_and_opposite_keys_cancel() {
        use KeyCode::{ArrowDown, ArrowLeft, ArrowRight, ArrowUp};
        for (held, direction) in [
            (vec![], Vec2::ZERO),
            (vec![ArrowLeft], Vec2::NEG_X),
            (vec![ArrowRight], Vec2::X),
            (vec![ArrowUp], Vec2::Y),
            (vec![ArrowDown], Vec2::NEG_Y),
            (vec![ArrowLeft, ArrowUp], Vec2::new(-1.0, 1.0)),
            (vec![ArrowLeft, ArrowRight, ArrowDown], Vec2::NEG_Y),
            (vec![ArrowUp, ArrowDown], Vec2::ZERO),
        ] {
            assert_eq!(key_direction(&keys(&held)), direction, "{held:?}");
        }
    }

    #[test]
    fn a_held_arrow_key_clears_the_goal_and_no_key_keeps_it() {
        for (held, kept) in [
            (vec![], Some(Vec2::new(50.0, 60.0))),
            (vec![KeyCode::ArrowDown], None),
            // Opposite keys do not walk, so they do not stop a walk to a click.
            (
                vec![KeyCode::ArrowLeft, KeyCode::ArrowRight],
                Some(Vec2::new(50.0, 60.0)),
            ),
        ] {
            let mut app = App::new();
            app.insert_resource(keys(&held))
                .add_systems(Update, stop_goal_on_keys);
            let (_, player) = player_at(Vec2::ZERO);
            let entity = app
                .world_mut()
                .spawn((player, MoveGoal(Some(Vec2::new(50.0, 60.0)))))
                .id();
            app.update();
            assert_eq!(
                app.world().get::<MoveGoal>(entity).unwrap().0,
                kept,
                "{held:?}"
            );
        }
    }

    const SETTINGS: CollisionSettings = CollisionSettings {
        allow_sliding: true,
        slide: game_kit::collision::SlideTuning::for_cell_size(48.0),
    };

    #[test]
    fn the_arrow_keys_walk_the_feet_at_walk_speed_and_a_trunk_stops_them_while_the_walk_plays() {
        // A diagonal is as fast as a straight walk: 0.1 s at 300 px/s is 30 pixels, 21.21 on each axis.
        let (mut transform, _) = player_at(Vec2::new(100.0, 100.0));
        let mut state = MoveState::default();
        walk_one(
            Vec2::new(-1.0, 1.0),
            false,
            0.1,
            &SETTINGS,
            &[],
            &mut transform,
            &mut state,
            &mut SlideAssist::default(),
        );
        let diagonal = 30.0 / 2.0_f32.sqrt();
        let expected = Vec2::new(100.0 - diagonal, 100.0 + diagonal);
        // f32 rounding of the normalised direction and the step.
        assert!(
            feet_of(&transform).distance(expected) < 1e-3,
            "{:?}",
            feet_of(&transform)
        );
        assert!(state.walking);
        assert!(state.facing.distance(Vec2::new(-1.0, 1.0).normalize()) < 1e-6);

        // A blocked area 10 pixels to the right: the feet stop at its left side, and the walk still plays.
        let mut world = CollisionWorld::default();
        world.add_blocked(&[Rect::new(110.0, 50.0, 200.0, 150.0)]);
        let (mut transform, _) = player_at(Vec2::new(100.0, 100.0));
        let mut state = MoveState {
            facing: Vec2::NEG_Y,
            ..default()
        };
        walk_one(
            Vec2::X,
            false,
            0.1,
            &SETTINGS,
            world.edges(),
            &mut transform,
            &mut state,
            &mut SlideAssist::default(),
        );
        assert!(
            feet_of(&transform).distance(Vec2::new(110.0, 100.0)) < 1e-3,
            "{:?}",
            feet_of(&transform)
        );
        assert_eq!((state.walking, state.facing), (true, Vec2::X));
    }

    #[test]
    fn no_arrow_key_or_a_transition_leaves_the_player_as_it_is() {
        for (direction, locked) in [(Vec2::ZERO, false), (Vec2::X, true)] {
            let (mut transform, _) = player_at(Vec2::new(100.0, 100.0));
            let before = transform;
            let mut state = MoveState {
                facing: Vec2::NEG_Y,
                ..default()
            };
            walk_one(
                direction,
                locked,
                0.1,
                &SETTINGS,
                &[],
                &mut transform,
                &mut state,
                &mut SlideAssist::default(),
            );
            assert_eq!(transform, before, "locked {locked}");
            assert_eq!(
                (state.walking, state.facing),
                (false, Vec2::NEG_Y),
                "locked {locked}"
            );
        }
    }

    #[test]
    fn an_arrival_puts_the_feet_in_the_walkable_area_and_turns_the_player_the_way_of_the_entry() {
        use bevy::ecs::system::RunSystemOnce;
        let (right, left) = right_and_left();
        for (entry, facing, scale) in [
            (None, Facing::Down, 0.5),
            (Some(EntryFacing::Up), Facing::Up, 0.5),
            (Some(EntryFacing::Down), Facing::Down, 0.5),
            (Some(EntryFacing::Left), Facing::Side, left),
            (Some(EntryFacing::Right), Facing::Side, right),
        ] {
            let mut app = App::new();
            app.insert_resource(ActiveMaps {
                current: "1".to_string(),
                maps: vec![game_kit::map_transition::ActiveMap {
                    id: "1".to_string(),
                    origin: Vec2::new(40.0, 0.0),
                    rect: Rect::new(40.0, 0.0, 520.0, 400.0),
                }],
                doorways: Vec::new(),
            });
            let (transform, mut player) = player_at(Vec2::new(200.0, 100.0));
            player.walking = true;
            let entity = app
                .world_mut()
                .spawn((
                    transform,
                    player,
                    SlideAssist::default(),
                    MoveGoal(Some(Vec2::ONE)),
                ))
                .id();
            // The feet arrive left of the map: they are moved in by half a body width.
            let arrival = Arrival {
                feet: Vec2::new(30.0, 150.0),
                facing: entry,
            };
            app.world_mut()
                .run_system_once_with(arrive_player, arrival)
                .unwrap();
            let world = app.world();
            let transform = world.get::<Transform>(entity).unwrap();
            let player = world.get::<Player>(entity).unwrap();
            assert_eq!(
                transform.translation.truncate(),
                Vec2::new(40.0 + PLAYER_HALF_WIDTH, 150.0 + PLAYER_FEET_OFFSET),
                "{entry:?}"
            );
            assert_eq!(
                (player.walking, player.facing, transform.scale.x),
                (false, facing, scale),
                "{entry:?}"
            );
            assert_eq!(world.get::<MoveGoal>(entity).unwrap().0, None);
        }
    }

    #[test]
    fn a_hit_plays_hurt_once_and_the_walking_tag_comes_back_when_it_ends() {
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
