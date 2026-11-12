//! What the player carries: a count of each energy colour. It grows when the kit reports a collected pickup
//! (`game_kit::collectable::Collected`), up to `MAX_ENERGY` each. A pickup collected at the maximum is lost
//! (`adr-lw34`). Energy has no use yet.

use bevy::prelude::*;
use game_kit::collectable::Collected;

/// The four kinds of energy, one per pickup colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Energy {
    Blue,
    Red,
    Violet,
    Black,
}

impl Energy {
    pub const ALL: [Energy; 4] = [Energy::Blue, Energy::Red, Energy::Violet, Energy::Black];

    /// The energy of a collected item, by its legend name.
    pub fn from_kind(kind: &str) -> Option<Energy> {
        match kind {
            "energy_blue" => Some(Energy::Blue),
            "energy_red" => Some(Energy::Red),
            "energy_violet" => Some(Energy::Violet),
            "energy_black" => Some(Energy::Black),
            _ => None,
        }
    }

    /// Its position in `Energy::ALL`, for arrays with one entry per energy.
    pub fn index(self) -> usize {
        self as usize
    }

    fn letter(self) -> char {
        match self {
            Energy::Blue => 'B',
            Energy::Red => 'R',
            Energy::Violet => 'V',
            Energy::Black => 'K',
        }
    }
}

/// The most of one energy the player can carry (the user's value, 2026-11-12).
pub const MAX_ENERGY: u32 = 10;

#[derive(Resource, Default, Debug)]
pub struct Inventory {
    energy: [u32; 4],
}

impl Inventory {
    /// Adds one of `energy`, unless it is at `MAX_ENERGY`.
    pub fn add(&mut self, energy: Energy) {
        let count = &mut self.energy[energy.index()];
        *count = (*count + 1).min(MAX_ENERGY);
    }

    pub fn count(&self, energy: Energy) -> u32 {
        self.energy[energy.index()]
    }

    /// How full `energy` is, from 0 (none) to 1 (`MAX_ENERGY`).
    pub fn fraction(&self, energy: Energy) -> f32 {
        self.count(energy) as f32 / MAX_ENERGY as f32
    }

    /// One line for the log, for example `B 1  R 0  V 2  K 0`.
    pub fn summary(&self) -> String {
        Energy::ALL
            .iter()
            .map(|&e| format!("{} {}", e.letter(), self.count(e)))
            .collect::<Vec<_>>()
            .join("  ")
    }
}

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Inventory>()
            .add_systems(Update, add_collected);
    }
}

fn add_collected(mut collected: MessageReader<Collected>, mut inventory: ResMut<Inventory>) {
    for item in collected.read() {
        match Energy::from_kind(&item.kind) {
            Some(energy) => {
                inventory.add(energy);
                info!("inventory: {}", inventory.summary());
            }
            None => warn!("inventory: collected `{}` has no energy", item.kind),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_energy_has_its_own_count_and_stops_at_the_maximum() {
        let mut inventory = Inventory::default();
        inventory.add(Energy::Blue);
        inventory.add(Energy::Violet);
        inventory.add(Energy::Violet);
        assert_eq!(inventory.summary(), "B 1  R 0  V 2  K 0");
        for _ in 0..MAX_ENERGY - 1 {
            inventory.add(Energy::Red);
        }
        assert_eq!(inventory.count(Energy::Red), MAX_ENERGY - 1);
        inventory.add(Energy::Red);
        assert_eq!(inventory.count(Energy::Red), MAX_ENERGY);
        inventory.add(Energy::Red);
        assert_eq!(inventory.summary(), "B 1  R 10  V 2  K 0");
        // The fraction for an empty, a partly full and a full energy. Exact in f32: these are n / 10 in one division.
        assert_eq!(
            [Energy::Black, Energy::Violet, Energy::Red].map(|e| inventory.fraction(e)),
            [0.0, 0.2, 1.0]
        );
    }

    #[test]
    fn only_energy_names_make_energy() {
        assert_eq!(Energy::from_kind("energy_black"), Some(Energy::Black));
        assert_eq!(Energy::from_kind("rock_0"), None);
    }

    #[test]
    fn a_collected_energy_is_counted_and_an_unknown_kind_is_not() {
        let mut app = App::new();
        app.add_message::<Collected>().add_plugins(InventoryPlugin);
        for kind in ["energy_red", "moss", "energy_red"] {
            app.world_mut().write_message(Collected {
                kind: kind.to_string(),
            });
        }
        app.update();
        assert_eq!(
            app.world().resource::<Inventory>().summary(),
            "B 0  R 2  V 0  K 0"
        );
    }
}
