//! The weapon drawer: a click on an energy icon in the HUD slides a drawer up over the game area, with one button for
//! each weapon of that energy. A click on a button sends `WeaponChosen` and the drawer slides away. The game does
//! not pause. Nothing reads `WeaponChosen` yet but a log line (`dev-docs/PLAN-weapon-drawer.md`).
//!
//! The drawer is its own UI root on the game camera (a child of `Hud` would be clipped to the HUD region). It is
//! `BlocksWorldClick`, so a click on it does not walk the player (`adr-x44`). Times use real time, so they would
//! still run if the game were paused later.

use bevy::prelude::*;
use game_kit::animation::{StaticFrame, all_frames_animation, tag_animation};
use game_kit::hud::label;
use game_kit::layers::UI_Z_INDEX;
use game_kit::pointer::{BlocksWorldClick, WorldClick};

use crate::camera::{HUD_BAR_RATIO, HUD_TEXT, VIEW_HEIGHT};
use crate::inventory::Energy;

/// Share of the HUD bar height that the drawer covers.
const DRAWER_RATIO: f32 = 0.6;
/// Height of the drawer in art pixels: 60% of the bar (160), so 96.
const DRAWER_HEIGHT: f32 = VIEW_HEIGHT * HUD_BAR_RATIO * DRAWER_RATIO;
/// Seconds the drawer takes to slide fully up, and fully down.
const OPEN_SECS: f32 = 0.2;
const CLOSE_SECS: f32 = 0.15;

/// The picture of the drawer (512 by 96, drawn by the user) and the weapon icons (32 by 32, one
/// tag per weapon, see `WeaponKind::icon_tag`).
const DRAWER_ART: &str = "art/ui/weapon_drawer.aseprite";
const ICON_ART: &str = "art/ui/weapon_icons.aseprite";
const FONT: &str = "fonts/CourierPrime-Regular.ttf";

/// Size of a weapon button, of its icon, and the text size, in art pixels. Four buttons and their gaps fit in 512.
const BUTTON_SIZE: Vec2 = Vec2::new(112.0, 72.0);
const BUTTON_GAP: f32 = 8.0;
const ICON_SIZE: f32 = 32.0;
const LABEL_SIZE: f32 = 16.0;
/// Space between the icon and the label of a button.
const ICON_LABEL_GAP: f32 = 4.0;
/// The top border of the drawer picture is this high: the buttons stand below it.
const BORDER_HEIGHT: f32 = 2.0;

/// Button colours: palette colours `#232f50`, `#415b94` (hover) and `#1f1932` (pressed).
const BUTTON_NORMAL: Color = Color::srgb(0.1373, 0.1843, 0.3137);
const BUTTON_HOVER: Color = Color::srgb(0.2549, 0.3569, 0.5804);
const BUTTON_PRESSED: Color = Color::srgb(0.1216, 0.0980, 0.1961);

/// The weapons the player can choose. The label is the text of the button, at most `MAX_LABEL` characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponKind {
    Fireball,
    FireRing,
    IceRing,
    FireWall,
    IceWall,
    Whirlpool,
}

#[cfg(test)]
const MAX_LABEL: usize = 10;

impl WeaponKind {
    #[cfg(test)]
    pub const ALL: [WeaponKind; 6] = [
        WeaponKind::Fireball,
        WeaponKind::FireRing,
        WeaponKind::IceRing,
        WeaponKind::FireWall,
        WeaponKind::IceWall,
        WeaponKind::Whirlpool,
    ];

    pub fn label(self) -> &'static str {
        match self {
            WeaponKind::Fireball => "Fireball",
            WeaponKind::FireRing => "Fire ring",
            WeaponKind::IceRing => "Ice ring",
            WeaponKind::FireWall => "Fire wall",
            WeaponKind::IceWall => "Ice wall",
            WeaponKind::Whirlpool => "Whirlpool",
        }
    }

    /// The tag of its icon in `weapon_icons.aseprite`.
    pub fn icon_tag(self) -> &'static str {
        match self {
            WeaponKind::Fireball => "fireball",
            WeaponKind::FireRing => "fire_ring",
            WeaponKind::IceRing => "ice_ring",
            WeaponKind::FireWall => "fire_wall",
            WeaponKind::IceWall => "ice_wall",
            WeaponKind::Whirlpool => "whirlpool",
        }
    }
}

/// The weapons in the drawer of an energy (the user's choice, 2026-11-11). Violet and black have none yet.
pub fn weapons_for(energy: Energy) -> &'static [WeaponKind] {
    match energy {
        Energy::Red => &[
            WeaponKind::Fireball,
            WeaponKind::FireRing,
            WeaponKind::FireWall,
        ],
        Energy::Blue => &[
            WeaponKind::IceRing,
            WeaponKind::IceWall,
            WeaponKind::Whirlpool,
        ],
        Energy::Violet | Energy::Black => &[],
    }
}

