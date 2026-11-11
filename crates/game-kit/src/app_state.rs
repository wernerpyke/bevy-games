//! The app flow: which screen the game is in (`AppState`), and pause inside the game (`PlayState`). The kit knows
//! the states and what changes with them, and nothing about what a title screen shows. See
//! `dev-docs/adr/EXTENSIONS.md` adr-x35 and `dev-docs/extensions/screen-layout.md`.
//!
//! | State | Screen layout | Time |
//! |---|---|---|
//! | `Loading`, `Title`, `GameOver` | `ScreenMode::Full`: the game region is the whole window, the HUD camera is off | runs |
//! | `Playing` + `Running` | the game's layout (`ScreenPlugin::mode`, for example a HUD bar) | runs |
//! | `Playing` + `Paused` | the same layout, dimmed, with an empty `PauseDialog` in the game region | `Time<Virtual>` stopped |
//!
//! A game builds a screen by spawning a UI root with `state_root` (`OnEnter(AppState::Title)`), which is removed when
//! the state is left. Gameplay systems that must not run while paused use `run_if(in_state(PlayState::Running))`.
//! Systems that use `Res<Time>` in `Update` freeze with the virtual clock (movement, the camera glide and the map
//! fade); a system on real time (the UI, a menu) does not.

use bevy::picking::Pickable;
use bevy::prelude::*;

use crate::hud::HudCamera;
use crate::layers::{DIM_Z_INDEX, UI_Z_INDEX};
use crate::screen::{PlayingLayout, ScreenLayout, ScreenMode};

/// Environment variable that skips the title screen once. `./dev-scripts/rust.sh run <game> --skip-title` sets it,
/// and `script` always sets it.
pub const SKIP_TITLE_VARIABLE: &str = "GAME_KIT_SKIP_TITLE";

/// True when the title screen is to be skipped (`SKIP_TITLE_VARIABLE` is set and not `0`).
pub fn skip_title_requested() -> bool {
    std::env::var_os(SKIP_TITLE_VARIABLE).is_some_and(|value| value != "0")
}

/// The screen the game is in.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Loading,
    Title,
    Playing,
    GameOver,
}

/// Pause, while the game is in `AppState::Playing`. It does not exist in the other states.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(AppState = AppState::Playing)]
pub enum PlayState {
    #[default]
    Running,
    Paused,
}

/// Sent by the game when everything is loaded. Leaves `AppState::Loading` for the title, or for the game when the
/// title is skipped.
#[derive(Message)]
pub struct LoadingFinished;

/// The state to start in: `start`, except that a start on the title is the game when the title is skipped.
pub fn initial_state(start: AppState, skip_title: bool) -> AppState {
    if start == AppState::Title && skip_title {
        AppState::Playing
    } else {
        start
    }
}

/// The state after loading.
pub fn after_loading(skip_title: bool) -> AppState {
    initial_state(AppState::Title, skip_title)
}

/// Adds `AppState` and `PlayState`, the layout that goes with each state, and pause. Add it after `ScreenPlugin`
/// to have the layout follow the state. Without `ScreenPlugin` only the states and pause exist.
pub struct AppFlowPlugin {
    /// The first state. A game with a loading screen starts in `Loading`. A game with no other screens starts in
    /// `Playing`.
    pub start: AppState,
    /// Skip the title screen once, at start or after loading. Debug use: `skip_title_requested()` by default.
    pub skip_title: bool,
    /// Key that toggles pause while playing. `None` for no pause key.
    pub pause_key: Option<KeyCode>,
    /// Colour laid over the game and the HUD while paused.
    pub dim: Color,
}

impl AppFlowPlugin {
    pub fn new(start: AppState) -> Self {
        Self {
            start,
            skip_title: skip_title_requested(),
            pause_key: Some(KeyCode::Escape),
            dim: Color::BLACK.with_alpha(0.5),
        }
    }
}

