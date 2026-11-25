//! Scene transitions: exits, exit triggers, the screen fade and the swap of one map for another.

use super::*;

/// Where the player left a map: used to place them at the same position along the entry strip.
#[derive(Clone, Debug)]
pub struct ExitUse {
    pub from: String,
    pub exit: String,
    pub feet: Vec2,
}

/// Asks for a move to another map. Ignored while a transition runs.
#[derive(Message, Clone, Debug)]
pub struct RequestMapChange {
    pub to: String,
    /// The entry of the target map. `None` puts the player at its `player_start` and keeps their facing.
    pub entry: Option<String>,
    pub transition: Transition,
    /// The exit that was used. `None` (a `goto` from a script) puts the player in the middle of the entry.
    pub source: Option<ExitUse>,
}

/// The screen fade of a scene transition.
#[derive(Clone, Copy, Debug)]
pub struct Fade {
    pub colour: Color,
    pub out_secs: f32,
    pub in_secs: f32,
}

impl Default for Fade {
    fn default() -> Self {
        Fade {
            colour: Color::BLACK,
            out_secs: 0.3,
            in_secs: 0.3,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) enum Phase {
    #[default]
    Idle,
    FadeOut {
        elapsed: f32,
        request: RequestMapChange,
    },
    /// The screen is covered. The next system swaps the maps.
    Switch {
        request: RequestMapChange,
    },
    FadeIn {
        elapsed: f32,
    },
    /// A seamless exit was used. The next system loads the neighbour map. There is no fade.
    Seamless {
        request: RequestMapChange,
    },
}

/// State of the lifecycle. A game reads `locks_input` to stop the player while the screen fades.
#[derive(Resource)]
pub struct MapTransition {
    pub(super) phase: Phase,
    pub(super) fade: Fade,
    /// How an arrival at a `player_start` on an edge cell is placed (`edge_start_feet`). `None`: the centre of the cell.
    pub(super) edge_fit: Option<EdgeFit>,
    /// An exit can be used. False after a transition, until the feet have left every exit area.
    pub(super) armed: bool,
    /// Seconds each active map that is not the current one has been out of view. A map is removed after
    /// `UNLOAD_AFTER_SECS`.
    pub(super) hidden_for: std::collections::HashMap<String, f32>,
}

impl MapTransition {
    pub(super) fn new(fade: Fade) -> Self {
        MapTransition {
            phase: Phase::Idle,
            fade,
            edge_fit: None,
            armed: false,
            hidden_for: std::collections::HashMap::new(),
        }
    }

    pub fn is_idle(&self) -> bool {
        matches!(self.phase, Phase::Idle)
    }

    /// True while the screen fades or the maps are swapped: the player must not move. A seamless transition never
    /// locks the player.
    pub fn locks_input(&self) -> bool {
        matches!(
            self.phase,
            Phase::FadeOut { .. } | Phase::Switch { .. } | Phase::FadeIn { .. }
        )
    }
}

/// How far a fade has gone, 0 to 1. A fade of zero seconds is done at once.
pub(super) fn ratio(elapsed: f32, secs: f32) -> f32 {
    if secs <= 0.0 {
        1.0
    } else {
        (elapsed / secs).clamp(0.0, 1.0)
    }
}

/// Opacity of the fade for a phase: 0 clear, 1 covered.
pub(super) fn alpha(phase: &Phase, fade: &Fade) -> f32 {
    match phase {
        Phase::Idle | Phase::Seamless { .. } => 0.0,
        Phase::FadeOut { elapsed, .. } => ratio(*elapsed, fade.out_secs),
        Phase::Switch { .. } => 1.0,
        Phase::FadeIn { elapsed } => 1.0 - ratio(*elapsed, fade.in_secs),
    }
}

/// Moves a phase on by `dt` seconds. The swap itself is done by `perform_switch`.
pub(super) fn advance(phase: Phase, dt: f32, fade: &Fade) -> Phase {
    match phase {
        Phase::FadeOut { elapsed, request } => {
            let elapsed = elapsed + dt;
            if elapsed >= fade.out_secs {
                Phase::Switch { request }
            } else {
                Phase::FadeOut { elapsed, request }
            }
        }
        Phase::FadeIn { elapsed } => {
            let elapsed = elapsed + dt;
            if elapsed >= fade.in_secs {
                Phase::Idle
            } else {
                Phase::FadeIn { elapsed }
            }
        }
        other => other,
    }
}

/// The world rectangle of an area of a map whose cell (0,0) is at `origin`.
pub fn area_rect(area: &Area, origin: Vec2, cell: f32) -> Rect {
    Rect::from_corners(
        origin + area.min.as_vec2() * cell,
        origin + (area.max + 1).as_vec2() * cell,
    )
}

/// The exit of `map` that the feet are in, if any. The edges of the area count as inside.
pub fn exit_at(map: &Map, origin: Vec2, feet: Vec2) -> Option<&MapExit> {
    map.exits.iter().find(|exit| {
        let rect = area_rect(&exit.area, origin, map.cell_size);
        feet.cmpge(rect.min).all() && feet.cmple(rect.max).all()
    })
}

/// How far along a strip the feet are, in world units from its low end (the smaller x of a horizontal strip, the
/// smaller y of a vertical one). `None` for a single cell, which has no direction.
pub fn strip_offset(area: &Area, origin: Vec2, cell: f32, feet: Vec2) -> Option<f32> {
    let low = origin + area.min.as_vec2() * cell;
    if area.length() == 1 {
        None
    } else if area.is_horizontal() {
        Some(feet.x - low.x)
    } else {
        Some(feet.y - low.y)
    }
}

/// Where the feet stand in an entry. With an `offset` along the exit strip, the player keeps that position along the
/// entry strip, counted from its low end. That also holds when one strip is horizontal and the other vertical: the
/// offset moves to the other axis and the low end stays the low end. Across the strip the player stands in the middle
/// of its depth. Without an offset, or for a single cell, the player stands in the middle of the entry.
pub fn entry_feet(area: &Area, origin: Vec2, cell: f32, offset: Option<f32>) -> Vec2 {
    let low = origin + area.min.as_vec2() * cell;
    let size = (area.max - area.min + 1).as_vec2() * cell;
    let middle = low + size / 2.0;
    match offset {
        Some(offset) if area.length() > 1 => {
            if area.is_horizontal() {
                Vec2::new(low.x + offset.clamp(0.0, size.x), middle.y)
            } else {
                Vec2::new(middle.x, low.y + offset.clamp(0.0, size.y))
            }
        }
        _ => middle,
    }
}

#[derive(Component)]
pub(super) struct FadeOverlay;

pub struct MapTransitionPlugin {
    pub sources: &'static [MapSource],
    /// The legend names the games' maps may use.
    pub kinds: &'static [&'static str],
    /// The id of the map the game starts in.
    pub start: String,
    /// World position of cell (0,0) of a map that a scene transition loads.
    pub origin: Vec2,
    pub fade: Fade,
    /// Time constant of the camera glide when the player crosses into another map, in seconds. The camera covers about
    /// two thirds of the way in this time, so larger is slower. `CAMERA_EASE_SECS` is a start.
    pub camera_ease_secs: f32,
    /// How an arrival at a `player_start` on an edge cell is placed. `None` for a game whose maps have no edges.
    pub edge_fit: Option<EdgeFit>,
}

impl Plugin for MapTransitionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MapLifecyclePlugin {
            sources: self.sources,
            kinds: self.kinds,
            start: self.start.clone(),
            origin: self.origin,
        })
        .insert_resource(MapTransition {
            edge_fit: self.edge_fit,
            ..MapTransition::new(self.fade)
        })
        .insert_resource(CameraSmoothing {
            tau: self.camera_ease_secs,
            snap: false,
        });
        install_transition(app);
    }
}

