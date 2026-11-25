//! The volcano: an enemy that stands still where a `spawn_volcano` marker of a map puts it (legend entry such as
//! `"^": spawn_volcano`) and attacks until it is killed. It loops its `Attack` tag (`characters/volcano.aseprite`,
//! durations set in the file). Its base blocks the player, who can walk up to it and behind it: only its shots hurt.
//!
//! **Weapons** (`adr-lw43`, `REACTIONS`). It has hit points, but no current weapon damages it, and nothing pushes it.
//! Red touches it with no effect. Blue interrupts it: it plays `Hurt` once, then `Attack` from the first frame. It
//! launches nothing while `Hurt` plays.
//!
//! **Shots.** Each time `Attack` reaches frame 7 and frame 9 the volcano launches a shot from its mouth in a random
//! direction. The
//! kit `ProjectilePath` moves it: an arc to the ground, 2 bounces, then a roll that slows to a stop after 5 cells, and
//! there it is removed. It collides where the ball is drawn (the user, 2026-11-23): a blocked area or the map edge
//! that the ball reaches on screen removes it, and a ball that touches the player's body takes 1 HP and is removed.
//! It flies through its own volcano's base (`Projectile::ignoring`). It does not hurt enemies and weapons do not hit
//! it. The player has no invincibility frames.

use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;
use game_kit::collision::{Blocker, grow_sideways};
use game_kit::depth::SortAtBlocker;
use game_kit::hitbox::HitBox;
use game_kit::map::Map;
use game_kit::map_transition::{MapId, MapLoad};
use game_kit::projectile::{PathShape, Projectile, ProjectilePath, ProjectilePlugin};
use game_kit::rng::GameRng;
use game_kit::touch::{ControlledTouched, TouchSet, Touchable};

use crate::collision::PLAYER_HALF_WIDTH;
use crate::enemy_hp::EnemyHp;
use crate::hp::DamagePlayer;
use crate::map::Kind;
use crate::reactions::{EnemyInterrupted, Hit, Reaction, Reactions};

const ART: &str = "art/characters/volcano.aseprite";
/// The layers drawn in the game. The file also has `test_bg`, a backdrop for drawing that is never drawn in the game,
/// even when it is saved visible. A new art layer must be added here.
const LAYERS: [&str; 3] = ["base", "rim", "lava"];
/// The volcano is drawn at this many times the art size. The file stays at the size it is drawn at (`adr-a11`).
const SCALE: f32 = 2.0;
/// The art canvas is 64 by 64 and the lowest row of the body is row 62 of 64, so the bottom of the body is 31 art
/// pixels below the centre.
const FEET_BELOW_CENTRE: f32 = 31.0 * SCALE;
/// The base of the cone in art pixels from the left and top of the canvas: x 11 to 53 is the main cone (the thin
/// skirt reaches x 5 to 62 at row 50), and rows 50 to 62 are the bottom, where the skirt is widest. The feet may not
/// enter it, so the player stops at the bottom and the sides and walks behind the cone above row 50.
const BASE_LEFT_PX: f32 = 11.0;
const BASE_RIGHT_PX: f32 = 54.0;
const BASE_TOP_PX: f32 = 50.0;
const BASE_BOTTOM_PX: f32 = 63.0;
const CANVAS_HALF_PX: f32 = 32.0;
/// `Attack` launches one shot when it reaches each of these frames: frames 7 and 9 of the tag, counted from 0 (the
/// user, 2026-11-23). Each comes with the row of the middle of the crater in that frame, at the centre x: the `rim`
/// layer covers rows 18 to 25 in frame 7 and rows 11 to 18 in frame 9.
const LAUNCHES: [(u16, f32); 2] = [(6, 22.0), (8, 15.0)];