#[derive(Resource)]
struct FlowSettings {
    skip_title: bool,
    pause_key: Option<KeyCode>,
    dim: Color,
}

impl Plugin for AppFlowPlugin {
    fn build(&self, app: &mut App) {
        let initial = initial_state(self.start, self.skip_title);
        // A skip that was used at the start is not used again.
        let skip_left = self.skip_title && !(self.start == AppState::Title);
        app.insert_state(initial)
            .add_sub_state::<PlayState>()
            .add_message::<LoadingFinished>()
            .insert_resource(FlowSettings {
                skip_title: skip_left,
                pause_key: self.pause_key,
                dim: self.dim,
            })
            .add_systems(Update, finish_loading.run_if(in_state(AppState::Loading)))
            .add_systems(Update, toggle_pause.run_if(in_state(AppState::Playing)))
            .add_systems(OnEnter(AppState::Loading), full_layout)
            .add_systems(OnEnter(AppState::Title), (full_layout, skip_title_once))
            .add_systems(OnEnter(AppState::GameOver), full_layout)
            .add_systems(OnEnter(AppState::Playing), playing_layout)
            .add_systems(OnEnter(PlayState::Paused), (freeze_time, spawn_pause_ui))
            .add_systems(OnExit(PlayState::Paused), thaw_time);
    }
}

fn finish_loading(
    mut finished: MessageReader<LoadingFinished>,
    mut settings: ResMut<FlowSettings>,
    mut next: ResMut<NextState<AppState>>,
) {
    if finished.read().next().is_some() {
        next.set(after_loading(settings.skip_title));
        settings.skip_title = false;
    }
}

/// Leaves the title at once when a skip is left over (`skip_title` is true).
fn skip_title_once(mut settings: ResMut<FlowSettings>, mut next: ResMut<NextState<AppState>>) {
    if settings.skip_title {
        settings.skip_title = false;
        next.set(AppState::Playing);
    }
}

fn full_layout(layout: Option<ResMut<ScreenLayout>>) {
    if let Some(mut layout) = layout {
        layout.mode = ScreenMode::Full;
    }
}

fn playing_layout(layout: Option<ResMut<ScreenLayout>>, playing: Option<Res<PlayingLayout>>) {
    if let (Some(mut layout), Some(playing)) = (layout, playing) {
        layout.mode = playing.0;
    }
}

fn toggle_pause(
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<FlowSettings>,
    state: Res<State<PlayState>>,
    mut next: ResMut<NextState<PlayState>>,
) {
    let Some(key) = settings.pause_key else {
        return;
    };
    if keys.just_pressed(key) {
        next.set(match state.get() {
            PlayState::Running => PlayState::Paused,
            PlayState::Paused => PlayState::Running,
        });
    }
}

fn freeze_time(mut time: ResMut<Time<Virtual>>) {
    time.pause();
}

fn thaw_time(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}

/// Empty and centred in the game region while paused: the place for the game's pause dialog. A game fills it
/// (`Added<PauseDialog>`) and gets it removed when pause ends.
#[derive(Component)]
pub struct PauseDialog;

fn dim_layer(colour: Color) -> impl Bundle {
    (
        DespawnOnExit(PlayState::Paused),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            ..default()
        },
        BackgroundColor(colour),
        GlobalZIndex(DIM_Z_INDEX),
        // Nothing below a paused game can be picked.
        Pickable {
            should_block_lower: true,
            is_hoverable: false,
        },
    )
}

fn spawn_pause_ui(
    mut commands: Commands,
    settings: Res<FlowSettings>,
    hud_camera: Option<Single<Entity, With<HudCamera>>>,
) {
    commands.spawn(dim_layer(settings.dim));
    if let Some(camera) = hud_camera {
        commands.spawn((dim_layer(settings.dim), UiTargetCamera(*camera)));
    }
    commands.spawn((
        PauseDialog,
        DespawnOnExit(PlayState::Paused),
        GlobalZIndex(UI_Z_INDEX),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
    ));
}

