//! Seamless joins: the map behind a seamless exit is placed beside the current one, the current map follows the
//! feet, a map out of view is removed, and the camera limits follow.

use super::*;

/// How long a map must be out of view before it is removed, in seconds. A map that was just loaded next to the
/// player is out of view until the camera bounds have eased, and must not go at once.
pub const UNLOAD_AFTER_SECS: f32 = 1.0;

/// Where a map goes so that its entry strip lies on the exit strip of the map it is joined to: it is placed beside
/// that map, on the edge the exit is on, and shifted along the edge until the two strips match. The maps are never
/// turned. `None` when the exit does not lie along exactly one edge.
pub fn seamless_origin(
    from_origin: Vec2,
    from: &Map,
    exit: &MapExit,
    to: &Map,
    entry: &MapEntry,
) -> Option<Vec2> {
    let edge = match exit.area.edges(from.size).as_slice() {
        [edge] => *edge,
        _ => return None,
    };
    let exit_low = from_origin + exit.area.min.as_vec2() * from.cell_size;
    let from_size = from.size.as_vec2() * from.cell_size;
    let to_size = to.size.as_vec2() * to.cell_size;
    let entry_low = entry.area.min.as_vec2() * to.cell_size;
    Some(match edge {
        Edge::Top => Vec2::new(exit_low.x - entry_low.x, from_origin.y + from_size.y),
        Edge::Bottom => Vec2::new(exit_low.x - entry_low.x, from_origin.y - to_size.y),
        Edge::Right => Vec2::new(from_origin.x + from_size.x, exit_low.y - entry_low.y),
        Edge::Left => Vec2::new(from_origin.x - to_size.x, exit_low.y - entry_low.y),
    })
}

/// True when two rectangles share area. Rectangles that only touch along an edge do not overlap.
pub fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.min.x < b.max.x && b.min.x < a.max.x && a.min.y < b.max.y && b.min.y < a.max.y
}

/// Keeps a move inside the union of `areas`. `from` is where the feet were, `to` where they want to go. The result is
/// the point nearest to `to` among the areas that hold `from` and the areas that touch them. A step that is longer than
/// the gap to a neighbour map therefore cannot jump a wall and land in a map that the feet were not next to.
pub fn clamp_to_union(areas: &[Rect], from: Vec2, to: Vec2) -> Vec2 {
    let holds =
        |area: &Rect, point: Vec2| point.cmpge(area.min).all() && point.cmple(area.max).all();
    let touches = |a: &Rect, b: &Rect| {
        a.min.x <= b.max.x && b.min.x <= a.max.x && a.min.y <= b.max.y && b.min.y <= a.max.y
    };
    let here: Vec<&Rect> = areas.iter().filter(|area| holds(area, from)).collect();
    areas
        .iter()
        .filter(|area| here.is_empty() || here.iter().any(|h| touches(h, area)))
        .map(|area| to.clamp(area.min, area.max))
        .min_by(|a, b| a.distance_squared(to).total_cmp(&b.distance_squared(to)))
        .unwrap_or(to)
}

/// Joins the map behind a seamless exit to the one the player is in: places it beside the current map and loads it. The
/// player is not moved, the screen does not fade and the current map does not change until the player crosses the seam.
pub(super) fn begin_seamless(world: &mut World, request: &RequestMapChange) -> Result<(), String> {
    let used = request
        .source
        .as_ref()
        .ok_or("a seamless transition needs the exit that was used")?;
    let (origin, rect, doorway) = {
        let registry = world.resource::<MapRegistry>();
        let active = world.resource::<ActiveMaps>();
        let from_active = active
            .get(&used.from)
            .ok_or_else(|| format!("map `{}` is not active", used.from))?;
        let from = registry
            .get(&used.from)
            .ok_or_else(|| format!("map `{}` is not registered", used.from))?;
        let exit = from
            .map
            .exits
            .iter()
            .find(|exit| exit.name == used.exit)
            .ok_or_else(|| format!("map `{}` has no exit `{}`", used.from, used.exit))?;
        let to = registry
            .get(&request.to)
            .ok_or_else(|| format!("map `{}` is not registered", request.to))?;
        let entry_name = request
            .entry
            .as_ref()
            .ok_or_else(|| "a seamless transition needs an entry".to_string())?;
        let entry = to
            .map
            .entries
            .iter()
            .find(|entry| entry.name == *entry_name)
            .ok_or_else(|| format!("map `{}` has no entry `{entry_name}`", request.to))?;
        let origin = seamless_origin(from_active.origin, &from.map, exit, &to.map, entry)
            .ok_or_else(|| format!("exit `{}` does not lie along one edge", exit.name))?;
        let rect = Rect::from_corners(origin, origin + to.map.size.as_vec2() * to.map.cell_size);
        let edge = exit.area.edges(from.map.size)[0];
        let doorway = Doorway {
            from: used.from.clone(),
            to: request.to.clone(),
            edge,
            area: area_rect(&exit.area, from_active.origin, from.map.cell_size),
        };
        (origin, rect, doorway)
    };
    let already_active = world.resource::<ActiveMaps>().get(&request.to).is_some();
    if !already_active {
        let active = world.resource::<ActiveMaps>();
        if let Some(clash) = active.maps.iter().find(|m| rects_overlap(m.rect, rect)) {
            return Err(format!(
                "map `{}` would lie on top of map `{}`",
                request.to, clash.id
            ));
        }
        let mut active = world.resource_mut::<ActiveMaps>();
        active.maps.push(ActiveMap {
            id: request.to.clone(),
            origin,
            rect,
        });
    }
    {
        let mut active = world.resource_mut::<ActiveMaps>();
        if !active.doorways.contains(&doorway) {
            active.doorways.push(doorway);
        }
    }
    world
        .resource_mut::<MapTransition>()
        .hidden_for
        .remove(&request.to);
    if !already_active {
        run_load(world, &request.to, origin);
        run_rebuild(world);
    }
    info!(
        "maps: `{}` joined to `{}` at {:?}",
        request.to, used.from, origin
    );
    Ok(())
}

