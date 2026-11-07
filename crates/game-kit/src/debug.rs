//! Reusable visual debug tools. F2 toggles the info box, F3 toggles the grid.
//! The collision overlay lives in `collision_debug.rs` and follows F3.
//!
//! The grid and the info box follow the camera (the `GameCamera`, orthographic), so they work for a fixed
//! or a scrolling view. The info box reports on the entity with the `Controlled` component.
//!
//! To show a value in the info box from any system, take `ResMut<DebugInfo>` and call `set`.
//!
//! Row numbers normally count up from the grid origin. A game with a top-left map file sets
//! `DebugSettings::rows_from_top` so the numbers match the file (0 is the top row).

use std::ops::RangeInclusive;

use bevy::camera::Projection;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::Controlled;
use crate::camera::{GameCamera, visible_rect};
use crate::depth::DepthSort;
use crate::layers::{GRID_DEBUG_Z, HUD_Z_INDEX};

// Sizes in world units, tuned for a cell size of 64 (`DebugSettings::scale` scales them to the game's cell size, so they
// look the same on screen at any art scale).
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
    /// Name in the info box rows for the `Controlled` entity, for example "player" gives "player x/y".
    pub target_name: &'static str,
    /// Draw order of the grid. Defaults to the kit layer (`layers.rs`, adr-x14).
    pub grid_z: f32,
    /// `GlobalZIndex` of the info box. Defaults to the kit layer.
    pub hud_z_index: i32,
    /// Whether the info box and the grid start visible. F2 and F3 toggle them either way. `new` sets it from `debug_requested`.
    pub start_visible: bool,
}

/// The environment variable that turns the debug tools on. `rust.sh run <game> --debug` sets it.
pub const DEBUG_VARIABLE: &str = "GAME_KIT_DEBUG";

/// True when the run was started with `--debug` (`GAME_KIT_DEBUG` is set and is not `0`). A web build has no
/// environment, so it is always false there.
pub fn debug_requested() -> bool {
    std::env::var_os(DEBUG_VARIABLE).is_some_and(|value| value != "0")
}

impl DebugPlugin {
    /// A plugin with the default target name "player" and the kit layers. The tools start visible only with `--debug`.
    pub fn new(cell_size: f32, grid_color: Color, grid_origin: GridOrigin) -> Self {
        DebugPlugin {
            cell_size,
            grid_color,
            grid_origin,
            target_name: "player",
            grid_z: GRID_DEBUG_Z,
            hud_z_index: HUD_Z_INDEX,
            start_visible: debug_requested(),
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
            rows_from_top: None,
            show_grid: self.start_visible,
            show_info: self.start_visible,
        })
        .init_resource::<DebugInfo>()
        .init_resource::<DebugGrids>()
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
    /// When set, row numbers count down from the top of a map with this many rows (0 is the top row), as in a map
    /// file with `origin: top-left`. When `None`, rows count up from the grid origin. Only the numbers change. The
    /// lines stay where they are.
    pub rows_from_top: Option<i32>,
    pub show_grid: bool,
    pub show_info: bool,
}

impl DebugSettings {
    /// The number shown for the cell row `row`, counted up from the grid origin.
    pub fn row_number(&self, row: i32) -> i32 {
        match self.rows_from_top {
            Some(rows) => rows - 1 - row,
            None => row,
        }
    }

    /// How much to scale the debug sizes: the cell size over 64, the cell size they are tuned for.
    pub fn scale(&self) -> f32 {
        self.cell_size / 64.0
    }
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

/// A column number. It sits at world `x` and is kept on the top edge of the view, but not above `top`, the top edge of
/// the map it belongs to (infinite for the one global grid).
#[derive(Component)]
struct ColumnLabel {
    x: f32,
    top: f32,
}

/// A row number. It sits at world `y` and is kept on the left edge of the view, but not left of `left`, the left edge
/// of the map it belongs to (negative infinity for the one global grid).
#[derive(Component)]
struct RowLabel {
    y: f32,
    left: f32,
}

/// The area and origin of one map. With regions set, the grid is drawn once per region: lines inside the region only,
/// and numbers counted from the region origin (`map_transition.rs` sets them from the active maps). Without regions
/// there is one grid from `DebugSettings::grid_origin` over the whole view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridRegion {
    pub origin: Vec2,
    pub rect: Rect,
}

