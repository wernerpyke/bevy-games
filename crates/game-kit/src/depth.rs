use bevy::prelude::*;

use crate::collision::Blocker;
use crate::layers::WORLD_HALF_BAND;

/// Z distance between one world unit of y and the draw order. Small enough to stay inside the camera range.
const Z_PER_Y: f32 = 0.001;
/// Z distance between one world unit of x, used only to break ties. Two sprites with the same feet y and an
/// equal z have no guaranteed draw order, so overlapping neighbours in a row would flicker. This is far below
/// `Z_PER_Y` even at x = 5000, so it never overrides a difference in y.
const Z_PER_X: f32 = 0.000_000_01;

// The world layer holds feet from y -99000 to 99000 (`layers.rs`).
const _: () = assert!(Z_PER_Y * 99_000.0 < WORLD_HALF_BAND);

/// Sorts every `DepthSort` entity by its feet each frame.
pub struct DepthPlugin {
    /// The world layer spans z from `-world_half_band` to `world_half_band` (`layers.rs`). Feet outside it trip a
    /// debug assertion. A game that changes its layers passes its own band.
    pub world_half_band: f32,
}

impl Default for DepthPlugin {
    fn default() -> Self {
        DepthPlugin {
            world_half_band: WORLD_HALF_BAND,
        }
    }
}

impl Plugin for DepthPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DepthSettings {
            world_half_band: self.world_half_band,
        })
        .add_systems(Update, (sort_at_blocker, sort_by_feet).chain());
        #[cfg(debug_assertions)]
        app.add_systems(Update, warn_sort_below_blocker);
    }
}

/// How far above the bottom of its blocked area an object sorts. A character's feet stop exactly on that bottom, so
/// without the bias the two would tie and the character could be drawn behind the object it stands in front of.
pub const BLOCKER_SORT_BIAS: f32 = 1.0;

/// Sorts a static object at the bottom of its `Blocker` plus `BLOCKER_SORT_BIAS`, so a character standing at it is
/// drawn in front and one whose feet are above that line is drawn behind (`adr-x2`). The kit keeps its `DepthSort` in
/// step with the blocker; the game sets no `feet_offset`.
#[derive(Component, Clone, Copy, Debug, Default)]
#[require(DepthSort)]
pub struct SortAtBlocker;

/// The draw line of an object whose feet may not enter `blocked`.
pub fn sort_line(blocked: Rect) -> f32 {
    blocked.min.y + BLOCKER_SORT_BIAS
}

fn sort_at_blocker(mut query: Query<(&Transform, &Blocker, &mut DepthSort), With<SortAtBlocker>>) {
    for (transform, blocker, mut sort) in &mut query {
        let feet_offset = transform.translation.y - sort_line(blocker.0);
        if sort.feet_offset != feet_offset {
            sort.feet_offset = feet_offset;
        }
    }
}

/// The warning for an object that sorts by hand at `feet_y` with the blocked area `blocked`, when a character standing
/// at its bottom would not be drawn in front of it. `None` when the sort line is above that bottom.
fn sort_below_blocker(feet_y: f32, blocked: Rect) -> Option<String> {
    (feet_y <= blocked.min.y).then(|| {
        format!(
            "sorts at y {feet_y}, at or below the bottom of its blocked area (y {}), so a character standing there can be drawn behind it: use SortAtBlocker",
            blocked.min.y
        )
    })
}

/// An object that has just got a hand-set sort or a blocker.
#[cfg(debug_assertions)]
type NewHandSort = (
    Without<SortAtBlocker>,
    Or<(Added<DepthSort>, Added<Blocker>)>,
);

/// Debug builds only: reports, once, a new object with a `Blocker` and a hand-set `DepthSort` that sorts at or below
/// the bottom of its blocked area.
#[cfg(debug_assertions)]
fn warn_sort_below_blocker(new: Query<(Entity, &Transform, &DepthSort, &Blocker), NewHandSort>) {
    for (entity, transform, sort, blocker) in &new {
        if let Some(message) = sort_below_blocker(sort.feet_y(transform.translation.y), blocker.0) {
            warn!("depth: {entity:?} {message}");
        }
    }
}

#[derive(Resource)]
struct DepthSettings {
    world_half_band: f32,
}

/// Sorts an entity by its feet so it draws behind things whose feet are lower on the screen.
/// `feet_offset` is the distance from the entity's centre down to its feet. An object that blocks uses
/// `SortAtBlocker` instead of setting it.
#[derive(Component, Default)]
pub struct DepthSort {
    pub feet_offset: f32,
}

impl DepthSort {
    pub fn feet_y(&self, centre_y: f32) -> f32 {
        centre_y - self.feet_offset
    }
}

