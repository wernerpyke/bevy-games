//! Moving between maps. A game registers its maps, the kit runs the lifecycle: the player walks into an exit zone, the
//! screen fades out, the old map is removed, the new one is loaded, the player is placed in the entry and the screen
//! fades in. See `dev-docs/extensions/map-transitions.md` and `dev-docs/adr/EXTENSIONS.md` adr-x31.
//!
//! The kit has no game types. The game gives it three one-shot systems (`MapHooks`):
//! - `load` spawns the objects of one map and tags every one with `MapId`, so the kit can remove them.
//! - `rebuild` rebuilds the collision world from the entities that exist after a load or unload.
//! - `arrive` puts the controlled entity in place after a transition and sets its facing.
//!
//! Hooks run in a fixed order inside one frame, while the screen is covered, so no frame shows half a map. The kit
//! reads the exits and entries of the maps (`map_areas.rs`) and keeps `ActiveMaps` (the maps in the world and where
//! each one is) up to date. A transition is asked for with a `RequestMapChange` message. The kit sends it when the
//! feet of the `Controlled` entity enter an exit area, and the script tool sends it for `goto`.
//!
//! Only the scene transition (fade) is built so far. A seamless exit uses the same fade for now.

use std::path::Path;

use bevy::ecs::system::SystemId;
use bevy::prelude::*;

use crate::Controlled;
use crate::camera::{CameraBounds, CameraSmoothing, visible_rect};
use crate::collision::register_blockers;
use crate::debug::{DebugGrids, DebugInfo, GridRegion};
use crate::depth::DepthSort;
use crate::layers::FADE_Z_INDEX;
use crate::map::{Area, Edge, Facing, Map, MapEntry, MapExit, Transition, parse, start_feet};
use crate::map_links::{LinkedMap, validate_links};

/// One map file a game registers. `file` is relative to the game folder: a native run reads it from disk (so F1 reloads
/// it), the web build has no disk and uses `bundled`, normally an `include_str!` of the same file.
#[derive(Clone, Copy, Debug)]
pub struct MapSource {
    pub id: &'static str,
    pub file: &'static str,
    pub bundled: &'static str,
}

/// A parsed map with the id it was registered under.
#[derive(Clone, Debug)]
pub struct RegisteredMap {
    pub id: String,
    /// The file name used in messages.
    pub file: String,
    pub map: Map,
}

/// All maps of the game, parsed and checked against each other (`validate_links`).
#[derive(Resource)]
pub struct MapRegistry {
    sources: Vec<MapSource>,
    kinds: &'static [&'static str],
    maps: Vec<RegisteredMap>,
}

impl MapRegistry {
    /// Reads and checks every map. The error names the file and line.
    pub fn load(
        sources: &[MapSource],
        kinds: &'static [&'static str],
    ) -> Result<MapRegistry, String> {
        let maps = sources
            .iter()
            .map(|source| read(source, kinds))
            .collect::<Result<Vec<_>, _>>()?;
        check_links(&maps)?;
        Ok(MapRegistry {
            sources: sources.to_vec(),
            kinds,
            maps,
        })
    }

    /// A registry of maps that are already parsed. It cannot reload them.
    pub fn from_parsed(maps: Vec<RegisteredMap>) -> Result<MapRegistry, String> {
        check_links(&maps)?;
        Ok(MapRegistry {
            sources: Vec::new(),
            kinds: &[],
            maps,
        })
    }

    pub fn get(&self, id: &str) -> Option<&RegisteredMap> {
        self.maps.iter().find(|m| m.id == id)
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.maps.iter().map(|m| m.id.as_str())
    }

    /// Reads one map again (a native run reads the file from disk) and checks the links. On an error nothing changes.
    pub fn reload(&mut self, id: &str) -> Result<(), String> {
        let index = self
            .maps
            .iter()
            .position(|m| m.id == id)
            .ok_or_else(|| format!("map `{id}` is not registered"))?;
        let source = self
            .sources
            .get(index)
            .ok_or_else(|| format!("map `{id}` has no file to read again"))?;
        let mut maps = self.maps.clone();
        maps[index] = read(source, self.kinds)?;
        check_links(&maps)?;
        self.maps = maps;
        Ok(())
    }
}

fn read(source: &MapSource, kinds: &[&str]) -> Result<RegisteredMap, String> {
    let (file, text) = match std::env::var_os("CARGO_MANIFEST_DIR") {
        None => (source.file.to_string(), source.bundled.to_string()),
        Some(dir) => {
            let path = Path::new(&dir).join(source.file);
            let name = path.display().to_string();
            let text =
                std::fs::read_to_string(&path).map_err(|error| format!("{name}: {error}"))?;
            (name, text)
        }
    };
    let map = parse(&file, &text, kinds).map_err(|error| error.to_string())?;
    Ok(RegisteredMap {
        id: source.id.to_string(),
        file,
        map,
    })
}

fn check_links(maps: &[RegisteredMap]) -> Result<(), String> {
    let linked: Vec<LinkedMap> = maps
        .iter()
        .map(|m| LinkedMap {
            id: &m.id,
            file: &m.file,
            map: &m.map,
        })
        .collect();
    validate_links(&linked).map_err(|error| error.to_string())
}

/// A map that is in the world now.
#[derive(Clone, Debug, PartialEq)]
pub struct ActiveMap {
    pub id: String,
    /// World position of the bottom left corner of cell (0,0).
    pub origin: Vec2,
    /// The world area of the whole map.
    pub rect: Rect,
}

/// A seamless link between two active maps: the exit strip of `from`, in world units, and the edge of `from` it is on.
#[derive(Clone, Debug, PartialEq)]
pub struct Doorway {
    pub from: String,
    pub to: String,
    pub edge: Edge,
    pub area: Rect,
}

/// The maps in the world, and the one that holds the player.
#[derive(Resource, Default, Debug)]
pub struct ActiveMaps {
    pub current: String,
    pub maps: Vec<ActiveMap>,
    /// The seamless links between active maps. The player may cross a map edge only through one of these.
    pub doorways: Vec<Doorway>,
}

impl ActiveMaps {
    /// The map whose area holds the point. Each map owns its bottom and left edges, so a point on a seam belongs to
    /// one map only.
    pub fn map_at(&self, point: Vec2) -> Option<&ActiveMap> {
        self.maps.iter().find(|m| {
            point.x >= m.rect.min.x
                && point.x < m.rect.max.x
                && point.y >= m.rect.min.y
                && point.y < m.rect.max.y
        })
    }

    /// The smallest rectangle that holds every active map.
    pub fn bounding_rect(&self) -> Rect {
        self.maps
            .iter()
            .map(|m| m.rect)
            .reduce(|a, b| a.union(b))
            .unwrap_or_default()
    }

    /// The areas the feet may stand in, as rectangles: every map shrunk by `margin_min` on its left and bottom and by
    /// `margin_max` on its right and top (the width of the body and the height of the sprite), plus one bridge over
    /// every doorway, so the player can cross a seam there and nowhere else. Use `clamp_to_union` to keep the feet in.
    pub fn walkable(&self, margin_min: Vec2, margin_max: Vec2) -> Vec<Rect> {
        let inner = |rect: Rect| Rect::from_corners(rect.min + margin_min, rect.max - margin_max);
        let mut areas: Vec<Rect> = self.maps.iter().map(|m| inner(m.rect)).collect();
        for door in &self.doorways {
            let (Some(from), Some(to)) = (self.get(&door.from), self.get(&door.to)) else {
                continue;
            };
            let (a, b) = (inner(from.rect), inner(to.rect));
            let (x0, x1) = (
                door.area.min.x + margin_min.x,
                door.area.max.x - margin_max.x,
            );
            let (y0, y1) = (
                door.area.min.y + margin_min.y,
                door.area.max.y - margin_max.y,
            );
            areas.push(match door.edge {
                Edge::Top => Rect::from_corners(Vec2::new(x0, a.max.y), Vec2::new(x1, b.min.y)),
                Edge::Bottom => Rect::from_corners(Vec2::new(x0, b.max.y), Vec2::new(x1, a.min.y)),
                Edge::Right => Rect::from_corners(Vec2::new(a.max.x, y0), Vec2::new(b.min.x, y1)),
                Edge::Left => Rect::from_corners(Vec2::new(b.max.x, y0), Vec2::new(a.min.x, y1)),
            });
        }
        areas
    }

