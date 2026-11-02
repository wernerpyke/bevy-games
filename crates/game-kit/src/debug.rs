//! Reusable visual debug tools. F2 toggles the grid, F3 toggles the info box.
//! The collision overlay lives in `collision_debug.rs` and follows F2.
//!
//! The grid and the info box follow the camera (`Camera2d` with an orthographic projection), so they work for a fixed
//! or a scrolling view. The info box reports on the entity with the `Controlled` component.
//!
//! To show a value in the info box from any system, take `ResMut<DebugInfo>` and call `set`.

use std::ops::RangeInclusive;

use bevy::camera::Projection;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::Controlled;
use crate::depth::DepthSort;
use crate::layers::{GRID_DEBUG_Z, HUD_Z_INDEX};

const LINE_THICKNESS: f32 = 2.0;
const LABEL_FONT_SIZE: f32 = 20.0;
const LABEL_MARGIN: f32 = 3.0;
const INFO_FONT_SIZE: f32 = 16.0;

/// Where grid coordinates (0, 0) are.
/// Options not used by a game yet stay available for configuration.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GridOrigin {
    /// World origin.
    Center,
    /// Bottom left corner of the visible area. The width of the view follows the window aspect ratio, so this
    /// corner moves in world units when the window is resized or the camera moves.
    BottomLeft,
    /// A fixed world position.
    World(Vec2),
}

impl GridOrigin {
    /// World position of (0, 0), given the visible area in world units.
    fn resolve(self, visible: Rect) -> Vec2 {
        match self {
            GridOrigin::Center => Vec2::ZERO,
            GridOrigin::BottomLeft => visible.min,
            GridOrigin::World(position) => position,
        }
    }
}

pub struct DebugPlugin {
    /// Grid cell size in world units.
    pub cell_size: f32,
    pub grid_color: Color,
    pub grid_origin: GridOrigin,
    /// Name in the info box rows for the `Controlled` entity, for example "guy" gives "guy x/y".
    pub target_name: &'static str,
    /// Draw order of the grid. Defaults to the kit layer (`layers.rs`, adr-x14).
    pub grid_z: f32,
    /// `GlobalZIndex` of the info box. Defaults to the kit layer.
    pub hud_z_index: i32,
}

impl DebugPlugin {
    /// A plugin with the default target name "player" and the kit layers.
    pub fn new(cell_size: f32, grid_color: Color, grid_origin: GridOrigin) -> Self {
        DebugPlugin {
            cell_size,
            grid_color,
            grid_origin,
            target_name: "player",
            grid_z: GRID_DEBUG_Z,
            hud_z_index: HUD_Z_INDEX,
        }
    }
}

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugSettings {
            cell_size: self.cell_size,
            grid_color: self.grid_color,
            grid_origin: self.grid_origin,
            target_name: self.target_name,
            grid_z: self.grid_z,
            hud_z_index: self.hud_z_index,
            show_grid: true,
            show_info: true,
        })
        .init_resource::<DebugInfo>()
        .add_systems(Startup, (spawn_grid_root, spawn_info_box))
        .add_systems(
            Update,
            (
                toggle_tools,
                rebuild_grid,
                pin_grid_labels,
                report_target,
                update_info_box,
            )
                .chain(),
        );
    }
}

#[derive(Resource)]
pub struct DebugSettings {
    pub cell_size: f32,
    pub grid_color: Color,
    pub grid_origin: GridOrigin,
    pub target_name: &'static str,
    pub grid_z: f32,
    pub hud_z_index: i32,
    pub show_grid: bool,
    pub show_info: bool,
}

/// Ordered label and value rows shown in the info box.
#[derive(Resource, Default)]
pub struct DebugInfo {
    rows: Vec<(String, String)>,
}

