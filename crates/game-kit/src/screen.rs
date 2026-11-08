//! Screen regions: the window is split into rectangles, each with its own camera and UI root. A game with a HUD bar
//! has a `game` region (the `GameCamera`) and a `hud` region (the `HudCamera`). See `dev-docs/adr/EXTENSIONS.md`
//! adr-x34.
//!
//! The split is a pure function of the window size and the `ScreenMode` (`regions`), in physical pixels, so it is
//! tested without a window. `ScreenPlugin` applies it to the cameras and sets `UiScale`, so UI written in art pixels
//! is as large on screen as the art.

use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, ScalingMode, Viewport};
use bevy::prelude::*;
use bevy::ui::UiScale;
use bevy::window::PrimaryWindow;

use crate::camera::GameCamera;
use crate::hud::HudCamera;

/// How the window is shared between the game and the HUD.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScreenMode {
    /// The game fills the window and there is no HUD.
    Full,
    /// The game fills the window and the HUD is drawn over it. The layout is computed, the HUD root is transparent.
    Overlay,
    /// The HUD bar is above the game. `ratio` is the bar height as a share of the game height (0.5 is half).
    BarTop { ratio: f32 },
    /// The HUD bar is below the game.
    BarBottom { ratio: f32 },
}

/// The rectangles of the window in physical pixels. The origin is the top left corner, as for `Viewport`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenRegions {
    pub full: URect,
    pub game: URect,
    /// `None` when the mode has no HUD.
    pub hud: Option<URect>,
}

/// Splits a window of `window` physical pixels. A bar is the full window width and `ratio` times the game height
/// high, so the game and the bar together fill the window height (the bar is rounded to whole pixels).
pub fn regions(window: UVec2, mode: ScreenMode) -> ScreenRegions {
    let full = URect::new(0, 0, window.x, window.y);
    let bar = |ratio: f32| -> u32 {
        let ratio = ratio.max(0.0);
        ((window.y as f32 * ratio / (1.0 + ratio)).round() as u32).min(window.y)
    };
    match mode {
        ScreenMode::Full => ScreenRegions {
            full,
            game: full,
            hud: None,
        },
        ScreenMode::Overlay => ScreenRegions {
            full,
            game: full,
            hud: Some(full),
        },
        ScreenMode::BarTop { ratio } => {
            let bar = bar(ratio);
            ScreenRegions {
                full,
                game: URect::new(0, bar, window.x, window.y),
                hud: Some(URect::new(0, 0, window.x, bar)),
            }
        }
        ScreenMode::BarBottom { ratio } => {
            let bar = bar(ratio);
            let split = window.y - bar;
            ScreenRegions {
                full,
                game: URect::new(0, 0, window.x, split),
                hud: Some(URect::new(0, split, window.x, window.y)),
            }
        }
    }
}

/// `UiScale` that makes one UI unit as large as one art pixel of the game view: the game region height in logical
/// pixels divided by the view height in art pixels.
pub fn ui_scale(game_height_physical: u32, window_scale_factor: f32, view_height: f32) -> f32 {
    if view_height <= 0.0 || window_scale_factor <= 0.0 {
        return 1.0;
    }
    game_height_physical as f32 / (view_height * window_scale_factor)
}

/// The layout of `AppState::Playing`: `ScreenLayout::mode` is set to this when the game is entered (`app_state.rs`).
/// The other screens use `ScreenMode::Full`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct PlayingLayout(pub ScreenMode);

/// The current screen mode and regions. A game changes `mode` (for example when the app state changes); the plugin
/// recomputes `regions` and the cameras.
#[derive(Resource, Clone, Debug)]
pub struct ScreenLayout {
    pub mode: ScreenMode,
    pub regions: ScreenRegions,
}

/// Splits the window into regions, spawns the `HudCamera` and the `Hud` root, and keeps both cameras and `UiScale`
/// in step with the window. Add `PixelCameraPlugin` as well: the view height of its camera sets the UI scale.
pub struct ScreenPlugin {
    /// The layout while playing. Other screens are `Full` (`app_state.rs`).
    pub mode: ScreenMode,
}

impl Plugin for ScreenPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ScreenLayout {
            mode: self.mode,
            regions: regions(UVec2::ZERO, self.mode),
        })
        .insert_resource(PlayingLayout(self.mode))
        .add_systems(Startup, crate::hud::spawn_hud)
        .add_systems(
            PostUpdate,
            apply_layout.before(bevy::transform::TransformSystems::Propagate),
        );
    }
}

/// Window state that the cameras were last set for.
#[derive(PartialEq)]
struct Applied {
    window: UVec2,
    scale_factor: f32,
    mode: ScreenMode,
}

fn viewport(rect: URect) -> Viewport {
    Viewport {
        physical_position: rect.min,
        physical_size: rect.size(),
        ..default()
    }
}

type GameCameraQuery = (&'static mut Camera, &'static Projection);

#[allow(clippy::too_many_arguments)]
fn apply_layout(
    window: Single<&Window, With<PrimaryWindow>>,
    mut layout: ResMut<ScreenLayout>,
    mut ui: ResMut<UiScale>,
    mut game: Single<GameCameraQuery, (With<GameCamera>, Without<HudCamera>)>,
    mut hud_camera: Single<&mut Camera, (With<HudCamera>, Without<GameCamera>)>,
    mut last: Local<Option<Applied>>,
) {
    let size = window.physical_size();
    if size.x == 0 || size.y == 0 {
        return;
    }
    let now = Applied {
        window: size,
        scale_factor: window.scale_factor(),
        mode: layout.mode,
    };
    if last.as_ref() == Some(&now) {
        return;
    }
    let screen = regions(size, now.mode);
    layout.regions = screen;
    let (game_camera, projection) = &mut *game;
    game_camera.viewport = Some(viewport(screen.game));
    match screen.hud {
        Some(rect) => {
            hud_camera.is_active = true;
            hud_camera.viewport = Some(viewport(rect));
        }
        None => hud_camera.is_active = false,
    }
    if let Projection::Orthographic(orthographic) = &**projection
        && let ScalingMode::FixedVertical { viewport_height } = orthographic.scaling_mode
    {
        ui.0 = ui_scale(screen.game.height(), now.scale_factor, viewport_height);
    }
    *last = Some(now);
}