#[derive(Resource, Default, Debug)]
pub struct DebugGrids(pub Vec<GridRegion>);

/// One grid to draw: its origin, and the map area it is clipped to (`None` for the global grid).
#[derive(Clone, Copy, Debug, PartialEq)]
struct GridPart {
    origin: Vec2,
    clip: Option<Rect>,
}

fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.min.x < b.max.x && b.min.x < a.max.x && a.min.y < b.max.y && b.min.y < a.max.y
}

/// The grids to draw for the visible area: one per region that is in view, or the global grid.
fn grid_parts(settings: &DebugSettings, grids: &DebugGrids, visible: Rect) -> Vec<GridPart> {
    if grids.0.is_empty() {
        return vec![GridPart {
            origin: settings.grid_origin.resolve(visible),
            clip: None,
        }];
    }
    grids
        .0
        .iter()
        .filter(|region| rects_overlap(region.rect, visible))
        .map(|region| GridPart {
            origin: region.origin,
            clip: Some(region.rect),
        })
        .collect()
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
        settings.show_info = !settings.show_info;
    }
    if input.just_pressed(KeyCode::F3) {
        settings.show_grid = !settings.show_grid;
    }
}

/// Numbers of the grid lines that fall inside the visible range on one axis.
/// `origin` is the world position of line 0, `low` and `high` are the visible range.
fn line_range(origin: f32, low: f32, high: f32, cell: f32) -> RangeInclusive<i32> {
    let first = ((low - origin) / cell).ceil() as i32;
    let last = ((high - origin) / cell).floor() as i32;
    first..=last
}

/// The lines of one grid that were last built: its visible columns and rows, origin and clip area.
type PartKey = (RangeInclusive<i32>, RangeInclusive<i32>, Vec2, Option<Rect>);
/// Everything the grid was last built for.
type GridKey = (Vec<PartKey>, Option<i32>);

