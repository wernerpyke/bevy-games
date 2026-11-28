//! The map files a game registers: read from disk, parsed and checked against each other.

use super::*;

/// One map file a game registers. `file` is relative to the game folder: it is read from disk at start and again on F1,
/// so a map edit needs no rebuild. The maps are not built into the program (the web build is paused, `adr-e11`).
#[derive(Clone, Copy, Debug)]
pub struct MapSource {
    pub id: &'static str,
    pub file: &'static str,
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
    cell_size: f32,
    maps: Vec<RegisteredMap>,
}

impl MapRegistry {
    /// Reads and checks every map, with the game's cell size. The error names the file and line.
    pub fn load(
        sources: &[MapSource],
        kinds: &'static [&'static str],
        cell_size: f32,
    ) -> Result<MapRegistry, String> {
        let maps = sources
            .iter()
            .map(|source| read(source, kinds, cell_size))
            .collect::<Result<Vec<_>, _>>()?;
        check_links(&maps)?;
        Ok(MapRegistry {
            sources: sources.to_vec(),
            kinds,
            cell_size,
            maps,
        })
    }

    /// A registry of maps that are already parsed. It cannot reload them.
    pub fn from_parsed(maps: Vec<RegisteredMap>) -> Result<MapRegistry, String> {
        check_links(&maps)?;
        Ok(MapRegistry {
            sources: Vec::new(),
            kinds: &[],
            // Only used to read a map again, which this registry cannot do.
            cell_size: 0.0,
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
        maps[index] = read(source, self.kinds, self.cell_size)?;
        check_links(&maps)?;
        self.maps = maps;
        Ok(())
    }
}

/// The folder that map files are relative to: the game folder in a run through Cargo (`CARGO_MANIFEST_DIR`), else the
/// folder of the program, as Bevy finds `assets/`.
pub fn game_folder() -> PathBuf {
    std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(Path::to_path_buf))
        })
        .unwrap_or_default()
}

pub(super) fn read(
    source: &MapSource,
    kinds: &[&str],
    cell_size: f32,
) -> Result<RegisteredMap, String> {
    let path = game_folder().join(source.file);
    let file = path.display().to_string();
    let text = std::fs::read_to_string(&path).map_err(|error| format!("{file}: {error}"))?;
    let map = parse(&file, &text, kinds, cell_size).map_err(|error| error.to_string())?;
    Ok(RegisteredMap {
        id: source.id.to_string(),
        file,
        map,
    })
}

pub(super) fn check_links(maps: &[RegisteredMap]) -> Result<(), String> {
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