/// The lifecycle and the transition systems, for tests that insert the resources themselves.
#[cfg(test)]
pub(super) fn install_systems(app: &mut App) {
    install_lifecycle(app);
    install_transition(app);
}

/// The systems of transitions and seamless joins, on top of the lifecycle (`install_lifecycle`).
fn install_transition(app: &mut App) {
    app.add_message::<RequestMapChange>()
        .add_systems(Startup, spawn_fade_overlay)
        .add_systems(
            Update,
            (
                detect_exit,
                accept_request,
                advance_transition,
                perform_switch,
                perform_seamless,
                update_current,
                unload_hidden,
                retarget_camera,
            )
                .chain()
                .before(sync_debug_grids)
                .before(show_map_id),
        );
}

pub(super) fn spawn_fade_overlay(mut commands: Commands, transition: Res<MapTransition>) {
    commands.spawn((
        FadeOverlay,
        // The overlay covers the whole game region and is transparent most of the time: it must not take picking
        // (UI clicks, hover) from the nodes below it.
        Pickable::IGNORE,
        GlobalZIndex(FADE_Z_INDEX),
        BackgroundColor(transition.fade.colour.with_alpha(0.0)),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            ..default()
        },
    ));
}

/// An object of a map that sends the player to another map when the feet touch `area` (world units, the edges count as
/// inside). The game adds it with a `MapId`, so it goes with its map. The player arrives at the `player_start` of the
/// target map and keeps their facing, and the change is always a scene transition. The map file names the target with
/// `exit_to` on the legend entry (`MapObject::exit_to`), and the game makes the area.
#[derive(Component, Clone, Debug)]
pub struct ExitTrigger {
    pub area: Rect,
    pub to: String,
}