/// An energy icon in the HUD was clicked.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnergyIconClicked(pub Energy);

/// The player chose a weapon in the drawer. Nothing uses it yet: `PLAN-weapons.md` reads it.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeaponChosen(pub WeaponKind);

/// What the drawer shows. `open` is the energy whose drawer is up (or sliding up), `slide` goes from 0 (hidden) to 1
/// (fully up), and `shown` is the energy that the buttons were built for. They stay while the drawer slides down.
#[derive(Resource, Default)]
pub struct DrawerState {
    open: Option<Energy>,
    slide: f32,
    shown: Option<Energy>,
}

#[derive(Component)]
struct Drawer;

/// The row of buttons. It is built again when the open energy changes.
#[derive(Component)]
struct ButtonRow;

#[derive(Component)]
struct WeaponButton(WeaponKind);

pub struct WeaponMenuPlugin;

impl Plugin for WeaponMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DrawerState>()
            .add_message::<EnergyIconClicked>()
            .add_message::<WeaponChosen>()
            .add_systems(Startup, spawn_drawer)
            .add_systems(
                Update,
                (
                    open_on_icon_click,
                    close_on_world_click,
                    rebuild_buttons,
                    slide_drawer,
                    log_choice,
                )
                    .chain(),
            );
    }
}

/// The drawer after a click on the icon of `clicked`: the same icon closes it, another one switches it.
fn next_open(current: Option<Energy>, clicked: Energy) -> Option<Energy> {
    if current == Some(clicked) {
        None
    } else {
        Some(clicked)
    }
}

/// How far up the drawer is, from 0 to 1, for a progress from 0 to 1: it starts fast and slows down.
fn slide_value(progress: f32) -> f32 {
    let remaining = 1.0 - progress.clamp(0.0, 1.0);
    1.0 - remaining * remaining
}

/// The new progress after `delta` seconds, going up while `open` and down otherwise.
fn step_slide(slide: f32, open: bool, delta: f32) -> f32 {
    let step = if open {
        delta / OPEN_SECS
    } else {
        -delta / CLOSE_SECS
    };
    (slide + step).clamp(0.0, 1.0)
}

fn spawn_drawer(mut commands: Commands, server: Res<AssetServer>) {
    commands
        .spawn((
            Drawer,
            BlocksWorldClick,
            GlobalZIndex(UI_Z_INDEX),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                bottom: px(-DRAWER_HEIGHT),
                width: percent(100),
                height: px(DRAWER_HEIGHT),
                padding: UiRect::top(px(BORDER_HEIGHT)),
                display: Display::None,
                ..default()
            },
        ))
        .with_children(|drawer| {
            // The picture is its own full-size child, so the padding of the drawer does not shrink it (`adr-lw25`).
            drawer.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    top: px(0),
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                all_frames_animation(&server, DRAWER_ART),
                ImageNode::default().with_mode(NodeImageMode::Stretch),
            ));
            drawer.spawn((
                ButtonRow,
                Node {
                    width: percent(100),
                    height: percent(100),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    column_gap: px(BUTTON_GAP),
                    ..default()
                },
            ));
        });
}

fn open_on_icon_click(
    mut clicks: MessageReader<EnergyIconClicked>,
    mut state: ResMut<DrawerState>,
) {
    for click in clicks.read() {
        state.open = next_open(state.open, click.0);
    }
}

/// A click in the game area, outside the drawer, closes it. The click still walks the player.
fn close_on_world_click(mut clicks: MessageReader<WorldClick>, mut state: ResMut<DrawerState>) {
    if clicks.read().next().is_some() {
        state.open = None;
    }
}

/// Builds the buttons of the open energy when it is not the one shown. An energy with no weapons shows a short text.
fn rebuild_buttons(
    mut state: ResMut<DrawerState>,
    row: Single<Entity, With<ButtonRow>>,
    server: Res<AssetServer>,
    mut commands: Commands,
) {
    let Some(energy) = state.open else {
        return;
    };
    if state.shown == Some(energy) {
        return;
    }
    state.shown = Some(energy);
    let font = TextFont::from_font_size(LABEL_SIZE).with_font(server.load(FONT));
    commands
        .entity(*row)
        .despawn_children()
        .with_children(|row| {
            let weapons = weapons_for(energy);
            if weapons.is_empty() {
                row.spawn(label("No weapons yet", font.clone(), HUD_TEXT));
            }
            for &weapon in weapons {
                row.spawn((
                    WeaponButton(weapon),
                    Node {
                        width: px(BUTTON_SIZE.x),
                        height: px(BUTTON_SIZE.y),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        row_gap: px(ICON_LABEL_GAP),
                        ..default()
                    },
                    BackgroundColor(BUTTON_NORMAL),
                ))
                .observe(set_colour::<PointerEnter>(BUTTON_HOVER))
                .observe(set_colour::<PointerLeave>(BUTTON_NORMAL))
                .observe(set_colour::<PointerPress>(BUTTON_PRESSED))
                .observe(set_colour::<PointerRelease>(BUTTON_HOVER))
                .observe(choose_weapon)
                .with_children(|button| {
                    button.spawn((
                        Node {
                            width: px(ICON_SIZE),
                            height: px(ICON_SIZE),
                            ..default()
                        },
                        tag_animation(&server, ICON_ART, weapon.icon_tag()),
                        StaticFrame { relative: 0 },
                        ImageNode::default().with_mode(NodeImageMode::Stretch),
                    ));
                    button.spawn(label(weapon.label(), font.clone(), HUD_TEXT));
                });
            }
        });
}

