//! Map items that do not block and that the player collects by touching them. The game spawns the entity with a
//! `Collectable` and a `MapId` and reads the `Collected` message. The kit knows no item kinds: `kind` is a name from
//! the map legend. See `dev-docs/extensions/collectables.md`.

use bevy::prelude::*;

use crate::Controlled;
use crate::depth::DepthSort;
use crate::map_transition::{ActiveMaps, MapId};

/// An item that is collected when the feet of the controlled entity touch `area` (world units, the edges count as
/// inside). The entity is despawned on collection. Add a `MapId` so it goes with its map.
#[derive(Component, Clone, Debug)]
pub struct Collectable {
    pub area: Rect,
    pub kind: String,
}

/// Sent once for each collected item.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct Collected {
    pub kind: String,
}

/// Whether `feet` is inside `area`, edges included.
pub fn touches(area: Rect, feet: Vec2) -> bool {
    feet.cmpge(area.min).all() && feet.cmple(area.max).all()
}

pub struct CollectablePlugin;

impl Plugin for CollectablePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Collected>().add_systems(Update, collect);
    }
}

/// Collects every item of the current map that the feet touch.
fn collect(
    mut commands: Commands,
    active: Res<ActiveMaps>,
    items: Query<(Entity, &Collectable, &MapId)>,
    target: Query<(&Transform, &DepthSort), With<Controlled>>,
    mut collected: MessageWriter<Collected>,
) {
    let (Ok((transform, sort)), Some(current)) = (target.single(), active.current_map()) else {
        return;
    };
    let feet = Vec2::new(
        transform.translation.x,
        sort.feet_y(transform.translation.y),
    );
    for (entity, item, id) in &items {
        if id.0 == current.id && touches(item.area, feet) {
            collected.write(Collected {
                kind: item.kind.clone(),
            });
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feet_inside_or_on_the_edge_touch() {
        let area = Rect::new(0.0, 0.0, 10.0, 20.0);
        assert!(touches(area, Vec2::new(5.0, 5.0)));
        assert!(touches(area, Vec2::new(0.0, 20.0)));
        assert!(touches(area, Vec2::new(10.0, 0.0)));
    }

    #[test]
    fn feet_outside_do_not_touch() {
        let area = Rect::new(0.0, 0.0, 10.0, 20.0);
        assert!(!touches(area, Vec2::new(-0.1, 5.0)));
        assert!(!touches(area, Vec2::new(5.0, 20.1)));
        assert!(!touches(area, Vec2::new(11.0, 21.0)));
    }
}