/// Lines are thin sprites, not gizmos, because gizmos always draw on top of everything.
/// Column numbers run along the top edge, row numbers along the left edge (`pin_grid_labels`).
/// Rebuilt only when the set of visible lines or the origin changes, so a scrolling camera rebuilds it when it
/// crosses a line. With `DebugGrids` set, every map in view has its own grid, with numbers counted from its own origin.
fn rebuild_grid(
    mut commands: Commands,
    settings: Res<DebugSettings>,
    grids: Res<DebugGrids>,
    camera: Single<(&GlobalTransform, &Projection), With<GameCamera>>,
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
    let cell = settings.cell_size;
    // The visible range of lines of each part, clipped to its map.
    let parts: Vec<(GridPart, RangeInclusive<i32>, RangeInclusive<i32>)> =
        grid_parts(&settings, &grids, visible)
            .into_iter()
            .map(|part| {
                let seen = part.clip.map_or(visible, |clip| {
                    Rect::new(
                        visible.min.x.max(clip.min.x),
                        visible.min.y.max(clip.min.y),
                        visible.max.x.min(clip.max.x),
                        visible.max.y.min(clip.max.y),
                    )
                });
                let columns = line_range(part.origin.x, seen.min.x, seen.max.x, cell);
                let rows = line_range(part.origin.y, seen.min.y, seen.max.y, cell);
                (part, columns, rows)
            })
            .collect();
    let key: GridKey = (
        parts
            .iter()
            .map(|(part, columns, rows)| (columns.clone(), rows.clone(), part.origin, part.clip))
            .collect(),
        settings.rows_from_top,
    );
    if last.as_ref() == Some(&key) {
        return;
    }
    *last = Some(key);

    let color = settings.grid_color;
    let z = settings.grid_z;
    let scale = settings.scale();
    let thickness = LINE_THICKNESS * scale;
    let margin = LABEL_MARGIN * scale;
    let font = TextFont::from_font_size(LABEL_FONT_SIZE * scale);
    commands
        .entity(root)
        .despawn_children()
        .with_children(|parent| {
            for (part, columns, rows) in parts {
                let origin = part.origin;
                // The global grid: a line runs one cell past the outermost line on the other axis, which covers the
                // whole view until the set of lines changes. A map grid: a line covers the whole map.
                let (x_from, x_to, y_from, y_to) = match part.clip {
                    Some(clip) => (clip.min.x, clip.max.x, clip.min.y, clip.max.y),
                    None => (
                        origin.x + (*columns.start() - 1) as f32 * cell,
                        origin.x + (*columns.end() + 1) as f32 * cell,
                        origin.y + (*rows.start() - 1) as f32 * cell,
                        origin.y + (*rows.end() + 1) as f32 * cell,
                    ),
                };
                let top = part.clip.map_or(f32::INFINITY, |clip| clip.max.y);
                let left = part.clip.map_or(f32::NEG_INFINITY, |clip| clip.min.x);
                for i in columns {
                    let x = origin.x + i as f32 * cell;
                    parent.spawn((
                        Sprite::from_color(color, Vec2::new(thickness, y_to - y_from)),
                        Transform::from_xyz(x, (y_from + y_to) / 2.0, z),
                    ));
                    // The line at the right edge of a map has no cell to its right to number.
                    if part.clip.is_some_and(|clip| x >= clip.max.x - 0.5) {
                        continue;
                    }
                    parent.spawn((
                        ColumnLabel { x, top },
                        Text2d::new(i.to_string()),
                        font.clone(),
                        TextColor(color),
                        Anchor::TOP_LEFT,
                        Transform::from_xyz(x + margin, visible.max.y.min(top) - margin, z),
                    ));
                }
                for j in rows {
                    let y = origin.y + j as f32 * cell;
                    parent.spawn((
                        Sprite::from_color(color, Vec2::new(x_to - x_from, thickness)),
                        Transform::from_xyz((x_from + x_to) / 2.0, y, z),
                    ));
                    // The line at the top edge of a map has no row above it to number. It is also the bottom line of
                    // the map above, whose label would sit on top of it.
                    if part.clip.is_some_and(|clip| y >= clip.max.y - 0.5) {
                        continue;
                    }
                    parent.spawn((
                        RowLabel { y, left },
                        Text2d::new(settings.row_number(j).to_string()),
                        font.clone(),
                        TextColor(color),
                        Anchor::BOTTOM_LEFT,
                        Transform::from_xyz(visible.min.x.max(left) + margin, y + margin, z),
                    ));
                }
            }
        });
}

/// Keeps the column numbers on the top edge and the row numbers on the left edge of the view while the camera moves.
fn pin_grid_labels(
    settings: Res<DebugSettings>,
    camera: Single<(&GlobalTransform, &Projection), With<GameCamera>>,
    mut columns: Query<(&ColumnLabel, &mut Transform), Without<RowLabel>>,
    mut rows: Query<(&RowLabel, &mut Transform), Without<ColumnLabel>>,
) {
    let (transform, projection) = camera.into_inner();
    let Some(visible) = visible_rect(transform, projection) else {
        return;
    };
    let margin = LABEL_MARGIN * settings.scale();
    for (label, mut transform) in &mut columns {
        transform.translation.x = label.x + margin;
        transform.translation.y = visible.max.y.min(label.top) - margin;
    }
    for (label, mut transform) in &mut rows {
        transform.translation.x = visible.min.x.max(label.left) + margin;
        transform.translation.y = label.y + margin;
    }
}

