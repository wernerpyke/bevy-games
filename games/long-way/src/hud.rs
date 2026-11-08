use bevy::prelude::*;
use game_kit::animation::all_frames_animation;
use game_kit::hud::{Hud, label};

use crate::camera::{HUD_FONT_SIZE, HUD_TEXT};
use crate::inventory::Inventory;

/// The content of the HUD bar: the `backgrounds/hud.aseprite` picture (512 by 160 art pixels, the size of the bar) and
/// one text label (`adr-lw24`, `adr-lw25`).
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, spawn_hud_content)
            .add_systems(Update, update_energy_label);
    }
}

/// The text line with the energy counts. Temporary until the HUD art has a place for them.
#[derive(Component)]
struct EnergyLabel;

fn spawn_hud_content(
    mut commands: Commands,
    hud: Single<Entity, With<Hud>>,
    server: Res<AssetServer>,
) {
    let font = TextFont::from_font_size(HUD_FONT_SIZE)
        .with_font(server.load("fonts/CourierPrime-Regular.ttf"));
    // The picture is its own full-size child, not on the padded `Hud` node: padding would shrink it.
    commands.entity(*hud).with_children(|hud| {
        hud.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                ..default()
            },
            all_frames_animation(&server, "art/backgrounds/hud.aseprite"),
            ImageNode::default().with_mode(NodeImageMode::Stretch),
        ));
        hud.spawn(label("Long Way", font.clone(), HUD_TEXT));
        hud.spawn((
            EnergyLabel,
            label(Inventory::default().summary(), font, HUD_TEXT),
            Node {
                position_type: PositionType::Absolute,
                left: px(8),
                top: px(64),
                ..default()
            },
        ));
    });
}

fn update_energy_label(inventory: Res<Inventory>, mut label: Single<&mut Text, With<EnergyLabel>>) {
    if inventory.is_changed() {
        label.0 = inventory.summary();
    }
}
