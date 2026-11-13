//! Debug tool: plays a scripted sequence of key presses into the running game and logs the position of the
//! `Controlled` entity, so behaviour can be checked without a person at the keyboard. Does nothing unless the
//! `<PREFIX>_SCRIPT` environment variable is set, for example `LONG_WAY_SCRIPT` (`ScriptPlugin::env_prefix`).
//!
//! Commands are separated by `;`:
//! - `teleport:<feet_x>:<feet_y>`   put the feet of the controlled entity at a position
//! - `hold:<keys>:<seconds>`        hold arrow keys, `z`, `x`, `c`, `v`, `1` to `4`, `escape` or `f1` to `f3`, for example `hold:up:3` or `hold:up+left:2`. A
//!   short hold of `x` (`hold:x:0.1`) is one key press
//! - `click:<world_x>:<world_y>`    a left mouse click at a world position (`pointer::WorldClick`)
//! - `ui_click:<x>:<y>`            a left click on the UI at a window position in art pixels from the top left (the
//!   same units as the UI, `UiScale` makes them logical pixels). It moves the picking pointer, presses and releases, so
//!   the node under it gets the click as with a real mouse, and a `BlocksWorldClick` panel blocks the world click
//! - `wait:<seconds>`
//! - `screenshot:<name>`            save the window to `<name>.png` and wait until it is written. The folder is
//!   `<PREFIX>_SCREENSHOT_DIR`, default `target/screenshots`. Names use letters, digits, `-` and `_`.
//!   Add a `wait` before it after a `teleport`, so the frame shows the new position.
//! - `goto:<map>:<entry>`           start a scene transition to an entry of a registered map and wait until it is done
//!   (`map_transition.rs`). The player is put in the middle of the entry
//! - `goto:<map>`                   the same, and the player arrives at the `player_start` of the map
//! - `quit`
//!
//! Example: `LONG_WAY_SCRIPT="teleport:225:-150;hold:up:3;quit" RUST_LOG=long_way=debug ...`. Game specific commands
//! are not supported yet (adr-x15): add the registration when a game needs its first one.

use std::collections::VecDeque;

use bevy::camera::RenderTarget;
use bevy::ecs::system::SystemParam;
use bevy::input::InputSystems;
use bevy::picking::PickingSystems;
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk};
use bevy::window::{PrimaryWindow, WindowRef};

use crate::Controlled;
use crate::depth::DepthSort;
use crate::map::Transition;
use crate::map_transition::{ActiveMaps, MapTransition, RequestMapChange};
use crate::pointer::{PointerSet, WorldClick};

/// Seconds to wait after start before the script runs, so the window and world are up.
const START_DELAY: f32 = 1.0;
/// Seconds between position log lines while a command runs.
const LOG_INTERVAL: f32 = 0.25;

pub struct ScriptPlugin {
    /// Prefix of the environment variables, in capitals with underscores, for example "LONG_WAY". `dev-scripts/rust.sh`
    /// builds it from the game name.
    pub env_prefix: &'static str,
}

impl Plugin for ScriptPlugin {
    fn build(&self, app: &mut App) {
        let Ok(text) = std::env::var(format!("{}_SCRIPT", self.env_prefix)) else {
            return;
        };
        let commands = parse(&text);
        info!("script: {} commands loaded", commands.len());
        app.add_message::<RequestMapChange>()
            .add_message::<WorldClick>()
            .add_message::<PointerInput>()
            .insert_resource(Script {
                commands,
                elapsed: -START_DELAY,
                since_log: 0.0,
                shot_requested: false,
                ui_phase: 0,
                shot_saved: false,
                goto_sent: false,
                goto_busy: false,
                screenshot_dir_variable: format!("{}_SCREENSHOT_DIR", self.env_prefix),
            })
            .add_systems(
                PreUpdate,
                run_script
                    .after(InputSystems)
                    .before(PickingSystems::ProcessInput)
                    .before(PointerSet),
            );
    }
}

#[derive(Clone)]
enum Command {
    Teleport(Vec2),
    Hold(Vec<KeyCode>, f32),
    Wait(f32),
    Click(Vec2),
    UiClick(Vec2),
    Screenshot(String),
    Goto(String, Option<String>),
    Quit,
}