    pub fn get(&self, id: &str) -> Option<&ActiveMap> {
        self.maps.iter().find(|m| m.id == id)
    }

    pub fn current_map(&self) -> Option<&ActiveMap> {
        self.get(&self.current)
    }

    /// The area of the current map, or an empty rectangle before a map is loaded.
    pub fn current_rect(&self) -> Rect {
        self.current_map().map(|m| m.rect).unwrap_or_default()
    }
}

/// Put this on everything a map spawns (rocks, trees and so on), so the kit can remove it when the map goes. The
/// controlled entity does not have it: it stays through a transition.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct MapId(pub String);

/// What the `load` hook gets: spawn the objects of this map with their cells counted from `origin`.
#[derive(Clone, Debug)]
pub struct MapLoad {
    pub id: String,
    pub origin: Vec2,
}

/// What the `arrive` hook gets: the feet position in the world, and the way the player should look.
#[derive(Clone, Copy, Debug)]
pub struct Arrival {
    pub feet: Vec2,
    pub facing: Option<Facing>,
}

/// The three systems a game gives the kit. Register them with `World::register_system`.
#[derive(Resource, Clone, Copy)]
pub struct MapHooks {
    pub load: SystemId<In<MapLoad>>,
    pub rebuild: SystemId,
    pub arrive: SystemId<In<Arrival>>,
}

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
enum Phase {
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
    phase: Phase,
    fade: Fade,
    /// World position of cell (0,0) of the first map of a scene transition.
    origin: Vec2,
    /// An exit can be used. False after a transition, until the feet have left every exit area.
    armed: bool,
    /// Seconds each active map that is not the current one has been out of view. A map is removed after
    /// `UNLOAD_AFTER_SECS`.
    hidden_for: std::collections::HashMap<String, f32>,
}