const SHOT_TAG: &str = "Projectile";
/// The shot is drawn at half size (the user, 2026-11-23): its art is drawn in 4 by 4 dots, so on screen a dot is 2 px,
/// as on the volcano.
const SHOT_SCALE: f32 = 0.5;
/// The ball is about 32 art px wide and centred on the canvas, so its bottom is 16 art px (8 px) below the centre. On
/// the ground its centre is this far up, so it rests on the ground.
const BALL_HALF: f32 = 16.0 * SHOT_SCALE;
/// The ball as drawn: it touches the player's body where they overlap on screen.
const SHOT_HIT_BOX: HitBox = HitBox {
    offset: Vec2::ZERO,
    half_size: Vec2::new(BALL_HALF, BALL_HALF),
};
const SHOT_DAMAGE: u32 = 1;
/// Hit points of a volcano. A placeholder: no weapon damages it yet (`docs/enemies.md`).
const HIT_POINTS: f32 = 300.0;
/// How a volcano reacts to each weapon energy (`adr-lw43`, the user, 2026-11-25). It never moves. Red touches it with
/// no effect (a fireball explodes on it). Blue interrupts it. Violet and black damage it: placeholders, as neither has
/// a weapon yet.
const REACTIONS: Reactions = Reactions {
    red: Reaction::Hit(Hit {
        damage: 0.0,
        push: 0.0,
        interrupt: false,
    }),
    blue: Reaction::Hit(Hit {
        damage: 0.0,
        push: 0.0,
        interrupt: true,
    }),
    violet: Reaction::Hit(Hit {
        damage: 1.0,
        push: 0.0,
        interrupt: false,
    }),
    black: Reaction::Hit(Hit {
        damage: 1.0,
        push: 0.0,
        interrupt: false,
    }),
};
/// The top of the cone at rest, in art pixels from the top of the canvas (frames 3 to 5 of `Attack`).
const CONE_TOP_PX: f32 = 20.0;
/// How far the hit box reaches past the cone and the blocked base. The ice ring stops at the base (`adr-lw35`), so a
/// body that ends exactly on its edge would not be reached; this margin is above the ring's edge tolerance (0.5 px).
const HIT_MARGIN: f32 = 2.0;
/// The part of a volcano that weapons touch, from its centre: the drawn cone and the blocked base around it (which
/// is wider by half the player's width), with `HIT_MARGIN` all round. x from 11 - 14/2 art px to 54 + 14/2, y from
/// the bottom of the base (row 63) to `CONE_TOP_PX`.
const HIT_BOX: HitBox = {
    let left = (BASE_LEFT_PX - CANVAS_HALF_PX) * SCALE - PLAYER_HALF_WIDTH - HIT_MARGIN;
    let right = (BASE_RIGHT_PX - CANVAS_HALF_PX) * SCALE + PLAYER_HALF_WIDTH + HIT_MARGIN;
    let bottom = -FEET_BELOW_CENTRE - HIT_MARGIN;
    let top = (CANVAS_HALF_PX - CONE_TOP_PX) * SCALE + HIT_MARGIN;
    HitBox {
        offset: Vec2::new((left + right) / 2.0, (bottom + top) / 2.0),
        half_size: Vec2::new((right - left) / 2.0, (top - bottom) / 2.0),
    }
};
const HURT_TAG: &str = "Hurt";
const ATTACK_TAG: &str = "Attack";
/// 5 cells of 64 px in 2.5 s. The ball starts with its centre at the mouth, reaches 96 px above the ground, bounces
/// twice at half height and rolls for the last 30 % of the time (`PathShape`). `start_height` is set per volcano.
const SHOT_PATH: PathShape = PathShape {
    distance: 5.0 * 64.0,
    duration: 2.5,
    start_height: 0.0,
    peak: 96.0,
    bounces: 2,
    bounce_ratio: 0.5,
    roll_share: 0.3,
};

pub struct VolcanoPlugin;

impl Plugin for VolcanoPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ProjectilePlugin>() {
            app.add_plugins(ProjectilePlugin);
        }
        app.add_systems(
            Update,
            (
                (interrupt, finish_hurt, launch_shots).chain(),
                hit_player.after(TouchSet),
            ),
        );
    }
}

/// On a volcano that plays `Hurt` after an interrupt. It launches nothing until `Attack` starts again.
#[derive(Component)]
struct Hurting;

