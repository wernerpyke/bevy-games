//! Debug tool: plays a scripted sequence of key presses into the running game and logs Guy's position,
//! so behaviour can be checked without a person at the keyboard. Does nothing unless the
//! `LONG_WAY_SCRIPT` environment variable is set.
//!
//! Commands are separated by `;`:
//! - `teleport:<feet_x>:<feet_y>`   put Guy's feet at a position
//! - `hold:<keys>:<seconds>`        hold arrow keys, for example `hold:up:3` or `hold:up+left:2`
//! - `wait:<seconds>`
//! - `quit`
//!
//! Example: `LONG_WAY_SCRIPT="teleport:225:-150;hold:up:3;quit" RUST_LOG=long_way=debug ...`

use std::collections::VecDeque;

use bevy::input::InputSystems;
use bevy::prelude::*;

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
        })
        .add_systems(PreUpdate, run_script.after(InputSystems));
    }
}

enum Command {
    Teleport(Vec2),
    Hold(Vec<KeyCode>, f32),
    Wait(f32),
    Quit,
}

#[derive(Resource)]
struct Script {
    commands: VecDeque<Command>,
    /// Seconds spent on the current command. Negative during the start delay.
    elapsed: f32,
    since_log: f32,
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

fn feet_of(transform: &Transform) -> Vec2 {
    transform.translation.truncate() - Vec2::new(0.0, GUY_FEET_OFFSET)
}

fn run_script(
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

    let Some(command) = script.commands.front() else {
        exit.write(AppExit::Success);
        return;
    };

    let finished = match command {
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
    }
}