/// The trigger among `triggers` that the feet touch, if any.
pub fn trigger_at<'a>(
    triggers: impl IntoIterator<Item = &'a ExitTrigger>,
    feet: Vec2,
) -> Option<&'a ExitTrigger> {
    triggers
        .into_iter()
        .find(|t| feet.cmpge(t.area.min).all() && feet.cmple(t.area.max).all())
}

/// Starts a transition when the feet of the controlled entity enter an exit or touch an `ExitTrigger`.
pub(super) fn detect_exit(
    mut transition: ResMut<MapTransition>,
    active: Res<ActiveMaps>,
    registry: Res<MapRegistry>,
    triggers: Query<(&ExitTrigger, &MapId)>,
    target: Query<(&Transform, &DepthSort), With<Controlled>>,
    mut requests: MessageWriter<RequestMapChange>,
) {
    if !transition.is_idle() {
        return;
    }
    let (Ok((transform, sort)), Some(current)) = (target.single(), active.current_map()) else {
        return;
    };
    let Some(registered) = registry.get(&current.id) else {
        return;
    };
    let feet = sort.feet(transform);
    let touched = trigger_at(
        triggers
            .iter()
            .filter(|(_, id)| id.0 == current.id)
            .map(|(trigger, _)| trigger),
        feet,
    );
    match exit_at(&registered.map, current.origin, feet) {
        None => match touched {
            None => transition.armed = true,
            Some(trigger) if transition.armed => {
                transition.armed = false;
                info!(
                    "maps: touched an exit of `{}`, going to `{}`",
                    current.id, trigger.to
                );
                requests.write(RequestMapChange {
                    to: trigger.to.clone(),
                    entry: None,
                    transition: Transition::Scene,
                    source: None,
                });
            }
            Some(_) => {}
        },
        Some(exit) if transition.armed => {
            transition.armed = false;
            info!(
                "maps: exit `{}` of `{}` used, going to `{}` entry `{}`",
                exit.name, current.id, exit.to, exit.entry
            );
            requests.write(RequestMapChange {
                to: exit.to.clone(),
                entry: Some(exit.entry.clone()),
                transition: exit.transition,
                source: Some(ExitUse {
                    from: current.id.clone(),
                    exit: exit.name.clone(),
                    feet,
                }),
            });
        }
        Some(_) => {}
    }
}

pub(super) fn accept_request(
    mut requests: MessageReader<RequestMapChange>,
    mut transition: ResMut<MapTransition>,
) {
    for request in requests.read() {
        if !transition.is_idle() {
            continue;
        }
        let seamless = request.transition == Transition::Seamless;
        let can_join = request.source.is_some() && request.entry.is_some();
        if seamless && !can_join {
            warn!(
                "maps: a seamless transition needs the exit that was used and an entry, using a scene transition"
            );
        }
        transition.phase = if seamless && can_join {
            Phase::Seamless {
                request: request.clone(),
            }
        } else {
            Phase::FadeOut {
                elapsed: 0.0,
                request: request.clone(),
            }
        };
    }
}