/// A volcano that plays `Attack` and can be interrupted.
type Attacking = (With<Volcano>, Without<Hurting>);
/// A volcano that plays `Hurt`.
type Hurt = (With<Volcano>, With<Hurting>);

/// The root of a volcano. Its `Transform` is the centre of the sprite.
#[derive(Component)]
#[require(Launcher)]
pub struct Volcano;

/// The `Attack` frame the volcano showed last frame, to see it reach a launch frame. `None` before the first look.
#[derive(Component, Default)]
struct Launcher {
    last_frame: Option<u16>,
}

/// A shot of a volcano, on the entity of its `Projectile`.
#[derive(Component)]
pub struct VolcanoShot;

/// True when the animation went from frame `last` to `now` through `frame`, counting a wrap to the start of the loop.
/// A slow frame that skips `frame` still launches.
fn reaches_frame(last: u16, now: u16, frame: u16) -> bool {
    if now >= last {
        last < frame && now >= frame
    } else {
        last < frame || now >= frame
    }
}

/// Where a shot of the volcano whose bottom is at `feet` starts when the middle of the crater is at art row
/// `mouth_row`: the ball's centre at the mouth, and the height of the ball's bottom above the ground under the mouth,
/// which is the middle of the base. That ground sets the draw order and where the arc meets the ground.
fn shot_start(feet: Vec2, mouth_row: f32) -> (Vec2, f32) {
    let ground = feet.y + blocked_area(feet).height() / 2.0;
    let mouth = Vec2::new(
        feet.x,
        feet.y + FEET_BELOW_CENTRE + (CANVAS_HALF_PX - mouth_row) * SCALE,
    );
    (mouth, mouth.y - ground - BALL_HALF)
}

/// An interrupted volcano (`EnemyInterrupted`, from a weapon whose energy interrupts it) plays `Hurt` once. An
/// interrupt while `Hurt` plays does not start it again: `Hurt` plays to its end (`docs/enemies.md`, "Stun and
/// interrupt").
fn interrupt(
    mut commands: Commands,
    mut interrupted: MessageReader<EnemyInterrupted>,
    mut volcanoes: Query<&mut AseAnimation, Attacking>,
) {
    for EnemyInterrupted(entity) in interrupted.read() {
        let Ok(mut ase) = volcanoes.get_mut(*entity) else {
            continue;
        };
        ase.animation.play(HURT_TAG, AnimationRepeat::Count(0));
        commands.entity(*entity).insert(Hurting);
        info!("volcano: {entity:?} interrupted, plays Hurt");
    }
}

/// When `Hurt` ends, the volcano starts `Attack` again from its first frame, so its next shot is a whole wind-up away.
/// The animation crate moves to frame 0 of `Attack` only in its next update, so for one frame the state still holds
/// `Hurt`'s frame. The launcher would read the jump from it to 0 as a loop that passed the launch frames, so the frame
/// is set to 0 here and the launcher forgets the frame it saw before.
fn finish_hurt(
    mut commands: Commands,
    mut events: MessageReader<AnimationEvents>,
    mut volcanoes: Query<(&mut AseAnimation, &mut AnimationState, &mut Launcher), Hurt>,
) {
    for event in events.read() {
        let AnimationEvents::Finished(entity) = event else {
            continue;
        };
        let Ok((mut ase, mut state, mut launcher)) = volcanoes.get_mut(*entity) else {
            continue;
        };
        ase.animation.play_loop(ATTACK_TAG);
        state.relative_frame = 0;
        launcher.last_frame = None;
        commands.entity(*entity).remove::<Hurting>();
        info!("volcano: {entity:?} attacks again from the start");
    }
}

