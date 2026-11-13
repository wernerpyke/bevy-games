use bevy::prelude::*;
use game_kit::animation::{StaticFrame, all_frames_animation, tag_animation};
use game_kit::hud::{Gauge, GaugeStyle, Hud, gauge, image_button};

use crate::camera::VIEW_HEIGHT;
use crate::hp::Hp;
use crate::inventory::{Energy, Inventory};
use crate::weapon_menu::{ActiveWeaponClicked, ActiveWeapons, GaugeClicked, weapon_icon};

/// The content of the HUD bar (`adr-lw34`): the `backgrounds/hud.aseprite` picture (512 by 160 art pixels,
/// the size of the bar) and three bands on it. Top: one active weapon button per energy (a click casts its weapon).
/// Middle: one energy gauge per energy (a click opens the weapon drawer, `weapon_menu.rs`). Bottom: the row of HP
/// icons (`adr-lw24`, `adr-lw25`, `adr-lw27`, `adr-lw28`, `adr-lw31`).
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, spawn_hud_content).add_systems(
            Update,
            (update_gauges, update_active_buttons, update_hp_row),
        );
    }
}

/// The energies from left to right: one column each for a button and a gauge.
const COLUMNS: [Energy; 4] = [Energy::Red, Energy::Blue, Energy::Violet, Energy::Black];
/// Width of the bar, and the columns in it, in art pixels. The columns fill the bar from x 2 to the right edge, under
/// the four fence segments of `backgrounds/hud.aseprite` (its separators are 2 pixels wide, the first at x 0).
const BAR_WIDTH: f32 = VIEW_HEIGHT;
const COLUMN_LEFT: f32 = 2.0;
const COLUMN_RIGHT: f32 = 0.0;
const COLUMN_WIDTH: f32 = 126.0;
const COLUMN_GAP: f32 = 2.0;
const _: () = assert!(
    COLUMN_LEFT + 4.0 * COLUMN_WIDTH + 3.0 * COLUMN_GAP + COLUMN_RIGHT == BAR_WIDTH,
    "the four columns fill the bar"
);

/// Top and height of the active weapon buttons, in art pixels. The height is that of `ui/weapon_button.aseprite`.
const BUTTON_TOP: f32 = 12.0;
const BUTTON_HEIGHT: f32 = 56.0;
const BUTTON_ART: &str = "art/ui/weapon_button.aseprite";

/// Top and height of a gauge, and the space above and below it that also takes the click, in art pixels.
const GAUGE_TOP: f32 = 70.0;
const GAUGE_HEIGHT: f32 = 12.0;
const GAUGE_CLICK_PAD: f32 = 2.0;
/// The empty part of a gauge: palette colour `#1f1932`.
const GAUGE_TRACK: Color = Color::srgb(0.1216, 0.0980, 0.1961);
/// The line just below a gauge while the pointer is over it (the user's choice, 2026-11-12), and its height.
const GAUGE_HOVER: Color = Color::WHITE;
const GAUGE_HOVER_LINE: f32 = 2.0;

/// The gauge of `energy` (`game_kit::hud::gauge`).
fn gauge_style(energy: Energy) -> GaugeStyle {
    GaugeStyle {
        size: Vec2::new(COLUMN_WIDTH, GAUGE_HEIGHT),
        track: GAUGE_TRACK,
        fill: gauge_colour(energy),
        click_pad: GAUGE_CLICK_PAD,
        hover: GAUGE_HOVER,
        hover_height: GAUGE_HOVER_LINE,
    }
}

/// Position of the HP row in the HUD bar, from its top left corner, the size of an icon (`ui/hp.aseprite`) and the gap
/// between icons, in art pixels.
const HP_LEFT: f32 = 8.0;
const HP_TOP: f32 = 104.0;
const HP_ICON_SIZE: f32 = 32.0;
const HP_GAP: f32 = 4.0;
const _: () = assert!(GAUGE_TOP + GAUGE_HEIGHT + GAUGE_CLICK_PAD <= HP_TOP);
const _: () = assert!(BUTTON_TOP + BUTTON_HEIGHT <= GAUGE_TOP - GAUGE_CLICK_PAD);

/// The filled part of a gauge: the main colour of the energy's pickup (`pickups.aseprite`, its own palette, so the
/// gauge matches the pickup in the world). Black uses the lighter grey of its pickup, which shows on the dark track.
fn gauge_colour(energy: Energy) -> Color {
    match energy {
        Energy::Red => Color::srgb_u8(0xfc, 0x58, 0x42),
        Energy::Blue => Color::srgb_u8(0x27, 0xab, 0xfa),
        Energy::Violet => Color::srgb_u8(0xc9, 0x5a, 0xf8),
        Energy::Black => Color::srgb_u8(0x6a, 0x6a, 0x6a),
    }
}

/// The active weapon button of an energy. `update_active_buttons` fills it with the face of the weapon in its slot.
#[derive(Component)]
struct ActiveButton(Energy);