#[derive(Resource)]
struct Script {
    commands: VecDeque<Command>,
    /// Seconds spent on the current command. Negative during the start delay.
    elapsed: f32,
    since_log: f32,
    /// State of the current `screenshot` command.
    shot_requested: bool,
    /// Step of the current `ui_click`: 0 moves the pointer, 1 presses, 2 releases (one step per frame).
    ui_phase: u8,
    shot_saved: bool,
    /// State of the current `goto` command: the request was sent, and the transition has been seen running.
    goto_sent: bool,
    goto_busy: bool,
    /// Name of the environment variable that sets the screenshot folder.
    screenshot_dir_variable: String,
}

fn parse(text: &str) -> VecDeque<Command> {
    text.split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .filter_map(|part| {
            let fields: Vec<&str> = part.split(':').collect();
            let command = match fields.as_slice() {
                ["teleport", x, y] => {
                    Command::Teleport(Vec2::new(x.parse().ok()?, y.parse().ok()?))
                }
                ["hold", keys, seconds] => {
                    let keys = keys.split('+').map(key).collect::<Option<Vec<_>>>()?;
                    Command::Hold(keys, seconds.parse().ok()?)
                }
                ["wait", seconds] => Command::Wait(seconds.parse().ok()?),
                ["click", x, y] => Command::Click(Vec2::new(x.parse().ok()?, y.parse().ok()?)),
                ["ui_click", x, y] => Command::UiClick(Vec2::new(x.parse().ok()?, y.parse().ok()?)),
                ["screenshot", name]
                    if !name.is_empty()
                        && name
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') =>
                {
                    Command::Screenshot(name.to_string())
                }
                ["goto", map] if !map.is_empty() => Command::Goto(map.to_string(), None),
                ["goto", map, entry] if !map.is_empty() && !entry.is_empty() => {
                    Command::Goto(map.to_string(), Some(entry.to_string()))
                }
                ["quit"] => Command::Quit,
                _ => None?,
            };
            Some(command)
        })
        .collect()
}

fn key(name: &str) -> Option<KeyCode> {
    match name {
        "up" => Some(KeyCode::ArrowUp),
        "down" => Some(KeyCode::ArrowDown),
        "left" => Some(KeyCode::ArrowLeft),
        "right" => Some(KeyCode::ArrowRight),
        "x" => Some(KeyCode::KeyX),
        "z" => Some(KeyCode::KeyZ),
        "c" => Some(KeyCode::KeyC),
        "v" => Some(KeyCode::KeyV),
        "1" => Some(KeyCode::Digit1),
        "2" => Some(KeyCode::Digit2),
        "3" => Some(KeyCode::Digit3),
        "4" => Some(KeyCode::Digit4),
        "escape" => Some(KeyCode::Escape),
        "f1" => Some(KeyCode::F1),
        "f2" => Some(KeyCode::F2),
        "f3" => Some(KeyCode::F3),
        _ => None,
    }
}

/// Runs after `save_to_disk`, which is added first and saves the file inside its observer.
fn mark_saved(_: On<ScreenshotCaptured>, mut script: ResMut<Script>) {
    script.shot_saved = true;
}

/// The feet as text for the log, or `none` when there is no controlled entity.
fn feet_text(feet: Option<Vec2>) -> String {
    feet.map_or_else(|| "none".to_string(), |feet| format!("{feet:?}"))
}

fn feet_of(transform: &Transform, sort: &DepthSort) -> Vec2 {
    Vec2::new(
        transform.translation.x,
        sort.feet_y(transform.translation.y),
    )
}

/// What a `goto` command needs from the map lifecycle.
#[derive(SystemParam)]
struct MapLink<'w, 's> {
    pointer: UiPointer<'w, 's>,
    changes: MessageWriter<'w, RequestMapChange>,
    clicks: MessageWriter<'w, WorldClick>,
    transition: Option<Res<'w, MapTransition>>,
    maps: Option<Res<'w, ActiveMaps>>,
}

/// What a `ui_click` command needs: the picking input, the mouse buttons and the window.
#[derive(SystemParam)]
struct UiPointer<'w, 's> {
    input: MessageWriter<'w, PointerInput>,
    mouse: ResMut<'w, ButtonInput<MouseButton>>,
    window: Query<'w, 's, Entity, With<PrimaryWindow>>,
    scale: Option<Res<'w, UiScale>>,
}