fn launch_shots(
    mut commands: Commands,
    mut rng: ResMut<GameRng>,
    mut volcanoes: Query<
        (
            &Transform,
            &AnimationState,
            &AseAnimation,
            &MapId,
            &mut Launcher,
        ),
        Attacking,
    >,
) {
    for (transform, state, ase, map_id, mut launcher) in &mut volcanoes {
        let now = state.relative_frame;
        let Some(last) = launcher.last_frame.replace(now) else {
            continue;
        };
        let feet = transform.translation.truncate() - Vec2::Y * FEET_BELOW_CENTRE;
        for (_, mouth_row) in LAUNCHES
            .iter()
            .filter(|(frame, _)| reaches_frame(last, now, *frame))
        {
            let (mouth, height) = shot_start(feet, *mouth_row);
            let path = PathShape {
                start_height: height,
                ..SHOT_PATH
            };
            let direction = Vec2::from_angle(rng.range(0.0, TAU));
            commands
                .spawn((
                    VolcanoShot,
                    map_id.clone(),
                    Projectile::new(direction, 0.0)
                        .without_targets()
                        .ignoring(blocked_area(feet)),
                    ProjectilePath::new(path, direction, BALL_HALF),
                    Touchable,
                    SHOT_HIT_BOX,
                    Transform::from_translation(mouth.extend(0.0)),
                    Visibility::default(),
                ))
                .with_child((
                    AseAnimation {
                        aseprite: ase.aseprite.clone(),
                        animation: Animation::tag(SHOT_TAG).with_repeat(AnimationRepeat::Loop),
                    },
                    Sprite::default(),
                    Transform::from_scale(Vec3::new(SHOT_SCALE, SHOT_SCALE, 1.0)),
                ));
            info!(
                "volcano: shot from ({:.0}, {:.0}) towards ({:.2}, {:.2})",
                mouth.x, mouth.y, direction.x, direction.y
            );
        }
    }
}

/// A shot that touches the player takes `SHOT_DAMAGE` and is removed.
fn hit_player(
    mut commands: Commands,
    mut touched: MessageReader<ControlledTouched>,
    shots: Query<&Transform, With<VolcanoShot>>,
    mut damage: MessageWriter<DamagePlayer>,
) {
    for touch in touched.read() {
        let Ok(transform) = shots.get(touch.by) else {
            continue;
        };
        damage.write(DamagePlayer {
            amount: SHOT_DAMAGE,
        });
        commands.entity(touch.by).try_despawn();
        info!(
            "volcano: shot hit the player at ({:.0}, {:.0})",
            transform.translation.x, transform.translation.y
        );
    }
}

/// The area the feet may not enter for a volcano whose bottom is at `feet`: the base of the cone, scaled, and widened
/// on each side by half the player's width, as a tree trunk is.
fn blocked_area(feet: Vec2) -> Rect {
    let x = |px: f32| feet.x + (px - CANVAS_HALF_PX) * SCALE;
    let base = Rect::new(
        x(BASE_LEFT_PX),
        feet.y,
        x(BASE_RIGHT_PX),
        feet.y + (BASE_BOTTOM_PX - BASE_TOP_PX) * SCALE,
    );
    grow_sideways(base, PLAYER_HALF_WIDTH)
}