/// The gauge of an energy. `update_gauges` sets its fraction to the share of `MAX_ENERGY` the player has.
#[derive(Component)]
struct GaugeEntry(Energy);

/// The row that holds one icon per point of max HP. `update_hp_row` fills it.
#[derive(Component)]
struct HpRow;

fn spawn_hud_content(
    mut commands: Commands,
    hud: Single<Entity, With<Hud>>,
    server: Res<AssetServer>,
) {
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
        hud.spawn(row(BUTTON_TOP)).with_children(|row| {
            for energy in COLUMNS {
                row.spawn((
                    ActiveButton(energy),
                    Node {
                        width: px(COLUMN_WIDTH),
                        height: px(BUTTON_HEIGHT),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    image_button(&server, BUTTON_ART, "Up", "Down"),
                ))
                .observe(click_button);
            }
        });
        hud.spawn(row(GAUGE_TOP - GAUGE_CLICK_PAD))
            .with_children(|row| {
                for energy in COLUMNS {
                    row.spawn((GaugeEntry(energy), gauge(gauge_style(energy))))
                        .observe(click_gauge);
                }
            });
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
    });
}

/// A row of the four columns, `top` art pixels below the top of the bar.
fn row(top: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(COLUMN_LEFT),
        top: px(top),
        column_gap: px(COLUMN_GAP),
        ..default()
    }
}

/// A primary click on an active weapon button asks for its weapon. `weapons/mod.rs` ignores a blank slot.
fn click_button(
    click: On<PointerClick>,
    buttons: Query<&ActiveButton>,
    mut clicked: MessageWriter<ActiveWeaponClicked>,
) {
    if click.button == PointerButton::Primary
        && let Ok(button) = buttons.get(click.event_target())
    {
        clicked.write(ActiveWeaponClicked(button.0));
    }
}

/// A primary click on a gauge opens, switches or closes the weapon drawer (`weapon_menu.rs`).
fn click_gauge(
    click: On<PointerClick>,
    entries: Query<&GaugeEntry>,
    mut clicked: MessageWriter<GaugeClicked>,
) {
    if click.button == PointerButton::Primary
        && let Ok(entry) = entries.get(click.event_target())
    {
        clicked.write(GaugeClicked(entry.0));
    }
}

fn update_gauges(inventory: Res<Inventory>, mut gauges: Query<(&GaugeEntry, &mut Gauge)>) {
    if inventory.is_changed() {
        for (entry, mut gauge) in &mut gauges {
            gauge.fraction = inventory.fraction(entry.0);
        }
    }
}

