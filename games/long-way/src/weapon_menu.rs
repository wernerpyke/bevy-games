//! The weapon drawer and the active weapons. A click on an energy gauge in the HUD slides a drawer up over the game
//! area, with one button for each weapon of that energy. A click on a button loads that weapon into the active slot
//! of its energy (`ActiveWeapons`) and the drawer slides away. It does not cast: a click on the active weapon button
//! in the HUD sends `ActiveWeaponClicked`, and `weapons/mod.rs` turns it into `CastWeapon` (`adr-lw31`,
//! `adr-lw34`). The game does not pause.
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
const _: () = assert!(DRAWER_HEIGHT > 95.999 && DRAWER_HEIGHT < 96.001);
/// Seconds the drawer takes to slide fully up, and fully down.
const OPEN_SECS: f32 = 0.2;
const CLOSE_SECS: f32 = 0.15;

/// The picture of the drawer (512 by 96, drawn by the user) and the weapon icons (32 by 32, one
/// tag per weapon, see `WeaponKind::icon_tag`).
const DRAWER_ART: &str = "art/ui/weapon_drawer.aseprite";
const ICON_ART: &str = "art/ui/weapon_icons.aseprite";
const FONT: &str = "fonts/CourierPrime-Regular.ttf";

/// Width of the drawer picture in art pixels.
const DRAWER_ART_WIDTH: f32 = 512.0;
/// Size of a weapon button, of its icon, and the text size, in art pixels. Four buttons and their gaps fit in the
/// drawer picture.
const BUTTON_SIZE: Vec2 = Vec2::new(112.0, 72.0);
const BUTTON_GAP: f32 = 8.0;
const MAX_BUTTONS: usize = 4;
const _: () = assert!(
    MAX_BUTTONS as f32 * BUTTON_SIZE.x + (MAX_BUTTONS - 1) as f32 * BUTTON_GAP <= DRAWER_ART_WIDTH
);
pub const ICON_SIZE: f32 = 32.0;
const LABEL_SIZE: f32 = 16.0;
/// Space between the icon and the label of a button.
const ICON_LABEL_GAP: f32 = 4.0;
/// The top border of the drawer picture is this high: the buttons stand below it.
const BORDER_HEIGHT: f32 = 2.0;

/// Button colours: palette colours `#232f50`, `#415b94` (hover) and `#1f1932` (pressed).
const BUTTON_NORMAL: Color = Color::srgb(0.1373, 0.1843, 0.3137);
const BUTTON_HOVER: Color = Color::srgb(0.2549, 0.3569, 0.5804);
const BUTTON_PRESSED: Color = Color::srgb(0.1216, 0.0980, 0.1961);
/// The opacity of the button, icon and label of a weapon that is not built yet (`WeaponKind::is_built`).
const DIM_ALPHA: f32 = 0.35;

/// The weapons the player can choose. The label is the text of the button, at most `MAX_LABEL` characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponKind {
    Fireball,
    FireRing,
    IceRing,
    FireWall,
    IceWall,
    Tornado,
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
        WeaponKind::Tornado,
    ];

    pub fn label(self) -> &'static str {
        match self {
            WeaponKind::Fireball => "Fireball",
            WeaponKind::FireRing => "Fire ring",
            WeaponKind::IceRing => "Ice ring",
            WeaponKind::FireWall => "Fire wall",
            WeaponKind::IceWall => "Ice wall",
            WeaponKind::Tornado => "Tornado",
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
            WeaponKind::Tornado => "tornado",
        }
    }

    /// Whether the weapon works yet. Each weapon is added here when it is built (`weapons.md`). A weapon
    /// that is not built is dimmed in the drawer and cannot be loaded, and an active button that holds it is blank.
    pub fn is_built(self) -> bool {
        matches!(
            self,
            WeaponKind::FireRing | WeaponKind::IceRing | WeaponKind::Tornado | WeaponKind::Fireball
        )
    }
}

/// The weapon in the active slot of an energy when the game starts (the user's choice, 2026-11-12; blue changed to the
/// tornado on 2026-11-14).
fn default_weapon(energy: Energy) -> Option<WeaponKind> {
    match energy {
        Energy::Red => Some(WeaponKind::FireRing),
        Energy::Blue => Some(WeaponKind::Tornado),
        Energy::Violet | Energy::Black => None,
    }
}

