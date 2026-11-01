//! Reusable visual debug tools. F2 toggles the grid, F3 toggles the info box.
//! The collision overlay (F1) lives in `collision.rs`.
//!
//! To show a value in the info box from any system, take `ResMut<DebugInfo>` and call `set`.

use std::ops::RangeInclusive;

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::camera::half_view;
use crate::depth::DepthSort;
use crate::player::Player;

/// Draw order of the grid. Depth-sorted entities stay within about -1 to 1, so the grid is below
/// them. A background must be placed below this value to stay under the grid.
pub const GRID_Z: f32 = -10.0;
const LINE_THICKNESS: f32 = 2.0;
const LABEL_FONT_SIZE: f32 = 20.0;
const LABEL_MARGIN: f32 = 3.0;
const INFO_FONT_SIZE: f32 = 16.0;

/// Where grid coordinates (0, 0) are.
/// Options not used by the game yet stay available for configuration.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GridOrigin {
    /// World origin, the centre of the screen.
    Center,
    /// Bottom left corner of the visible area. The width of the view follows the window aspect
    /// ratio, so this corner moves sideways in world units when the window is resized.
    BottomLeft,
    /// A fixed world position.
    World(Vec2),
}

impl GridOrigin {
    /// World position of (0, 0), given half the visible area.
    fn resolve(self, half_view: Vec2) -> Vec2 {
        match self {
            GridOrigin::Center => Vec2::ZERO,
            GridOrigin::BottomLeft => -half_view,
            GridOrigin::World(position) => position,
        }
    }
}

pub struct DebugPlugin {
    /// Grid cell size in world units.
    pub cell_size: f32,
    pub grid_color: Color,
    pub grid_origin: GridOrigin,
}

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugSettings {
            cell_size: self.cell_size,
            grid_color: self.grid_color,
            grid_origin: self.grid_origin,
            show_grid: true,
            show_info: true,
        })
        .init_resource::<DebugInfo>()
        .add_systems(Startup, (spawn_grid_root, spawn_info_box))
        .add_systems(
            Update,
            (toggle_tools, rebuild_grid, report_player, update_info_box).chain(),
        );
    }
}

#[derive(Resource)]
pub struct DebugSettings {
    pub cell_size: f32,
    pub grid_color: Color,
    pub grid_origin: GridOrigin,
    pub show_grid: bool,
    pub show_info: bool,
}

/// Ordered label and value rows shown in the info box.
#[derive(Resource, Default)]
pub struct DebugInfo {
    rows: Vec<(&'static str, String)>,
}

impl DebugInfo {
    /// Sets the value for a label. A new label is added at the end, so rows keep the order of first use.
    pub fn set(&mut self, label: &'static str, value: impl Into<String>) {
        let value = value.into();
        match self.rows.iter_mut().find(|(l, _)| *l == label) {
            Some(row) => row.1 = value,
            None => self.rows.push((label, value)),
        }
    }