/// Moves the x that breaks a tie between equal feet (`depth_z`) this many world units away from the entity's x. A
/// sprite whose `Transform` is not at the middle of its footprint (one anchored at a corner) uses it, so ties break at
/// the footprint centre like those of a centred sprite, and two objects never tie only because of their anchors.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct TieBreakX(pub f32);

fn sort_by_feet(
    settings: Res<DepthSettings>,
    mut query: Query<(&mut Transform, &DepthSort, Option<&TieBreakX>)>,
) {
    for (mut transform, sort, tie) in &mut query {
        let feet_y = sort.feet_y(transform.translation.y);
        let tie_x = transform.translation.x + tie.map_or(0.0, |tie| tie.0);
        let z = depth_z(tie_x, feet_y);
        debug_assert!(
            z.abs() < settings.world_half_band,
            "feet at y {feet_y} are outside the world layer, see layers.rs"
        );
        transform.translation.z = z;
    }
}

/// Draw order from the feet. Lower feet are in front. At the same feet y, the sprite further right is in front,
/// so the order is fixed and never flickers.
fn depth_z(x: f32, feet_y: f32) -> f32 {
    -feet_y * Z_PER_Y + x * Z_PER_X
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_feet_y_gives_different_z_by_x() {
        assert!(depth_z(64.0, 50.0) > depth_z(0.0, 50.0));
        // Neighbouring rocks 64 apart must differ by more than f32 rounding at this z.
        let z = depth_z(0.0, 50.0);
        assert!(depth_z(64.0, 50.0) - z > z.abs() * f32::EPSILON * 4.0);
    }

    #[test]
    fn a_tie_break_offset_moves_the_x_that_breaks_a_tie_of_feet() {
        let mut app = App::new();
        app.add_plugins(DepthPlugin::default());
        // Three sprites at the same place: the offset alone decides their order.
        let at = Transform::from_xyz(-640.0, -256.0, 0.0);
        let sort = || DepthSort { feet_offset: 63.0 };
        let plain = app.world_mut().spawn((at, sort())).id();
        let right = app.world_mut().spawn((at, sort(), TieBreakX(64.0))).id();
        let left = app.world_mut().spawn((at, sort(), TieBreakX(-32.0))).id();
        app.update();
        let z = |entity| app.world().get::<Transform>(entity).unwrap().translation.z;
        assert_eq!(z(plain), depth_z(-640.0, -319.0));
        assert_eq!(z(right), depth_z(-576.0, -319.0));
        assert_eq!(z(left), depth_z(-672.0, -319.0));
        assert!(z(left) < z(plain) && z(plain) < z(right));
    }

    #[test]
    fn an_object_sorts_just_above_its_blocked_bottom_so_a_character_standing_there_left_of_it_is_in_front()
     {
        let mut app = App::new();
        app.add_plugins(DepthPlugin::default());
        // The object's centre is not on its blocked area, so the line comes from the blocker alone.
        let object = app
            .world_mut()
            .spawn((
                SortAtBlocker,
                Blocker(Rect::new(-70.0, 12.5, 30.0, 40.0)),
                Transform::from_xyz(-20.0, 90.0, 0.0),
            ))
            .id();
        // A character's feet stop on the blocked bottom, left of the object, where a tie would put it behind.
        let character = app
            .world_mut()
            .spawn((
                DepthSort { feet_offset: 25.0 },
                Transform::from_xyz(-60.0, 37.5, 0.0),
            ))
            .id();
        app.update();
        let z = |app: &App, entity| app.world().get::<Transform>(entity).unwrap().translation.z;
        assert_eq!(z(&app, object), depth_z(-20.0, 13.5));
        assert!(z(&app, character) > z(&app, object));
        // A new blocker moves the line with it.
        app.world_mut()
            .entity_mut(object)
            .insert(Blocker(Rect::new(-70.0, -8.0, 30.0, 40.0)));
        app.update();
        assert_eq!(z(&app, object), depth_z(-20.0, -7.0));
    }

    #[test]
    fn a_hand_set_sort_at_or_below_the_blocked_bottom_is_reported_and_one_above_it_is_not() {
        let blocked = Rect::new(0.0, -16.0, 48.0, 8.0);
        assert_eq!(
            sort_below_blocker(-16.0, blocked).as_deref(),
            Some(
                "sorts at y -16, at or below the bottom of its blocked area (y -16), so a character standing there can be drawn behind it: use SortAtBlocker"
            )
        );
        assert!(
            sort_below_blocker(-20.0, blocked).is_some_and(|m| m.starts_with("sorts at y -20,"))
        );
        assert_eq!(sort_below_blocker(-15.5, blocked), None);
    }

    #[test]
    fn x_never_beats_a_difference_in_y() {
        // One unit of y lower is in front, even with 5000 units of x against it.
        assert!(depth_z(-5000.0, 49.0) > depth_z(5000.0, 50.0));
    }
}
