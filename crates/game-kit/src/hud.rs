//! The HUD: a `Hud` root node on the `HudCamera`, and generic widgets that a game puts inside it. The kit has no
//! game values: the game decides what the HUD shows. See `dev-docs/adr/EXTENSIONS.md` adr-x34.
//!
//! UI is written in art pixels (`screen::ui_scale`), so a node 512 wide is as wide as the game view.

use bevy::prelude::*;

use crate::layers::HUD_Z_INDEX;
use crate::screen::hud_camera_bundle;

/// Marks the camera that draws the HUD. It sits in the `hud` region of the window.
#[derive(Component)]
pub struct HudCamera;

/// The root UI node of the HUD: it fills the HUD region. A game spawns its HUD widgets as children of this entity
/// (`commands.entity(hud).with_children(..)`), for example in a system that runs after `Startup`.
#[derive(Component)]
pub struct Hud;

/// Background colour of the `Hud` root in a bar mode.
#[derive(Resource)]
pub(crate) struct HudBackground(pub Color);

pub(crate) fn spawn_hud(mut commands: Commands, background: Res<HudBackground>) {
    let camera = commands.spawn(hud_camera_bundle()).id();
    commands.spawn((
        Hud,
        UiTargetCamera(camera),
        GlobalZIndex(HUD_Z_INDEX),
        BackgroundColor(background.0),
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