/// A UI root for a screen of state `state` (a title, a game over screen): it fills the game region, has its own
/// background, is above the HUD, and is removed when the state is left.
pub fn state_root(state: AppState, background: Color) -> impl Bundle {
    (
        DespawnOnExit(state),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            ..default()
        },
        BackgroundColor(background),
        GlobalZIndex(UI_Z_INDEX),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    fn flow(start: AppState, skip_title: bool) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(PlayingLayout(ScreenMode::BarTop { ratio: 0.5 }))
            .insert_resource(ScreenLayout {
                mode: ScreenMode::BarTop { ratio: 0.5 },
                regions: crate::screen::regions(UVec2::new(100, 100), ScreenMode::Full),
            })
            .add_plugins(AppFlowPlugin {
                start,
                skip_title,
                pause_key: Some(KeyCode::Escape),
                dim: Color::BLACK,
            });
        app.update();
        app
    }

    fn state(app: &App) -> AppState {
        *app.world().resource::<State<AppState>>().get()
    }

    fn play_state(app: &App) -> Option<PlayState> {
        app.world()
            .get_resource::<State<PlayState>>()
            .map(|s| *s.get())
    }

    fn mode(app: &App) -> ScreenMode {
        app.world().resource::<ScreenLayout>().mode
    }

    fn press_escape(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::Escape);
        keys.clear();
        app.update();
        app.update();
    }

    fn count<C: Component>(app: &mut App) -> usize {
        let world = app.world_mut();
        world.query::<&C>().iter(world).count()
    }

    /// The camera of each dim layer: `None` for the game camera, else its `UiTargetCamera`.
    fn dim_layers(app: &mut App) -> Vec<Option<Entity>> {
        let world = app.world_mut();
        world
            .query::<(&GlobalZIndex, Option<&UiTargetCamera>)>()
            .iter(world)
            .filter(|(z, _)| z.0 == DIM_Z_INDEX)
            .map(|(_, target)| target.map(|t| t.entity()))
            .collect()
    }

    #[test]
    fn the_start_state_follows_the_skip_switch() {
        assert_eq!(initial_state(AppState::Title, true), AppState::Playing);
        assert_eq!(initial_state(AppState::Title, false), AppState::Title);
        assert_eq!(initial_state(AppState::Loading, true), AppState::Loading);
        assert_eq!(after_loading(true), AppState::Playing);
        assert_eq!(after_loading(false), AppState::Title);
    }

    #[test]
    fn loading_ends_on_the_title_or_on_the_game_when_the_title_is_skipped() {
        for (skip, expected) in [(false, AppState::Title), (true, AppState::Playing)] {
            let mut app = flow(AppState::Loading, skip);
            assert_eq!(state(&app), AppState::Loading);
            app.update();
            assert_eq!(state(&app), AppState::Loading, "it waits for the game");
            app.world_mut().write_message(LoadingFinished);
            app.update();
            app.update();
            assert_eq!(state(&app), expected);
        }
    }

    #[test]
    fn a_skip_is_used_once() {
        let mut app = flow(AppState::Title, true);
        app.update();
        assert_eq!(state(&app), AppState::Playing);
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Title);
        app.update();
        app.update();
        assert_eq!(state(&app), AppState::Title, "the second title is shown");
    }

    #[test]
    fn a_skip_used_by_loading_does_not_skip_a_later_title() {
        let mut app = flow(AppState::Loading, true);
        app.world_mut().write_message(LoadingFinished);
        app.update();
        app.update();
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Title);
        app.update();
        app.update();
        assert_eq!(state(&app), AppState::Title);
    }

    // Kept on purpose: the skip was not used at the start, so it is used on the first title shown later.
    #[test]
    fn a_game_that_starts_playing_with_the_skip_skips_its_first_later_title() {
        let mut app = flow(AppState::Playing, true);
        assert_eq!(state(&app), AppState::Playing);
        for expected in [AppState::Playing, AppState::Title] {
            app.world_mut()
                .resource_mut::<NextState<AppState>>()
                .set(AppState::Title);
            app.update();
            app.update();
            assert_eq!(state(&app), expected);
        }
    }

    #[test]
    fn the_layout_follows_the_state() {
        let mut app = flow(AppState::Title, false);
        assert_eq!(mode(&app), ScreenMode::Full);
        for (next, expected) in [
            (AppState::Playing, ScreenMode::BarTop { ratio: 0.5 }),
            (AppState::GameOver, ScreenMode::Full),
            (AppState::Playing, ScreenMode::BarTop { ratio: 0.5 }),
        ] {
            app.world_mut()
                .resource_mut::<NextState<AppState>>()
                .set(next);
            app.update();
            app.update();
            assert_eq!(mode(&app), expected, "{next:?}");
        }
    }

    #[test]
    fn the_game_starts_running_and_a_screen_root_goes_with_its_state() {
        let mut app = flow(AppState::Playing, false);
        assert_eq!(play_state(&app), Some(PlayState::Running));
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::GameOver);
        app.update();
        app.update();
        assert_eq!(play_state(&app), None);
        app.world_mut()
            .spawn(state_root(AppState::GameOver, Color::BLACK));
        app.update();
        assert_eq!(count::<DespawnOnExit<AppState>>(&mut app), 1);
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Title);
        app.update();
        app.update();
        assert_eq!(count::<DespawnOnExit<AppState>>(&mut app), 0);
    }

    #[test]
    fn pause_stops_the_clock_dims_the_game_and_opens_the_dialog_slot() {
        let mut app = flow(AppState::Playing, false);
        press_escape(&mut app);
        assert_eq!(play_state(&app), Some(PlayState::Paused));
        assert!(app.world().resource::<Time<Virtual>>().is_paused());
        assert_eq!(count::<PauseDialog>(&mut app), 1);
        assert_eq!(dim_layers(&mut app), vec![None]);
        press_escape(&mut app);
        assert_eq!(play_state(&app), Some(PlayState::Running));
        assert!(!app.world().resource::<Time<Virtual>>().is_paused());
        assert_eq!(count::<PauseDialog>(&mut app), 0);
        assert_eq!(dim_layers(&mut app), vec![]);
    }

    #[test]
    fn the_hud_is_dimmed_too_when_there_is_a_hud_camera() {
        let mut app = flow(AppState::Playing, false);
        let hud = app.world_mut().spawn(HudCamera).id();
        press_escape(&mut app);
        let mut layers = dim_layers(&mut app);
        layers.sort();
        assert_eq!(layers, vec![None, Some(hud)]);
    }

    #[test]
    fn leaving_the_game_while_paused_starts_the_clock_again() {
        let mut app = flow(AppState::Playing, false);
        press_escape(&mut app);
        assert!(app.world().resource::<Time<Virtual>>().is_paused());
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::GameOver);
        app.update();
        app.update();
        assert!(!app.world().resource::<Time<Virtual>>().is_paused());
        assert_eq!(count::<PauseDialog>(&mut app), 0);
    }

    #[test]
    fn pause_does_nothing_outside_the_game() {
        let mut app = flow(AppState::Title, false);
        press_escape(&mut app);
        assert_eq!(play_state(&app), None);
        assert!(!app.world().resource::<Time<Virtual>>().is_paused());
    }

    #[test]
    fn without_a_pause_key_the_game_cannot_be_paused() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(AppFlowPlugin {
                start: AppState::Playing,
                skip_title: false,
                pause_key: None,
                dim: Color::BLACK,
            });
        app.update();
        press_escape(&mut app);
        assert_eq!(play_state(&app), Some(PlayState::Running));
    }
}
