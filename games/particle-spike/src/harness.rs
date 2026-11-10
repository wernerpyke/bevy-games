//! Test harness: pick an effect, fire it at the cursor, see particle count and frame time,
//! save screenshots, reload effect files. Effects read `FireEffect` and draw themselves.
//!
//! Keys: 0-4 pick an effect, Space fire, H hide the text, R reload effect files, P screenshot (native).
//! Env `SPIKE_SHOT=<effect>:<seconds>` (native) picks and fires an effect, saves a screenshot and quits.

use crate::rules::Rect2;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_enoki::prelude::*;

pub const EFFECTS: [&str; 5] = ["smoke test", "fireball", "ring", "wall", "whirlpool"];
const SELECT_KEYS: [KeyCode; 5] = [
    KeyCode::Digit0,
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
];

/// Where the stand-in player stands.
pub const PLAYER: Vec2 = Vec2::ZERO;
const ENEMY_START_X: f32 = 410.0;
const ENEMY_SIZE: Vec2 = Vec2::new(40.0, 48.0);
/// Speed of the red block when the arrow keys move it.
const ENEMY_SPEED: f32 = 150.0;
pub const ENEMY_COLOR: Color = Color::srgb(0.8, 0.25, 0.25);
const BLOCKER: Rect2 = Rect2::new(Vec2::new(-410.0, 60.0), Vec2::new(36.0, 130.0));

/// Draw order of the stand-ins (below effects).
const Z_BODY: f32 = 0.5;
const Z_EFFECT: f32 = 2.0;
const Z_CROSSHAIR: f32 = 5.0;

/// Index into `EFFECTS`.
#[derive(Resource, Default)]
pub struct Selected(pub usize);

/// Cursor position in world art pixels.
#[derive(Resource, Default)]
pub struct Aim(pub Vec2);

/// Keeps every effect file loaded for the whole run. Without it a file is dropped when its last emitter dies,
/// and a reload finds nothing to reload.
#[derive(Resource, Default)]
pub struct EffectLibrary(pub Vec<Handle<Particle2dEffect>>);

/// Sent when the user fires the selected effect. Each effect reads it and spawns itself.
#[derive(Message)]
pub struct FireEffect {
    pub effect: usize,
    pub from: Vec2,
    pub aim: Vec2,
}

/// A stand-in enemy.
#[derive(Component)]
pub struct Enemy;

/// A stand-in blocked feature (a tree or a rock).
#[derive(Component)]
pub struct Blocker;

/// The rectangle a stand-in occupies.
#[derive(Component, Deref)]
pub struct Body(pub Rect2);

#[derive(Component)]
struct HudText;

#[derive(Component)]
struct Crosshair;

pub struct HarnessPlugin;

impl Plugin for HarnessPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .init_resource::<Selected>()
            .init_resource::<Aim>()
            .init_resource::<EffectLibrary>()
            .add_message::<FireEffect>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    select_effect,
                    track_aim,
                    fire.after(select_effect).after(track_aim),
                    smoke_test.after(fire),
                    toggle_hud,
                    move_enemy,
                    reload_effects,
                    refresh_instances,
                    update_hud,
                ),
            );

        #[cfg(not(target_family = "wasm"))]
        native::add(app);
    }
}

fn setup(mut commands: Commands) {
    // `SPIKE_ENEMY=<x>` starts the red block at that x, for scripted checks.
    let enemy_x = std::env::var("SPIKE_ENEMY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(ENEMY_START_X);
    let enemy = Rect2::new(Vec2::new(enemy_x, 0.0), ENEMY_SIZE);
    for (rect, colour) in [(enemy, ENEMY_COLOR), (BLOCKER, Color::srgb(0.3, 0.45, 0.3))] {
        let mut stand_in = commands.spawn((
            Body(rect),
            Sprite::from_color(colour, rect.size),
            Transform::from_translation(rect.centre.extend(Z_BODY)),
        ));
        if rect == enemy {
            stand_in.insert(Enemy);
        } else {
            stand_in.insert(Blocker);
        }
    }

    commands.spawn((
        Crosshair,
        Sprite::from_color(Color::srgb(1.0, 0.9, 0.3), Vec2::splat(8.0)),
        Transform::from_translation(Vec3::new(0.0, 0.0, Z_CROSSHAIR)),
    ));

    commands.spawn((
        HudText,
        Text::new(""),
        TextFont::from_font_size(14.0),
        TextColor(Color::WHITE),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            top: px(8),
            left: px(8),
            ..default()
        },
    ));
}