impl MapTransition {
    fn new(fade: Fade, origin: Vec2) -> Self {
        MapTransition {
            phase: Phase::Idle,
            fade,
            origin,
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
fn ratio(elapsed: f32, secs: f32) -> f32 {
    if secs <= 0.0 {
        1.0
    } else {
        (elapsed / secs).clamp(0.0, 1.0)
    }
}

/// Opacity of the fade for a phase: 0 clear, 1 covered.
fn alpha(phase: &Phase, fade: &Fade) -> f32 {
    match phase {
        Phase::Idle | Phase::Seamless { .. } => 0.0,
        Phase::FadeOut { elapsed, .. } => ratio(*elapsed, fade.out_secs),
        Phase::Switch { .. } => 1.0,
        Phase::FadeIn { elapsed } => 1.0 - ratio(*elapsed, fade.in_secs),
    }
}

/// Moves a phase on by `dt` seconds. The swap itself is done by `perform_switch`.
fn advance(phase: Phase, dt: f32, fade: &Fade) -> Phase {
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
struct FadeOverlay;

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
}

impl Plugin for MapTransitionPlugin {
    fn build(&self, app: &mut App) {
        let registry =
            MapRegistry::load(self.sources, self.kinds).unwrap_or_else(|error| panic!("{error}"));
        let known: Vec<&str> = registry.ids().collect();
        assert!(
            known.contains(&self.start.as_str()),
            "the start map `{}` is not registered (registered: {})",
            self.start,
            known.join(", ")
        );
        info!(
            "maps: {} registered, starting in `{}`",
            known.len(),
            self.start
        );
        app.insert_resource(registry)
            .insert_resource(ActiveMaps {
                current: self.start.clone(),
                ..default()
            })
            .insert_resource(MapTransition::new(self.fade, self.origin))
            .insert_resource(CameraSmoothing {
                tau: self.camera_ease_secs,
                snap: false,
            });
        install_systems(app);
    }
}

fn install_systems(app: &mut App) {
    app.add_message::<RequestMapChange>()
        .add_systems(Startup, spawn_fade_overlay)
        .add_systems(PostStartup, load_start.after(register_blockers))
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
                sync_debug_grids,
                show_map_id,
            )
                .chain(),
        );
}

fn spawn_fade_overlay(mut commands: Commands, transition: Res<MapTransition>) {
    commands.spawn((
        FadeOverlay,
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
fn detect_exit(
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
    let feet = Vec2::new(
        transform.translation.x,
        sort.feet_y(transform.translation.y),
    );
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

fn accept_request(
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

fn advance_transition(
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
struct Switch {
    id: String,
    origin: Vec2,
    arrival: Arrival,
}

fn plan_switch(world: &World, request: &RequestMapChange) -> Result<Switch, String> {
    let registry = world.resource::<MapRegistry>();
    let active = world.resource::<ActiveMaps>();
    let origin = world.resource::<MapTransition>().origin;
    let to = registry
        .get(&request.to)
        .ok_or_else(|| format!("map `{}` is not registered", request.to))?;
    let Some(entry_name) = &request.entry else {
        let feet = start_feet(&to.map, origin)
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

/// Removes every entity with a `MapId`.
fn unload_all(world: &mut World) {
    let old: Vec<Entity> = world
        .query_filtered::<Entity, With<MapId>>()
        .iter(world)
        .collect();
    for entity in old {
        world.despawn(entity);
    }
}

/// Removes the entities of one map.
fn unload_entities(world: &mut World, id: &str) {
    let old: Vec<Entity> = world
        .query::<(Entity, &MapId)>()
        .iter(world)
        .filter(|(_, map)| map.0 == id)
        .map(|(entity, _)| entity)
        .collect();
    for entity in old {
        world.despawn(entity);
    }
}

/// Runs the `load` hook for one map.
fn run_load(world: &mut World, id: &str, origin: Vec2) {
    let Some(hooks) = world.get_resource::<MapHooks>().copied() else {
        error!("maps: the game has not inserted `MapHooks`");
        return;
    };
    let load = MapLoad {
        id: id.to_string(),
        origin,
    };
    if let Err(error) = world.run_system_with(hooks.load, load) {
        error!("maps: the load hook failed: {error}");
    }
}

/// Runs the `rebuild` hook: the game fills the collision world from the entities that exist now.
fn run_rebuild(world: &mut World) {
    let Some(hooks) = world.get_resource::<MapHooks>().copied() else {
        return;
    };
    if let Err(error) = world.run_system(hooks.rebuild) {
        error!("maps: the rebuild hook failed: {error}");
    }
}

/// Puts the camera limits on the current map at once, and makes the camera jump with them (a scene transition, the
/// screen is covered). A seamless transition changes the limits in `retarget_camera` and the camera glides.
fn snap_bounds(world: &mut World) {
    let rect = world.resource::<ActiveMaps>().current_rect();
    world.insert_resource(CameraBounds(rect));
    if let Some(mut smoothing) = world.get_resource_mut::<CameraSmoothing>() {
        smoothing.snap = true;
    }
}

/// Makes `id` the only active map at `origin`, sets the camera limits and runs the `load` and `rebuild` hooks.
fn install_map(world: &mut World, id: &str, origin: Vec2) {
    let rect = {
        let registry = world.resource::<MapRegistry>();
        let Some(registered) = registry.get(id) else {
            error!("maps: `{id}` is not registered");
            return;
        };
        Rect::from_corners(
            origin,
            origin + registered.map.size.as_vec2() * registered.map.cell_size,
        )
    };
    *world.resource_mut::<ActiveMaps>() = ActiveMaps {
        current: id.to_string(),
        maps: vec![ActiveMap {
            id: id.to_string(),
            origin,
            rect,
        }],
        doorways: Vec::new(),
    };
    world.resource_mut::<MapTransition>().hidden_for.clear();
    snap_bounds(world);
    run_load(world, id, origin);
    run_rebuild(world);
}

/// Loads the first map after the game has set up its own start up systems.
fn load_start(world: &mut World) {
    let id = world.resource::<ActiveMaps>().current.clone();
    let origin = world.resource::<MapTransition>().origin;
    install_map(world, &id, origin);
}

/// Reads the current map file again and loads every active map again, for the F1 key. The controlled entity stays
/// where it is. On an error in the file nothing changes.
pub fn reload_current_map(world: &mut World) -> Result<(), String> {
    let (id, maps) = {
        let active = world.resource::<ActiveMaps>();
        if active.current_map().is_none() {
            return Err("no map is loaded".to_string());
        }
        (active.current.clone(), active.maps.clone())
    };
    world.resource_mut::<MapRegistry>().reload(&id)?;
    unload_all(world);
    for map in &maps {
        run_load(world, &map.id, map.origin);
    }
    run_rebuild(world);
    snap_bounds(world);
    info!("maps: reloaded `{id}`");
    Ok(())
}

/// With the screen covered: removes the old map, loads the new one and puts the player in the entry.
fn perform_switch(world: &mut World) {
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
    install_map(world, &switch.id, switch.origin);
    if let Some(hooks) = world.get_resource::<MapHooks>().copied()
        && let Err(error) = world.run_system_with(hooks.arrive, switch.arrival)
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

/// How long a map must be out of view before it is removed, in seconds. A map that was just loaded next to the
/// player is out of view until the camera bounds have eased, and must not go at once.
pub const UNLOAD_AFTER_SECS: f32 = 1.0;
/// Default time constant of the camera glide, in seconds.
pub const CAMERA_EASE_SECS: f32 = 0.9;

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
fn begin_seamless(world: &mut World, request: &RequestMapChange) -> Result<(), String> {
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

fn perform_seamless(world: &mut World) {
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
fn update_current(
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
    let feet = Vec2::new(
        transform.translation.x,
        sort.feet_y(transform.translation.y),
    );
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
fn unload_hidden(world: &mut World) {
    if world.resource::<ActiveMaps>().maps.len() < 2 {
        return;
    }
    let visible = {
        let mut cameras = world.query_filtered::<(&GlobalTransform, &Projection), With<Camera2d>>();
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
fn retarget_camera(
    active: Res<ActiveMaps>,
    bounds: Option<ResMut<CameraBounds>>,
    player: Query<(&Transform, &DepthSort), With<Controlled>>,
) {
    let Some(mut bounds) = bounds else {
        return;
    };
    let feet = player.single().ok().map(|(transform, sort)| {
        Vec2::new(
            transform.translation.x,
            sort.feet_y(transform.translation.y),
        )
    });
    let wanted = camera_target(&active, feet);
    if bounds.0 != wanted {
        bounds.0 = wanted;
    }
}

/// Gives the debug grid one region per active map, so each map has its own grid and its own numbers.
fn sync_debug_grids(active: Res<ActiveMaps>, grids: Option<ResMut<DebugGrids>>) {
    let Some(mut grids) = grids else {
        return;
    };
    if !active.is_changed() {
        return;
    }
    let regions: Vec<GridRegion> = active
        .maps
        .iter()
        .map(|m| GridRegion {
            origin: m.origin,
            rect: m.rect,
        })
        .collect();
    if grids.0 != regions {
        grids.0 = regions;
    }
}

/// Shows the id of the current map in the F2 info box.
fn show_map_id(active: Res<ActiveMaps>, info: Option<ResMut<DebugInfo>>) {
    if let Some(mut info) = info {
        info.set("map", active.current.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{PLAYER_START, parse};

    fn cell(x: i32, y: i32) -> IVec2 {
        IVec2::new(x, y)
    }

    fn area(a: IVec2, b: IVec2) -> Area {
        Area::new(a, b)
    }

    const FADE: Fade = Fade {
        colour: Color::BLACK,
        out_secs: 1.0,
        in_secs: 2.0,
    };

    fn request() -> RequestMapChange {
        RequestMapChange {
            to: "2".into(),
            entry: Some("e".into()),
            transition: Transition::Scene,
            source: None,
        }
    }

    #[test]
    fn the_fade_goes_out_swaps_and_comes_back_in_order() {
        let fade_out = Phase::FadeOut {
            elapsed: 0.0,
            request: request(),
        };
        assert!((alpha(&fade_out, &FADE) - 0.0).abs() < 1e-6);
        let half = advance(fade_out, 0.5, &FADE);
        assert!(matches!(half, Phase::FadeOut { .. }));
        assert!((alpha(&half, &FADE) - 0.5).abs() < 1e-6);
        let covered = advance(half, 0.5, &FADE);
        assert!(matches!(covered, Phase::Switch { .. }));
        assert_eq!(alpha(&covered, &FADE), 1.0);
        // The swap system sets this phase. It lasts two seconds.
        let fade_in = Phase::FadeIn { elapsed: 0.0 };
        assert_eq!(alpha(&fade_in, &FADE), 1.0);
        let half = advance(fade_in, 1.0, &FADE);
        assert!((alpha(&half, &FADE) - 0.5).abs() < 1e-6);
        let done = advance(half, 1.0, &FADE);
        assert!(matches!(done, Phase::Idle));
        assert_eq!(alpha(&done, &FADE), 0.0);
    }

    #[test]
    fn a_fade_of_zero_seconds_does_not_divide_by_zero() {
        let instant = Fade {
            out_secs: 0.0,
            in_secs: 0.0,
            ..FADE
        };
        let phase = advance(
            Phase::FadeOut {
                elapsed: 0.0,
                request: request(),
            },
            0.0,
            &instant,
        );
        assert!(matches!(phase, Phase::Switch { .. }));
        assert_eq!(alpha(&Phase::FadeIn { elapsed: 0.0 }, &instant), 0.0);
        assert!(matches!(
            advance(Phase::FadeIn { elapsed: 0.0 }, 0.0, &instant),
            Phase::Idle
        ));
    }

    #[test]
    fn idle_and_switch_do_not_advance_by_time() {
        assert!(matches!(advance(Phase::Idle, 5.0, &FADE), Phase::Idle));
        let covered = advance(Phase::Switch { request: request() }, 5.0, &FADE);
        assert!(matches!(covered, Phase::Switch { .. }));
    }

    #[test]
    fn an_area_becomes_a_world_rectangle() {
        let rect = area_rect(
            &area(cell(3, 9), cell(6, 9)),
            Vec2::new(-100.0, -50.0),
            64.0,
        );
        assert_eq!(rect.min, Vec2::new(92.0, 526.0));
        assert_eq!(rect.max, Vec2::new(348.0, 590.0));
    }

    fn map_with_exit() -> Map {
        let text = "---\nmap: t\ncell: 64\norigin: bottom-left\nsize: 10x10\nlegend:\n  \".\": empty\nexits:\n  up: area=3..6,9 to=2 entry=e transition=scene\n---\n";
        let row = ["."; 10].join(" ");
        let rows: String = (0..10).rev().map(|y| format!("{y:>2} |{row}|\n")).collect();
        parse("t.txt", &format!("{text}{rows}"), &[]).unwrap()
    }

    #[test]
    fn feet_in_an_exit_area_find_the_exit() {
        let map = map_with_exit();
        let origin = Vec2::new(-1000.0, -500.0);
        // Row 9 spans y 76..140 above the origin, columns 3 to 6 span x 192..448.
        let inside = origin + Vec2::new(250.0, 580.0);
        assert_eq!(
            exit_at(&map, origin, inside).map(|e| e.name.as_str()),
            Some("up")
        );
        // The bottom edge of the area counts as inside.
        let on_edge = origin + Vec2::new(192.0, 576.0);
        assert!(exit_at(&map, origin, on_edge).is_some());
        for outside in [
            Vec2::new(250.0, 575.0),
            Vec2::new(191.0, 600.0),
            Vec2::new(449.0, 600.0),
        ] {
            assert!(
                exit_at(&map, origin, origin + outside).is_none(),
                "{outside}"
            );
        }
    }

    #[test]
    fn the_offset_along_a_strip_counts_from_its_low_end() {
        let origin = Vec2::ZERO;
        let top = area(cell(3, 9), cell(6, 9));
        assert_eq!(
            strip_offset(&top, origin, 64.0, Vec2::new(250.0, 580.0)),
            Some(58.0)
        );
        let right = area(cell(9, 4), cell(9, 5));
        assert_eq!(
            strip_offset(&right, origin, 64.0, Vec2::new(600.0, 300.0)),
            Some(44.0)
        );
        let single = area(cell(4, 0), cell(4, 0));
        assert_eq!(
            strip_offset(&single, origin, 64.0, Vec2::new(280.0, 10.0)),
            None
        );
    }

    #[test]
    fn an_entry_keeps_the_position_along_the_strip() {
        let origin = Vec2::new(-1000.0, -500.0);
        // Same kind of edge: a top exit to a bottom entry at the same x.
        let bottom = area(cell(3, 0), cell(6, 0));
        assert_eq!(
            entry_feet(&bottom, origin, 64.0, Some(58.0)),
            origin + Vec2::new(192.0 + 58.0, 32.0)
        );
        // A right exit to a left entry: the offset along y carries over.
        let left = area(cell(0, 8), cell(0, 9));
        assert_eq!(
            entry_feet(&left, origin, 64.0, Some(44.0)),
            origin + Vec2::new(32.0, 512.0 + 44.0)
        );
    }

    #[test]
    fn an_entry_on_another_kind_of_edge_moves_the_offset_to_the_other_axis() {
        let origin = Vec2::ZERO;
        let vertical = area(cell(0, 6), cell(0, 7));
        // A horizontal exit strip to a vertical entry strip: x offset becomes y offset, low end stays low end.
        assert_eq!(
            entry_feet(&vertical, origin, 64.0, Some(8.0)),
            Vec2::new(32.0, 384.0 + 8.0)
        );
        let horizontal = area(cell(3, 0), cell(4, 0));
        // And the other way round: a y offset becomes an x offset.
        assert_eq!(
            entry_feet(&horizontal, origin, 64.0, Some(100.0)),
            Vec2::new(192.0 + 100.0, 32.0)
        );
    }

    #[test]
    fn every_pair_of_edges_gives_a_position_inside_the_entry() {
        let exits = [
            area(cell(3, 9), cell(4, 9)),
            area(cell(3, 0), cell(4, 0)),
            area(cell(9, 3), cell(9, 4)),
            area(cell(0, 3), cell(0, 4)),
        ];
        for exit in exits {
            for entry in exits {
                // The feet 20 units into the strip, then right at its far end.
                for along in [20.0, 127.9] {
                    let feet_offset = Some(along);
                    let at = entry_feet(&entry, Vec2::ZERO, 64.0, feet_offset);
                    let rect = area_rect(&entry, Vec2::ZERO, 64.0);
                    assert!(
                        at.cmpge(rect.min).all() && at.cmple(rect.max).all(),
                        "{exit:?} to {entry:?}: {at}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_single_cell_or_no_offset_puts_the_player_in_the_middle() {
        let single = area(cell(4, 0), cell(4, 0));
        assert_eq!(
            entry_feet(&single, Vec2::ZERO, 64.0, Some(30.0)),
            Vec2::new(288.0, 32.0)
        );
        let strip = area(cell(3, 0), cell(6, 0));
        assert_eq!(
            entry_feet(&strip, Vec2::ZERO, 64.0, None),
            Vec2::new(320.0, 32.0)
        );
    }

    #[test]
    fn an_offset_past_the_end_of_the_strip_is_held_at_the_end() {
        let strip = area(cell(3, 0), cell(4, 0));
        assert_eq!(
            entry_feet(&strip, Vec2::ZERO, 64.0, Some(500.0)).x,
            192.0 + 128.0
        );
        assert_eq!(entry_feet(&strip, Vec2::ZERO, 64.0, Some(-5.0)).x, 192.0);
    }

    #[derive(Resource, Default)]
    struct Log(Vec<String>);

    /// A trigger that the test hook spawns with map 2, leading back to map 1.
    #[derive(Resource)]
    struct TriggerInMap2(Rect);

    fn load_hook(
        In(load): In<MapLoad>,
        mut log: ResMut<Log>,
        mut commands: Commands,
        extra: Option<Res<TriggerInMap2>>,
    ) {
        log.0.push(format!("load {} at {}", load.id, load.origin));
        if let (Some(extra), "2") = (extra, load.id.as_str()) {
            commands.spawn((
                MapId(load.id.clone()),
                ExitTrigger {
                    area: extra.0,
                    to: "1".into(),
                },
            ));
        }
        commands.spawn(MapId(load.id));
    }

    fn rebuild_hook(mut log: ResMut<Log>, maps: Query<&MapId>) {
        log.0
            .push(format!("rebuild with {} map entities", maps.iter().count()));
    }

    fn arrive_hook(
        In(arrival): In<Arrival>,
        mut log: ResMut<Log>,
        mut player: Query<(&mut Transform, &DepthSort), With<Controlled>>,
    ) {
        log.0.push(format!("arrive {:?}", arrival.facing));
        for (mut transform, sort) in &mut player {
            transform.translation.x = arrival.feet.x;
            transform.translation.y = arrival.feet.y + sort.feet_offset;
        }
    }

    fn registered(id: &str, blocks: &str) -> RegisteredMap {
        let head = format!(
            "---\nmap: {id}\ncell: 64\norigin: bottom-left\nsize: 10x10\nlegend:\n  \".\": empty\n{blocks}---\n"
        );
        let row = ["."; 10].join(" ");
        let rows: String = (0..10).rev().map(|y| format!("{y:>2} |{row}|\n")).collect();
        RegisteredMap {
            id: id.into(),
            file: format!("{id}.txt"),
            map: parse("t.txt", &format!("{head}{rows}"), &[]).unwrap(),
        }
    }

    const ORIGIN: Vec2 = Vec2::new(-1000.0, -500.0);
    const FEET_OFFSET: f32 = 56.0;

    /// An app with two maps: map 1 has an exit up (3..6,9) to the entry `from_1` of map 2 (3..6,0, facing up).
    fn app_with_two_maps() -> App {
        app_with("scene")
    }

    /// The same two maps, with the exit of the given transition style. A second update step of 0.6 seconds makes the
    /// timers of the lifecycle run fast.
    fn app_with(transition: &str) -> App {
        let one = registered(
            "1",
            &format!("exits:\n  up: area=3..6,9 to=2 entry=from_1 transition={transition}\n"),
        );
        let two = registered("2", "entries:\n  from_1: area=3..6,0 facing=up\n");
        let registry = MapRegistry::from_parsed(vec![one, two]).unwrap();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Log>()
            .insert_resource(registry)
            .insert_resource(ActiveMaps {
                current: "1".into(),
                ..default()
            })
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(600),
            ))
            .insert_resource(MapTransition::new(
                Fade {
                    colour: Color::BLACK,
                    out_secs: 0.0,
                    in_secs: 0.0,
                },
                ORIGIN,
            ));
        let hooks = MapHooks {
            load: app.world_mut().register_system(load_hook),
            rebuild: app.world_mut().register_system(rebuild_hook),
            arrive: app.world_mut().register_system(arrive_hook),
        };
        app.insert_resource(hooks);
        install_systems(&mut app);
        app
    }

    fn player_feet(app: &mut App) -> Vec2 {
        let mut query = app
            .world_mut()
            .query_filtered::<(&Transform, &DepthSort), With<Controlled>>();
        let (transform, sort) = query.single(app.world()).unwrap();
        Vec2::new(
            transform.translation.x,
            sort.feet_y(transform.translation.y),
        )
    }

    fn set_feet(app: &mut App, feet: Vec2) {
        let mut query = app
            .world_mut()
            .query_filtered::<&mut Transform, With<Controlled>>();
        let mut transform = query.single_mut(app.world_mut()).unwrap();
        transform.translation.x = feet.x;
        transform.translation.y = feet.y + FEET_OFFSET;
    }

    fn put_player(app: &mut App, feet: Vec2) {
        app.world_mut().spawn((
            Controlled,
            DepthSort {
                feet_offset: FEET_OFFSET,
            },
            Transform::from_xyz(feet.x, feet.y + FEET_OFFSET, 0.0),
        ));
    }

    #[test]
    fn the_first_map_is_loaded_at_start() {
        let mut app = app_with_two_maps();
        app.update();
        let log = &app.world().resource::<Log>().0;
        assert_eq!(log[0], format!("load 1 at {ORIGIN}"));
        assert!(log[1].starts_with("rebuild with 1 map"));
        let active = app.world().resource::<ActiveMaps>();
        assert_eq!(active.current, "1");
        assert_eq!(
            active.current_rect(),
            Rect::from_corners(ORIGIN, ORIGIN + Vec2::splat(640.0))
        );
        assert_eq!(
            app.world().resource::<CameraBounds>().0,
            active.current_rect()
        );
    }

    #[test]
    fn walking_into_an_exit_runs_the_lifecycle_in_order() {
        let mut app = app_with_two_maps();
        app.update();
        // Away from the exit first, so the exit is armed, then into the exit 58 units along its strip.
        put_player(&mut app, ORIGIN + Vec2::new(300.0, 300.0));
        app.update();
        set_feet(&mut app, ORIGIN + Vec2::new(192.0 + 58.0, 600.0));
        app.update();
        app.update();
        app.update();
        let log = app.world().resource::<Log>().0.clone();
        assert_eq!(log.len(), 5, "{log:?}");
        assert_eq!(log[2], format!("load 2 at {ORIGIN}"));
        assert!(log[3].starts_with("rebuild with 1 map"), "{log:?}");
        assert_eq!(log[4], "arrive Some(Up)");
        assert_eq!(app.world().resource::<ActiveMaps>().current, "2");
        assert!(app.world().resource::<MapTransition>().is_idle());
        // The same x along the strip, and the middle of the bottom row of the new map.
        assert_eq!(player_feet(&mut app), ORIGIN + Vec2::new(250.0, 32.0));
        // The old map entity is gone and the new one is there.
        let ids: Vec<String> = app
            .world_mut()
            .query::<&MapId>()
            .iter(app.world())
            .map(|id| id.0.clone())
            .collect();
        assert_eq!(ids, vec!["2".to_string()]);
    }

    #[test]
    fn the_exit_does_not_fire_again_while_the_player_stands_in_it() {
        let mut app = app_with_two_maps();
        app.update();
        // Start inside the exit: the exit is not armed until the feet have left every exit.
        put_player(&mut app, ORIGIN + Vec2::new(250.0, 590.0));
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<Log>().0.len(),
            2,
            "no transition expected"
        );
        assert_eq!(app.world().resource::<ActiveMaps>().current, "1");
    }

    #[test]
    fn goto_without_an_exit_puts_the_player_in_the_middle_of_the_entry() {
        let mut app = app_with_two_maps();
        app.update();
        put_player(&mut app, ORIGIN + Vec2::new(300.0, 300.0));
        app.world_mut().write_message(RequestMapChange {
            to: "2".into(),
            entry: Some("from_1".into()),
            transition: Transition::Scene,
            source: None,
        });
        app.update();
        app.update();
        assert_eq!(app.world().resource::<ActiveMaps>().current, "2");
        assert_eq!(player_feet(&mut app), ORIGIN + Vec2::new(320.0, 32.0));
    }

    /// A 10 by 10 map with a `player_start` at `start` and no exits.
    fn registered_with_start(id: &str, start: (usize, usize)) -> RegisteredMap {
        let head = format!(
            "---\nmap: {id}\ncell: 64\norigin: bottom-left\nsize: 10x10\nlegend:\n  \".\": empty\n  \"@\": player_start\n---\n"
        );
        let rows: String = (0..10)
            .rev()
            .map(|y| {
                let row: Vec<&str> = (0..10)
                    .map(|x| if (x, y) == start { "@" } else { "." })
                    .collect();
                format!("{y:>2} |{}|\n", row.join(" "))
            })
            .collect();
        RegisteredMap {
            id: id.into(),
            file: format!("{id}.txt"),
            map: parse("t.txt", &format!("{head}{rows}"), &[PLAYER_START]).unwrap(),
        }
    }

    /// Two maps with a start in map 2 at cell (4,0), and a trigger of map 1 over the area (100..160, 100..160).
    fn app_with_trigger() -> App {
        let one = registered_with_start("1", (0, 0));
        let two = registered_with_start("2", (4, 0));
        let registry = MapRegistry::from_parsed(vec![one, two]).unwrap();
        let mut app = app_with_two_maps();
        app.insert_resource(registry);
        app.world_mut().spawn((
            MapId("1".into()),
            ExitTrigger {
                area: Rect::from_corners(ORIGIN + Vec2::splat(100.0), ORIGIN + Vec2::splat(160.0)),
                to: "2".into(),
            },
        ));
        app
    }

    #[test]
    fn a_request_without_an_entry_arrives_at_the_player_start_and_keeps_the_facing() {
        let mut app = app_with_trigger();
        app.update();
        put_player(&mut app, ORIGIN + Vec2::new(300.0, 300.0));
        app.world_mut().write_message(RequestMapChange {
            to: "2".into(),
            entry: None,
            transition: Transition::Scene,
            source: None,
        });
        app.update();
        app.update();
        assert_eq!(app.world().resource::<ActiveMaps>().current, "2");
        // Cell (4,0) of 64 px cells: the centre is (288, 32).
        assert_eq!(player_feet(&mut app), ORIGIN + Vec2::new(288.0, 32.0));
        assert_eq!(
            app.world().resource::<Log>().0.last().unwrap(),
            "arrive None"
        );
    }

    #[test]
    fn a_request_without_an_entry_to_a_map_without_a_start_leaves_the_world_alone() {
        let mut app = app_with_two_maps();
        app.update();
        put_player(&mut app, ORIGIN + Vec2::new(300.0, 300.0));
        app.world_mut().write_message(RequestMapChange {
            to: "2".into(),
            entry: None,
            transition: Transition::Scene,
            source: None,
        });
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(app.world().resource::<ActiveMaps>().current, "1");
        assert!(app.world().resource::<MapTransition>().is_idle());
        assert_eq!(player_feet(&mut app), ORIGIN + Vec2::new(300.0, 300.0));
    }

    #[test]
    fn touching_a_trigger_sends_the_player_to_the_start_of_its_map() {
        let mut app = app_with_trigger();
        app.update();
        put_player(&mut app, ORIGIN + Vec2::new(300.0, 300.0));
        app.update();
        // The edge of the area counts as touching.
        set_feet(&mut app, ORIGIN + Vec2::new(160.0, 130.0));
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(app.world().resource::<ActiveMaps>().current, "2");
        assert_eq!(player_feet(&mut app), ORIGIN + Vec2::new(288.0, 32.0));
        assert!(app.world().resource::<MapTransition>().is_idle());
    }

    #[test]
    fn a_trigger_fires_once_and_the_player_is_armed_again_outside() {
        let mut app = app_with_trigger();
        app.update();
        put_player(&mut app, ORIGIN + Vec2::new(300.0, 300.0));
        app.update();
        set_feet(&mut app, ORIGIN + Vec2::new(130.0, 130.0));
        for _ in 0..3 {
            app.update();
        }
        let loads = |app: &App| {
            app.world()
                .resource::<Log>()
                .0
                .iter()
                .filter(|line| line.starts_with("load"))
                .count()
        };
        assert_eq!(loads(&app), 2);
        // Map 2 has no trigger. The map of the trigger is gone, so walking back in does nothing and nothing loops.
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(loads(&app), 2);
        assert!(app.world().resource::<MapTransition>().armed);
    }

    #[test]
    fn a_player_who_arrives_in_a_trigger_does_not_fire_it_at_once() {
        let mut app = app_with_trigger();
        // Map 2 has a trigger over its start cell (4,0), so the player arrives inside it.
        app.insert_resource(TriggerInMap2(Rect::from_corners(
            ORIGIN + Vec2::new(256.0, 0.0),
            ORIGIN + Vec2::new(320.0, 64.0),
        )));
        app.update();
        put_player(&mut app, ORIGIN + Vec2::new(300.0, 300.0));
        app.update();
        set_feet(&mut app, ORIGIN + Vec2::new(130.0, 130.0));
        for _ in 0..6 {
            app.update();
        }
        assert_eq!(app.world().resource::<ActiveMaps>().current, "2");
        assert!(!app.world().resource::<MapTransition>().armed);
        // It fires again only after the feet have left it.
        set_feet(&mut app, ORIGIN + Vec2::new(450.0, 300.0));
        app.update();
        assert!(app.world().resource::<MapTransition>().armed);
    }

    #[test]
    fn a_request_for_an_unknown_map_leaves_the_world_alone() {
        let mut app = app_with_two_maps();
        app.update();
        put_player(&mut app, ORIGIN + Vec2::new(300.0, 300.0));
        app.world_mut().write_message(RequestMapChange {
            to: "9".into(),
            entry: Some("x".into()),
            transition: Transition::Scene,
            source: None,
        });
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(app.world().resource::<ActiveMaps>().current, "1");
        assert_eq!(app.world().resource::<Log>().0.len(), 2);
        assert!(app.world().resource::<MapTransition>().is_idle());
        assert_eq!(player_feet(&mut app), ORIGIN + Vec2::new(300.0, 300.0));
    }

    #[test]
    fn requests_during_a_transition_are_ignored() {
        let mut app = app_with_two_maps();
        app.update();
        put_player(&mut app, ORIGIN + Vec2::new(300.0, 300.0));
        for _ in 0..2 {
            app.world_mut().write_message(RequestMapChange {
                to: "2".into(),
                entry: Some("from_1".into()),
                transition: Transition::Scene,
                source: None,
            });
        }
        app.update();
        app.update();
        let loads = app
            .world()
            .resource::<Log>()
            .0
            .iter()
            .filter(|line| line.starts_with("load"))
            .count();
        assert_eq!(loads, 2, "the start map and one transition");
    }

    #[test]
    fn a_registry_checks_its_links_and_reports_unknown_maps() {
        let one = registered(
            "1",
            "exits:\n  up: area=3..6,9 to=9 entry=from_1 transition=scene\n",
        );
        let error = MapRegistry::from_parsed(vec![one]).err().expect("an error");
        assert!(error.contains("not registered"), "{error}");
        let registry = MapRegistry::from_parsed(vec![registered("1", "")]).unwrap();
        assert!(registry.get("1").is_some() && registry.get("2").is_none());
        assert_eq!(registry.ids().collect::<Vec<_>>(), vec!["1"]);
    }

    fn map_of(w: i32, h: i32, blocks: &str) -> Map {
        let head = format!(
            "---\nmap: t\ncell: 64\norigin: bottom-left\nsize: {w}x{h}\nlegend:\n  \".\": empty\n{blocks}---\n"
        );
        let row = vec!["."; w as usize].join(" ");
        let rows: String = (0..h).rev().map(|y| format!("{y:>2} |{row}|\n")).collect();
        parse("t.txt", &format!("{head}{rows}"), &[]).unwrap()
    }

    #[test]
    fn a_seamless_map_is_placed_beside_the_exit_edge_with_the_strips_on_top_of_each_other() {
        let origin = Vec2::new(-1000.0, -500.0);
        let exit_map = |area: &str| {
            map_of(
                10,
                10,
                &format!("exits:\n  x: area={area} to=2 entry=e transition=seamless\n"),
            )
        };
        let entry_map =
            |w: i32, h: i32, area: &str| map_of(w, h, &format!("entries:\n  e: area={area}\n"));
        let place = |exit: &Map, entry: &Map| {
            seamless_origin(origin, exit, &exit.exits[0], entry, &entry.entries[0]).unwrap()
        };
        // A top exit at x 3..6 and a bottom entry at x 1..4 of a 8 by 6 map: the new map is above, shifted left by 2.
        let up = place(&exit_map("3..6,9"), &entry_map(8, 6, "1..4,0"));
        assert_eq!(up, Vec2::new(-1000.0 + 192.0 - 64.0, -500.0 + 640.0));
        // A bottom exit: the new map is below.
        let down = place(&exit_map("3..6,0"), &entry_map(8, 6, "1..4,5"));
        assert_eq!(down, Vec2::new(-1000.0 + 192.0 - 64.0, -500.0 - 6.0 * 64.0));
        // A right exit at y 4..5 and a left entry at y 2..3 of a 6 by 8 map: the new map is to the right, shifted down.
        let right = place(&exit_map("9,4..5"), &entry_map(6, 8, "0,2..3"));
        assert_eq!(right, Vec2::new(-1000.0 + 640.0, -500.0 + 256.0 - 128.0));
        // A left exit: the new map is to the left.
        let left = place(&exit_map("0,4..5"), &entry_map(6, 8, "5,2..3"));
        assert_eq!(
            left,
            Vec2::new(-1000.0 - 6.0 * 64.0, -500.0 + 256.0 - 128.0)
        );
    }

    #[test]
    fn rectangles_that_only_touch_do_not_overlap() {
        let a = Rect::new(0.0, 0.0, 640.0, 640.0);
        assert!(!rects_overlap(a, Rect::new(0.0, 640.0, 640.0, 1280.0)));
        assert!(!rects_overlap(a, Rect::new(640.0, 0.0, 1280.0, 640.0)));
        assert!(rects_overlap(a, Rect::new(0.0, 639.0, 640.0, 1279.0)));
        assert!(rects_overlap(a, Rect::new(100.0, 100.0, 200.0, 200.0)));
    }

    #[test]
    fn a_move_is_kept_inside_the_union_of_areas() {
        let low = Rect::new(0.0, 0.0, 100.0, 100.0);
        let high = Rect::new(0.0, 200.0, 100.0, 300.0);
        let bridge = Rect::new(40.0, 100.0, 60.0, 200.0);
        let areas = [low, high, bridge];
        let at = |from: Vec2, to: Vec2| clamp_to_union(&areas, from, to);
        // Inside an area, or across the bridge: unchanged.
        assert_eq!(
            at(Vec2::new(50.0, 90.0), Vec2::new(50.0, 150.0)),
            Vec2::new(50.0, 150.0)
        );
        assert_eq!(
            at(Vec2::new(10.0, 10.0), Vec2::new(20.0, 20.0)),
            Vec2::new(20.0, 20.0)
        );
        // From the bridge the feet may step into either map.
        assert_eq!(
            at(Vec2::new(50.0, 150.0), Vec2::new(10.0, 210.0)),
            Vec2::new(10.0, 210.0)
        );
        // Beside the bridge, in the gap between the two maps: pushed back to the map the feet were in.
        assert_eq!(
            at(Vec2::new(10.0, 100.0), Vec2::new(10.0, 105.0)),
            Vec2::new(10.0, 100.0)
        );
        // A long step from the lower map into the gap does not jump the wall into the upper map, even though the upper
        // map is nearer. It ends in the corridor at most.
        assert_eq!(
            at(Vec2::new(10.0, 90.0), Vec2::new(10.0, 190.0)),
            Vec2::new(40.0, 190.0)
        );
        // Past the outer edge: pushed back inside.
        assert_eq!(
            at(Vec2::new(5.0, 50.0), Vec2::new(-30.0, 50.0)),
            Vec2::new(0.0, 50.0)
        );
        // No areas, or feet that were nowhere: the nearest point of any area.
        assert_eq!(
            clamp_to_union(&[], Vec2::ZERO, Vec2::new(5.0, 5.0)),
            Vec2::new(5.0, 5.0)
        );
        assert_eq!(
            at(Vec2::new(500.0, 500.0), Vec2::new(10.0, 120.0)),
            Vec2::new(10.0, 100.0)
        );
    }

    fn active_pair() -> ActiveMaps {
        // Two 640 by 640 maps, the second above the first, joined by a doorway at x 192..448 on the top edge.
        ActiveMaps {
            current: "1".into(),
            maps: vec![
                ActiveMap {
                    id: "1".into(),
                    origin: Vec2::ZERO,
                    rect: Rect::new(0.0, 0.0, 640.0, 640.0),
                },
                ActiveMap {
                    id: "2".into(),
                    origin: Vec2::new(0.0, 640.0),
                    rect: Rect::new(0.0, 640.0, 640.0, 1280.0),
                },
            ],
            doorways: vec![Doorway {
                from: "1".into(),
                to: "2".into(),
                edge: Edge::Top,
                area: Rect::new(192.0, 576.0, 448.0, 640.0),
            }],
        }
    }

    #[test]
    fn the_player_can_cross_a_seam_only_through_the_doorway() {
        let active = active_pair();
        // The body is 14 wide on each side and 50 tall, as in Long Way.
        let areas = active.walkable(Vec2::new(14.0, 0.0), Vec2::new(14.0, 50.0));
        let walk = |from: Vec2, to: Vec2| clamp_to_union(&areas, from, to);
        // In the doorway, crossing from the lower map to the upper one in small steps: free.
        for (from, to) in [
            (Vec2::new(300.0, 580.0), Vec2::new(300.0, 600.0)),
            (Vec2::new(300.0, 600.0), Vec2::new(300.0, 640.0)),
            (Vec2::new(300.0, 640.0), Vec2::new(300.0, 700.0)),
        ] {
            assert_eq!(walk(from, to), to, "{from} to {to}");
        }
        // Beside the doorway the top of the lower map is a wall, 50 below the edge, also for a long step.
        assert_eq!(
            walk(Vec2::new(100.0, 585.0), Vec2::new(100.0, 630.0)),
            Vec2::new(100.0, 590.0)
        );
        assert_eq!(
            walk(Vec2::new(100.0, 585.0), Vec2::new(100.0, 700.0)),
            Vec2::new(100.0, 590.0)
        );
        // The doorway is narrower than the strip by the body width on each side.
        assert_eq!(
            walk(Vec2::new(230.0, 585.0), Vec2::new(195.0, 620.0)).x,
            192.0 + 14.0
        );
        // The upper map has its own walls: not past its top.
        assert_eq!(
            walk(Vec2::new(300.0, 1200.0), Vec2::new(300.0, 1400.0)),
            Vec2::new(300.0, 1230.0)
        );
        // Without the doorway (the neighbour was removed) the seam is a wall again.
        let mut closed = active;
        closed.doorways.clear();
        closed.maps.pop();
        let areas = closed.walkable(Vec2::new(14.0, 0.0), Vec2::new(14.0, 50.0));
        assert_eq!(
            clamp_to_union(&areas, Vec2::new(300.0, 585.0), Vec2::new(300.0, 700.0)),
            Vec2::new(300.0, 590.0)
        );
    }

    #[test]
    fn a_doorway_on_a_side_edge_bridges_sideways() {
        let active = ActiveMaps {
            current: "1".into(),
            maps: vec![
                ActiveMap {
                    id: "1".into(),
                    origin: Vec2::ZERO,
                    rect: Rect::new(0.0, 0.0, 640.0, 640.0),
                },
                ActiveMap {
                    id: "2".into(),
                    origin: Vec2::new(640.0, 0.0),
                    rect: Rect::new(640.0, 0.0, 1280.0, 640.0),
                },
            ],
            doorways: vec![Doorway {
                from: "1".into(),
                to: "2".into(),
                edge: Edge::Right,
                area: Rect::new(576.0, 256.0, 640.0, 384.0),
            }],
        };
        let areas = active.walkable(Vec2::new(14.0, 0.0), Vec2::new(14.0, 50.0));
        let through = Vec2::new(640.0, 300.0);
        assert_eq!(
            clamp_to_union(&areas, Vec2::new(620.0, 300.0), through),
            through
        );
        assert_eq!(
            clamp_to_union(&areas, Vec2::new(620.0, 500.0), Vec2::new(640.0, 500.0)).x,
            640.0 - 14.0
        );
    }

    #[test]
    fn the_bounds_box_holds_every_active_map() {
        let active = active_pair();
        assert_eq!(active.bounding_rect(), Rect::new(0.0, 0.0, 640.0, 1280.0));
        assert_eq!(active.map_at(Vec2::new(10.0, 639.9)).unwrap().id, "1");
        assert_eq!(active.map_at(Vec2::new(10.0, 640.0)).unwrap().id, "2");
        assert!(active.map_at(Vec2::new(700.0, 10.0)).is_none());
    }

    #[test]
    fn the_camera_shows_the_current_map_and_looks_ahead_only_in_the_exit() {
        let active = active_pair();
        let first = Rect::new(0.0, 0.0, 640.0, 640.0);
        let both = Rect::new(0.0, 0.0, 640.0, 1280.0);
        let second = Rect::new(0.0, 640.0, 640.0, 1280.0);
        // In the first map, away from the exit: only the first map.
        assert_eq!(camera_target(&active, Some(Vec2::new(300.0, 300.0))), first);
        // In the exit (x 192..448, the top row): the box holds the joined map as well.
        assert_eq!(camera_target(&active, Some(Vec2::new(300.0, 600.0))), both);
        // Beside the exit, in the same row: not yet.
        assert_eq!(camera_target(&active, Some(Vec2::new(100.0, 600.0))), first);
        // No player: the current map.
        assert_eq!(camera_target(&active, None), first);
        // Across the seam the second map is the current one and the box is that map alone: the map that was left is
        // not in it, even though it is still loaded.
        let mut crossed = active;
        crossed.current = "2".into();
        assert_eq!(
            camera_target(&crossed, Some(Vec2::new(300.0, 700.0))),
            second
        );
        assert_eq!(
            camera_target(&crossed, Some(Vec2::new(300.0, 600.0))),
            second
        );
    }

    /// Spawns a camera that sees the area around `centre`, 200 by 200 units.
    fn put_camera(app: &mut App, centre: Vec2) -> Entity {
        app.world_mut()
            .spawn((
                Camera2d,
                GlobalTransform::from(Transform::from_translation(centre.extend(0.0))),
                Projection::Orthographic(OrthographicProjection {
                    area: Rect::new(-100.0, -100.0, 100.0, 100.0),
                    ..OrthographicProjection::default_2d()
                }),
            ))
            .id()
    }

    #[test]
    fn a_seamless_exit_joins_the_next_map_without_a_fade_or_a_lock() {
        let mut app = app_with("seamless");
        app.update();
        put_camera(&mut app, ORIGIN + Vec2::new(320.0, 300.0));
        put_player(&mut app, ORIGIN + Vec2::new(300.0, 300.0));
        app.update();
        set_feet(&mut app, ORIGIN + Vec2::new(192.0 + 58.0, 600.0));
        app.update();
        let log = app.world().resource::<Log>().0.clone();
        // The start map, then the second map at the top of the first, aligned strip on strip (both start at x 3).
        assert_eq!(
            log[2],
            format!("load 2 at {}", ORIGIN + Vec2::new(0.0, 640.0)),
            "{log:?}"
        );
        assert!(log[3].starts_with("rebuild with 2 map"), "{log:?}");
        assert_eq!(log.len(), 4, "no arrive hook: {log:?}");
        let active = app.world().resource::<ActiveMaps>();
        assert_eq!(active.maps.len(), 2);
        assert_eq!(active.current, "1", "the player is still in the first map");
        assert_eq!(active.doorways.len(), 1);
        assert_eq!(active.doorways[0].edge, Edge::Top);
        assert!(!app.world().resource::<MapTransition>().locks_input());
        assert!(app.world().resource::<MapTransition>().is_idle());
        // The player was not moved.
        assert_eq!(
            player_feet(&mut app),
            ORIGIN + Vec2::new(192.0 + 58.0, 600.0)
        );
        // The player is in the exit, so the camera limits are the box around both maps.
        assert_eq!(
            app.world().resource::<CameraBounds>().0,
            Rect::from_corners(ORIGIN, ORIGIN + Vec2::new(640.0, 1280.0))
        );
    }

    #[test]
    fn crossing_the_seam_changes_the_current_map_and_the_old_one_goes_when_out_of_view() {
        let mut app = app_with("seamless");
        app.update();
        let camera = put_camera(&mut app, ORIGIN + Vec2::new(320.0, 600.0));
        put_player(&mut app, ORIGIN + Vec2::new(300.0, 300.0));
        app.update();
        set_feet(&mut app, ORIGIN + Vec2::new(250.0, 600.0));
        app.update();
        assert_eq!(app.world().resource::<ActiveMaps>().maps.len(), 2);
        // Across the seam: the second map holds the feet. Both maps are in view, so nothing is removed.
        set_feet(&mut app, ORIGIN + Vec2::new(250.0, 700.0));
        app.update();
        assert_eq!(app.world().resource::<ActiveMaps>().current, "2");
        app.update();
        let second = Rect::from_corners(
            ORIGIN + Vec2::new(0.0, 640.0),
            ORIGIN + Vec2::new(640.0, 1280.0),
        );
        assert_eq!(
            app.world().resource::<CameraBounds>().0,
            second,
            "after the crossing the camera limits head for the new map alone"
        );
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<ActiveMaps>().maps.len(),
            2,
            "the first map is still in view"
        );
        // The camera moves up: the first map is out of view. It goes after the grace time. Virtual time takes at most
        // 0.25 s per update, so that is a few updates.
        app.world_mut()
            .entity_mut(camera)
            .insert(GlobalTransform::from(Transform::from_translation(
                (ORIGIN + Vec2::new(320.0, 1100.0)).extend(0.0),
            )));
        app.update();
        assert_eq!(
            app.world().resource::<ActiveMaps>().maps.len(),
            2,
            "not yet, the grace time has not passed"
        );
        for _ in 0..6 {
            app.update();
        }
        let active = app.world().resource::<ActiveMaps>();
        assert_eq!(active.maps.len(), 1);
        assert_eq!(active.maps[0].id, "2");
        assert!(active.doorways.is_empty());
        let ids: Vec<String> = app
            .world_mut()
            .query::<&MapId>()
            .iter(app.world())
            .map(|id| id.0.clone())
            .collect();
        assert_eq!(ids, vec!["2".to_string()], "only the second map is left");
        assert!(
            app.world()
                .resource::<Log>()
                .0
                .last()
                .unwrap()
                .starts_with("rebuild with 1 map")
        );
        assert_eq!(
            app.world().resource::<CameraBounds>().0,
            Rect::from_corners(
                ORIGIN + Vec2::new(0.0, 640.0),
                ORIGIN + Vec2::new(640.0, 1280.0)
            )
        );
    }

    #[test]
    fn walking_back_through_the_doorway_keeps_the_first_map_and_does_not_load_it_twice() {
        let mut app = app_with("seamless");
        app.update();
        put_camera(&mut app, ORIGIN + Vec2::new(320.0, 600.0));
        put_player(&mut app, ORIGIN + Vec2::new(300.0, 300.0));
        app.update();
        set_feet(&mut app, ORIGIN + Vec2::new(250.0, 600.0));
        app.update();
        set_feet(&mut app, ORIGIN + Vec2::new(250.0, 700.0));
        app.update();
        // Back into the first map, out of its exit and into it again: the second map is already there.
        set_feet(&mut app, ORIGIN + Vec2::new(250.0, 300.0));
        app.update();
        assert_eq!(app.world().resource::<ActiveMaps>().current, "1");
        set_feet(&mut app, ORIGIN + Vec2::new(250.0, 600.0));
        app.update();
        let loads = app
            .world()
            .resource::<Log>()
            .0
            .iter()
            .filter(|line| line.starts_with("load 2"))
            .count();
        assert_eq!(loads, 1);
        assert_eq!(app.world().resource::<ActiveMaps>().doorways.len(), 1);
    }

    #[test]
    fn a_seamless_map_that_would_overlap_another_is_refused() {
        // Two exits of map 1 lead to maps that both sit above it: the second one overlaps the first.
        let one = registered(
            "1",
            "exits:\n  a: area=0..1,9 to=2 entry=e transition=seamless\n  b: area=8..9,9 to=3 entry=e transition=seamless\n",
        );
        let two = registered("2", "entries:\n  e: area=0..1,0\n");
        let three = registered("3", "entries:\n  e: area=8..9,0\n");
        let registry = MapRegistry::from_parsed(vec![one, two, three]).unwrap();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Log>()
            .insert_resource(registry)
            .insert_resource(ActiveMaps {
                current: "1".into(),
                ..default()
            })
            .insert_resource(MapTransition::new(Fade::default(), ORIGIN));
        let hooks = MapHooks {
            load: app.world_mut().register_system(load_hook),
            rebuild: app.world_mut().register_system(rebuild_hook),
            arrive: app.world_mut().register_system(arrive_hook),
        };
        app.insert_resource(hooks);
        install_systems(&mut app);
        app.update();
        put_player(&mut app, ORIGIN + Vec2::new(40.0, 300.0));
        app.update();
        set_feet(&mut app, ORIGIN + Vec2::new(40.0, 600.0));
        app.update();
        assert_eq!(app.world().resource::<ActiveMaps>().maps.len(), 2);
        // Back down, out of the exit, then into the second exit: map 3 would be placed where map 2 already is.
        set_feet(&mut app, ORIGIN + Vec2::new(300.0, 300.0));
        app.update();
        set_feet(&mut app, ORIGIN + Vec2::new(560.0, 600.0));
        app.update();
        let active = app.world().resource::<ActiveMaps>();
        assert_eq!(active.maps.len(), 2, "map 3 is refused");
        assert!(active.get("3").is_none());
    }
}
