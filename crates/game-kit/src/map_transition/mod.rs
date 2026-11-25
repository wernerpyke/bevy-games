//! Moving between maps. A game registers its maps, the kit runs the lifecycle: the player walks into an exit zone, the
//! screen fades out, the old map is removed, the new one is loaded, the player is placed in the entry and the screen
//! fades in. See `dev-docs/extensions/map-transitions.md` and `dev-docs/adr/EXTENSIONS.md` adr-x31.
//!
//! The kit has no game types. The game gives it three one-shot systems (`MapHooks`):
//! - `load` spawns the objects of one map. The kit makes every entity it spawns a child of the map's `MapRoot`, with the
//!   map's `MapId`, so the kit removes them with the map and the game tags nothing by hand (adr-x58).
//! - `rebuild` rebuilds the collision world from the entities that exist after a load or unload.
//! - `arrive` puts the controlled entity in place after a transition and sets its facing.
//!
//! Hooks run in a fixed order inside one frame, while the screen is covered, so no frame shows half a map. The kit
//! reads the exits and entries of the maps (`map_areas.rs`) and keeps `ActiveMaps` (the maps in the world and where
//! each one is) up to date. A transition is asked for with a `RequestMapChange` message. The kit sends it when the
//! feet of the `Controlled` entity enter an exit area, and the script tool sends it for `goto`.
//!
//! Only the scene transition (fade) is built so far. A seamless exit uses the same fade for now.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use bevy::ecs::observer::Observer;
use bevy::ecs::resource::IsResource;
use bevy::ecs::system::{SystemId, SystemIdMarker};
use bevy::prelude::*;

use crate::Controlled;
use crate::camera::{CameraBounds, CameraSmoothing, GameCamera, visible_rect};
use crate::collision::rebuild_blockers;
use crate::debug::{DebugGrids, DebugInfo, GridRegion};
use crate::depth::DepthSort;
use crate::layers::FADE_Z_INDEX;
use crate::map::{Area, Edge, Facing, Map, MapEntry, MapExit, Transition, parse, start_feet};
use crate::map_edge::{EdgeFit, edge_start_feet};
use crate::map_links::{LinkedMap, validate_links};

mod lifecycle;
mod registry;
mod seamless;
mod transition;