pub(super) fn advance_transition(
    time: Res<Time>,
    mut transition: ResMut<MapTransition>,
    mut overlay: Query<&mut BackgroundColor, With<FadeOverlay>>,
) {
    let fade = transition.fade;
    let phase = std::mem::take(&mut transition.phase);
    let phase = advance(phase, time.delta_secs(), &fade);
    let opacity = alpha(&phase, &fade);
    transition.phase = phase;
    for mut colour in &mut overlay {
        colour.0 = fade.colour.with_alpha(opacity);
    }
}

/// Everything about one map that the swap needs, read before anything is removed.
pub(super) struct Switch {
    pub(super) id: String,
    pub(super) origin: Vec2,
    pub(super) arrival: Arrival,
}

pub(super) fn plan_switch(world: &World, request: &RequestMapChange) -> Result<Switch, String> {
    let registry = world.resource::<MapRegistry>();
    let active = world.resource::<ActiveMaps>();
    let origin = world.resource::<MapOrigin>().0;
    let edge_fit = world.resource::<MapTransition>().edge_fit;
    let to = registry
        .get(&request.to)
        .ok_or_else(|| format!("map `{}` is not registered", request.to))?;
    let Some(entry_name) = &request.entry else {
        let feet = match edge_fit {
            Some(fit) => edge_start_feet(&to.map, origin, fit.step, fit.half_width),
            None => start_feet(&to.map, origin),
        }
        .ok_or_else(|| format!("map `{}` has no `player_start`", request.to))?;
        return Ok(Switch {
            id: request.to.clone(),
            origin,
            arrival: Arrival { feet, facing: None },
        });
    };
    let entry = to
        .map
        .entries
        .iter()
        .find(|entry| entry.name == *entry_name)
        .ok_or_else(|| format!("map `{}` has no entry `{entry_name}`", request.to))?;
    let offset = request.source.as_ref().and_then(|used| {
        let from = registry.get(&used.from)?;
        let exit = from.map.exits.iter().find(|exit| exit.name == used.exit)?;
        let from_origin = active.get(&used.from)?.origin;
        let along = strip_offset(&exit.area, from_origin, from.map.cell_size, used.feet)?;
        Some(along * to.map.cell_size / from.map.cell_size)
    });
    Ok(Switch {
        id: request.to.clone(),
        origin,
        arrival: Arrival {
            feet: entry_feet(&entry.area, origin, to.map.cell_size, offset),
            facing: entry.facing,
        },
    })
}

/// With the screen covered: removes the old map, loads the new one and puts the player in the entry.
pub(super) fn perform_switch(world: &mut World) {
    let request = {
        let mut transition = world.resource_mut::<MapTransition>();
        match std::mem::take(&mut transition.phase) {
            Phase::Switch { request } => request,
            other => {
                transition.phase = other;
                return;
            }
        }
    };
    let switch = match plan_switch(world, &request) {
        Ok(switch) => switch,
        Err(message) => {
            error!("maps: cannot go to `{}`: {message}", request.to);
            world.resource_mut::<MapTransition>().phase = Phase::FadeIn { elapsed: 0.0 };
            return;
        }
    };
    unload_all(world);
    world.resource_mut::<MapTransition>().hidden_for.clear();
    install_map(world, &switch.id, switch.origin);
    if let Some(arrive) = world
        .get_resource::<MapHooks>()
        .and_then(|hooks| hooks.arrive)
        && let Err(error) = world.run_system_with(arrive, switch.arrival)
    {
        error!("maps: the arrive hook failed: {error}");
    }
    info!(
        "maps: arrived in `{}` at feet {:?}",
        switch.id, switch.arrival.feet
    );
    let mut transition = world.resource_mut::<MapTransition>();
    transition.armed = false;
    transition.phase = Phase::FadeIn { elapsed: 0.0 };
}

/// Default time constant of the camera glide, in seconds.
pub const CAMERA_EASE_SECS: f32 = 0.9;