fn select_effect(keys: Res<ButtonInput<KeyCode>>, mut selected: ResMut<Selected>) {
    for (index, key) in SELECT_KEYS.iter().enumerate() {
        if keys.just_pressed(*key) {
            selected.0 = index;
        }
    }
}

fn track_aim(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut aim: ResMut<Aim>,
    mut crosshair: Single<&mut Transform, With<Crosshair>>,
) {
    let (camera, camera_transform) = *camera;
    if let Some(cursor) = window.cursor_position()
        && let Ok(world) = camera.viewport_to_world_2d(camera_transform, cursor)
    {
        aim.0 = world;
    }
    crosshair.translation = aim.0.extend(Z_CROSSHAIR);
}

fn fire(
    keys: Res<ButtonInput<KeyCode>>,
    selected: Res<Selected>,
    aim: Res<Aim>,
    mut fire: MessageWriter<FireEffect>,
) {
    if keys.just_pressed(KeyCode::Space) {
        fire.write(FireEffect {
            effect: selected.0,
            from: PLAYER,
            aim: aim.0,
        });
    }
}

/// Effect 0 only checks the harness: a burst of default particles at the cursor.
fn smoke_test(
    mut fired: MessageReader<FireEffect>,
    mut effects: ResMut<Assets<Particle2dEffect>>,
    mut commands: Commands,
) {
    for shot in fired.read().filter(|shot| shot.effect == 0) {
        let effect = effects.add(Particle2dEffect {
            spawn_amount: 20,
            ..default()
        });
        commands.spawn((
            ParticleSpawner::default(),
            ParticleEffectHandle(effect),
            OneShot::Despawn,
            Transform::from_translation(shot.aim.extend(Z_EFFECT)),
        ));
    }
}

/// The arrow keys move the red block, to walk it into an effect.
fn move_enemy(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut enemy: Single<(&mut Body, &mut Transform), With<Enemy>>,
) {
    let mut step = Vec2::ZERO;
    for (key, direction) in [
        (KeyCode::ArrowLeft, Vec2::NEG_X),
        (KeyCode::ArrowRight, Vec2::X),
        (KeyCode::ArrowUp, Vec2::Y),
        (KeyCode::ArrowDown, Vec2::NEG_Y),
    ] {
        if keys.pressed(key) {
            step += direction;
        }
    }
    if step != Vec2::ZERO {
        let (body, transform) = &mut *enemy;
        body.0.centre += step.normalize() * ENEMY_SPEED * time.delta_secs();
        transform.translation.x = body.0.centre.x;
        transform.translation.y = body.0.centre.y;
    }
}

fn toggle_hud(keys: Res<ButtonInput<KeyCode>>, mut hud: Single<&mut Visibility, With<HudText>>) {
    if keys.just_pressed(KeyCode::KeyH) {
        **hud = match **hud {
            Visibility::Hidden => Visibility::Inherited,
            _ => Visibility::Hidden,
        };
    }
}

/// Reloads every effect file (R). With the `hot-reload` feature on native, saving a file does the same. The web needs the key.
fn reload_effects(
    keys: Res<ButtonInput<KeyCode>>,
    effects: Res<Assets<Particle2dEffect>>,
    server: Res<AssetServer>,
) {
    if !keys.just_pressed(KeyCode::KeyR) {
        return;
    }
    let mut count = 0;
    for id in effects.ids() {
        if let Some(path) = server.get_path(id) {
            server.reload(path.into_owned());
            count += 1;
        }
    }
    info!("reloaded {count} effect file(s)");
}

