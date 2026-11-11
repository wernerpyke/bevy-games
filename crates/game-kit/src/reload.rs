//! The F1 key. It sends a `ReloadMap` message. The game decides what a reload does: it reads the message (for example
//! with `on_message::<ReloadMap>`), loads its map again and resets its state.

use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;

pub struct ReloadPlugin;

impl Plugin for ReloadPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ReloadMap>().add_systems(
            Update,
            request_reload.run_if(input_just_pressed(KeyCode::F1)),
        );
    }
}

/// F1 was pressed: the game should load its map again and reset.
#[derive(Message, Clone, Copy, Debug)]
pub struct ReloadMap;

fn request_reload(mut requests: MessageWriter<ReloadMap>) {
    requests.write(ReloadMap);
}
