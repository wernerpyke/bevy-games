//! What the player carries: a count of each energy colour. It grows when the kit reports a collected pickup
//! (`game_kit::collectable::Collected`). Energy has no use yet and no cap.

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

    fn index(self) -> usize {
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

#[derive(Resource, Default, Debug)]
pub struct Inventory {
    energy: [u32; 4],
}

impl Inventory {
    pub fn add(&mut self, energy: Energy) {
        self.energy[energy.index()] += 1;
    }

    pub fn count(&self, energy: Energy) -> u32 {
        self.energy[energy.index()]
    }

    /// One line for the HUD, for example `B 1  R 0  V 2  K 0`.
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
    fn each_energy_has_its_own_count() {
        let mut inventory = Inventory::default();
        inventory.add(Energy::Blue);
        inventory.add(Energy::Violet);
        inventory.add(Energy::Violet);
        assert_eq!(inventory.count(Energy::Blue), 1);
        assert_eq!(inventory.count(Energy::Red), 0);
        assert_eq!(inventory.count(Energy::Violet), 2);
        assert_eq!(inventory.summary(), "B 1  R 0  V 2  K 0");
    }

    #[test]
    fn only_energy_names_make_energy() {
        assert_eq!(Energy::from_kind("energy_black"), Some(Energy::Black));
        assert_eq!(Energy::from_kind("rock_0"), None);
    }
}
