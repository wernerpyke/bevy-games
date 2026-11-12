//! The HUD: a `Hud` root node on the `HudCamera`, and generic widgets that a game puts inside it: a text label, an
//! image button and a gauge. The kit has no game values: the game decides what the HUD shows and what a click does.
//! See `dev-docs/adr/EXTENSIONS-SCREEN.md` adr-x34 and adr-x45, and `dev-docs/extensions/screen-layout.md`.
//!
//! UI is written in art pixels (`screen::ui_scale`), so a node 512 wide is as wide as the game view.

use bevy::prelude::*;
use bevy::ui::Pressed;
use bevy_aseprite_ultra::prelude::AseAnimation;

use crate::animation::{play_if_changed, tag_animation};
use crate::layers::HUD_Z_INDEX;
use crate::screen::hud_camera_bundle;

/// Marks the camera that draws the HUD. It sits in the `hud` region of the window.
#[derive(Component)]
pub struct HudCamera;

/// The root UI node of the HUD: it fills the HUD region. A game spawns its HUD widgets as children of this entity
/// (`commands.entity(hud).with_children(..)`), for example in a system that runs after `Startup`.
#[derive(Component)]
pub struct Hud;

/// The behaviour of the HUD widgets: the frames of an `ImageButton` and the fill and hover line of a `Gauge`.
/// `ScreenPlugin` adds it, with the `Hud` root.
pub(crate) struct HudWidgetsPlugin;

impl Plugin for HudWidgetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(press_image_button)
            .add_observer(release_image_button::<PointerRelease>)
            .add_observer(release_image_button::<PointerLeave>)
            .add_observer(release_image_button::<PointerCancel>)
            .add_observer(gauge_hover::<PointerEnter>(true))
            .add_observer(gauge_hover::<PointerLeave>(false))
            .add_systems(Update, fill_gauges);
    }
}

pub(crate) fn spawn_hud(mut commands: Commands) {
    let camera = commands.spawn(hud_camera_bundle()).id();
    commands.spawn((
        Hud,
        UiTargetCamera(camera),
        GlobalZIndex(HUD_Z_INDEX),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            padding: UiRect::all(px(8)),
            ..default()
        },
    ));
}

/// A text label. `font` carries the font face and the size in art pixels.
pub fn label(text: impl Into<String>, font: TextFont, colour: Color) -> impl Bundle {
    (Text::new(text), font, TextColor(colour))
}

/// A button drawn from two tags of an Aseprite file: `up` normally, `down` while a primary press holds it (Bevy's
/// `Pressed` marker is on it then). The kit only swaps the frames: the game observes `PointerClick` on the entity for
/// what a click does, and its children (an icon) are drawn on top.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageButton {
    pub up: &'static str,
    pub down: &'static str,
}

/// An `ImageButton` drawn from `path`, stretched to its node. Add a `Node` with the size, in the same bundle.
pub fn image_button(
    server: &AssetServer,
    path: &'static str,
    up: &'static str,
    down: &'static str,
) -> impl Bundle {
    (
        ImageButton { up, down },
        tag_animation(server, path, up),
        ImageNode::default().with_mode(NodeImageMode::Stretch),
    )
}

fn press_image_button(
    press: On<PointerPress>,
    mut buttons: Query<(&ImageButton, &mut AseAnimation)>,
    mut commands: Commands,
) {
    if press.button != PointerButton::Primary {
        return;
    }
    let entity = press.event_target();
    if let Ok((button, mut ase)) = buttons.get_mut(entity) {
        play_if_changed(&mut ase, button.down, 1.0);
        commands.entity(entity).insert(Pressed);
    }
}

/// The press ends: the button is released, the pointer leaves it, or the press is cancelled.
fn release_image_button<E: EntityEvent>(
    event: On<E>,
    mut buttons: Query<(&ImageButton, &mut AseAnimation)>,
    mut commands: Commands,
) {
    let entity = event.event_target();
    if let Ok((button, mut ase)) = buttons.get_mut(entity) {
        play_if_changed(&mut ase, button.up, 1.0);
        commands.entity(entity).remove::<Pressed>();
    }
}