/// The feet of the `Controlled` entity are the point collision uses, so they are shown instead of the sprite centre.
/// "<name> x/y" and "<name> cell" use grid coordinates. "<name> world" is the raw world position.
fn report_target(
    target: Single<(&Transform, &DepthSort), With<Controlled>>,
    camera: Single<(&GlobalTransform, &Projection), With<GameCamera>>,
    settings: Res<DebugSettings>,
    grids: Res<DebugGrids>,
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
    // The numbers count from the map that holds the feet, or from the global grid.
    let origin = grids
        .0
        .iter()
        .find(|region| region.rect.contains(world))
        .map_or_else(
            || settings.grid_origin.resolve(visible),
            |region| region.origin,
        );
    let grid = world - origin;
    let cell = (grid / settings.cell_size).floor();
    let name = settings.target_name;
    let (grid_y, cell_y) = match settings.rows_from_top {
        Some(rows) => (
            rows as f32 * settings.cell_size - grid.y,
            settings.row_number(cell.y as i32) as f32,
        ),
        None => (grid.y, cell.y),
    };
    info.set(
        format!("{name} x/y"),
        format!("{:.0}, {:.0}", grid.x, grid_y),
    );
    info.set(format!("{name} cell"), format!("{}, {}", cell.x, cell_y));
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

    fn settings_for_parts() -> DebugSettings {
        DebugSettings {
            cell_size: 64.0,
            grid_color: Color::WHITE,
            grid_origin: GridOrigin::World(Vec2::new(5.0, 6.0)),
            target_name: "t",
            grid_z: 0.0,
            hud_z_index: 0,
            rows_from_top: None,
            show_grid: true,
            show_info: true,
        }
    }

    #[test]
    fn without_regions_there_is_one_global_grid() {
        let visible = Rect::new(0.0, 0.0, 100.0, 100.0);
        let parts = grid_parts(&settings_for_parts(), &DebugGrids::default(), visible);
        assert_eq!(
            parts,
            vec![GridPart {
                origin: Vec2::new(5.0, 6.0),
                clip: None
            }]
        );
    }

    #[test]
    fn each_region_in_view_gets_its_own_grid_with_its_own_origin() {
        let lower = GridRegion {
            origin: Vec2::new(0.0, 0.0),
            rect: Rect::new(0.0, 0.0, 640.0, 640.0),
        };
        let upper = GridRegion {
            origin: Vec2::new(0.0, 640.0),
            rect: Rect::new(0.0, 640.0, 640.0, 1280.0),
        };
        let far = GridRegion {
            origin: Vec2::new(0.0, 5000.0),
            rect: Rect::new(0.0, 5000.0, 640.0, 5640.0),
        };
        let grids = DebugGrids(vec![lower, upper, far]);
        // A view that straddles the seam between the first two maps.
        let visible = Rect::new(100.0, 400.0, 600.0, 900.0);
        let parts = grid_parts(&settings_for_parts(), &grids, visible);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].origin, lower.origin);
        assert_eq!(parts[1].origin, upper.origin);
        assert_eq!(parts[1].clip, Some(upper.rect));
        // A view that only touches the seam from below does not include the upper map.
        let below = Rect::new(100.0, 100.0, 600.0, 640.0);
        assert_eq!(grid_parts(&settings_for_parts(), &grids, below).len(), 1);
    }
    #[test]
    fn info_keeps_order_and_updates_in_place() {
        let mut info = DebugInfo::default();
        info.set("a", "1");
        info.set("b", "2");
        info.set("a", "3");
        assert_eq!(info.text(), "a: 3\nb: 2");
    }

    #[test]
    fn rows_count_down_from_the_top_when_asked() {
        let mut settings = DebugSettings {
            cell_size: 16.0,
            grid_color: Color::WHITE,
            grid_origin: GridOrigin::World(Vec2::ZERO),
            target_name: "t",
            grid_z: 0.0,
            hud_z_index: 0,
            rows_from_top: None,
            show_grid: true,
            show_info: true,
        };
        assert_eq!(settings.row_number(30), 30);
        settings.rows_from_top = Some(60);
        assert_eq!(settings.row_number(0), 59);
        assert_eq!(settings.row_number(59), 0);
        assert_eq!(settings.row_number(30), 29);
    }
}
