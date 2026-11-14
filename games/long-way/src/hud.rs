use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use game_kit::animation::{StaticFrame, all_frames_animation, tag_animation};
use game_kit::hud::{Gauge, GaugeStyle, Hud, approach, gauge, image_button};

use crate::camera::VIEW_HEIGHT;
use crate::hp::Hp;
use crate::inventory::{Energy, Inventory};
use crate::weapon_menu::{
    ActiveWeaponClicked, ActiveWeapons, GaugeClicked, ICON_SIZE, weapon_icon,
};
use crate::weapons::values::values;

/// The content of the HUD bar (`adr-lw34`): the `backgrounds/hud.aseprite` picture (512 by 160 art pixels,
/// the size of the bar) and three bands on it. Top: one active weapon button per energy (a click casts its weapon).
/// Middle: one energy gauge per energy (a click opens the weapon drawer, `weapon_menu.rs`). Bottom: the row of HP
/// icons (`adr-lw24`, `adr-lw25`, `adr-lw27`, `adr-lw28`, `adr-lw31`). An active button is disabled, and its icon greyed
/// from the top, while the player has less energy than the cost of its weapon (`adr-lw39`).
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, spawn_hud_content).add_systems(
            Update,
            (
                press_weapon_keys,
                update_gauges,
                update_active_buttons,
                update_cost_overlays,
                disable_unaffordable,
                update_hp_row,
            ),
        );
    }
}

/// The keys that do what a click on a gauge (open the drawer) and on an active weapon button (cast) do, per energy.
const WEAPON_KEYS: [(Energy, KeyCode, KeyCode); 4] = [
    (Energy::Red, KeyCode::Digit1, KeyCode::KeyZ),
    (Energy::Blue, KeyCode::Digit2, KeyCode::KeyX),
    (Energy::Violet, KeyCode::Digit3, KeyCode::KeyC),
    (Energy::Black, KeyCode::Digit4, KeyCode::KeyV),
];