/// The weapon loaded in the active slot of each energy. It belongs to no map, so it stays when the player changes
/// map.
#[derive(Resource, Debug)]
pub struct ActiveWeapons([Option<WeaponKind>; 4]);

impl Default for ActiveWeapons {
    fn default() -> Self {
        Self(Energy::ALL.map(default_weapon))
    }
}

impl ActiveWeapons {
    /// The weapon that the active button of `energy` shows and casts: the loaded weapon, if it is built.
    pub fn get(&self, energy: Energy) -> Option<WeaponKind> {
        self.0[energy.index()].filter(|weapon| weapon.is_built())
    }

    fn load(&mut self, energy: Energy, weapon: WeaponKind) {
        self.0[energy.index()] = Some(weapon);
    }

    #[cfg(test)]
    pub fn set_for_test(&mut self, energy: Energy, weapon: Option<WeaponKind>) {
        self.0[energy.index()] = weapon;
    }
}

/// The weapons in the drawer of an energy, from left to right (the user's choice, 2026-11-11; the tornado first in blue,
/// 2026-11-14). Violet and black have none yet.
pub fn weapons_for(energy: Energy) -> &'static [WeaponKind] {
    match energy {
        Energy::Red => &[
            WeaponKind::Fireball,
            WeaponKind::FireRing,
            WeaponKind::FireWall,
        ],
        Energy::Blue => &[
            WeaponKind::Tornado,
            WeaponKind::IceRing,
            WeaponKind::IceWall,
        ],
        Energy::Violet | Energy::Black => &[],
    }
}

/// An energy gauge in the HUD was clicked: open, switch or close the drawer.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GaugeClicked(pub Energy);

/// The active weapon button of an energy in the HUD was clicked. `weapons/mod.rs` casts the weapon in its slot.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveWeaponClicked(pub Energy);

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

/// A drawer button: the weapon it loads into the slot of `energy`.
#[derive(Component)]
struct WeaponButton {
    energy: Energy,
    weapon: WeaponKind,
}

pub struct WeaponMenuPlugin;

impl Plugin for WeaponMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DrawerState>()
            .init_resource::<ActiveWeapons>()
            .add_message::<GaugeClicked>()
            .add_message::<ActiveWeaponClicked>()
            .add_systems(Startup, spawn_drawer)
            .add_systems(
                Update,
                (
                    open_on_gauge_click,
                    close_on_world_click,
                    close_on_active_click,
                    rebuild_buttons,
                    slide_drawer,
                )
                    .chain(),
            );
    }
}

/// The drawer after a click on the gauge of `clicked`: the same gauge closes it, another one switches it.
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

fn open_on_gauge_click(mut clicks: MessageReader<GaugeClicked>, mut state: ResMut<DrawerState>) {
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

/// A click on an active weapon button closes the drawer (the user's choice, 2026-11-12), also on a blank button.
fn close_on_active_click(
    mut clicks: MessageReader<ActiveWeaponClicked>,
    mut state: ResMut<DrawerState>,
) {
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
    commands
        .entity(*row)
        .despawn_children()
        .with_children(|row| {
            let weapons = weapons_for(energy);
            if weapons.is_empty() {
                let font = TextFont::from_font_size(LABEL_SIZE).with_font(server.load(FONT));
                row.spawn(label("No weapons yet", font, HUD_TEXT));
            }
            for &weapon in weapons {
                // A weapon that is not built is dimmed and takes no clicks.
                let alpha = if weapon.is_built() { 1.0 } else { DIM_ALPHA };
                let mut button = row.spawn((
                    WeaponButton { energy, weapon },
                    Node {
                        width: px(BUTTON_SIZE.x),
                        height: px(BUTTON_SIZE.y),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        row_gap: px(ICON_LABEL_GAP),
                        ..default()
                    },
                    BackgroundColor(BUTTON_NORMAL.with_alpha(alpha)),
                ));
                if weapon.is_built() {
                    button
                        .observe(set_colour::<PointerEnter>(BUTTON_HOVER))
                        .observe(set_colour::<PointerLeave>(BUTTON_NORMAL))
                        .observe(set_colour::<PointerPress>(BUTTON_PRESSED))
                        .observe(set_colour::<PointerRelease>(BUTTON_HOVER))
                        .observe(load_weapon);
                }
                button.with_children(|button| weapon_face(button, &server, weapon, alpha));
            }
        });
}

/// The icon of `weapon` above its label, at `alpha` opacity: the face of a drawer button.
fn weapon_face(
    button: &mut ChildSpawnerCommands,
    server: &AssetServer,
    weapon: WeaponKind,
    alpha: f32,
) {
    weapon_icon(button, server, weapon, alpha);
    let font = TextFont::from_font_size(LABEL_SIZE).with_font(server.load(FONT));
    button.spawn(label(weapon.label(), font, HUD_TEXT.with_alpha(alpha)));
}

/// The icon of `weapon` at `alpha` opacity: in the drawer above its label, and alone on an active weapon button in
/// the HUD (with the cost overlay as its child).
pub fn weapon_icon<'a>(
    button: &'a mut ChildSpawnerCommands,
    server: &AssetServer,
    weapon: WeaponKind,
    alpha: f32,
) -> EntityCommands<'a> {
    button.spawn((
        Node {
            width: px(ICON_SIZE),
            height: px(ICON_SIZE),
            ..default()
        },
        tag_animation(server, ICON_ART, weapon.icon_tag()),
        StaticFrame { relative: 0 },
        ImageNode::default()
            .with_mode(NodeImageMode::Stretch)
            .with_color(Color::WHITE.with_alpha(alpha)),
    ))
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

