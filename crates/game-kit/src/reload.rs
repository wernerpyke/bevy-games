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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Resource, Default)]
    struct Count(usize);

    fn count(mut reader: MessageReader<ReloadMap>, mut seen: ResMut<Count>) {
        seen.0 += reader.read().count();
    }

    #[test]
    fn f1_sends_one_reload_message() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Count>()
            .add_plugins(ReloadPlugin)
            .add_systems(Update, count.after(request_reload));
        app.update();
        assert_eq!(app.world().resource::<Count>().0, 0);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F1);
        app.update();
        assert_eq!(app.world().resource::<Count>().0, 1);
    }
}