impl UiPointer<'_, '_> {
    /// One step of a `ui_click` at `art` (art pixels from the window top left). Returns true after the last step.
    fn step(&mut self, phase: u8, art: Vec2) -> bool {
        let Ok(window) = self.window.single() else {
            warn!("script: ui_click skipped, there is no primary window");
            return true;
        };
        let target = RenderTarget::Window(WindowRef::Primary).normalize(Some(window));
        let Some(target) = target else {
            return true;
        };
        let scale = self.scale.as_ref().map_or(1.0, |scale| scale.0);
        let location = Location {
            target,
            position: art * scale,
        };
        // The move is sent in every step, after the real mouse events of the frame, so a real cursor over the window
        // cannot take the pointer away between the steps.
        self.input.write(PointerInput::new(
            PointerId::Mouse,
            location.clone(),
            PointerAction::Move { delta: Vec2::ZERO },
        ));
        match phase {
            0 => {}
            1 => {
                self.mouse.press(MouseButton::Left);
                self.input.write(PointerInput::new(
                    PointerId::Mouse,
                    location,
                    PointerAction::Press(PointerButton::Primary),
                ));
            }
            _ => {
                self.mouse.release(MouseButton::Left);
                self.input.write(PointerInput::new(
                    PointerId::Mouse,
                    location,
                    PointerAction::Release(PointerButton::Primary),
                ));
            }
        }
        phase >= 2
    }
}