pub(super) fn perform_seamless(world: &mut World) {
    let request = {
        let mut transition = world.resource_mut::<MapTransition>();
        match std::mem::take(&mut transition.phase) {
            Phase::Seamless { request } => request,
            other => {
                transition.phase = other;
                return;
            }
        }
    };
    if let Err(message) = begin_seamless(world, &request) {
        error!("maps: cannot join `{}`: {message}", request.to);
    }
    world.resource_mut::<MapTransition>().phase = Phase::Idle;
}

/// The current map is the one that holds the feet. It changes when the player crosses a seam.
pub(super) fn update_current(
    mut active: ResMut<ActiveMaps>,
    mut transition: ResMut<MapTransition>,
    target: Query<(&Transform, &DepthSort), With<Controlled>>,
) {
    if !transition.is_idle() || active.maps.len() < 2 {
        return;
    }
    let Ok((transform, sort)) = target.single() else {
        return;
    };
    let feet = sort.feet(transform);
    let Some(here) = active.map_at(feet).map(|m| m.id.clone()) else {
        return;
    };
    if here != active.current {
        info!("maps: the player crossed into `{here}`");
        active.current = here;
        transition.armed = false;
    }
}

/// Removes a map that is not the current one when it has been out of view for `UNLOAD_AFTER_SECS`.
pub(super) fn unload_hidden(world: &mut World) {
    if world.resource::<ActiveMaps>().maps.len() < 2 {
        return;
    }
    let visible = {
        let mut cameras =
            world.query_filtered::<(&GlobalTransform, &Projection), With<GameCamera>>();
        cameras
            .single(world)
            .ok()
            .and_then(|(transform, projection)| visible_rect(transform, projection))
    };
    let Some(visible) = visible else {
        return;
    };
    let dt = world.resource::<Time>().delta_secs();
    let mut gone: Vec<String> = Vec::new();
    {
        let hidden: Vec<(String, bool)> = {
            let active = world.resource::<ActiveMaps>();
            active
                .maps
                .iter()
                .filter(|m| m.id != active.current)
                .map(|m| (m.id.clone(), !rects_overlap(m.rect, visible)))
                .collect()
        };
        let mut transition = world.resource_mut::<MapTransition>();
        for (id, out_of_view) in hidden {
            if out_of_view {
                let seconds = transition.hidden_for.entry(id.clone()).or_insert(0.0);
                *seconds += dt;
                if *seconds >= UNLOAD_AFTER_SECS {
                    gone.push(id);
                }
            } else {
                transition.hidden_for.remove(&id);
            }
        }
    }
    if gone.is_empty() {
        return;
    }
    for id in &gone {
        unload_entities(world, id);
        let mut active = world.resource_mut::<ActiveMaps>();
        active.maps.retain(|m| m.id != *id);
        active.doorways.retain(|d| d.from != *id && d.to != *id);
        world.resource_mut::<MapTransition>().hidden_for.remove(id);
        info!("maps: `{id}` left the world, it was out of view");
    }
    run_rebuild(world);
}

/// Where the camera limits should be: the current map, so the camera shows only the map the player is in. While the feet
/// are in the exit of the current map that leads into a joined map, the box also holds that map, so the player sees
/// ahead and the sprite is not cut off at the seam. Once the player has crossed into the new map the box is that map
/// alone, and the camera moves on to it: the map that was left goes out of view.
pub fn camera_target(active: &ActiveMaps, feet: Option<Vec2>) -> Rect {
    let mut target = active.current_rect();
    let Some(feet) = feet else {
        return target;
    };
    for door in active.doorways.iter().filter(|d| d.from == active.current) {
        let inside = feet.cmpge(door.area.min).all() && feet.cmple(door.area.max).all();
        if let (true, Some(next)) = (inside, active.get(&door.to)) {
            target = target.union(next.rect);
        }
    }
    target
}

/// Sets the camera limits every frame (`camera_target`). When they change the camera glides to its new place
/// (`CameraSmoothing`), so there is no jump.
pub(super) fn retarget_camera(
    active: Res<ActiveMaps>,
    bounds: Option<ResMut<CameraBounds>>,
    player: Query<(&Transform, &DepthSort), With<Controlled>>,
) {
    let Some(mut bounds) = bounds else {
        return;
    };
    let feet = player
        .single()
        .ok()
        .map(|(transform, sort)| sort.feet(transform));
    let wanted = camera_target(&active, feet);
    if bounds.0 != wanted {
        bounds.0 = wanted;
    }
}
