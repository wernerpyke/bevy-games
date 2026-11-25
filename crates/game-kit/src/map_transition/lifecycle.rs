//! The maps in the world and their lifecycle: `MapId`, `MapRoot`, the game's hooks, loading, unloading and the F1
//! reload. `MapLifecyclePlugin` is this alone, for a game with one map and no exits (R-Town).

use super::*;

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

/// The map an entity belongs to. The kit removes every entity with a `MapId` when that map goes. Everything the `load`
/// hook spawns gets it from the kit (`MapRoot`). An entity spawned later, during play, gets it in one of two ways
/// (adr-x60): what the player causes (a weapon, a shot) requires
/// `OnCurrentMap`; what a map object spawns copies that object's `MapId` (`SpawnedBy` does it for a spawner, the Long Way
/// volcano by hand for its shots), because with seamless
/// joins the object can be in a neighbour map. The controlled entity does not have it: it stays through a transition.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct MapId(pub String);

/// An entity that belongs to the map the player is in when it appears: the kit gives it that map's `MapId`
/// (`ActiveMaps::current`), so a map change or F1 removes it with the map. Require it on a type that is spawned during
/// play (`#[require(OnCurrentMap)]`). A `MapId` the entity is spawned with wins.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct OnCurrentMap;

/// Gives a new `OnCurrentMap` entity the current map's `MapId`, unless it already has one.
fn bind_to_current_map(
    add: On<Add<OnCurrentMap>>,
    maps: Res<ActiveMaps>,
    has_map: Query<(), With<MapId>>,
    mut commands: Commands,
) {
    if !has_map.contains(add.entity) {
        commands
            .entity(add.entity)
            .insert(MapId(maps.current.clone()));
    }
}

/// The root of one loaded map, with the map's `MapId`. The kit spawns it before the `load` hook runs, then makes every
/// top-level entity the hook spawned its child and gives it the same `MapId` (adr-x58). Removing the map despawns the
/// root, and so everything in it. It sits at the world origin, so a child's `Transform` is its world position.
#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub struct MapRoot;

/// What the `load` hook gets: spawn the objects of this map with their cells counted from `origin`. The kit adopts
/// them into `root` after the hook.
#[derive(Clone, Debug)]
pub struct MapLoad {
    pub id: String,
    pub origin: Vec2,
    pub root: Entity,
}

/// What the `arrive` hook gets: the feet position in the world, and the way the player should look.
#[derive(Clone, Copy, Debug)]
pub struct Arrival {
    pub feet: Vec2,
    pub facing: Option<Facing>,
}

/// The systems a game gives the kit. Register them with `World::register_system`. `rebuild` is usually
/// `collision::rebuild_blockers`. `arrive` places the controlled entity after a scene transition; a game with one map
/// and no exits has none.
#[derive(Resource, Clone, Copy)]
pub struct MapHooks {
    pub load: SystemId<In<MapLoad>>,
    pub rebuild: SystemId,
    pub arrive: Option<SystemId<In<Arrival>>>,
}

/// World position of cell (0,0) of a map that is loaded on its own: the start map, after F1, or after a scene transition.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct MapOrigin(pub Vec2);

/// The map lifecycle alone: the game registers its maps, the kit loads the start map in `PostStartup` (after
/// `rebuild_blockers`), adopts what the `load` hook spawns into a `MapRoot` (adr-x58), runs the `rebuild` hook, sets the
/// camera limits, and reloads the current map on `reload_current_map` (F1). A game with one map and no exits uses this
/// alone; `MapTransitionPlugin` adds exits, fades and seamless joins on top of it.
pub struct MapLifecyclePlugin {
    pub sources: &'static [MapSource],
    /// The legend names the game's maps may use.
    pub kinds: &'static [&'static str],
    /// The id of the map the game starts in.
    pub start: String,
    /// World position of cell (0,0) of the start map (`MapOrigin`).
    pub origin: Vec2,
}

impl Plugin for MapLifecyclePlugin {
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
            .insert_resource(MapOrigin(self.origin));
        install_lifecycle(app);
    }
}

/// The systems of the lifecycle, for `MapLifecyclePlugin` and for tests that insert the resources themselves.
pub(super) fn install_lifecycle(app: &mut App) {
    app.add_systems(PostStartup, load_start.after(rebuild_blockers))
        .add_systems(Update, (sync_debug_grids, show_map_id))
        .add_observer(bind_to_current_map);
}