fn update_hud(
    selected: Res<Selected>,
    diagnostics: Res<DiagnosticsStore>,
    stores: Query<&ParticleStore>,
    mut hud: Single<&mut Text, With<HudText>>,
) {
    let frame_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let particles: usize = stores.iter().map(ParticleStore::len).sum();
    hud.0 = format!(
        "{} {} | particles {particles} | {frame_ms:.1} ms ({:.0} fps)\n0-4 pick, Space fire, I ice or fire ring, B squares, arrows move red, R reload, H hide{}",
        selected.0,
        EFFECTS[selected.0],
        if frame_ms > 0.0 {
            1000.0 / frame_ms
        } else {
            0.0
        },
        if cfg!(target_family = "wasm") {
            ""
        } else {
            ", P shot"
        },
    );
}

#[cfg(not(target_family = "wasm"))]
mod native {
    use super::*;
    use bevy::render::view::screenshot::{Screenshot, save_to_disk};

    const DIR: &str = "target/screenshots";

    pub fn add(app: &mut App) {
        app.add_systems(Update, (screenshot_key, shot_script));
    }

    pub fn save_screenshot(commands: &mut Commands, name: &str) {
        let _ = std::fs::create_dir_all(DIR);
        let path = format!("{DIR}/{name}.png");
        info!("screenshot {path}");
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }

    fn screenshot_key(
        keys: Res<ButtonInput<KeyCode>>,
        selected: Res<Selected>,
        mut commands: Commands,
    ) {
        if keys.just_pressed(KeyCode::KeyP) {
            save_screenshot(&mut commands, EFFECTS[selected.0]);
        }
    }

    /// `SPIKE_SHOT=<effect>:<seconds>`: pick and fire the effect at 0.3 s, screenshot at `seconds`, quit 1 s later.
    fn shot_script(
        time: Res<Time>,
        mut stage: Local<u8>,
        mut selected: ResMut<Selected>,
        mut aim: ResMut<Aim>,
        mut fire: MessageWriter<FireEffect>,
        mut exit: MessageWriter<AppExit>,
        mut commands: Commands,
    ) {
        let Some((effect, seconds)) = std::env::var("SPIKE_SHOT")
            .ok()
            .and_then(|v| v.split_once(':').map(|(a, b)| (a.to_owned(), b.to_owned())))
            .and_then(|(a, b)| Some((a.parse::<usize>().ok()?, b.parse::<f32>().ok()?)))
            .filter(|(effect, _)| *effect < EFFECTS.len())
        else {
            return;
        };
        let t = time.elapsed_secs();
        if *stage == 0 && t >= 0.3 {
            selected.0 = effect;
            aim.0 = Vec2::new(410.0, 0.0);
            fire.write(FireEffect {
                effect,
                from: PLAYER,
                aim: aim.0,
            });
            *stage = 1;
        } else if *stage == 1 && t >= seconds {
            save_screenshot(&mut commands, EFFECTS[effect]);
            *stage = 2;
        } else if *stage == 2 && t >= seconds + 1.0 {
            exit.write(AppExit::Success);
            *stage = 3;
        }
    }
}

/// Copies a reloaded effect into the emitters that use it. Enoki only does this for a first load,
/// so without it a shot that is in flight keeps its old values.
fn refresh_instances(
    mut events: MessageReader<AssetEvent<Particle2dEffect>>,
    effects: Res<Assets<Particle2dEffect>>,
    server: Res<AssetServer>,
    mut spawners: Query<(&ParticleEffectHandle, &mut ParticleEffectInstance)>,
) {
    for event in events.read() {
        if let AssetEvent::Modified { id } = event
            && let Some(effect) = effects.get(*id)
        {
            for (handle, mut instance) in &mut spawners {
                if handle.id() == *id {
                    instance.0 = Some(effect.clone());
                }
            }
            if let Some(path) = server.get_path(*id) {
                info!("effect reloaded: {path}");
            }
        }
    }
}