/// Spawns the volcanoes of one map with the bottom of the body on the bottom middle of their cells. They carry the map
/// id, so the kit removes them with the map. Their bases are blockers, which `rebuild_blockers` (the kit) adds to the collision
/// world.
pub fn spawn_volcanoes(commands: &mut Commands, server: &AssetServer, map: &Map, load: &MapLoad) {
    for object in &map.objects {
        if Kind::of(object) != Kind::SpawnVolcano {
            continue;
        }
        // Whole pixels, so the art pixels line up with the screen pixels.
        let feet =
            (load.origin + (object.cell.as_vec2() + Vec2::new(0.5, 0.0)) * map.cell_size).round();
        let centre = feet + Vec2::Y * FEET_BELOW_CENTRE;
        commands.spawn((
            Volcano,
            EnemyHp::full(HIT_POINTS),
            REACTIONS,
            HIT_BOX,
            Blocker(blocked_area(feet)),
            SortAtBlocker,
            AseAnimation {
                aseprite: server
                    .load_builder()
                    .with_settings::<AsepriteLoaderSettings>(|settings| {
                        settings.layer_selection =
                            LayerSelectionSetting::Mask(LAYERS.map(String::from).to_vec());
                    })
                    .load(ART),
                animation: Animation::tag(ATTACK_TAG).with_repeat(AnimationRepeat::Loop),
            },
            Sprite::default(),
            Transform::from_translation(centre.extend(0.0))
                .with_scale(Vec3::new(SCALE, SCALE, 1.0)),
        ));
        debug!("volcano: placed at {centre:?} in map {}", load.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{asset_app, map, messages, object, spawn_with, timed_app};

    #[test]
    fn a_frame_is_reached_once_per_loop_even_when_a_slow_frame_skips_it() {
        // (last, now, frame, reached) in a loop of 10 frames (0 to 9).
        let cases = [
            (5, 6, 6, true),
            (6, 6, 6, false),
            (6, 7, 6, false),
            (4, 8, 6, true),
            (5, 2, 6, true),
            (9, 6, 6, true),
            (9, 0, 6, false),
            (7, 3, 6, false),
            (0, 5, 6, false),
            (7, 8, 8, true),
            (8, 9, 8, false),
            (9, 0, 8, false),
            (9, 8, 8, true),
        ];
        for (last, now, frame, reached) in cases {
            assert_eq!(
                reaches_frame(last, now, frame),
                reached,
                "{last} to {now}, frame {frame}"
            );
        }
    }

    #[test]
    fn a_shot_starts_with_the_ball_centre_at_the_mouth_above_the_middle_of_the_base() {
        // Feet (130, 164): the base is 13 art px x 2 high, so the ground under the mouth is 13 up, at 177. With the
        // crater at row 22 the mouth is 31 + 10 art px x 2 above the feet, at 246; at row 15, 31 + 17, at 260. The
        // ball's bottom is 8 below its centre (16 art px at 0.5x): 246 - 177 - 8 = 61 and 260 - 177 - 8 = 75.
        let feet = Vec2::new(130.0, 164.0);
        assert_eq!(shot_start(feet, 22.0), (Vec2::new(130.0, 246.0), 61.0));
        assert_eq!(shot_start(feet, 15.0), (Vec2::new(130.0, 260.0), 75.0));
    }

    /// An app that runs `launch_shots` with a seeded `GameRng`, and one volcano with its centre at (130, 226), so its
    /// feet are at (130, 164). The test sets its animation frame by hand.
    fn launch_app() -> (App, Entity) {
        let mut app = timed_app(16);
        app.insert_resource(GameRng::seeded(7))
            .add_systems(Update, launch_shots);
        let volcano = app
            .world_mut()
            .spawn((
                Volcano,
                MapId("m".to_string()),
                AseAnimation::default(),
                AnimationState::default(),
                Transform::from_xyz(130.0, 226.0, 0.0),
            ))
            .id();
        (app, volcano)
    }

    #[test]
    fn a_volcano_launches_at_frames_7_and_9_from_the_mouth_of_that_frame_each_in_a_random_direction()
     {
        let (mut app, volcano) = launch_app();
        let mut launched_at = Vec::new();
        for frame in (0..10).chain(0..10) {
            app.world_mut()
                .get_mut::<AnimationState>(volcano)
                .unwrap()
                .relative_frame = frame;
            app.update();
            let shots = app
                .world_mut()
                .query_filtered::<(), With<VolcanoShot>>()
                .iter(app.world())
                .count();
            launched_at.push(shots);
        }
        // One shot when frame 6 shows and one when frame 8 shows in each loop, none on the other frames.
        assert_eq!(
            launched_at,
            [0, 0, 0, 0, 0, 0, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 4, 4]
        );
        // The draws in launch order: frame 7, frame 9, frame 7, frame 9. Frame 7 shots start at the mouth at 246 with
        // the ball 61 up, frame 9 shots at 260 with 75 (`shot_start`).
        let mut rng = GameRng::seeded(7);
        let mut expected: Vec<(Vec2, Vec2, f32)> = [246.0, 260.0, 246.0, 260.0]
            .into_iter()
            .map(|mouth: f32| {
                let direction = Vec2::from_angle(rng.range(0.0, TAU));
                (direction, Vec2::new(130.0, mouth), mouth - 177.0 - 8.0)
            })
            .collect();
        let mut shots: Vec<(Vec2, Vec2, f32, bool, Option<Rect>, String)> = app
            .world_mut()
            .query_filtered::<(&Transform, &ProjectilePath, &Projectile, &MapId), With<VolcanoShot>>()
            .iter(app.world())
            .map(|(transform, path, projectile, id)| {
                (
                    path.direction,
                    transform.translation.truncate(),
                    path.shape.start_height,
                    projectile.hits_targets,
                    projectile.ignored,
                    id.0.clone(),
                )
            })
            .collect();
        shots.sort_by(|a, b| a.0.x.total_cmp(&b.0.x));
        expected.sort_by(|a, b| a.0.x.total_cmp(&b.0.x));
        assert_eq!(shots.len(), 4);
        for (shot, (direction, mouth, height)) in shots.iter().zip(expected) {
            // At the mouth of its frame, hitting no targets, through its own base, removed with the map.
            assert_eq!(
                (shot.1, shot.2, shot.3, shot.4, shot.5.as_str()),
                (
                    mouth,
                    height,
                    false,
                    Some(blocked_area(Vec2::new(130.0, 164.0))),
                    "m"
                )
            );
            // 1e-6: `ProjectilePath::new` normalises the unit vector again.
            assert!(shot.0.distance(direction) < 1e-6, "{shot:?} vs {direction}");
        }
    }

    #[test]
    fn a_shot_that_touches_the_player_takes_1_hp_and_goes_and_two_take_2() {
        let mut app = App::new();
        app.add_message::<ControlledTouched>()
            .add_message::<DamagePlayer>()
            .add_systems(Update, hit_player);
        let shots = [
            app.world_mut()
                .spawn((VolcanoShot, Transform::default()))
                .id(),
            app.world_mut()
                .spawn((VolcanoShot, Transform::default()))
                .id(),
        ];
        // Something else that touches the player (a ghost) is not a shot.
        let other = app.world_mut().spawn(Transform::default()).id();
        for by in [shots[0], shots[1], other] {
            app.world_mut().write_message(ControlledTouched { by });
        }
        app.update();
        assert_eq!(
            messages::<DamagePlayer>(&app),
            [DamagePlayer { amount: 1 }, DamagePlayer { amount: 1 }]
        );
        assert!(
            shots
                .iter()
                .all(|shot| app.world().get_entity(*shot).is_err())
        );
        assert!(app.world().get_entity(other).is_ok());
    }

    #[test]
    fn a_marker_puts_a_volcano_on_the_bottom_of_its_cell_on_whole_pixels_with_its_base_blocked_and_sorted_at_it()
     {
        let mut app = asset_app(16);
        let map = map(
            48.0,
            vec![
                object("spawn_volcano", 2, 3, &[]),
                object("rock_0", 0, 0, &[]),
            ],
        );
        // The feet are at x 10.4 + 2.5 * 48 = 130.4, so 130, and y 20 + 3 * 48 = 164.
        let load = crate::test_util::map_load(Vec2::new(10.4, 20.0));
        spawn_with(&mut app, move |commands, server| {
            spawn_volcanoes(commands, server, &map, &load);
        });
        let volcanoes: Vec<(Vec2, Rect, bool)> = app
            .world_mut()
            .query_filtered::<(&Transform, &Blocker, Has<SortAtBlocker>), With<Volcano>>()
            .iter(app.world())
            .map(|(transform, blocker, sorted)| {
                (transform.translation.truncate(), blocker.0, sorted)
            })
            .collect();
        // Centre 31 art px x 2 above the feet. Base x 11 and 54 of the 64 px canvas, so -21 and +22 art px x 2 from the
        // feet, widened by 14 on each side: 130 - 42 - 14 = 74 and 130 + 44 + 14 = 188. Rows 50 to 63 are 13 art px x 2.
        // The player stops on the bottom of the blocked area, so the volcano sorts at it (`SortAtBlocker`): a player there is
        // in front whatever its x.
        assert_eq!(
            volcanoes,
            vec![(
                Vec2::new(130.0, 226.0),
                Rect::new(74.0, 164.0, 188.0, 190.0),
                true
            )]
        );
        // What weapons touch: the blocked area and the cone (top at row 20, 12 art px x 2 above the centre), 2 px
        // further all round, so the ice ring, which stops at the blocked area, still reaches it from every side.
        let bodies: Vec<(Rect, EnemyHp, Reactions)> = app
            .world_mut()
            .query_filtered::<(&Transform, &HitBox, &EnemyHp, &Reactions), With<Volcano>>()
            .iter(app.world())
            .map(|(transform, hit_box, hp, reactions)| {
                (
                    hit_box.rect_at(transform.translation.truncate()),
                    *hp,
                    *reactions,
                )
            })
            .collect();
        assert_eq!(
            bodies,
            vec![(
                Rect::new(72.0, 162.0, 190.0, 252.0),
                EnemyHp::full(HIT_POINTS),
                REACTIONS
            )]
        );
    }

    /// An app that runs the interrupt systems and `launch_shots` on one volcano playing `Attack`.
    fn interrupt_app() -> (App, Entity) {
        let (mut app, volcano) = launch_app();
        app.add_message::<EnemyInterrupted>()
            .add_message::<AnimationEvents>()
            .add_systems(Update, (interrupt, finish_hurt).before(launch_shots));
        app.world_mut()
            .get_mut::<AseAnimation>(volcano)
            .unwrap()
            .animation
            .play_loop(ATTACK_TAG);
        (app, volcano)
    }

    fn set_frame(app: &mut App, volcano: Entity, frame: u16) {
        app.world_mut()
            .get_mut::<AnimationState>(volcano)
            .unwrap()
            .relative_frame = frame;
    }

    fn tag(app: &App, volcano: Entity) -> Option<&str> {
        app.world()
            .get::<AseAnimation>(volcano)
            .unwrap()
            .animation
            .tag
            .as_deref()
    }

    fn shots(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<VolcanoShot>>()
            .iter(app.world())
            .count()
    }

    #[test]
    fn an_interrupt_plays_hurt_to_its_end_without_shots_then_attack_starts_again_from_its_first_frame()
     {
        let (mut app, volcano) = interrupt_app();
        // Up to frame 5, one before the first launch frame.
        for frame in 0..6 {
            set_frame(&mut app, volcano, frame);
            app.update();
        }
        app.world_mut().write_message(EnemyInterrupted(volcano));
        app.update();
        assert_eq!(tag(&app, volcano), Some(HURT_TAG));
        assert!(app.world().get::<Hurting>(volcano).is_some());
        // A second interrupt while `Hurt` plays does not start it again.
        app.world_mut()
            .get_mut::<AseAnimation>(volcano)
            .unwrap()
            .animation
            .tag = Some("still hurt".into());
        app.world_mut().write_message(EnemyInterrupted(volcano));
        app.update();
        assert_eq!(tag(&app, volcano), Some("still hurt"));
        // No shot while hurt, even when the frame number passes the launch frames.
        for frame in [6, 7, 8, 9] {
            set_frame(&mut app, volcano, frame);
            app.update();
        }
        assert_eq!(shots(&mut app), 0);
        // `Hurt` ends on its last frame, 3. The animation crate moves to frame 0 of `Attack` only one frame later.
        set_frame(&mut app, volcano, 3);
        app.update();
        app.world_mut()
            .write_message(AnimationEvents::Finished(volcano));
        app.update();
        assert_eq!(tag(&app, volcano), Some(ATTACK_TAG));
        assert!(app.world().get::<Hurting>(volcano).is_none());
        // Attack starts from frame 0: the jump from frame 9 back to 0 is no wrap through a launch frame.
        set_frame(&mut app, volcano, 0);
        app.update();
        assert_eq!(shots(&mut app), 0);
        for frame in 1..7 {
            set_frame(&mut app, volcano, frame);
            app.update();
        }
        assert_eq!(shots(&mut app), 1);
    }
}