/// A key press sends the same message as a click on its gauge or active weapon button. The cast key of a disabled
/// button sends nothing, as a click on it does.
fn press_weapon_keys(
    keys: Res<ButtonInput<KeyCode>>,
    disabled: Query<&ActiveButton, With<InteractionDisabled>>,
    mut gauge: MessageWriter<GaugeClicked>,
    mut button: MessageWriter<ActiveWeaponClicked>,
) {
    for (energy, drawer_key, cast_key) in WEAPON_KEYS {
        if keys.just_pressed(drawer_key) {
            gauge.write(GaugeClicked(energy));
        }
        if keys.just_pressed(cast_key) && !disabled.iter().any(|button| button.0 == energy) {
            button.write(ActiveWeaponClicked(energy));
        }
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
/// Seconds a gauge and a cost overlay take from empty to full, at a constant speed (the user, 2026-11-14).
const FILL_SECS: f32 = 0.3;
/// The cost overlay over an active weapon icon: palette colour `#1f1932` at 50% alpha (the user, 2026-11-14).
const OVERLAY_COLOUR: Color = Color::srgba(0.1216, 0.0980, 0.1961, 0.5);

/// The gauge of `energy` (`game_kit::hud::gauge`).
fn gauge_style(energy: Energy) -> GaugeStyle {
    GaugeStyle {
        size: Vec2::new(COLUMN_WIDTH, GAUGE_HEIGHT),
        track: GAUGE_TRACK,
        fill: gauge_colour(energy),
        click_pad: GAUGE_CLICK_PAD,
        hover: GAUGE_HOVER,
        hover_height: GAUGE_HOVER_LINE,
        fill_secs: FILL_SECS,
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

/// The share of the cost of its weapon that the overlay of an active button shows now. It moves towards the share the
/// player has (`cost_share`) at the speed of `FILL_SECS`. It is on the button, so it stays when the icon is built again.
#[derive(Component)]
struct ShownShare(f32);

/// The grey layer over the top of the icon of an active button: the missing share of the cost of its weapon.
#[derive(Component)]
struct CostOverlay(Energy);

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
                    // The player starts with no energy (`adr-lw39`), so the overlay starts full.
                    ShownShare(0.0),
                    Node {
                        width: px(COLUMN_WIDTH),
                        height: px(BUTTON_HEIGHT),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    image_button(&server, BUTTON_ART, "Up", "Down", Some("Disabled")),
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

/// A primary click on an active weapon button asks for its weapon. `weapons/mod.rs` ignores a blank slot. A disabled
/// button sends nothing (the kit swaps its frames, but cannot stop the click).
fn click_button(
    click: On<PointerClick>,
    buttons: Query<&ActiveButton, Without<InteractionDisabled>>,
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

/// Fills each active weapon button again when the slots change: the icon of its weapon (no label) with its cost
/// overlay, or nothing.
fn update_active_buttons(
    mut commands: Commands,
    server: Res<AssetServer>,
    active: Res<ActiveWeapons>,
    buttons: Query<(Entity, &ActiveButton, &ShownShare)>,
) {
    if !active.is_changed() {
        return;
    }
    for (entity, button, shown) in &buttons {
        let mut button_commands = commands.entity(entity);
        button_commands.despawn_children();
        let Some(weapon) = active.get(button.0) else {
            continue;
        };
        let energy = button.0;
        button_commands.with_children(|face| {
            weapon_icon(face, &server, weapon, 1.0).with_children(|icon| {
                icon.spawn((
                    CostOverlay(energy),
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        top: px(0),
                        width: percent(100),
                        // The height of the moment, so a new icon does not show one frame without it.
                        height: px(overlay_height(shown.0)),
                        ..default()
                    },
                    BackgroundColor(OVERLAY_COLOUR),
                ));
            });
        });
    }
}

/// The share of the cost of the weapon in the slot of `energy` that the player has, from 0 to 1. `None` for a blank
/// slot.
fn cost_share(active: &ActiveWeapons, inventory: &Inventory, energy: Energy) -> Option<f32> {
    active
        .get(energy)
        .map(|weapon| inventory.cost_share(energy, values(weapon).cost))
}

/// The height of the cost overlay in whole art pixels: the missing share of the icon, from the top.
fn overlay_height(shown_share: f32) -> f32 {
    ((1.0 - shown_share) * ICON_SIZE).round()
}

/// Moves the shown share of each active button towards the share the player has (1 for a blank slot), and sets the
/// height of its overlay.
fn update_cost_overlays(
    time: Res<Time>,
    active: Res<ActiveWeapons>,
    inventory: Res<Inventory>,
    mut buttons: Query<(&ActiveButton, &mut ShownShare)>,
    mut overlays: Query<(&CostOverlay, &mut Node)>,
) {
    let mut shown = [1.0; 4];
    for (button, mut share) in &mut buttons {
        let target = cost_share(&active, &inventory, button.0).unwrap_or(1.0);
        let next = approach(share.0, target, time.delta_secs() / FILL_SECS);
        if next != share.0 {
            share.0 = next;
        }
        shown[button.0.index()] = share.0;
    }
    for (overlay, mut node) in &mut overlays {
        let height = px(overlay_height(shown[overlay.0.index()]));
        if node.height != height {
            node.height = height;
        }
    }
}

/// Disables the active button of each energy while the player has less than the cost of its weapon. A blank slot is
/// never disabled. It uses the count, not the shown share, so a button is enabled as soon as the energy is there.
fn disable_unaffordable(
    mut commands: Commands,
    active: Res<ActiveWeapons>,
    inventory: Res<Inventory>,
    buttons: Query<(Entity, &ActiveButton, Has<InteractionDisabled>)>,
) {
    for (entity, button, disabled) in &buttons {
        let short = cost_share(&active, &inventory, button.0).is_some_and(|share| share < 1.0);
        if short && !disabled {
            commands.entity(entity).insert(InteractionDisabled);
        } else if !short && disabled {
            commands.entity(entity).remove::<InteractionDisabled>();
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
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(
                Update,
                (
                    press_weapon_keys,
                    update_gauges,
                    update_active_buttons,
                    update_cost_overlays,
                    disable_unaffordable,
                    update_hp_row,
                ),
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
        assert_eq!(button_icons(&mut app, Energy::Blue), ["tornado"]);
        // Violet has no weapon, then one that is not built.
        assert!(button_icons(&mut app, Energy::Violet).is_empty());
        app.world_mut()
            .resource_mut::<ActiveWeapons>()
            .set_for_test(Energy::Violet, Some(WeaponKind::IceWall));
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
        // Enough red for the fire ring, so its button is not disabled.
        set_energy(&mut app, Energy::Red, 1);
        app.update();
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

    #[test]
    fn each_weapon_key_sends_the_energy_of_its_gauge_or_button_once_per_press() {
        // No time plugin, so the messages are swapped in every update (`test_util::messages`).
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .add_message::<ActiveWeaponClicked>()
            .add_message::<GaugeClicked>()
            .add_systems(Update, press_weapon_keys);
        // One key per frame, so a key that sends the energy of another one shows.
        for (key, gauge, cast) in [
            (KeyCode::Digit1, Some(Energy::Red), None),
            (KeyCode::Digit2, Some(Energy::Blue), None),
            (KeyCode::Digit3, Some(Energy::Violet), None),
            (KeyCode::Digit4, Some(Energy::Black), None),
            (KeyCode::KeyZ, None, Some(Energy::Red)),
            (KeyCode::KeyX, None, Some(Energy::Blue)),
            (KeyCode::KeyC, None, Some(Energy::Violet)),
            (KeyCode::KeyV, None, Some(Energy::Black)),
        ] {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(key);
            app.update();
            assert_eq!(
                crate::test_util::messages::<GaugeClicked>(&app),
                gauge.map(GaugeClicked).into_iter().collect::<Vec<_>>(),
                "{key:?}"
            );
            assert_eq!(
                crate::test_util::messages::<ActiveWeaponClicked>(&app),
                cast.map(ActiveWeaponClicked)
                    .into_iter()
                    .collect::<Vec<_>>(),
                "{key:?}"
            );
        }
        // A held key of each kind sends nothing more (the input plugin clears `just_pressed` in each frame).
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::Digit3);
        keys.clear();
        app.update();
        assert_eq!(crate::test_util::messages::<GaugeClicked>(&app), vec![]);
        assert_eq!(
            crate::test_util::messages::<ActiveWeaponClicked>(&app),
            vec![]
        );
    }

    fn set_energy(app: &mut App, energy: Energy, count: u32) {
        app.world_mut()
            .resource_mut::<Inventory>()
            .set_for_test(energy, count);
    }

    fn load(app: &mut App, energy: Energy, weapon: Option<WeaponKind>) {
        app.world_mut()
            .resource_mut::<ActiveWeapons>()
            .set_for_test(energy, weapon);
    }

    fn disabled(app: &mut App, energy: Energy) -> bool {
        let button = find::<ActiveButton>(app, |button| button.0 == energy);
        app.world().get::<InteractionDisabled>(button).is_some()
    }

    /// The height of the cost overlay of `energy` in art pixels, or `None` when it has none.
    fn overlay(app: &mut App, energy: Energy) -> Option<f32> {
        app.world_mut()
            .query::<(&CostOverlay, &Node)>()
            .iter(app.world())
            .find(|(overlay, _)| overlay.0 == energy)
            .map(|(_, node)| match node.height {
                Val::Px(height) => height,
                other => panic!("overlay height {other:?}"),
            })
    }

    /// Presses `key` for one update. Without the input plugin nothing clears `just_pressed`, so it is cleared here.
    fn press_key(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release_all();
        keys.clear();
    }

    #[test]
    fn an_active_button_is_disabled_while_the_energy_is_below_the_cost_and_a_blank_slot_never_is() {
        let mut app = app();
        app.update();
        // No energy at the start: red (fire ring, 1) and blue (tornado, 1) are disabled, the blank slots are not.
        let all = |app: &mut App| Energy::ALL.map(|energy| disabled(app, energy));
        // In `Energy::ALL` order: blue, red, violet, black.
        assert_eq!(all(&mut app), [true, true, false, false]);
        set_energy(&mut app, Energy::Red, 1);
        app.update();
        assert_eq!(all(&mut app), [true, false, false, false]);
        // The fireball costs 3.
        load(&mut app, Energy::Red, Some(WeaponKind::Fireball));
        app.update();
        assert!(disabled(&mut app, Energy::Red));
        set_energy(&mut app, Energy::Red, 3);
        app.update();
        assert!(!disabled(&mut app, Energy::Red));
        set_energy(&mut app, Energy::Red, 2);
        app.update();
        assert!(disabled(&mut app, Energy::Red));
        // A weapon that is not built makes a blank slot, which is never disabled.
        load(&mut app, Energy::Violet, Some(WeaponKind::IceWall));
        app.update();
        assert!(!disabled(&mut app, Energy::Violet));
    }

    #[test]
    fn a_click_or_cast_key_on_a_disabled_button_sends_nothing_and_on_a_blank_slot_still_does() {
        let mut app = app();
        app.update();
        let sent = |app: &App| crate::test_util::messages::<ActiveWeaponClicked>(app);
        let red = find::<ActiveButton>(&mut app, |button| button.0 == Energy::Red);
        let black = find::<ActiveButton>(&mut app, |button| button.0 == Energy::Black);
        crate::test_util::click(&mut app, red, PointerButton::Primary);
        assert_eq!(sent(&app), vec![]);
        press_key(&mut app, KeyCode::KeyZ);
        assert_eq!(sent(&app), vec![]);
        crate::test_util::click(&mut app, black, PointerButton::Primary);
        assert_eq!(sent(&app), vec![ActiveWeaponClicked(Energy::Black)]);
        press_key(&mut app, KeyCode::KeyV);
        assert_eq!(sent(&app), vec![ActiveWeaponClicked(Energy::Black)]);
        // With enough red, the same click and key cast.
        set_energy(&mut app, Energy::Red, 1);
        app.update();
        crate::test_util::click(&mut app, red, PointerButton::Primary);
        assert_eq!(sent(&app), vec![ActiveWeaponClicked(Energy::Red)]);
        press_key(&mut app, KeyCode::KeyZ);
        assert_eq!(sent(&app), vec![ActiveWeaponClicked(Energy::Red)]);
    }

    #[test]
    fn the_overlay_covers_the_missing_share_of_the_cost_from_the_top_and_moves_at_the_fill_speed() {
        // Each update is 16 ms, and the overlay takes 0.3 s from full to none: 1/18.75 of the icon per update.
        let mut app = app();
        app.update();
        // No energy: the icons of red and blue are covered. A blank slot has no overlay.
        assert_eq!(overlay(&mut app, Energy::Red), Some(32.0));
        assert_eq!(overlay(&mut app, Energy::Blue), Some(32.0));
        assert_eq!(overlay(&mut app, Energy::Violet), None);
        // 1 red pays for the fire ring: the overlay shrinks for 0.3 s, so it is not gone after 18 updates.
        set_energy(&mut app, Energy::Red, 1);
        for _ in 0..18 {
            app.update();
        }
        assert_eq!(overlay(&mut app, Energy::Red), Some(1.0));
        app.update();
        assert_eq!(overlay(&mut app, Energy::Red), Some(0.0));
        assert_eq!(overlay(&mut app, Energy::Blue), Some(32.0));
        // Loading the fireball (1 of 3 red) grows it to 2/3 of 32 (21) over 0.2 s, 12.5 updates.
        load(&mut app, Energy::Red, Some(WeaponKind::Fireball));
        for _ in 0..12 {
            app.update();
        }
        assert_eq!(overlay(&mut app, Energy::Red), Some(20.0));
        app.update();
        assert_eq!(overlay(&mut app, Energy::Red), Some(21.0));
        load(&mut app, Energy::Red, None);
        app.update();
        assert_eq!(overlay(&mut app, Energy::Red), None);
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