/// Render layer of the HUD camera. The world is on layer 0, so the HUD camera does not draw world sprites.
pub const HUD_RENDER_LAYER: usize = 1;

/// The components of the `HudCamera`: it only draws UI (no clear, so the game camera's clear stays), above the game
/// camera, on its own render layer.
pub(crate) fn hud_camera_bundle() -> impl Bundle {
    (
        Camera2d,
        HudCamera,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Msaa::Off,
        RenderLayers::layer(HUD_RENDER_LAYER),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: UVec2 = UVec2::new(1440, 2160);

    #[test]
    fn a_bar_above_takes_half_the_game_height_and_the_full_width() {
        let screen = regions(WINDOW, ScreenMode::BarTop { ratio: 0.5 });
        assert_eq!(screen.hud, Some(URect::new(0, 0, 1440, 720)));
        assert_eq!(screen.game, URect::new(0, 720, 1440, 2160));
        assert_eq!(screen.game.height(), 1440);
        assert_eq!(screen.hud.unwrap().height() * 2, screen.game.height());
    }

    #[test]
    fn a_bar_below_sits_under_the_game() {
        let screen = regions(WINDOW, ScreenMode::BarBottom { ratio: 0.5 });
        assert_eq!(screen.game, URect::new(0, 0, 1440, 1440));
        assert_eq!(screen.hud, Some(URect::new(0, 1440, 1440, 2160)));
    }

    #[test]
    fn the_regions_fill_the_window_with_no_gap_at_odd_sizes() {
        for height in [1u32, 7, 999, 1081, 2159] {
            let screen = regions(UVec2::new(640, height), ScreenMode::BarTop { ratio: 0.5 });
            let hud = screen.hud.unwrap();
            assert_eq!(hud.height() + screen.game.height(), height);
            assert_eq!(hud.max.y, screen.game.min.y);
        }
    }

    #[test]
    fn full_and_overlay_give_the_game_the_whole_window() {
        let full = regions(WINDOW, ScreenMode::Full);
        assert_eq!((full.game, full.hud), (full.full, None));
        let overlay = regions(WINDOW, ScreenMode::Overlay);
        assert_eq!(
            (overlay.game, overlay.hud),
            (overlay.full, Some(overlay.full))
        );
    }

    #[test]
    fn a_zero_ratio_gives_an_empty_bar() {
        let screen = regions(WINDOW, ScreenMode::BarTop { ratio: 0.0 });
        assert_eq!(screen.hud.unwrap().height(), 0);
        assert_eq!(screen.game, screen.full);
    }

    #[test]
    fn the_ui_scale_makes_a_ui_unit_as_large_as_an_art_pixel() {
        // Game region 1440 physical pixels high on a 2x screen is 720 logical pixels for 512 art pixels.
        assert_eq!(ui_scale(1440, 2.0, 512.0), 720.0 / 512.0);
        assert_eq!(ui_scale(720, 1.0, 512.0), 720.0 / 512.0);
        assert_eq!(ui_scale(720, 1.0, 0.0), 1.0);
    }

    fn app_with_screen(mode: ScreenMode) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<UiScale>()
            .add_plugins((
                crate::camera::PixelCameraPlugin {
                    view_height: 512.0,
                    clear_color: Color::BLACK,
                },
                ScreenPlugin { mode },
            ));
        app.world_mut().spawn((
            Window {
                resolution: (720, 1080).into(),
                ..default()
            },
            PrimaryWindow,
        ));
        app
    }

    #[test]
    fn the_plugin_gives_each_camera_its_region_and_sets_the_ui_scale() {
        let mut app = app_with_screen(ScreenMode::BarTop { ratio: 0.5 });
        app.update();
        let world = app.world_mut();
        let game = world
            .query_filtered::<&Camera, With<GameCamera>>()
            .single(world)
            .unwrap()
            .viewport
            .clone()
            .unwrap();
        assert_eq!(game.physical_position, UVec2::new(0, 360));
        assert_eq!(game.physical_size, UVec2::new(720, 720));
        let hud = world
            .query_filtered::<&Camera, With<HudCamera>>()
            .single(world)
            .unwrap();
        assert!(hud.is_active);
        let hud = hud.viewport.clone().unwrap();
        assert_eq!(hud.physical_position, UVec2::ZERO);
        assert_eq!(hud.physical_size, UVec2::new(720, 360));
        assert_eq!(world.resource::<UiScale>().0, 720.0 / 512.0);
    }

    #[test]
    fn a_mode_without_a_hud_switches_the_hud_camera_off() {
        let mut app = app_with_screen(ScreenMode::BarTop { ratio: 0.5 });
        app.update();
        app.world_mut().resource_mut::<ScreenLayout>().mode = ScreenMode::Full;
        app.update();
        let world = app.world_mut();
        let hud = world
            .query_filtered::<&Camera, With<HudCamera>>()
            .single(world)
            .unwrap();
        assert!(!hud.is_active);
        let game = world
            .query_filtered::<&Camera, With<GameCamera>>()
            .single(world)
            .unwrap();
        assert_eq!(
            game.viewport.as_ref().unwrap().physical_size,
            UVec2::new(720, 1080)
        );
    }
}