pub use lifecycle::*;
pub use registry::*;
pub use seamless::*;
pub use transition::*;
// The items the modules share with each other and with the tests (`pub(super)`).
#[allow(unused_imports)]
use {lifecycle::*, registry::*, seamless::*, transition::*};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{PLAYER_START, parse};
    use crate::test_util::{empty_map, empty_map_text, timed_app};

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
    fn only_the_fade_and_the_swap_lock_the_player() {
        let mut transition = MapTransition::new(FADE);
        for (phase, locked) in [
            (Phase::Idle, false),
            (
                Phase::FadeOut {
                    elapsed: 0.0,
                    request: request(),
                },
                true,
            ),
            (Phase::Switch { request: request() }, true),
            (Phase::FadeIn { elapsed: 0.0 }, true),
            (Phase::Seamless { request: request() }, false),
        ] {
            let name = format!("{phase:?}");
            transition.phase = phase;
            assert_eq!(transition.locks_input(), locked, "{name}");
        }
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
        empty_map(
            64,
            10,
            10,
            "exits:\n  up: area=3..6,9 to=2 entry=e transition=scene\n",
        )
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

    /// A resource the test load hook inserts, as a game keeps the state of the map it loaded.
    #[derive(Resource)]
    struct LastLoad(String);

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
            commands.spawn(ExitTrigger {
                area: extra.0,
                to: "1".into(),
            });
        }
        // No `MapId`: the kit adopts it into the map root. The resource is an entity too, and must not be adopted.
        commands.spawn(Transform::default());
        commands.insert_resource(LastLoad(load.id.clone()));
    }

    fn rebuild_hook(mut log: ResMut<Log>, maps: Query<&MapId, Without<MapRoot>>) {
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

    fn registered_sized(id: &str, cell: u32, w: i32, h: i32, blocks: &str) -> RegisteredMap {
        RegisteredMap {
            id: id.into(),
            file: format!("{id}.txt"),
            map: parse("t.txt", &empty_map_text(id, cell, w, h, blocks), &[]).unwrap(),
        }
    }

    /// A 10 by 10 map of 64 unit cells.
    fn registered(id: &str, blocks: &str) -> RegisteredMap {
        registered_sized(id, 64, 10, 10, blocks)
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
        let mut app = timed_app(std::time::Duration::from_millis(600));
        app.init_resource::<Log>()
            .insert_resource(registry)
            .insert_resource(ActiveMaps {
                current: "1".into(),
                ..default()
            })
            .insert_resource(MapOrigin(ORIGIN))
            .insert_resource(MapTransition::new(Fade {
                colour: Color::BLACK,
                out_secs: 0.0,
                in_secs: 0.0,
            }));
        let hooks = MapHooks {
            load: app.world_mut().register_system(load_hook),
            rebuild: app.world_mut().register_system(rebuild_hook),
            arrive: Some(app.world_mut().register_system(arrive_hook)),
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
        sort.feet(transform)
    }

    fn set_feet(app: &mut App, feet: Vec2) {
        let mut query = app
            .world_mut()
            .query_filtered::<&mut Transform, With<Controlled>>();
        let mut transform = query.single_mut(app.world_mut()).unwrap();
        transform.translation.x = feet.x;
        transform.translation.y = feet.y + FEET_OFFSET;
    }

    /// The index of the first line of the log, from index `from` on, that starts with `prefix`.
    fn line_after(log: &[String], from: usize, prefix: &str) -> Option<usize> {
        (from..log.len()).find(|&i| log[i].starts_with(prefix))
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
    fn the_lifecycle_alone_loads_the_start_map_into_a_root_without_transitions_or_an_arrive_hook() {
        let mut app = timed_app(std::time::Duration::from_millis(16));
        app.init_resource::<Log>()
            .insert_resource(MapRegistry::from_parsed(vec![registered("solo", "")]).unwrap())
            .insert_resource(ActiveMaps {
                current: "solo".into(),
                ..default()
            })
            .insert_resource(MapOrigin(Vec2::new(-48.0, 96.0)));
        let hooks = MapHooks {
            load: app.world_mut().register_system(load_hook),
            rebuild: app.world_mut().register_system(rebuild_hook),
            arrive: None,
        };
        app.insert_resource(hooks);
        install_lifecycle(&mut app);
        app.update();
        app.update();
        assert!(app.world().get_resource::<MapTransition>().is_none());
        let log = &app.world().resource::<Log>().0;
        assert_eq!(
            log[..],
            [
                "load solo at [-48, 96]".to_string(),
                "rebuild with 1 map entities".to_string()
            ]
        );
        let roots = app
            .world_mut()
            .query_filtered::<&MapId, With<MapRoot>>()
            .iter(app.world())
            .map(|id| id.0.clone())
            .collect::<Vec<_>>();
        assert_eq!(roots, ["solo".to_string()]);
        assert_eq!(
            app.world().resource::<CameraBounds>().0,
            Rect::from_corners(Vec2::new(-48.0, 96.0), Vec2::new(592.0, 736.0))
        );
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
        // Load the new map, rebuild with the old map gone, then arrive.
        let load = line_after(&log, 0, "load 2").expect("map 2 is loaded");
        assert_eq!(log[load], format!("load 2 at {ORIGIN}"));
        let rebuild = line_after(&log, load, "rebuild").expect("a rebuild after the load");
        assert_eq!(log[rebuild], "rebuild with 1 map entities", "{log:?}");
        let arrive = line_after(&log, rebuild, "arrive").expect("an arrival after the rebuild");
        assert_eq!(log[arrive], "arrive Some(Up)");
        assert_eq!(line_after(&log, load + 1, "load"), None, "{log:?}");
        assert_eq!(app.world().resource::<ActiveMaps>().current, "2");
        assert!(app.world().resource::<MapTransition>().is_idle());
        // The same x along the strip, and the middle of the bottom row of the new map.
        assert_eq!(player_feet(&mut app), ORIGIN + Vec2::new(250.0, 32.0));
        // The old map is gone. The new one is a root, and the entity its hook spawned without a `MapId` is the root's
        // child with the map's `MapId`.
        let mut maps = app
            .world_mut()
            .query::<(Entity, &MapId, Option<&ChildOf>, Has<MapRoot>)>();
        let entities: Vec<_> = maps.iter(app.world()).collect();
        let [(root, root_id, None, true)] = entities
            .iter()
            .copied()
            .filter(|(.., is_root)| *is_root)
            .collect::<Vec<_>>()[..]
        else {
            panic!("one map root: {entities:?}");
        };
        assert_eq!(root_id.0, "2");
        let children: Vec<(String, Option<Entity>)> = entities
            .iter()
            .filter(|(.., is_root)| !is_root)
            .map(|(_, id, parent, _)| (id.0.clone(), parent.map(|p| p.parent())))
            .collect();
        assert_eq!(children, vec![("2".to_string(), Some(root))]);
        // The resource the hook inserted was not adopted: it outlived map 1 and holds the id of map 2.
        assert_eq!(app.world().resource::<LastLoad>().0, "2");
    }

    /// The `MapId` of every entity with `OnCurrentMap`, sorted.
    fn on_current_map(app: &mut App) -> Vec<String> {
        let mut ids: Vec<String> = app
            .world_mut()
            .query_filtered::<&MapId, With<OnCurrentMap>>()
            .iter(app.world())
            .map(|id| id.0.clone())
            .collect();
        ids.sort();
        ids
    }

    #[test]
    fn an_entity_spawned_during_play_belongs_to_the_current_map_and_goes_with_it() {
        let mut app = app_with_two_maps();
        app.update();
        put_player(&mut app, ORIGIN + Vec2::new(300.0, 300.0));
        app.world_mut().spawn(OnCurrentMap);
        // A `MapId` it is spawned with wins.
        app.world_mut().spawn((OnCurrentMap, MapId("2".into())));
        app.update();
        assert_eq!(on_current_map(&mut app), ["1", "2"]);
        // Into the exit to map 2. A scene transition unloads every map, so both go, and a new one belongs to map 2.
        set_feet(&mut app, ORIGIN + Vec2::new(250.0, 600.0));
        update_until(&mut app, |world| {
            world.resource::<ActiveMaps>().current == "2"
        });
        assert_eq!(on_current_map(&mut app), Vec::<String>::new());
        app.world_mut().spawn(OnCurrentMap);
        app.update();
        assert_eq!(on_current_map(&mut app), ["2"]);
    }

    #[test]
    fn an_on_current_map_entity_that_a_load_hook_spawns_belongs_to_the_map_being_loaded() {
        // A seamless neighbour loads while the player is still in map 1, so the observer gives the entity map 1
        // first; the adoption gives it the map that is loading.
        let mut app = app_with_two_maps();
        app.update();
        let root = app.world_mut().spawn(MapRoot).id();
        let before: HashSet<Entity> = app
            .world_mut()
            .query::<Entity>()
            .iter(app.world())
            .collect();
        let spawned = app.world_mut().spawn(OnCurrentMap).id();
        assert_eq!(app.world().get::<MapId>(spawned), Some(&MapId("1".into())));
        adopt_new_entities(app.world_mut(), root, "2", &before);
        assert_eq!(app.world().get::<MapId>(spawned), Some(&MapId("2".into())));
        assert_eq!(
            app.world().get::<ChildOf>(spawned).map(ChildOf::parent),
            Some(root)
        );
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
        let log = &app.world().resource::<Log>().0;
        assert_eq!(line_after(log, 0, "load 2"), None, "{log:?}");
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
    fn an_arrival_at_a_start_on_an_edge_cell_is_placed_off_its_blocked_half() {
        // Map 2 has an edge ring with the start on its west side, at cell (0,4): a `west` piece.
        let head = "---\nmap: 2\ncell: 64\norigin: bottom-left\nsize: 10x10\nlegend:\n  \".\": edge\n  \"@\": player_start\n---\n";
        let rows: String = (0..10)
            .rev()
            .map(|y| {
                let row: Vec<&str> = (0..10)
                    .map(|x| match (x, y) {
                        (0, 4) => "@",
                        (0 | 9, _) | (_, 0 | 9) => ".",
                        _ => " ",
                    })
                    .collect();
                format!("{y:>2} |{}|\n", row.join(" "))
            })
            .collect();
        let two = RegisteredMap {
            id: "2".into(),
            file: "2.txt".into(),
            map: parse("t.txt", &format!("{head}{rows}"), &[PLAYER_START]).unwrap(),
        };
        let mut app = app_with_two_maps();
        app.insert_resource(
            MapRegistry::from_parsed(vec![registered_with_start("1", (0, 0)), two]).unwrap(),
        );
        app.world_mut().resource_mut::<MapTransition>().edge_fit = Some(EdgeFit {
            step: 16.0,
            half_width: 10.0,
        });
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
        // The west half of the cell ends at x 32, grown by the half width 10. The y is the middle of row 4.
        assert_eq!(player_feet(&mut app), ORIGIN + Vec2::new(42.0, 288.0));
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
    fn a_trigger_fires_once_and_goes_with_its_map() {
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
        assert_eq!(app.world().resource::<ActiveMaps>().current, "2");
        // Map 2 has no trigger. The feet move back onto the place of the trigger of map 1, with the exits armed: the
        // trigger went with its map, so nothing fires and nothing loops.
        set_feet(&mut app, ORIGIN + Vec2::new(130.0, 130.0));
        for _ in 0..3 {
            app.update();
        }
        assert!(app.world().resource::<MapTransition>().armed);
        assert_eq!(loads(&app), 2);
        assert_eq!(app.world().resource::<ActiveMaps>().current, "2");
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
        let log = &app.world().resource::<Log>().0;
        assert_eq!(line_after(log, 1, "load"), None, "{log:?}");
        assert!(app.world().resource::<MapTransition>().is_idle());
        assert_eq!(player_feet(&mut app), ORIGIN + Vec2::new(300.0, 300.0));
    }

    /// Updates until `done` holds, at most 40 times.
    fn update_until(app: &mut App, done: impl Fn(&World) -> bool) {
        for _ in 0..40 {
            if done(app.world()) {
                return;
            }
            app.update();
        }
        panic!("the condition did not hold after 40 updates");
    }

    #[test]
    fn requests_during_a_transition_are_ignored() {
        let mut app = app_with_two_maps();
        app.insert_resource(MapTransition::new(Fade {
            colour: Color::BLACK,
            out_secs: 1.0,
            in_secs: 1.0,
        }));
        app.update();
        put_player(&mut app, ORIGIN + Vec2::new(300.0, 300.0));
        app.world_mut().write_message(RequestMapChange {
            to: "2".into(),
            entry: Some("from_1".into()),
            transition: Transition::Scene,
            source: None,
        });
        app.update();
        let transition = app.world().resource::<MapTransition>();
        assert!(matches!(transition.phase, Phase::FadeOut { .. }));
        assert!(transition.locks_input());
        // A second request in a later frame, while the screen fades out. It leads nowhere, so it must not replace the
        // first one.
        app.world_mut().write_message(RequestMapChange {
            to: "9".into(),
            entry: Some("x".into()),
            transition: Transition::Scene,
            source: None,
        });
        update_until(&mut app, |world| {
            world.resource::<ActiveMaps>().current == "2"
        });
        let transition = app.world().resource::<MapTransition>();
        assert!(matches!(transition.phase, Phase::FadeIn { .. }));
        assert!(transition.locks_input());
        update_until(&mut app, |world| {
            world.resource::<MapTransition>().is_idle()
        });
        assert_eq!(player_feet(&mut app), ORIGIN + Vec2::new(320.0, 32.0));
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

    #[test]
    fn a_seamless_map_is_placed_beside_the_exit_edge_with_the_strips_on_top_of_each_other() {
        let origin = Vec2::new(-1000.0, -500.0);
        let exit_map = |area: &str| {
            empty_map(
                64,
                10,
                10,
                &format!("exits:\n  x: area={area} to=2 entry=e transition=seamless\n"),
            )
        };
        let entry_map = |w: i32, h: i32, area: &str| {
            empty_map(64, w, h, &format!("entries:\n  e: area={area}\n"))
        };
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
        // The body is 14 wide on each side and 50 tall.
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
    fn a_doorway_on_each_edge_bridges_from_the_inner_side_of_one_map_to_the_other() {
        let first = ActiveMap {
            id: "1".into(),
            origin: Vec2::ZERO,
            rect: Rect::new(0.0, 0.0, 640.0, 640.0),
        };
        // Different margins on each side, so a swapped value shows. The first map shrinks to (14,6)..(630,590).
        let margin_min = Vec2::new(14.0, 6.0);
        let margin_max = Vec2::new(10.0, 50.0);
        // The second map, the doorway on the edge of the first, and the bridge: the doorway shrunk along the edge,
        // from the inner side of the first map to the inner side of the second.
        for (edge, second, door, bridge) in [
            (
                Edge::Top,
                Rect::new(0.0, 640.0, 640.0, 1280.0),
                Rect::new(192.0, 576.0, 448.0, 640.0),
                Rect::new(206.0, 590.0, 438.0, 646.0),
            ),
            (
                Edge::Bottom,
                Rect::new(0.0, -640.0, 640.0, 0.0),
                Rect::new(192.0, 0.0, 448.0, 64.0),
                Rect::new(206.0, -50.0, 438.0, 6.0),
            ),
            (
                Edge::Right,
                Rect::new(640.0, 0.0, 1280.0, 640.0),
                Rect::new(576.0, 256.0, 640.0, 384.0),
                Rect::new(630.0, 262.0, 654.0, 334.0),
            ),
            (
                Edge::Left,
                Rect::new(-640.0, 0.0, 0.0, 640.0),
                Rect::new(0.0, 256.0, 64.0, 384.0),
                Rect::new(-10.0, 262.0, 14.0, 334.0),
            ),
        ] {
            let active = ActiveMaps {
                current: "1".into(),
                maps: vec![
                    first.clone(),
                    ActiveMap {
                        id: "2".into(),
                        origin: second.min,
                        rect: second,
                    },
                ],
                doorways: vec![Doorway {
                    from: "1".into(),
                    to: "2".into(),
                    edge,
                    area: door,
                }],
            };
            let areas = active.walkable(margin_min, margin_max);
            assert_eq!(areas.len(), 3, "{edge:?}");
            assert_eq!(areas[2], bridge, "{edge:?}");
        }
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
                GameCamera,
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
        // The second map at the top of the first, aligned strip on strip (both start at x 3), and a rebuild that keeps
        // the first map. No arrive hook: the player is not moved.
        let load = line_after(&log, 0, "load 2").expect("map 2 is loaded");
        assert_eq!(
            log[load],
            format!("load 2 at {}", ORIGIN + Vec2::new(0.0, 640.0))
        );
        let rebuild = line_after(&log, load, "rebuild").expect("a rebuild after the load");
        assert_eq!(log[rebuild], "rebuild with 2 map entities");
        assert_eq!(line_after(&log, 0, "arrive"), None, "{log:?}");
        let active = app.world().resource::<ActiveMaps>();
        assert_eq!(active.maps.len(), 2);
        assert_eq!(active.current, "1", "the player is still in the first map");
        assert_eq!(active.doorways.len(), 1);
        assert_eq!(active.doorways[0].edge, Edge::Top);
        // No fade: the transition is over in the same frame (`only_the_fade_and_the_swap_lock_the_player`).
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
        // Explicit time: steps of 1/8 s add up exactly, and no step is cut short by the largest frame time.
        let step = 0.125;
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(step),
        ));
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .set_max_delta(std::time::Duration::from_secs(1));
        let steps_to_unload = (UNLOAD_AFTER_SECS / step) as usize;
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
        // Longer than the grace time with the first map in view: it stays.
        for _ in 0..=steps_to_unload {
            app.update();
        }
        assert_eq!(
            app.world().resource::<ActiveMaps>().maps.len(),
            2,
            "the first map is still in view"
        );
        // The camera moves up: the first map is out of view. It goes when it has been out of view for the grace time.
        app.world_mut()
            .entity_mut(camera)
            .insert(GlobalTransform::from(Transform::from_translation(
                (ORIGIN + Vec2::new(320.0, 1100.0)).extend(0.0),
            )));
        for _ in 1..steps_to_unload {
            app.update();
        }
        assert_eq!(
            app.world().resource::<ActiveMaps>().maps.len(),
            2,
            "not yet, one step short of the grace time"
        );
        app.update();
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
        // The root of map 2 and the entity its hook spawned.
        assert_eq!(ids, vec!["2".to_string(); 2], "only the second map is left");
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
            .insert_resource(MapOrigin(ORIGIN))
            .insert_resource(MapTransition::new(Fade::default()));
        let hooks = MapHooks {
            load: app.world_mut().register_system(load_hook),
            rebuild: app.world_mut().register_system(rebuild_hook),
            arrive: Some(app.world_mut().register_system(arrive_hook)),
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

    #[test]
    fn a_trigger_of_a_joined_map_does_not_fire_while_the_player_is_in_the_current_map() {
        let mut app = app_with("seamless");
        app.update();
        put_camera(&mut app, ORIGIN + Vec2::new(320.0, 600.0));
        put_player(&mut app, ORIGIN + Vec2::new(300.0, 300.0));
        app.update();
        set_feet(&mut app, ORIGIN + Vec2::new(250.0, 600.0));
        app.update();
        assert_eq!(app.world().resource::<ActiveMaps>().maps.len(), 2);
        // A trigger of map 2 whose area reaches into map 1.
        app.world_mut().spawn((
            MapId("2".into()),
            ExitTrigger {
                area: Rect::from_corners(
                    ORIGIN + Vec2::new(80.0, 280.0),
                    ORIGIN + Vec2::new(120.0, 320.0),
                ),
                to: "1".into(),
            },
        ));
        // Out of the exit first, so the exits are armed again.
        set_feet(&mut app, ORIGIN + Vec2::new(300.0, 300.0));
        app.update();
        assert!(app.world().resource::<MapTransition>().armed);
        set_feet(&mut app, ORIGIN + Vec2::new(100.0, 300.0));
        app.update();
        let transition = app.world().resource::<MapTransition>();
        assert!(transition.armed, "the trigger did not fire");
        assert!(transition.is_idle());
        assert_eq!(app.world().resource::<ActiveMaps>().current, "1");
    }

    /// A world with the resources that `plan_switch` and `begin_seamless` read. Map 1 (64 unit cells) is active at
    /// `ORIGIN` and has a seamless exit `up` (3..6,9) to the entry `from_1` (2..5,0) of map 2 (32 unit cells, 12 by 8). A
    /// scene transition loads a map at (300,-200). Map 7 is active but not registered.
    fn world_for_plans() -> World {
        let one = registered(
            "1",
            "exits:\n  up: area=3..6,9 to=2 entry=from_1 transition=seamless\n",
        );
        let two = registered_sized(
            "2",
            32,
            12,
            8,
            "entries:\n  from_1: area=2..5,0 facing=up\n",
        );
        let mut world = World::new();
        world.insert_resource(MapRegistry::from_parsed(vec![one, two]).unwrap());
        world.insert_resource(ActiveMaps {
            current: "1".into(),
            maps: vec![
                ActiveMap {
                    id: "1".into(),
                    origin: ORIGIN,
                    rect: Rect::from_corners(ORIGIN, ORIGIN + Vec2::splat(640.0)),
                },
                ActiveMap {
                    id: "7".into(),
                    origin: Vec2::new(5000.0, 0.0),
                    rect: Rect::new(5000.0, 0.0, 5640.0, 640.0),
                },
            ],
            doorways: Vec::new(),
        });
        world.insert_resource(MapOrigin(Vec2::new(300.0, -200.0)));
        world.insert_resource(MapTransition::new(Fade::default()));
        world
    }

    fn used(from: &str, exit: &str, feet: Vec2) -> Option<ExitUse> {
        Some(ExitUse {
            from: from.into(),
            exit: exit.into(),
            feet,
        })
    }

    #[test]
    fn the_position_along_the_strip_is_scaled_to_the_cells_of_the_new_map() {
        let world = world_for_plans();
        // 58 units along an exit of 64 unit cells is 29 units along an entry of 32 unit cells.
        let request = RequestMapChange {
            to: "2".into(),
            entry: Some("from_1".into()),
            transition: Transition::Scene,
            source: used("1", "up", ORIGIN + Vec2::new(192.0 + 58.0, 600.0)),
        };
        let switch = plan_switch(&world, &request).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(switch.id, "2");
        assert_eq!(switch.origin, Vec2::new(300.0, -200.0));
        assert_eq!(
            switch.arrival.feet,
            Vec2::new(300.0 + 64.0 + 29.0, -200.0 + 16.0)
        );
        assert_eq!(switch.arrival.facing, Some(Facing::Up));
    }

    #[test]
    fn a_switch_that_cannot_be_planned_names_the_problem() {
        let world = world_for_plans();
        for (to, entry, message) in [
            ("9", Some("x"), "map `9` is not registered"),
            ("2", None, "map `2` has no `player_start`"),
            ("2", Some("nope"), "map `2` has no entry `nope`"),
        ] {
            let request = RequestMapChange {
                to: to.into(),
                entry: entry.map(String::from),
                transition: Transition::Scene,
                source: None,
            };
            assert_eq!(
                plan_switch(&world, &request).err().as_deref(),
                Some(message)
            );
        }
    }

    #[test]
    fn a_seamless_join_that_cannot_be_made_names_the_problem_and_changes_nothing() {
        let feet = ORIGIN + Vec2::new(250.0, 600.0);
        for (to, entry, source, message) in [
            (
                "2",
                Some("from_1"),
                None,
                "a seamless transition needs the exit that was used",
            ),
            (
                "2",
                Some("from_1"),
                used("2", "up", feet),
                "map `2` is not active",
            ),
            (
                "2",
                Some("from_1"),
                used("7", "up", feet),
                "map `7` is not registered",
            ),
            (
                "2",
                Some("from_1"),
                used("1", "nope", feet),
                "map `1` has no exit `nope`",
            ),
            (
                "9",
                Some("from_1"),
                used("1", "up", feet),
                "map `9` is not registered",
            ),
            (
                "2",
                None,
                used("1", "up", feet),
                "a seamless transition needs an entry",
            ),
            (
                "2",
                Some("nope"),
                used("1", "up", feet),
                "map `2` has no entry `nope`",
            ),
        ] {
            let mut world = world_for_plans();
            let request = RequestMapChange {
                to: to.into(),
                entry: entry.map(String::from),
                transition: Transition::Seamless,
                source,
            };
            assert_eq!(
                begin_seamless(&mut world, &request),
                Err(message.to_string())
            );
            let active = world.resource::<ActiveMaps>();
            assert_eq!(active.maps.len(), 2, "{message}");
            assert!(active.doorways.is_empty(), "{message}");
        }
    }

    /// A new empty folder for the map files of one test.
    fn temp_folder(test: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("game-kit-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Writes `text` to `<dir>/<id>.txt`. The path is absolute, so `read` does not join it to the crate folder.
    fn map_file(dir: &Path, id: &'static str, text: &str) -> MapSource {
        let path = dir.join(format!("{id}.txt"));
        std::fs::write(&path, text).unwrap();
        MapSource {
            id,
            file: path.display().to_string().leak(),
        }
    }

    #[test]
    fn loading_names_the_file_of_a_missing_file_a_bad_file_and_a_broken_link() {
        let dir = temp_folder("load");
        let good = map_file(&dir, "1", &empty_map_text("1", 64, 10, 10, ""));
        let missing = MapSource {
            id: "2",
            file: dir.join("none.txt").display().to_string().leak(),
        };
        let bad = map_file(&dir, "3", "no front matter\n");
        let lost = map_file(
            &dir,
            "4",
            &empty_map_text(
                "4",
                64,
                10,
                10,
                "exits:\n  up: area=3..6,9 to=9 entry=e transition=scene\n",
            ),
        );
        let error = MapRegistry::load(&[good, missing], &[]).err().unwrap();
        // The rest of the message comes from the operating system.
        assert!(error.starts_with(&format!("{}: ", missing.file)), "{error}");
        assert_eq!(
            MapRegistry::load(&[good, bad], &[]).err(),
            Some(format!(
                "{}:1: the file must start with a `---` frontmatter line",
                bad.file
            ))
        );
        let error = MapRegistry::load(&[good, lost], &[]).err().unwrap();
        assert!(
            error.starts_with(&format!(
                "{}:9: exit `up` leads to map `9`, which is not registered",
                lost.file
            )),
            "{error}"
        );
        let registry = MapRegistry::load(&[good], &[]).unwrap();
        assert_eq!(registry.ids().collect::<Vec<_>>(), vec!["1"]);
        assert_eq!(registry.get("1").unwrap().file, good.file);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_failed_reload_leaves_the_maps_unchanged() {
        let dir = temp_folder("reload");
        let one = map_file(
            &dir,
            "1",
            &empty_map_text(
                "1",
                64,
                10,
                10,
                "exits:\n  up: area=3..6,9 to=2 entry=e transition=scene\n",
            ),
        );
        let two = map_file(
            &dir,
            "2",
            &empty_map_text("2", 64, 10, 10, "entries:\n  e: area=3..6,0\n"),
        );
        let mut registry = MapRegistry::load(&[one, two], &[]).unwrap();
        let before = registry.get("2").unwrap().map.clone();
        // A file that does not parse.
        std::fs::write(two.file, "no front matter\n").unwrap();
        assert_eq!(
            registry.reload("2"),
            Err(format!(
                "{}:1: the file must start with a `---` frontmatter line",
                two.file
            ))
        );
        assert_eq!(registry.get("2").unwrap().map, before);
        // A file that parses but breaks a link: the exit of map 1 leads to the entry `e`, which is gone.
        std::fs::write(
            two.file,
            empty_map_text("2", 64, 10, 10, "entries:\n  f: area=3..6,0\n"),
        )
        .unwrap();
        let error = registry.reload("2").unwrap_err();
        assert!(
            error.contains("leads to entry `e`, but map `2` has no such entry"),
            "{error}"
        );
        assert_eq!(registry.get("2").unwrap().map, before);
        // A good file is read again.
        std::fs::write(
            two.file,
            empty_map_text("2", 64, 10, 10, "entries:\n  e: area=3..6,1\n"),
        )
        .unwrap();
        registry.reload("2").unwrap();
        assert_eq!(
            registry.get("2").unwrap().map.entries[0].area,
            area(cell(3, 1), cell(6, 1))
        );
        assert_eq!(
            registry.reload("9"),
            Err("map `9` is not registered".to_string())
        );
        let mut parsed = MapRegistry::from_parsed(vec![registered("1", "")]).unwrap();
        assert_eq!(
            parsed.reload("1"),
            Err("map `1` has no file to read again".to_string())
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