/// A completed left click on the button of a built weapon: load it into the slot of its energy and close the drawer.
fn load_weapon(
    click: On<PointerClick>,
    buttons: Query<&WeaponButton>,
    mut active: ResMut<ActiveWeapons>,
    mut state: ResMut<DrawerState>,
) {
    if click.button != PointerButton::Primary {
        return;
    }
    if let Ok(&WeaponButton { energy, weapon }) = buttons.get(click.event_target()) {
        active.load(energy, weapon);
        info!("weapons: loaded {} for {energy:?}", weapon.label());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_gauge_closes_and_another_switches() {
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
    fn each_energy_starts_with_its_default_and_only_a_built_weapon_is_active() {
        let mut active = ActiveWeapons::default();
        // Blue holds the tornado and red the fire ring. Violet and black have no weapon.
        let defaults = [
            Some(WeaponKind::Tornado),
            Some(WeaponKind::FireRing),
            None,
            None,
        ];
        assert_eq!(active.0, defaults);
        assert_eq!(Energy::ALL.map(|energy| active.get(energy)), defaults);
        // A weapon that is not built is held but not active.
        active.set_for_test(Energy::Violet, Some(WeaponKind::IceWall));
        assert_eq!(active.get(Energy::Violet), None);
        for energy in Energy::ALL {
            if let Some(weapon) = default_weapon(energy) {
                assert!(weapons_for(energy).contains(&weapon), "{energy:?}");
            }
        }
    }

    #[test]
    fn no_drawer_has_more_buttons_than_fit_its_picture() {
        // The row of `MAX_BUTTONS` fits the picture: a compile-time assert beside the constants.
        for energy in Energy::ALL {
            assert!(weapons_for(energy).len() <= MAX_BUTTONS, "{energy:?}");
        }
    }

    fn app() -> App {
        let mut app = crate::test_util::asset_app(50);
        app.add_message::<WorldClick>()
            .add_plugins(WeaponMenuPlugin);
        app.update();
        app
    }

    fn open(app: &App) -> Option<Energy> {
        app.world().resource::<DrawerState>().open
    }

    fn click_gauge(app: &mut App, energy: Energy) {
        app.world_mut().write_message(GaugeClicked(energy));
        app.update();
    }

    #[test]
    fn a_gauge_opens_its_drawer_and_the_same_gauge_a_click_in_the_world_or_an_active_button_closes_it()
     {
        let mut app = app();
        click_gauge(&mut app, Energy::Red);
        assert_eq!(open(&app), Some(Energy::Red));
        click_gauge(&mut app, Energy::Red);
        assert_eq!(open(&app), None);
        click_gauge(&mut app, Energy::Blue);
        assert_eq!(open(&app), Some(Energy::Blue));
        app.world_mut().write_message(WorldClick {
            position: Vec2::new(30.0, 40.0),
            button: MouseButton::Left,
        });
        app.update();
        assert_eq!(open(&app), None);
        // A blank slot (violet) closes it too.
        click_gauge(&mut app, Energy::Red);
        assert_eq!(open(&app), Some(Energy::Red));
        app.world_mut()
            .write_message(ActiveWeaponClicked(Energy::Violet));
        app.update();
        assert_eq!(open(&app), None);
    }

    /// The weapons of the buttons in the row, and the texts that are direct children of the row.
    fn row(app: &mut App) -> (Vec<WeaponKind>, Vec<String>) {
        let world = app.world_mut();
        let row = world
            .query_filtered::<&Children, With<ButtonRow>>()
            .single(world)
            .map(|children| children.to_vec())
            .unwrap_or_default();
        let buttons = row
            .iter()
            .filter_map(|child| {
                world
                    .get::<WeaponButton>(*child)
                    .map(|button| button.weapon)
            })
            .collect();
        let texts = row
            .iter()
            .filter_map(|child| world.get::<Text>(*child).map(|text| text.0.clone()))
            .collect();
        (buttons, texts)
    }

    #[test]
    fn an_energy_with_no_weapons_shows_a_text_and_one_with_weapons_a_button_each() {
        let mut app = app();
        click_gauge(&mut app, Energy::Violet);
        assert_eq!(row(&mut app), (vec![], vec!["No weapons yet".to_string()]));
        click_gauge(&mut app, Energy::Red);
        assert_eq!(
            row(&mut app),
            (
                vec![
                    WeaponKind::Fireball,
                    WeaponKind::FireRing,
                    WeaponKind::FireWall
                ],
                vec![]
            )
        );
        // The tornado is first in blue (the user's choice, 2026-11-14).
        click_gauge(&mut app, Energy::Blue);
        assert_eq!(
            row(&mut app),
            (
                vec![
                    WeaponKind::Tornado,
                    WeaponKind::IceRing,
                    WeaponKind::IceWall
                ],
                vec![]
            )
        );
    }

    fn button(app: &mut App, weapon: WeaponKind) -> Entity {
        app.world_mut()
            .query::<(Entity, &WeaponButton)>()
            .iter(app.world())
            .find(|(_, button)| button.weapon == weapon)
            .map(|(entity, _)| entity)
            .unwrap()
    }

    #[test]
    fn only_a_primary_click_on_a_built_weapon_loads_it_into_its_own_slot_and_closes_the_drawer() {
        let mut app = app();
        // Empty the red slot, so that loading the fire ring changes it.
        app.world_mut()
            .resource_mut::<ActiveWeapons>()
            .set_for_test(Energy::Red, None);
        click_gauge(&mut app, Energy::Red);
        let slots = |app: &App| app.world().resource::<ActiveWeapons>().0;
        let before = slots(&app);
        let ring = button(&mut app, WeaponKind::FireRing);
        for other in [PointerButton::Secondary, PointerButton::Middle] {
            crate::test_util::click(&mut app, ring, other);
            assert_eq!(slots(&app), before, "{other:?}");
            assert_eq!(open(&app), Some(Energy::Red), "{other:?}");
        }
        let wall = button(&mut app, WeaponKind::FireWall);
        crate::test_util::click(&mut app, wall, PointerButton::Primary);
        assert_eq!(slots(&app), before, "the fire wall is not built");
        assert_eq!(open(&app), Some(Energy::Red));
        crate::test_util::click(&mut app, ring, PointerButton::Primary);
        assert_eq!(
            slots(&app),
            [
                Some(WeaponKind::Tornado),
                Some(WeaponKind::FireRing),
                None,
                None
            ]
        );
        assert_eq!(open(&app), None);
    }

    #[test]
    fn the_drawer_is_drawn_while_the_slide_is_above_zero_and_hidden_at_zero() {
        let mut app = app();
        let display = |app: &mut App| {
            app.world_mut()
                .query_filtered::<&Node, With<Drawer>>()
                .single(app.world())
                .unwrap()
                .display
        };
        assert_eq!(display(&mut app), Display::None);
        click_gauge(&mut app, Energy::Red);
        assert_eq!(display(&mut app), Display::Flex);
        while app.world().resource::<DrawerState>().slide < 1.0 {
            app.update();
        }
        click_gauge(&mut app, Energy::Red);
        let mut frames = 0;
        while app.world().resource::<DrawerState>().slide > 0.0 {
            assert_eq!(display(&mut app), Display::Flex);
            app.update();
            frames += 1;
            assert!(frames < 100, "the drawer never closed");
        }
        assert_eq!(display(&mut app), Display::None);
    }
}