impl DebugInfo {
    /// Sets the value for a label. A new label is added at the end, so rows keep the order of first use.
    pub fn set(&mut self, label: impl Into<String>, value: impl Into<String>) {
        let label = label.into();
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

/// A column number. It sits at world `x` and is kept on the top edge of the view.
#[derive(Component)]
struct ColumnLabel {
    x: f32,
}

/// A row number. It sits at world `y` and is kept on the left edge of the view.
#[derive(Component)]
struct RowLabel {
    y: f32,
}

#[derive(Component)]
struct InfoBox;

fn spawn_grid_root(mut commands: Commands) {
    commands.spawn((GridRoot, Transform::default(), Visibility::default()));
}

fn spawn_info_box(mut commands: Commands, settings: Res<DebugSettings>) {
    commands.spawn((
        InfoBox,
        GlobalZIndex(settings.hud_z_index),
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

/// The part of the world the camera shows. `None` for a camera that is not orthographic.
fn visible_rect(camera: &GlobalTransform, projection: &Projection) -> Option<Rect> {
    let Projection::Orthographic(orthographic) = projection else {
        return None;
    };
    let area = orthographic.area;
    if area.width() <= 0.0 || area.height() <= 0.0 {
        return None;
    }
    Some(Rect::from_center_size(
        camera.translation().truncate() + area.center(),
        area.size(),
    ))
}

/// Numbers of the grid lines that fall inside the visible range on one axis.
/// `origin` is the world position of line 0, `low` and `high` are the visible range.
fn line_range(origin: f32, low: f32, high: f32, cell: f32) -> RangeInclusive<i32> {
    let first = ((low - origin) / cell).ceil() as i32;
    let last = ((high - origin) / cell).floor() as i32;
    first..=last
}

/// The lines and origin the grid was last built for.
type GridKey = (RangeInclusive<i32>, RangeInclusive<i32>, Vec2);

/// Lines are thin sprites, not gizmos, because gizmos always draw on top of everything.
/// Column numbers run along the top edge, row numbers along the left edge (`pin_grid_labels`).
/// Rebuilt only when the set of visible lines or the origin changes, so a scrolling camera rebuilds it when it
/// crosses a line.
fn rebuild_grid(
    mut commands: Commands,
    settings: Res<DebugSettings>,
    camera: Single<(&GlobalTransform, &Projection), With<Camera2d>>,
    root: Single<(Entity, &mut Visibility), With<GridRoot>>,
    mut last: Local<Option<GridKey>>,
) {
    let (root, mut visibility) = root.into_inner();
    *visibility = if settings.show_grid {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let (transform, projection) = camera.into_inner();
    let Some(visible) = visible_rect(transform, projection) else {
        return;
    };
    let origin = settings.grid_origin.resolve(visible);
    let cell = settings.cell_size;
    let columns = line_range(origin.x, visible.min.x, visible.max.x, cell);
    let rows = line_range(origin.y, visible.min.y, visible.max.y, cell);
    let key = (columns.clone(), rows.clone(), origin);
    if last.as_ref() == Some(&key) {
        return;
    }
    *last = Some(key);

    // A line runs one cell past the outermost line on the other axis, which covers the whole view until the set of
    // lines changes.
    let x_from = origin.x + (*columns.start() - 1) as f32 * cell;
    let x_to = origin.x + (*columns.end() + 1) as f32 * cell;
    let y_from = origin.y + (*rows.start() - 1) as f32 * cell;
    let y_to = origin.y + (*rows.end() + 1) as f32 * cell;
    let color = settings.grid_color;
    let z = settings.grid_z;
    let font = TextFont::from_font_size(LABEL_FONT_SIZE);
    commands
        .entity(root)
        .despawn_children()
        .with_children(|parent| {
            for i in columns {
                let x = origin.x + i as f32 * cell;
                parent.spawn((
                    Sprite::from_color(color, Vec2::new(LINE_THICKNESS, y_to - y_from)),
                    Transform::from_xyz(x, (y_from + y_to) / 2.0, z),
                ));
                parent.spawn((
                    ColumnLabel { x },
                    Text2d::new(i.to_string()),
                    font.clone(),
                    TextColor(color),
                    Anchor::TOP_LEFT,
                    Transform::from_xyz(x + LABEL_MARGIN, visible.max.y - LABEL_MARGIN, z),
                ));
            }
            for j in rows {
                let y = origin.y + j as f32 * cell;
                parent.spawn((
                    Sprite::from_color(color, Vec2::new(x_to - x_from, LINE_THICKNESS)),
                    Transform::from_xyz((x_from + x_to) / 2.0, y, z),
                ));
                parent.spawn((
                    RowLabel { y },
                    Text2d::new(j.to_string()),
                    font.clone(),
                    TextColor(color),
                    Anchor::BOTTOM_LEFT,
                    Transform::from_xyz(visible.min.x + LABEL_MARGIN, y + LABEL_MARGIN, z),
                ));
            }
        });
}

/// Keeps the column numbers on the top edge and the row numbers on the left edge of the view while the camera moves.
fn pin_grid_labels(
    camera: Single<(&GlobalTransform, &Projection), With<Camera2d>>,
    mut columns: Query<(&ColumnLabel, &mut Transform), Without<RowLabel>>,
    mut rows: Query<(&RowLabel, &mut Transform), Without<ColumnLabel>>,
) {
    let (transform, projection) = camera.into_inner();
    let Some(visible) = visible_rect(transform, projection) else {
        return;
    };
    for (label, mut transform) in &mut columns {
        transform.translation.x = label.x + LABEL_MARGIN;
        transform.translation.y = visible.max.y - LABEL_MARGIN;
    }
    for (label, mut transform) in &mut rows {
        transform.translation.x = visible.min.x + LABEL_MARGIN;
        transform.translation.y = label.y + LABEL_MARGIN;
    }
}

/// The feet of the `Controlled` entity are the point collision uses, so they are shown instead of the sprite centre.
/// "<name> x/y" and "<name> cell" use grid coordinates. "<name> world" is the raw world position.
fn report_target(
    target: Single<(&Transform, &DepthSort), With<Controlled>>,
    camera: Single<(&GlobalTransform, &Projection), With<Camera2d>>,
    settings: Res<DebugSettings>,
    mut info: ResMut<DebugInfo>,
) {
    let (camera_transform, projection) = camera.into_inner();
    let Some(visible) = visible_rect(camera_transform, projection) else {
        return;
    };
    let (transform, sort) = target.into_inner();
    let world = Vec2::new(
        transform.translation.x,
        sort.feet_y(transform.translation.y),
    );
    let origin = settings.grid_origin.resolve(visible);
    let grid = world - origin;
    let cell = (grid / settings.cell_size).floor();
    let name = settings.target_name;
    info.set(
        format!("{name} x/y"),
        format!("{:.0}, {:.0}", grid.x, grid.y),
    );
    info.set(format!("{name} cell"), format!("{}, {}", cell.x, cell.y));
    info.set(
        format!("{name} world"),
        format!("{:.0}, {:.0}", world.x, world.y),
    );
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
        assert_eq!(line_range(0.0, -200.0, 200.0, 64.0), -3..=3);
        assert_eq!(line_range(0.0, -10.0, 10.0, 64.0), 0..=0);
    }

    #[test]
    fn line_range_bottom_left_origin_starts_at_zero() {
        // Visible height 1200, origin at the bottom edge: lines 0 to 18 (1152 of 1200).
        assert_eq!(line_range(-600.0, -600.0, 600.0, 64.0), 0..=18);
    }

    #[test]
    fn line_range_follows_a_camera_that_moved() {
        // Visible x from 1000 to 1400, origin 0: lines 16 to 21 (1024 to 1344).
        assert_eq!(line_range(0.0, 1000.0, 1400.0, 64.0), 16..=21);
    }

    #[test]
    fn bottom_left_origin_is_the_bottom_left_of_the_visible_area() {
        let visible = Rect::new(-960.0, -600.0, 960.0, 600.0);
        assert_eq!(
            GridOrigin::BottomLeft.resolve(visible),
            Vec2::new(-960.0, -600.0)
        );
        assert_eq!(GridOrigin::Center.resolve(visible), Vec2::ZERO);
        assert_eq!(
            GridOrigin::World(Vec2::new(5.0, 6.0)).resolve(visible),
            Vec2::new(5.0, 6.0)
        );
    }

    #[test]
    fn the_visible_rect_follows_the_camera_position() {
        let projection = Projection::Orthographic(OrthographicProjection {
            area: Rect::new(-100.0, -50.0, 100.0, 50.0),
            ..OrthographicProjection::default_2d()
        });
        let camera = GlobalTransform::from_xyz(30.0, -20.0, 0.0);
        assert_eq!(
            visible_rect(&camera, &projection),
            Some(Rect::new(-70.0, -70.0, 130.0, 30.0))
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