/// Removes every entity with a `MapId`.
pub(super) fn unload_all(world: &mut World) {
    let old: Vec<Entity> = world
        .query_filtered::<Entity, With<MapId>>()
        .iter(world)
        .collect();
    despawn_all(world, old);
}

/// Removes the entities of one map.
pub(super) fn unload_entities(world: &mut World, id: &str) {
    let old: Vec<Entity> = world
        .query::<(Entity, &MapId)>()
        .iter(world)
        .filter(|(_, map)| map.0 == id)
        .map(|(entity, _)| entity)
        .collect();
    despawn_all(world, old);
}

/// Despawns each entity that still exists. A child of a `MapRoot` has its own `MapId` and is already gone with its root.
pub(super) fn despawn_all(world: &mut World, entities: Vec<Entity>) {
    for entity in entities {
        if let Ok(entity) = world.get_entity_mut(entity) {
            entity.despawn();
        }
    }
}

/// Runs the `load` hook for one map, inside a new `MapRoot` that adopts what the hook spawns.
pub(super) fn run_load(world: &mut World, id: &str, origin: Vec2) {
    let Some(hooks) = world.get_resource::<MapHooks>().copied() else {
        error!("maps: the game has not inserted `MapHooks`");
        return;
    };
    let root = world.spawn((MapRoot, MapId(id.to_string()))).id();
    let before: HashSet<Entity> = world.query::<Entity>().iter(world).collect();
    let load = MapLoad {
        id: id.to_string(),
        origin,
        root,
    };
    if let Err(error) = world.run_system_with(hooks.load, load) {
        error!("maps: the load hook failed: {error}");
    }
    adopt_new_entities(world, root, id, &before);
}

/// New top-level entities that belong to the map: not the engine's own (systems, observers, and resources, which are
/// entities too and must never be despawned with a map).
pub(super) type Adoptable = (
    Without<ChildOf>,
    Without<MapRoot>,
    Without<Observer>,
    Without<SystemIdMarker>,
    Without<IsResource>,
);

/// Makes each top-level entity that is not in `before` a child of `root`, with the map's `MapId`.
pub(super) fn adopt_new_entities(
    world: &mut World,
    root: Entity,
    id: &str,
    before: &HashSet<Entity>,
) {
    let new: Vec<Entity> = world
        .query_filtered::<Entity, Adoptable>()
        .iter(world)
        .filter(|entity| !before.contains(entity))
        .collect();
    for entity in new {
        let mut adopted = world.entity_mut(entity);
        // An `OnCurrentMap` entity spawned by the hook took the current map from its observer; it belongs to the
        // map being loaded, which differs from the current one when a seamless neighbour loads.
        if !adopted.contains::<MapId>() || adopted.contains::<OnCurrentMap>() {
            adopted.insert(MapId(id.to_string()));
        }
        world.entity_mut(root).add_child(entity);
    }
}

/// Runs the `rebuild` hook: the game fills the collision world from the entities that exist now.
pub(super) fn run_rebuild(world: &mut World) {
    let Some(hooks) = world.get_resource::<MapHooks>().copied() else {
        return;
    };
    if let Err(error) = world.run_system(hooks.rebuild) {
        error!("maps: the rebuild hook failed: {error}");
    }
}

/// Puts the camera limits on the current map at once, and makes the camera jump with them (a scene transition, the
/// screen is covered). A seamless transition changes the limits in `retarget_camera` and the camera glides.
pub(super) fn snap_bounds(world: &mut World) {
    let rect = world.resource::<ActiveMaps>().current_rect();
    world.insert_resource(CameraBounds(rect));
    if let Some(mut smoothing) = world.get_resource_mut::<CameraSmoothing>() {
        smoothing.snap = true;
    }
}

/// Makes `id` the only active map at `origin`, sets the camera limits and runs the `load` and `rebuild` hooks.
pub(super) fn install_map(world: &mut World, id: &str, origin: Vec2) {
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
    snap_bounds(world);
    run_load(world, id, origin);
    run_rebuild(world);
}

/// Loads the first map after the game has set up its own start up systems.
pub(super) fn load_start(world: &mut World) {
    let id = world.resource::<ActiveMaps>().current.clone();
    let origin = world.resource::<MapOrigin>().0;
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

/// Gives the debug grid one region per active map, so each map has its own grid and its own numbers.
pub(super) fn sync_debug_grids(active: Res<ActiveMaps>, grids: Option<ResMut<DebugGrids>>) {
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
pub(super) fn show_map_id(active: Res<ActiveMaps>, info: Option<ResMut<DebugInfo>>) {
    if let Some(mut info) = info {
        info.set("map", active.current.clone());
    }
}