fn run_script(
    mut commands: Commands,
    mut script: ResMut<Script>,
    mut input: ResMut<ButtonInput<KeyCode>>,
    time: Res<Time<Real>>,
    mut target: Query<(&mut Transform, &DepthSort), With<Controlled>>,
    mut exit: MessageWriter<AppExit>,
    mut link: MapLink,
) {
    script.elapsed += time.delta_secs();
    if script.elapsed < 0.0 {
        return;
    }
    // The controlled entity may be gone (the player died). The script goes on without it.
    let mut target = target.single_mut().ok();

    let Some(command) = script.commands.front().cloned() else {
        exit.write(AppExit::Success);
        return;
    };

    let finished = match &command {
        Command::Teleport(position) => {
            if let Some((transform, sort)) = &mut target {
                transform.translation.x = position.x;
                transform.translation.y = position.y + sort.feet_offset;
                info!("script: teleport, feet now {:?}", position);
            } else {
                info!("script: teleport skipped, there is no controlled entity");
            }
            true
        }
        Command::Hold(keys, seconds) => {
            for key in keys {
                if !input.pressed(*key) {
                    input.press(*key);
                }
            }
            if script.elapsed >= *seconds {
                for key in keys {
                    input.release(*key);
                }
                true
            } else {
                false
            }
        }
        Command::Wait(seconds) => script.elapsed >= *seconds,
        Command::Click(position) => {
            info!("script: click at {position:?}");
            link.clicks.write(WorldClick {
                position: *position,
                button: MouseButton::Left,
            });
            true
        }
        Command::UiClick(position) => {
            let done = link.pointer.step(script.ui_phase, *position);
            script.ui_phase += 1;
            if done {
                info!("script: ui click at {position:?}");
            }
            done
        }
        Command::Screenshot(name) => {
            if !script.shot_requested {
                let dir = std::env::var(&script.screenshot_dir_variable)
                    .unwrap_or_else(|_| "target/screenshots".to_string());
                let path = std::path::Path::new(&dir).join(format!("{name}.png"));
                if let Err(error) = std::fs::create_dir_all(&dir) {
                    warn!("script: cannot create {dir}: {error}");
                }
                let shown = std::path::absolute(&path).unwrap_or_else(|_| path.clone());
                info!(
                    "script: screenshot requested, saving to {}",
                    shown.display()
                );
                script.shot_requested = true;
                script.shot_saved = false;
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(path))
                    .observe(mark_saved);
                false
            } else {
                script.shot_saved
            }
        }
        Command::Goto(map, entry) => {
            if !script.goto_sent {
                info!("script: goto map {map} entry {entry:?}");
                link.changes.write(RequestMapChange {
                    to: map.clone(),
                    entry: entry.clone(),
                    transition: Transition::Scene,
                    source: None,
                });
                script.goto_sent = true;
                script.goto_busy = false;
                false
            } else {
                let idle = link.transition.as_ref().is_none_or(|t| t.is_idle());
                script.goto_busy |= !idle;
                let arrived = link.maps.as_ref().is_some_and(|m| m.current == *map);
                // A request for an unknown map is refused and never shows as busy, so stop waiting after 5 seconds.
                if idle && ((script.goto_busy && arrived) || script.elapsed > 5.0) {
                    if !arrived {
                        warn!("script: goto {map} {entry:?} did not arrive");
                    }
                    true
                } else {
                    false
                }
            }
        }
        Command::Quit => {
            exit.write(AppExit::Success);
            true
        }
    };

    script.since_log += time.delta_secs();
    if script.since_log >= LOG_INTERVAL {
        script.since_log = 0.0;
        info!(
            "script: t={:.2} feet {}",
            script.elapsed,
            feet_text(
                target
                    .as_ref()
                    .map(|(transform, sort)| feet_of(transform, sort))
            )
        );
    }
    if finished {
        info!(
            "script: command done, feet {}",
            feet_text(
                target
                    .as_ref()
                    .map(|(transform, sort)| feet_of(transform, sort))
            )
        );
        script.commands.pop_front();
        script.elapsed = 0.0;
        script.shot_requested = false;
        script.ui_phase = 0;
        script.goto_sent = false;
        script.goto_busy = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::timed_app;

    #[test]
    fn commands_are_parsed_in_order() {
        let commands = parse("teleport:225:-150; hold:up+left:2 ;wait:0.5;screenshot:a_1;quit");
        assert_eq!(commands.len(), 5);
        assert!(matches!(commands[0], Command::Teleport(p) if p == Vec2::new(225.0, -150.0)));
        assert!(matches!(&commands[1], Command::Hold(keys, s) if keys.len() == 2 && *s == 2.0));
        assert!(matches!(commands[2], Command::Wait(s) if s == 0.5));
        assert!(matches!(&commands[3], Command::Screenshot(name) if name == "a_1"));
        assert!(matches!(commands[4], Command::Quit));
    }

    #[test]
    fn click_takes_a_world_position() {
        let commands = parse("click:100:-20.5;click:1;click:a:b");
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], Command::Click(p) if p == Vec2::new(100.0, -20.5)));
    }

    #[test]
    fn ui_click_takes_a_window_position() {
        let commands = parse("ui_click:60:-20.5;ui_click:1;ui_click:a:b");
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], Command::UiClick(p) if p == Vec2::new(60.0, -20.5)));
    }

    #[test]
    fn goto_takes_a_map_and_an_optional_entry() {
        let commands = parse("goto:2:from_1;goto:2;goto::x;goto:");
        assert_eq!(commands.len(), 2);
        assert!(
            matches!(&commands[0], Command::Goto(map, Some(entry)) if map == "2" && entry == "from_1")
        );
        assert!(matches!(&commands[1], Command::Goto(map, None) if map == "2"));
    }

    #[test]
    fn bad_commands_are_skipped() {
        let commands = parse("teleport:1;hold:sideways:2;screenshot:bad/name;wait:x;quit");
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], Command::Quit));
    }

    #[test]
    fn empty_parts_are_ignored() {
        assert!(parse(";; ;").is_empty());
    }

    #[test]
    fn every_key_name_gives_its_key() {
        for (name, code) in [
            ("up", KeyCode::ArrowUp),
            ("down", KeyCode::ArrowDown),
            ("left", KeyCode::ArrowLeft),
            ("right", KeyCode::ArrowRight),
            ("x", KeyCode::KeyX),
            ("z", KeyCode::KeyZ),
            ("c", KeyCode::KeyC),
            ("v", KeyCode::KeyV),
            ("1", KeyCode::Digit1),
            ("2", KeyCode::Digit2),
            ("3", KeyCode::Digit3),
            ("4", KeyCode::Digit4),
            ("escape", KeyCode::Escape),
            ("f1", KeyCode::F1),
            ("f2", KeyCode::F2),
            ("f3", KeyCode::F3),
        ] {
            assert_eq!(key(name), Some(code), "{name}");
        }
        assert_eq!(key("Up"), None);
    }

    /// Seconds per frame of the runner tests.
    const FRAME: f32 = 0.1;

    #[derive(Resource, Default)]
    struct Seen {
        pointer: Vec<(&'static str, Vec2)>,
        changes: Vec<(String, Option<String>)>,
    }

    fn collect(
        mut pointer: MessageReader<PointerInput>,
        mut changes: MessageReader<RequestMapChange>,
        mut seen: ResMut<Seen>,
    ) {
        for input in pointer.read() {
            let action = match input.action {
                PointerAction::Move { .. } => "move",
                PointerAction::Press(PointerButton::Primary) => "press",
                PointerAction::Release(PointerButton::Primary) => "release",
                _ => "other",
            };
            seen.pointer.push((action, input.location.position));
        }
        for change in changes.read() {
            seen.changes.push((change.to.clone(), change.entry.clone()));
        }
    }

    /// The runner on `text`, with frames of `FRAME` seconds of real time and no start delay. The first update, which
    /// has no time step, runs before the script is added.
    fn runner(text: &str) -> App {
        let mut app = timed_app(std::time::Duration::from_secs_f32(FRAME));
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<Seen>()
            .add_message::<RequestMapChange>()
            .add_message::<WorldClick>()
            .add_message::<PointerInput>();
        app.update();
        app.insert_resource(Script {
            commands: parse(text),
            elapsed: 0.0,
            since_log: 0.0,
            shot_requested: false,
            ui_phase: 0,
            shot_saved: false,
            goto_sent: false,
            goto_busy: false,
            screenshot_dir_variable: "GAME_KIT_TEST_SCREENSHOT_DIR".to_string(),
        })
        .add_systems(Update, (run_script, collect).chain());
        app
    }

    fn left(app: &App) -> usize {
        app.world().resource::<Script>().commands.len()
    }

    fn held(app: &App, key: KeyCode) -> bool {
        app.world().resource::<ButtonInput<KeyCode>>().pressed(key)
    }

    #[test]
    fn a_hold_presses_its_keys_until_its_time_is_up() {
        let mut app = runner("hold:up+left:0.25;wait:5");
        for _ in 0..2 {
            app.update();
            assert!(held(&app, KeyCode::ArrowUp) && held(&app, KeyCode::ArrowLeft));
            assert_eq!(left(&app), 2);
        }
        // 0.3 s: past the hold.
        app.update();
        assert!(!held(&app, KeyCode::ArrowUp) && !held(&app, KeyCode::ArrowLeft));
        assert_eq!(left(&app), 1);
    }

    #[test]
    fn a_wait_ends_when_its_time_is_up_and_quit_ends_the_app() {
        let mut app = runner("wait:0.25;quit");
        for _ in 0..2 {
            app.update();
            assert_eq!(left(&app), 2);
        }
        app.update();
        assert_eq!(left(&app), 1);
        assert_eq!(app.should_exit(), None);
        app.update();
        assert_eq!(app.should_exit(), Some(AppExit::Success));
    }

    #[test]
    fn an_empty_script_ends_the_app() {
        let mut app = runner("");
        app.update();
        assert_eq!(app.should_exit(), Some(AppExit::Success));
    }

    #[test]
    fn a_teleport_puts_the_feet_at_the_position() {
        let mut app = runner("teleport:30:40;wait:5");
        let player = app
            .world_mut()
            .spawn((
                Controlled,
                Transform::from_xyz(-5.0, -5.0, 3.0),
                DepthSort { feet_offset: 12.0 },
            ))
            .id();
        app.update();
        let at = app.world().get::<Transform>(player).unwrap().translation;
        assert_eq!(at, Vec3::new(30.0, 52.0, 3.0));
        assert_eq!(left(&app), 1);
    }

    #[test]
    fn a_ui_click_moves_then_presses_then_releases_one_step_per_frame() {
        let mut app = runner("ui_click:10:20;wait:5");
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.insert_resource(UiScale(2.0));
        let at = Vec2::new(20.0, 40.0);
        let mouse = |app: &App| {
            app.world()
                .resource::<ButtonInput<MouseButton>>()
                .pressed(MouseButton::Left)
        };
        let mut steps = Vec::new();
        for _ in 0..3 {
            app.update();
            let seen = std::mem::take(&mut app.world_mut().resource_mut::<Seen>().pointer);
            steps.push((seen, mouse(&app), left(&app)));
        }
        assert_eq!(
            steps,
            [
                (vec![("move", at)], false, 2),
                (vec![("move", at), ("press", at)], true, 2),
                (vec![("move", at), ("release", at)], false, 1),
            ]
        );
    }

    #[test]
    fn a_goto_that_never_arrives_gives_up_after_5_seconds() {
        let mut app = runner("goto:9;wait:5");
        app.update();
        assert_eq!(
            app.world().resource::<Seen>().changes,
            [("9".to_string(), None)]
        );
        // The limit is 5 s after the request. Steps of 0.1 s add up with a small error, so check a frame either side.
        for _ in 0..48 {
            app.update();
        }
        assert_eq!(left(&app), 2, "4.9 s");
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(left(&app), 1, "5.2 s");
        assert_eq!(app.world().resource::<Seen>().changes.len(), 1);
    }
}
