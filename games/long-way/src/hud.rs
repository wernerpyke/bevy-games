use bevy::prelude::*;
use game_kit::hud::{Hud, label};

use crate::camera::{HUD_FONT_SIZE, HUD_TEXT};

/// The content of the HUD bar. For now one text label (`adr-lw24`).
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, spawn_hud_content);
    }
}

fn spawn_hud_content(
    mut commands: Commands,
    hud: Single<Entity, With<Hud>>,
    server: Res<AssetServer>,
) {
    let font = TextFont::from_font_size(HUD_FONT_SIZE)
        .with_font(server.load("fonts/CourierPrime-Regular.ttf"));
    commands.entity(*hud).with_children(|hud| {
        hud.spawn(label("Long Way", font, HUD_TEXT));
    });
}