/// How full a gauge is, from 0 (empty) to 1 (full). The game sets it; values outside are drawn as 0 or 1.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Gauge {
    pub fraction: f32,
}

/// The look of a gauge, in art pixels: a `track` bar of `size` with a `fill` bar over its left part, `click_pad` above
/// and below it that also takes the pointer, and a line of `hover` colour and `hover_height` just below the bar while
/// the pointer is over the gauge (`hover_height` 0 for none). The line takes its height from the pad below.
#[derive(Clone, Copy, Debug)]
pub struct GaugeStyle {
    pub size: Vec2,
    pub track: Color,
    pub fill: Color,
    pub click_pad: f32,
    pub hover: Color,
    pub hover_height: f32,
}

/// The filled part of a gauge.
#[derive(Component)]
struct GaugeFill;

/// The line below a gauge while the pointer is over it, and its colour then.
#[derive(Component)]
struct GaugeHoverLine(Color);

/// A gauge, empty at the start. The game observes `PointerClick` on the entity if a click does something.
pub fn gauge(style: GaugeStyle) -> impl Bundle {
    (
        Gauge { fraction: 0.0 },
        Node {
            width: px(style.size.x),
            flex_direction: FlexDirection::Column,
            padding: UiRect::new(
                px(0),
                px(0),
                px(style.click_pad),
                px((style.click_pad - style.hover_height).max(0.0)),
            ),
            ..default()
        },
        children![
            (
                Node {
                    width: percent(100),
                    height: px(style.size.y),
                    ..default()
                },
                BackgroundColor(style.track),
                children![(
                    GaugeFill,
                    Node {
                        width: percent(0),
                        height: percent(100),
                        ..default()
                    },
                    BackgroundColor(style.fill),
                )],
            ),
            (
                GaugeHoverLine(style.hover),
                Node {
                    width: percent(100),
                    height: px(style.hover_height),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ),
        ],
    )
}

/// Sets the width of the fill of every gauge whose fraction changed.
fn fill_gauges(
    gauges: Query<(Entity, &Gauge), Changed<Gauge>>,
    children: Query<&Children>,
    mut fills: Query<&mut Node, With<GaugeFill>>,
) {
    for (entity, gauge) in &gauges {
        for descendant in children.iter_descendants(entity) {
            if let Ok(mut node) = fills.get_mut(descendant) {
                node.width = percent(gauge.fraction.clamp(0.0, 1.0) * 100.0);
            }
        }
    }
}

type GaugeChildren<'w, 's> = Query<'w, 's, &'static Children, With<Gauge>>;
type HoverLines<'w, 's> = Query<'w, 's, (&'static GaugeHoverLine, &'static mut BackgroundColor)>;

/// An observer that shows (`on`) or hides the hover line of the gauge the pointer enters or leaves.
fn gauge_hover<E: EntityEvent>(
    on: bool,
) -> impl Fn(On<E>, GaugeChildren<'_, '_>, HoverLines<'_, '_>) {
    move |event, gauges, mut lines| {
        let Ok(children) = gauges.get(event.event_target()) else {
            return;
        };
        for child in children {
            if let Ok((line, mut background)) = lines.get_mut(*child) {
                background.0 = if on { line.0 } else { Color::NONE };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::picking::backend::HitData;
    use bevy::picking::events::Pointer;
    use bevy::picking::pointer::{Location, PointerId};
    use bevy_aseprite_ultra::prelude::Aseprite;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Aseprite>()
            .add_plugins(HudWidgetsPlugin);
        app
    }

    fn pointer() -> Pointer {
        let location = Location {
            target: bevy::camera::NormalizedRenderTarget::None {
                width: 1,
                height: 1,
            },
            position: Vec2::ZERO,
        };
        Pointer::new(PointerId::Mouse, location)
    }

    fn hit() -> HitData {
        HitData::new(Entity::PLACEHOLDER, 0.0, None, None)
    }

    fn press(app: &mut App, entity: Entity, button: PointerButton) {
        app.world_mut().trigger(PointerPress {
            entity,
            pointer: pointer(),
            button,
            hit: hit(),
            count: 1,
        });
        app.update();
    }

    /// The tag the button shows and whether it has `Pressed`.
    fn state(app: &App, entity: Entity) -> (String, bool) {
        let world = app.world();
        let tag = world
            .get::<AseAnimation>(entity)
            .unwrap()
            .animation
            .tag
            .clone();
        (
            tag.unwrap_or_default(),
            world.get::<Pressed>(entity).is_some(),
        )
    }

    #[test]
    fn a_primary_press_shows_down_until_release_leave_or_cancel() {
        let mut app = app();
        let server = app.world().resource::<AssetServer>().clone();
        let button = app
            .world_mut()
            .spawn(image_button(&server, "b.aseprite", "rest", "held"))
            .id();
        press(&mut app, button, PointerButton::Secondary);
        press(&mut app, button, PointerButton::Middle);
        assert_eq!(state(&app, button), ("rest".into(), false));
        // Each way a press ends puts the button back up.
        let ends: [fn(&mut World, Entity); 3] = [
            |world, entity| {
                world.trigger(PointerRelease {
                    entity,
                    pointer: pointer(),
                    button: PointerButton::Primary,
                    hit: hit(),
                })
            },
            |world, entity| {
                world.trigger(PointerLeave {
                    entity,
                    pointer: pointer(),
                    hit: hit(),
                    was_in_bounds: true,
                })
            },
            |world, entity| {
                world.trigger(PointerCancel {
                    entity,
                    pointer: pointer(),
                    hit: hit(),
                })
            },
        ];
        for (index, end) in ends.iter().enumerate() {
            press(&mut app, button, PointerButton::Primary);
            assert_eq!(state(&app, button), ("held".into(), true), "end {index}");
            end(app.world_mut(), button);
            app.update();
            assert_eq!(state(&app, button), ("rest".into(), false), "end {index}");
        }
    }

    fn style() -> GaugeStyle {
        GaugeStyle {
            size: Vec2::new(90.0, 7.0),
            track: Color::BLACK,
            fill: Color::srgb(0.2, 0.4, 0.6),
            click_pad: 5.0,
            hover: Color::srgb(0.9, 0.8, 0.7),
            hover_height: 3.0,
        }
    }

    fn find<C: Component>(app: &mut App) -> Entity {
        app.world_mut()
            .query_filtered::<Entity, With<C>>()
            .single(app.world())
            .unwrap()
    }

    #[test]
    fn the_fill_follows_the_fraction_and_stays_inside_the_track() {
        let mut app = app();
        let gauge = app.world_mut().spawn(gauge(style())).id();
        app.update();
        let fill = find::<GaugeFill>(&mut app);
        let width = |app: &App| app.world().get::<Node>(fill).unwrap().width;
        assert_eq!(width(&app), percent(0));
        // Fractions that are exact in f32 after the multiplication by 100.
        for (fraction, expected) in [(0.25, 25.0), (1.5, 100.0), (-0.5, 0.0), (0.75, 75.0)] {
            app.world_mut().get_mut::<Gauge>(gauge).unwrap().fraction = fraction;
            app.update();
            assert_eq!(width(&app), percent(expected), "{fraction}");
        }
    }

    #[test]
    fn the_hover_line_shows_its_colour_while_the_pointer_is_over_the_gauge() {
        let mut app = app();
        let gauge = app.world_mut().spawn(gauge(style())).id();
        app.update();
        let line = find::<GaugeHoverLine>(&mut app);
        let colour = |app: &App| app.world().get::<BackgroundColor>(line).unwrap().0;
        assert_eq!(colour(&app), Color::NONE);
        app.world_mut().trigger(PointerEnter {
            entity: gauge,
            pointer: pointer(),
            hit: hit(),
            is_in_bounds: true,
        });
        assert_eq!(colour(&app), style().hover);
        app.world_mut().trigger(PointerLeave {
            entity: gauge,
            pointer: pointer(),
            hit: hit(),
            was_in_bounds: true,
        });
        assert_eq!(colour(&app), Color::NONE);
    }
}