    fn text(&self) -> String {
        self.rows
            .iter()
            .map(|(label, value)| format!("{label}: {value}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Parent of the grid lines and labels, so they hide and show as one.
#[derive(Component)]
struct GridRoot;

#[derive(Component)]
struct InfoBox;

fn spawn_grid_root(mut commands: Commands) {
    commands.spawn((GridRoot, Transform::default(), Visibility::default()));
}

fn spawn_info_box(mut commands: Commands) {
    commands.spawn((
        InfoBox,
        Text::new(""),
        TextFont::from_font_size(INFO_FONT_SIZE),
        TextColor(Color::WHITE),
        BackgroundColor(Color::BLACK.with_alpha(0.6)),
        Node {
            position_type: PositionType::Absolute,
            top: px(8),
            left: px(8),
            padding: UiRect::all(px(6)),
            ..default()
        },
    ));
}

fn toggle_tools(input: Res<ButtonInput<KeyCode>>, mut settings: ResMut<DebugSettings>) {
    if input.just_pressed(KeyCode::F2) {
        settings.show_grid = !settings.show_grid;
    }
    if input.just_pressed(KeyCode::F3) {
        settings.show_info = !settings.show_info;
    }
}

/// Numbers of the grid lines that fall inside the visible range on one axis.
/// `origin` is the world position of line 0 and `half` is half the visible size.
fn line_range(origin: f32, half: f32, cell: f32) -> RangeInclusive<i32> {
    let first = ((-half - origin) / cell).ceil() as i32;
    let last = ((half - origin) / cell).floor() as i32;
    first..=last
}

/// Lines are thin sprites, not gizmos, because gizmos always draw on top of everything.
/// Column numbers run along the top edge, row numbers along the left edge.
/// Rebuilt only when the visible area or the origin changes.
fn rebuild_grid(
    mut commands: Commands,
    settings: Res<DebugSettings>,
    window: Single<&Window>,
    root: Single<(Entity, &mut Visibility), With<GridRoot>>,
    mut last: Local<Option<(Vec2, Vec2)>>,
) {
    let (root, mut visibility) = root.into_inner();
    *visibility = if settings.show_grid {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if window.height() <= 0.0 {
        return;
    }
    let half = half_view(&window);
    let origin = settings.grid_origin.resolve(half);
    if *last == Some((half, origin)) {
        return;
    }
    *last = Some((half, origin));

    let cell = settings.cell_size;
    let color = settings.grid_color;
    let font = TextFont::from_font_size(LABEL_FONT_SIZE);
    commands
        .entity(root)
        .despawn_children()
        .with_children(|parent| {
            for i in line_range(origin.x, half.x, cell) {
                let x = origin.x + i as f32 * cell;
                parent.spawn((
                    Sprite::from_color(color, Vec2::new(LINE_THICKNESS, half.y * 2.0)),
                    Transform::from_xyz(x, 0.0, GRID_Z),
                ));
                parent.spawn((
                    Text2d::new(i.to_string()),
                    font.clone(),
                    TextColor(color),
                    Anchor::TOP_LEFT,
                    Transform::from_xyz(x + LABEL_MARGIN, half.y - LABEL_MARGIN, GRID_Z),
                ));
            }
            for j in line_range(origin.y, half.y, cell) {
                let y = origin.y + j as f32 * cell;
                parent.spawn((
                    Sprite::from_color(color, Vec2::new(half.x * 2.0, LINE_THICKNESS)),
                    Transform::from_xyz(0.0, y, GRID_Z),
                ));
                parent.spawn((
                    Text2d::new(j.to_string()),
                    font.clone(),
                    TextColor(color),
                    Anchor::BOTTOM_LEFT,
                    Transform::from_xyz(-half.x + LABEL_MARGIN, y + LABEL_MARGIN, GRID_Z),
                ));
            }
        });
}

/// Guy's feet are the point collision uses, so they are shown instead of the sprite centre.
/// "guy x/y" and "guy cell" use grid coordinates. "guy world" is the raw world position.
fn report_player(
    player: Single<(&Transform, &DepthSort), With<Player>>,
    window: Single<&Window>,
    settings: Res<DebugSettings>,
    mut info: ResMut<DebugInfo>,
) {
    if window.height() <= 0.0 {
        return;
    }
    let (transform, sort) = player.into_inner();
    let world = Vec2::new(
        transform.translation.x,
        sort.feet_y(transform.translation.y),
    );
    let origin = settings.grid_origin.resolve(half_view(&window));
    let grid = world - origin;
    let cell = (grid / settings.cell_size).floor();
    info.set("guy x/y", format!("{:.0}, {:.0}", grid.x, grid.y));
    info.set("guy cell", format!("{}, {}", cell.x, cell.y));
    info.set("guy world", format!("{:.0}, {:.0}", world.x, world.y));
}

fn update_info_box(
    settings: Res<DebugSettings>,
    info: Res<DebugInfo>,
    info_box: Single<(&mut Text, &mut Visibility), With<InfoBox>>,
) {
    let (mut text, mut visibility) = info_box.into_inner();
    *visibility = if settings.show_info {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let new_text = info.text();
    if text.0 != new_text {
        text.0 = new_text;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_range_centred_origin_has_negative_lines() {
        assert_eq!(line_range(0.0, 200.0, 64.0), -3..=3);
        assert_eq!(line_range(0.0, 10.0, 64.0), 0..=0);
    }

    #[test]
    fn line_range_bottom_left_origin_starts_at_zero() {
        // Visible half height 600, origin at the bottom edge: lines 0 to 18 (1152 of 1200).
        assert_eq!(line_range(-600.0, 600.0, 64.0), 0..=18);
    }

    #[test]
    fn bottom_left_origin_is_the_negated_half_view() {
        let half = Vec2::new(960.0, 600.0);
        assert_eq!(
            GridOrigin::BottomLeft.resolve(half),
            Vec2::new(-960.0, -600.0)
        );
        assert_eq!(GridOrigin::Center.resolve(half), Vec2::ZERO);
        assert_eq!(
            GridOrigin::World(Vec2::new(5.0, 6.0)).resolve(half),
            Vec2::new(5.0, 6.0)
        );
    }

    #[test]
    fn info_keeps_order_and_updates_in_place() {
        let mut info = DebugInfo::default();
        info.set("a", "1");
        info.set("b", "2");
        info.set("a", "3");
        assert_eq!(info.text(), "a: 3\nb: 2");
    }
}
