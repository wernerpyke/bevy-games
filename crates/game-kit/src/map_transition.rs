//! Moving between maps. A game registers its maps, the kit runs the lifecycle: the player walks into an exit zone, the
//! screen fades out, the old map is removed, the new one is loaded, the player is placed in the entry and the screen
//! fades in. See `dev-docs/PLAN-map-transitions.md` and `dev-docs/adr/EXTENSIONS.md` adr-x31.
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
use crate::camera::CameraBounds;
use crate::collision::register_blockers;
use crate::debug::DebugInfo;
use crate::depth::DepthSort;
use crate::layers::FADE_Z_INDEX;
use crate::map::{Area, Facing, Map, MapExit, Transition, parse};
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

/// The maps in the world, and the one that holds the player.
#[derive(Resource, Default, Debug)]
pub struct ActiveMaps {
    pub current: String,
    pub maps: Vec<ActiveMap>,
}

impl ActiveMaps {
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
    pub entry: String,
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
}

impl MapTransition {
    fn new(fade: Fade, origin: Vec2) -> Self {
        MapTransition {
            phase: Phase::Idle,
            fade,
            origin,
            armed: false,
        }
    }

    pub fn is_idle(&self) -> bool {
        matches!(self.phase, Phase::Idle)
    }

    /// True while the screen fades or the maps are swapped: the player must not move.
    pub fn locks_input(&self) -> bool {
        !self.is_idle()
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
        Phase::Idle => 0.0,
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
                maps: Vec::new(),
            })
            .insert_resource(MapTransition::new(self.fade, self.origin));
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

/// Starts a transition when the feet of the controlled entity enter an exit.
fn detect_exit(
    mut transition: ResMut<MapTransition>,
    active: Res<ActiveMaps>,
    registry: Res<MapRegistry>,
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
    match exit_at(&registered.map, current.origin, feet) {
        None => transition.armed = true,
        Some(exit) if transition.armed => {
            transition.armed = false;
            info!(
                "maps: exit `{}` of `{}` used, going to `{}` entry `{}`",
                exit.name, current.id, exit.to, exit.entry
            );
            requests.write(RequestMapChange {
                to: exit.to.clone(),
                entry: exit.entry.clone(),
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
        if transition.is_idle() {
            if request.transition == Transition::Seamless {
                warn!("maps: a seamless transition is not built yet, using a scene transition");
            }
            transition.phase = Phase::FadeOut {
                elapsed: 0.0,
                request: request.clone(),
            };
        }
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
    let entry = to
        .map
        .entries
        .iter()
        .find(|entry| entry.name == request.entry)
        .ok_or_else(|| format!("map `{}` has no entry `{}`", request.to, request.entry))?;
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
    };
    world.insert_resource(CameraBounds(rect));
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
    if let Err(error) = world.run_system(hooks.rebuild) {
        error!("maps: the rebuild hook failed: {error}");
    }
}

/// Loads the first map after the game has set up its own start up systems.
fn load_start(world: &mut World) {
    let id = world.resource::<ActiveMaps>().current.clone();
    let origin = world.resource::<MapTransition>().origin;
    install_map(world, &id, origin);
}

/// Reads the current map file again and loads it again, for the F1 key. The controlled entity stays where it is. On an
/// error in the file nothing changes.
pub fn reload_current_map(world: &mut World) -> Result<(), String> {
    let (id, origin) = {
        let active = world.resource::<ActiveMaps>();
        let current = active
            .current_map()
            .ok_or_else(|| "no map is loaded".to_string())?;
        (current.id.clone(), current.origin)
    };
    world.resource_mut::<MapRegistry>().reload(&id)?;
    unload_all(world);
    install_map(world, &id, origin);
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

/// Shows the id of the current map in the F2 info box.
fn show_map_id(active: Res<ActiveMaps>, info: Option<ResMut<DebugInfo>>) {
    if let Some(mut info) = info {
        info.set("map", active.current.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::parse;

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
            entry: "e".into(),
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

    fn load_hook(In(load): In<MapLoad>, mut log: ResMut<Log>, mut commands: Commands) {
        log.0.push(format!("load {} at {}", load.id, load.origin));
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
        let one = registered(
            "1",
            "exits:\n  up: area=3..6,9 to=2 entry=from_1 transition=scene\n",
        );
        let two = registered("2", "entries:\n  from_1: area=3..6,0 facing=up\n");
        let registry = MapRegistry::from_parsed(vec![one, two]).unwrap();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Log>()
            .insert_resource(registry)
            .insert_resource(ActiveMaps {
                current: "1".into(),
                maps: Vec::new(),
            })
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
            entry: "from_1".into(),
            transition: Transition::Scene,
            source: None,
        });
        app.update();
        app.update();
        assert_eq!(app.world().resource::<ActiveMaps>().current, "2");
        assert_eq!(player_feet(&mut app), ORIGIN + Vec2::new(320.0, 32.0));
    }

    #[test]
    fn a_request_for_an_unknown_map_leaves_the_world_alone() {
        let mut app = app_with_two_maps();
        app.update();
        put_player(&mut app, ORIGIN + Vec2::new(300.0, 300.0));
        app.world_mut().write_message(RequestMapChange {
            to: "9".into(),
            entry: "x".into(),
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
                entry: "from_1".into(),
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
}
