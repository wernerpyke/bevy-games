//! Contact between the controlled entity and things that hurt on touch (enemies). An entity with `Touchable` and a
//! `HitBox` sends `ControlledTouched` while its box overlaps the box of the `Controlled` entity. The kit decides
//! nothing else: the game reads the message and chooses the result (lose HP, die, knock back). See
//! `dev-docs/extensions/enemy-movement.md`.

use bevy::prelude::*;

use crate::Controlled;
use crate::hitbox::{HitBox, overlaps};

/// Marks an entity (with a `HitBox`) that the controlled entity can touch. Remove it to stop the messages, for
/// example when the entity dies.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Touchable;

/// The controlled entity overlaps `by`. Sent in every frame in which they overlap, so a game that wants one hit
/// removes `Touchable` from `by` when it reads the message.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlledTouched {
    pub by: Entity,
}

/// The system that sends `ControlledTouched` runs in this set. A system that reads the message runs `.after(TouchSet)`,
/// so it sees a touch in the frame it happens.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct TouchSet;

pub struct TouchPlugin;

impl Plugin for TouchPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ControlledTouched>()
            .add_systems(Update, send_touches.in_set(TouchSet));
    }
}

fn send_touches(
    controlled: Query<(&Transform, &HitBox), With<Controlled>>,
    touchables: Query<(Entity, &Transform, &HitBox), With<Touchable>>,
    mut touched: MessageWriter<ControlledTouched>,
) {
    let Ok((transform, hit_box)) = controlled.single() else {
        return;
    };
    let body = hit_box.rect_at(transform.translation.truncate());
    for (entity, other, other_box) in &touchables {
        if overlaps(body, other_box.rect_at(other.translation.truncate())) {
            touched.write(ControlledTouched { by: entity });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{Collected, collect};

    fn body() -> HitBox {
        HitBox {
            offset: Vec2::new(0.0, 6.0),
            half_size: Vec2::new(4.0, 6.0),
        }
    }

    type Seen = Collected<ControlledTouched>;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(TouchPlugin)
            .init_resource::<Seen>()
            .add_systems(Update, collect::<ControlledTouched>.after(TouchSet));
        app
    }

    fn spawn_player(app: &mut App, x: f32) -> Entity {
        app.world_mut()
            .spawn((Controlled, body(), Transform::from_xyz(x, 100.0, 0.0)))
            .id()
    }

    fn spawn_touchable(app: &mut App, x: f32) -> Entity {
        app.world_mut()
            .spawn((Touchable, body(), Transform::from_xyz(x, 100.0, 0.0)))
            .id()
    }

    #[test]
    fn an_overlapping_touchable_is_reported_in_each_frame() {
        let mut app = app();
        spawn_player(&mut app, 100.0);
        let enemy = spawn_touchable(&mut app, 106.0);
        app.update();
        app.update();
        assert_eq!(
            app.world().resource::<Seen>().0,
            vec![ControlledTouched { by: enemy }; 2]
        );
    }

    #[test]
    fn one_that_only_stands_near_is_not_reported() {
        let mut app = app();
        spawn_player(&mut app, 100.0);
        spawn_touchable(&mut app, 108.0);
        app.update();
        assert!(app.world().resource::<Seen>().0.is_empty());
    }

    #[test]
    fn removing_touchable_stops_the_messages() {
        let mut app = app();
        spawn_player(&mut app, 100.0);
        let enemy = spawn_touchable(&mut app, 100.0);
        app.update();
        app.world_mut().entity_mut(enemy).remove::<Touchable>();
        app.update();
        assert_eq!(app.world().resource::<Seen>().0.len(), 1);
    }

    #[test]
    fn without_a_controlled_entity_nothing_happens() {
        let mut app = app();
        spawn_touchable(&mut app, 100.0);
        app.update();
        assert!(app.world().resource::<Seen>().0.is_empty());
    }
}