/// Fills each active weapon button again when the slots change: the icon of its weapon (no label), or nothing.
fn update_active_buttons(
    mut commands: Commands,
    server: Res<AssetServer>,
    active: Res<ActiveWeapons>,
    buttons: Query<(Entity, &ActiveButton)>,
) {
    if !active.is_changed() {
        return;
    }
    for (entity, button) in &buttons {
        let mut button_commands = commands.entity(entity);
        button_commands.despawn_children();
        if let Some(weapon) = active.get(button.0) {
            button_commands.with_children(|face| weapon_icon(face, &server, weapon, 1.0));
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
    use crate::weapon_menu::WeaponKind;
    use bevy::ecs::system::RunSystemOnce;
    use bevy_aseprite_ultra::prelude::AseAnimation;

    #[test]
    fn lost_points_show_the_empty_icon() {
        let hp = Hp { current: 2, max: 3 };
        let tags: Vec<_> = (0..hp.max).map(|i| hp_icon_tag(i, &hp)).collect();
        assert_eq!(tags, ["Full", "Full", "Empty"]);
        let none = Hp { current: 0, max: 3 };
        assert!((0..3).all(|i| hp_icon_tag(i, &none) == "Empty"));
    }

    /// An app with the HUD content spawned in a `Hud` root.
    fn app() -> App {
        let mut app = crate::test_util::asset_app(16);
        app.init_resource::<Inventory>()
            .init_resource::<ActiveWeapons>()
            .add_message::<ActiveWeaponClicked>()
            .add_message::<GaugeClicked>()
            .add_systems(
                Update,
                (update_gauges, update_active_buttons, update_hp_row),
            );
        app.world_mut().spawn(Hud);
        app.world_mut().run_system_once(spawn_hud_content).unwrap();
        app.update();
        app
    }

    fn find<C: Component>(app: &mut App, matches: impl Fn(&C) -> bool) -> Entity {
        app.world_mut()
            .query::<(Entity, &C)>()
            .iter(app.world())
            .find(|(_, component)| matches(component))
            .map(|(entity, _)| entity)
            .unwrap()
    }

    #[test]
    fn each_gauge_follows_its_own_energy_after_a_change() {
        let mut app = app();
        let red = find::<GaugeEntry>(&mut app, |entry| entry.0 == Energy::Red);
        let blue = find::<GaugeEntry>(&mut app, |entry| entry.0 == Energy::Blue);
        let fraction = |app: &App, entity| app.world().get::<Gauge>(entity).unwrap().fraction;
        assert_eq!((fraction(&app, red), fraction(&app, blue)), (0.0, 0.0));
        for _ in 0..5 {
            app.world_mut().resource_mut::<Inventory>().add(Energy::Red);
        }
        app.update();
        // 5 of 10 is exactly 0.5 in f32.
        assert_eq!((fraction(&app, red), fraction(&app, blue)), (0.5, 0.0));
    }

    /// Every child of the active button of `energy`: an icon as its tag, a text as `text:<text>`.
    fn button_icons(app: &mut App, energy: Energy) -> Vec<String> {
        let button = find::<ActiveButton>(app, |button| button.0 == energy);
        let world = app.world();
        world
            .get::<Children>(button)
            .map(|children| children.to_vec())
            .unwrap_or_default()
            .iter()
            .map(
                |child| match (world.get::<AseAnimation>(*child), world.get::<Text>(*child)) {
                    (Some(ase), _) => ase.animation.tag.clone().unwrap_or_default(),
                    (None, Some(text)) => format!("text:{}", text.0),
                    (None, None) => "other".to_string(),
                },
            )
            .collect()
    }

    #[test]
    fn an_active_button_shows_the_weapon_of_its_slot_and_is_blank_without_a_built_weapon() {
        let mut app = app();
        // The icon alone, no label (the user's choice, 2026-11-12).
        assert_eq!(button_icons(&mut app, Energy::Red), ["fire_ring"]);
        assert_eq!(button_icons(&mut app, Energy::Blue), ["ice_ring"]);
        // Violet has no weapon, then one that is not built.
        assert!(button_icons(&mut app, Energy::Violet).is_empty());
        app.world_mut()
            .resource_mut::<ActiveWeapons>()
            .set_for_test(Energy::Violet, Some(WeaponKind::Whirlpool));
        app.update();
        assert!(button_icons(&mut app, Energy::Violet).is_empty());
        app.world_mut()
            .resource_mut::<ActiveWeapons>()
            .set_for_test(Energy::Red, None);
        app.update();
        assert!(button_icons(&mut app, Energy::Red).is_empty());
        app.world_mut()
            .resource_mut::<ActiveWeapons>()
            .set_for_test(Energy::Violet, Some(WeaponKind::FireRing));
        app.update();
        assert_eq!(button_icons(&mut app, Energy::Violet), ["fire_ring"]);
    }

    #[test]
    fn only_a_primary_click_sends_the_energy_of_its_button_or_gauge() {
        let mut app = app();
        let red_button = find::<ActiveButton>(&mut app, |button| button.0 == Energy::Red);
        let blue_gauge = find::<GaugeEntry>(&mut app, |entry| entry.0 == Energy::Blue);
        for other in [PointerButton::Secondary, PointerButton::Middle] {
            crate::test_util::click(&mut app, red_button, other);
            crate::test_util::click(&mut app, blue_gauge, other);
        }
        assert_eq!(
            crate::test_util::messages::<ActiveWeaponClicked>(&app),
            vec![]
        );
        assert_eq!(crate::test_util::messages::<GaugeClicked>(&app), vec![]);
        // Two of each, so that a fixed energy would show. A blank slot (black) still sends: the cast ignores it.
        let black_button = find::<ActiveButton>(&mut app, |button| button.0 == Energy::Black);
        let red_gauge = find::<GaugeEntry>(&mut app, |entry| entry.0 == Energy::Red);
        for entity in [red_button, black_button, blue_gauge, red_gauge] {
            crate::test_util::click(&mut app, entity, PointerButton::Primary);
        }
        assert_eq!(
            crate::test_util::messages::<ActiveWeaponClicked>(&app),
            vec![
                ActiveWeaponClicked(Energy::Red),
                ActiveWeaponClicked(Energy::Black)
            ]
        );
        assert_eq!(
            crate::test_util::messages::<GaugeClicked>(&app),
            vec![GaugeClicked(Energy::Blue), GaugeClicked(Energy::Red)]
        );
    }

    fn hp_row_tags(app: &mut App) -> Vec<String> {
        let world = app.world_mut();
        let icons = world
            .query_filtered::<&Children, With<HpRow>>()
            .single(world)
            .unwrap()
            .to_vec();
        icons
            .iter()
            .map(|icon| {
                world
                    .get::<AseAnimation>(*icon)
                    .and_then(|ase| ase.animation.tag.clone())
                    .unwrap_or_default()
            })
            .collect()
    }

    #[test]
    fn the_hp_row_is_filled_again_when_the_points_change() {
        let mut app = app();
        let player = app.world_mut().spawn(Hp { current: 4, max: 4 }).id();
        app.update();
        assert_eq!(hp_row_tags(&mut app), ["Full", "Full", "Full", "Full"]);
        app.world_mut().get_mut::<Hp>(player).unwrap().current = 1;
        app.update();
        assert_eq!(hp_row_tags(&mut app), ["Full", "Empty", "Empty", "Empty"]);
    }
}
