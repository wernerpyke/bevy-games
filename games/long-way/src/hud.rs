use bevy::prelude::*;
use game_kit::animation::{StaticFrame, all_frames_animation, tag_animation};
use game_kit::hud::{Hud, label};

use crate::camera::{HUD_FONT_SIZE, HUD_TEXT};
use crate::hp::Hp;
use crate::inventory::{Energy, Inventory};
use crate::pickups::DRAWN_SIZE;
use crate::weapon_menu::EnergyIconClicked;

/// The content of the HUD bar: the `backgrounds/hud.aseprite` picture (512 by 160 art pixels, the size of the bar), one
/// text label, the energy counts and the row of HP icons (`adr-lw24`, `adr-lw25`, `adr-lw27`, `adr-lw28`).
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, spawn_hud_content)
            .add_systems(Update, (update_energy_counts, update_hp_row));
    }
}

/// Position of the HP row in the HUD bar, from its top left corner, the size of an icon (`ui/hp.aseprite`) and the gap
/// between icons, in art pixels.
const HP_LEFT: f32 = 8.0;
const HP_TOP: f32 = 104.0;
const HP_ICON_SIZE: f32 = 32.0;
const HP_GAP: f32 = 4.0;

/// The row that holds one icon per point of max HP. `update_hp_row` fills it.
#[derive(Component)]
struct HpRow;

/// Size of the count next to an energy icon, in art pixels.
const ENERGY_FONT_SIZE: f32 = 24.0;
/// The frame of the pickup pulse shown as the icon, counted from the first frame of the tag: a full flame.
const ICON_FRAME: u16 = 3;
/// Position of the energy row in the HUD bar, from its top left corner, and the gaps in it, in art pixels. Each entry
/// (icon and count) has `CLICK_PAD` of space around it that takes the click, so the row and its gap are made smaller by
/// the same amount and the icons stay where they were.
const ENERGY_LEFT: f32 = 8.0;
const ENERGY_TOP: f32 = 64.0;
const ENERGY_GAP: f32 = 16.0;
const ICON_COUNT_GAP: f32 = 4.0;
const CLICK_PAD: f32 = 4.0;
/// Colour behind an energy entry while the pointer is over it.
const ENTRY_HOVER: Color = Color::srgba(1.0, 1.0, 1.0, 0.12);

/// The count text next to the icon of one energy.
#[derive(Component)]
struct EnergyCount(Energy);

fn spawn_hud_content(
    mut commands: Commands,
    hud: Single<Entity, With<Hud>>,
    server: Res<AssetServer>,
) {
    let font = TextFont::from_font_size(HUD_FONT_SIZE)
        .with_font(server.load("fonts/CourierPrime-Regular.ttf"));
    let count_font = TextFont::from_font_size(ENERGY_FONT_SIZE)
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
        hud.spawn(label("Long Way", font, HUD_TEXT));
        hud.spawn((
            HpRow,
            Node {
                position_type: PositionType::Absolute,
                left: px(HP_LEFT),
                top: px(HP_TOP),
                column_gap: px(HP_GAP),
                ..default()
            },
        ));
        hud.spawn(Node {
            position_type: PositionType::Absolute,
            left: px(ENERGY_LEFT - CLICK_PAD),
            top: px(ENERGY_TOP - CLICK_PAD),
            align_items: AlignItems::Center,
            column_gap: px(ENERGY_GAP - 2.0 * CLICK_PAD),
            ..default()
        })
        .with_children(|row| {
            for energy in Energy::ALL {
                // A click on the entry opens the weapon drawer of this energy (`weapon_menu.rs`).
                row.spawn((
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: px(ICON_COUNT_GAP),
                        padding: UiRect::all(px(CLICK_PAD)),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                ))
                .observe(
                    |event: On<PointerEnter>, mut colours: Query<&mut BackgroundColor>| {
                        if let Ok(mut colour) = colours.get_mut(event.event_target()) {
                            colour.0 = ENTRY_HOVER;
                        }
                    },
                )
                .observe(
                    |event: On<PointerLeave>, mut colours: Query<&mut BackgroundColor>| {
                        if let Ok(mut colour) = colours.get_mut(event.event_target()) {
                            colour.0 = Color::NONE;
                        }
                    },
                )
                .observe(
                    move |click: On<PointerClick>,
                          mut clicked: MessageWriter<EnergyIconClicked>| {
                        if click.button == PointerButton::Primary {
                            clicked.write(EnergyIconClicked(energy));
                        }
                    },
                )
                .with_children(|entry| {
                    // A still frame of the pickup art at the size the pickup is drawn in the world.
                    entry.spawn((
                        Node {
                            width: px(DRAWN_SIZE.x),
                            height: px(DRAWN_SIZE.y),
                            ..default()
                        },
                        tag_animation(&server, "art/pickups/pickups.aseprite", energy.pickup_tag()),
                        StaticFrame {
                            relative: ICON_FRAME,
                        },
                        ImageNode::default().with_mode(NodeImageMode::Stretch),
                    ));
                    entry.spawn((
                        EnergyCount(energy),
                        label("0", count_font.clone(), HUD_TEXT),
                    ));
                });
            }
        });
    });
}

fn update_energy_counts(inventory: Res<Inventory>, mut counts: Query<(&EnergyCount, &mut Text)>) {
    if inventory.is_changed() {
        for (count, mut text) in &mut counts {
            text.0 = inventory.count(count.0).to_string();
        }
    }
}

/// The tag of an HP icon: `Full` for a point the player has, `Empty` for a lost one.
fn hp_icon_tag(index: u32, hp: &Hp) -> &'static str {
    if index < hp.current { "Full" } else { "Empty" }
}

/// Fills the HP row again when `Hp` changes: one icon per point of max HP, the first `current` full, the rest empty.
fn update_hp_row(
    mut commands: Commands,
    server: Res<AssetServer>,
    hp: Query<&Hp, Changed<Hp>>,
    row: Single<Entity, With<HpRow>>,
) {
    let Ok(hp) = hp.single() else {
        return;
    };
    commands
        .entity(*row)
        .despawn_children()
        .with_children(|row| {
            for index in 0..hp.max {
                row.spawn((
                    Node {
                        width: px(HP_ICON_SIZE),
                        height: px(HP_ICON_SIZE),
                        ..default()
                    },
                    tag_animation(&server, "art/ui/hp.aseprite", hp_icon_tag(index, hp)),
                    StaticFrame { relative: 0 },
                    ImageNode::default().with_mode(NodeImageMode::Stretch),
                ));
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lost_points_show_the_empty_icon() {
        let hp = Hp { current: 2, max: 3 };
        let tags: Vec<_> = (0..hp.max).map(|i| hp_icon_tag(i, &hp)).collect();
        assert_eq!(tags, ["Full", "Full", "Empty"]);
        let none = Hp { current: 0, max: 3 };
        assert!((0..3).all(|i| hp_icon_tag(i, &none) == "Empty"));
    }
}