/// An observer that gives the button under the pointer a colour.
fn set_colour<E: EntityEvent>(
    colour: Color,
) -> impl Fn(On<E>, Query<&mut BackgroundColor, With<WeaponButton>>) {
    move |event, mut buttons| {
        if let Ok(mut background) = buttons.get_mut(event.event_target()) {
            background.0 = colour;
        }
    }
}

/// A completed left click on a weapon button: report the weapon and close the drawer.
fn choose_weapon(
    click: On<PointerClick>,
    buttons: Query<&WeaponButton>,
    mut chosen: MessageWriter<WeaponChosen>,
    mut state: ResMut<DrawerState>,
) {
    if click.button != PointerButton::Primary {
        return;
    }
    if let Ok(button) = buttons.get(click.event_target()) {
        chosen.write(WeaponChosen(button.0));
        state.open = None;
    }
}

/// Moves the drawer towards open or closed. It is not drawn at all while it is fully down.
fn slide_drawer(
    time: Res<Time<Real>>,
    mut state: ResMut<DrawerState>,
    mut drawer: Single<&mut Node, With<Drawer>>,
) {
    let slide = step_slide(state.slide, state.open.is_some(), time.delta_secs());
    if slide != state.slide {
        state.slide = slide;
    }
    drawer.display = if slide > 0.0 {
        Display::Flex
    } else {
        Display::None
    };
    drawer.bottom = px(-DRAWER_HEIGHT * (1.0 - slide_value(slide)));
}

fn log_choice(mut chosen: MessageReader<WeaponChosen>) {
    for choice in chosen.read() {
        info!("weapon chosen: {}", choice.0.label());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_icon_closes_and_another_switches() {
        assert_eq!(next_open(None, Energy::Red), Some(Energy::Red));
        assert_eq!(next_open(Some(Energy::Red), Energy::Red), None);
        assert_eq!(
            next_open(Some(Energy::Red), Energy::Blue),
            Some(Energy::Blue)
        );
    }

    #[test]
    fn the_slide_goes_from_hidden_to_up_without_overshoot() {
        assert_eq!(slide_value(0.0), 0.0);
        assert_eq!(slide_value(1.0), 1.0);
        assert_eq!(slide_value(-3.0), 0.0);
        assert_eq!(slide_value(7.0), 1.0);
        let values: Vec<f32> = (0..=10).map(|i| slide_value(i as f32 / 10.0)).collect();
        assert!(values.windows(2).all(|pair| pair[0] < pair[1]));
        // It starts fast: a quarter of the time covers more than a quarter of the way.
        assert!(slide_value(0.25) > 0.25);
    }

    #[test]
    fn the_slide_takes_the_set_times() {
        assert_eq!(step_slide(0.0, true, OPEN_SECS), 1.0);
        assert!((step_slide(0.0, true, OPEN_SECS / 2.0) - 0.5).abs() < 1e-6);
        assert_eq!(step_slide(1.0, false, CLOSE_SECS), 0.0);
        assert_eq!(step_slide(0.5, true, 10.0), 1.0);
        assert_eq!(step_slide(0.5, false, 10.0), 0.0);
    }

    #[test]
    fn the_drawer_is_60_percent_of_the_bar() {
        assert!((DRAWER_HEIGHT - 96.0).abs() < 1e-3);
    }

    #[test]
    fn every_weapon_is_in_exactly_one_drawer_and_fits_a_button() {
        for weapon in WeaponKind::ALL {
            let drawers = Energy::ALL
                .iter()
                .filter(|energy| weapons_for(**energy).contains(&weapon))
                .count();
            assert_eq!(drawers, 1, "{weapon:?}");
            assert!(!weapon.label().is_empty());
            assert!(weapon.label().chars().count() <= MAX_LABEL, "{weapon:?}");
            assert!(!weapon.icon_tag().is_empty());
        }
    }

    #[test]
    fn a_row_of_buttons_fits_the_view() {
        let most = Energy::ALL
            .iter()
            .map(|energy| weapons_for(*energy).len())
            .max()
            .unwrap();
        assert!(most <= 4);
        let width = most as f32 * BUTTON_SIZE.x + (most as f32 - 1.0) * BUTTON_GAP;
        assert!(width <= VIEW_HEIGHT);
    }
}
