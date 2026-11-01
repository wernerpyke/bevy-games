//! Debug tool: plays a scripted sequence of key presses into the running game and logs Guy's position,
//! so behaviour can be checked without a person at the keyboard. Does nothing unless the
//! `LONG_WAY_SCRIPT` environment variable is set.
//!
//! Commands are separated by `;`:
//! - `teleport:<feet_x>:<feet_y>`   put Guy's feet at a position
//! - `hold:<keys>:<seconds>`        hold arrow keys, for example `hold:up:3` or `hold:up+left:2`
//! - `wait:<seconds>`
//! - `screenshot:<name>`            save the window to `<name>.png` and wait until it is written. The folder is
//!   `LONG_WAY_SCREENSHOT_DIR`, default `target/screenshots`. Names use letters, digits, `-` and `_`.
//!   Add a `wait` before it after a `teleport`, so the frame shows the new position.
//! - `quit`
//!
//! Example: `LONG_WAY_SCRIPT="teleport:225:-150;hold:up:3;quit" RUST_LOG=long_way=debug ...`

use std::collections::VecDeque;

use bevy::input::InputSystems;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk};

use crate::collision::GUY_FEET_OFFSET;
use crate::player::Player;

/// Seconds to wait after start before the script runs, so the window and world are up.
const START_DELAY: f32 = 1.0;
/// Seconds between position log lines while a command runs.
const LOG_INTERVAL: f32 = 0.25;

pub struct ScriptPlugin;

impl Plugin for ScriptPlugin {
    fn build(&self, app: &mut App) {
        let Ok(text) = std::env::var("LONG_WAY_SCRIPT") else {
            return;
        };
        let commands = parse(&text);
        info!("script: {} commands loaded", commands.len());
        app.insert_resource(Script {
            commands,
            elapsed: -START_DELAY,
            since_log: 0.0,
            shot_requested: false,
            shot_saved: false,
        })
        .add_systems(PreUpdate, run_script.after(InputSystems));
    }
}

#[derive(Clone)]
enum Command {
    Teleport(Vec2),
    Hold(Vec<KeyCode>, f32),
    Wait(f32),
    Screenshot(String),
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
    shot_saved: bool,
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
                ["screenshot", name]
                    if !name.is_empty()
                        && name
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') =>
                {
                    Command::Screenshot(name.to_string())
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
        _ => None,
    }
}

/// Runs after `save_to_disk`, which is added first and saves the file inside its observer.
fn mark_saved(_: On<ScreenshotCaptured>, mut script: ResMut<Script>) {
    script.shot_saved = true;
}

fn feet_of(transform: &Transform) -> Vec2 {
    transform.translation.truncate() - Vec2::new(0.0, GUY_FEET_OFFSET)
}

fn run_script(
    mut commands: Commands,
    mut script: ResMut<Script>,
    mut input: ResMut<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut player: Query<&mut Transform, With<Player>>,
    mut exit: MessageWriter<AppExit>,
) {
    script.elapsed += time.delta_secs();
    if script.elapsed < 0.0 {
        return;
    }
    let Ok(mut transform) = player.single_mut() else {
        return;
    };

    let Some(command) = script.commands.front().cloned() else {
        exit.write(AppExit::Success);
        return;
    };

    let finished = match &command {
        Command::Teleport(position) => {
            transform.translation.x = position.x;
            transform.translation.y = position.y + GUY_FEET_OFFSET;
            info!("script: teleport, feet now {:?}", position);
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
        Command::Screenshot(name) => {
            if !script.shot_requested {
                let dir = std::env::var("LONG_WAY_SCREENSHOT_DIR")
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
        Command::Quit => {
            exit.write(AppExit::Success);
            true
        }
    };

    script.since_log += time.delta_secs();
    if script.since_log >= LOG_INTERVAL {
        script.since_log = 0.0;
        info!(
            "script: t={:.2} feet {:?}",
            script.elapsed,
            feet_of(&transform)
        );
    }
    if finished {
        info!("script: command done, feet {:?}", feet_of(&transform));
        script.commands.pop_front();
        script.elapsed = 0.0;
        script.shot_requested = false;
    }
}
