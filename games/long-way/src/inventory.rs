//! What the player carries: a count of each energy colour. It grows when the kit reports a collected pickup
//! (`game_kit::collectable::Collected`), up to `MAX_ENERGY` each. A pickup collected at the maximum is lost
//! (`adr-lw34`). A cast spends the cost of its weapon (`try_spend`, called by `weapons/mod.rs`).
//!
//! The script command `pickup:<kind>:<count>` (`PICKUP_COMMAND`, `game_kit::script`) adds `count` of a pickup kind as if
//! the player had collected it that many times, for example `pickup:energy_red:2`.

use bevy::prelude::*;
use game_kit::collectable::Collected;
use game_kit::script::ScriptCommand;

/// The name of the script command that adds pickups.
pub const PICKUP_COMMAND: &str = "pickup";

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

    /// Takes `cost` of `energy` and returns true when the player has at least that much. Otherwise it changes nothing
    /// and returns false.
    pub fn try_spend(&mut self, energy: Energy, cost: u32) -> bool {
        let count = &mut self.energy[energy.index()];
        if *count < cost {
            return false;
        }
        *count -= cost;
        true
    }

    /// The share of `cost` that the player has of `energy`, from 0 to 1 (1 at or above the cost, and for a cost of 0).
    pub fn cost_share(&self, energy: Energy, cost: u32) -> f32 {
        if cost == 0 {
            return 1.0;
        }
        (self.count(energy) as f32 / cost as f32).min(1.0)
    }

    #[cfg(test)]
    pub fn set_for_test(&mut self, energy: Energy, count: u32) {
        self.energy[energy.index()] = count;
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
            .add_systems(Update, (add_collected, add_from_script));
    }
}

/// The energy and count of a `pickup` command's parts: a pickup kind from the map legend and a whole number above 0.
fn pickup_args(args: &[String]) -> Result<(Energy, u32), String> {
    let [kind, count] = args else {
        return Err(format!(
            "needs a kind and a count, for example `pickup:energy_red:2`, found {} parts",
            args.len()
        ));
    };
    let energy = Energy::from_kind(kind).ok_or_else(|| format!("`{kind}` is not a pickup kind"))?;
    match count.parse::<u32>() {
        Ok(count) if count > 0 => Ok((energy, count)),
        _ => Err(format!("`{count}` is not a count above 0")),
    }
}

/// The script's `pickup` commands: each adds its count, one at a time, so the maximum holds as for a real pickup.
fn add_from_script(mut commands: MessageReader<ScriptCommand>, mut inventory: ResMut<Inventory>) {
    for command in commands.read().filter(|c| c.name == PICKUP_COMMAND) {
        match pickup_args(&command.args) {
            Ok((energy, count)) => {
                let before = inventory.count(energy);
                for _ in 0..count {
                    inventory.add(energy);
                }
                let added = inventory.count(energy) - before;
                info!(
                    "inventory: script pickup of {count} {energy:?}, {added} added (at most {MAX_ENERGY}): {}",
                    inventory.summary()
                );
            }
            Err(problem) => warn!("inventory: script `pickup` {problem}, skipped"),
        }
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
    fn a_spend_takes_the_cost_only_when_there_is_enough_and_only_from_its_own_energy() {
        let mut inventory = Inventory::default();
        inventory.set_for_test(Energy::Red, 3);
        inventory.set_for_test(Energy::Blue, 7);
        // One short changes nothing.
        assert!(!inventory.try_spend(Energy::Red, 4));
        assert_eq!(inventory.summary(), "B 7  R 3  V 0  K 0");
        // Exactly enough leaves 0.
        assert!(inventory.try_spend(Energy::Red, 3));
        assert_eq!(inventory.summary(), "B 7  R 0  V 0  K 0");
        assert!(!inventory.try_spend(Energy::Red, 1));
        assert!(inventory.try_spend(Energy::Blue, 2));
        assert_eq!(inventory.summary(), "B 5  R 0  V 0  K 0");
        inventory.set_for_test(Energy::Black, MAX_ENERGY);
        assert!(inventory.try_spend(Energy::Black, MAX_ENERGY));
        assert_eq!(inventory.count(Energy::Black), 0);
    }

    #[test]
    fn the_cost_share_is_the_part_of_the_cost_the_player_has_up_to_one() {
        let mut inventory = Inventory::default();
        let shares = |inventory: &Inventory, cost| {
            Energy::ALL.map(|energy| inventory.cost_share(energy, cost))
        };
        assert_eq!(shares(&inventory, 3), [0.0; 4]);
        inventory.set_for_test(Energy::Blue, 1);
        inventory.set_for_test(Energy::Red, 2);
        inventory.set_for_test(Energy::Violet, 3);
        inventory.set_for_test(Energy::Black, 9);
        assert_eq!(shares(&inventory, 3), [1.0 / 3.0, 2.0 / 3.0, 1.0, 1.0]);
        assert_eq!(shares(&inventory, 2), [0.5, 1.0, 1.0, 1.0]);
        assert_eq!(inventory.cost_share(Energy::Blue, 0), 1.0);
    }

    #[test]
    fn only_energy_names_make_energy() {
        assert_eq!(Energy::from_kind("energy_black"), Some(Energy::Black));
        assert_eq!(Energy::from_kind("rock_0"), None);
    }

    #[test]
    fn pickup_takes_a_pickup_kind_and_a_count_above_zero() {
        let args = |parts: &[&str]| parts.iter().map(|p| p.to_string()).collect::<Vec<_>>();
        assert_eq!(
            pickup_args(&args(&["energy_red", "2"])),
            Ok((Energy::Red, 2))
        );
        assert_eq!(
            pickup_args(&args(&["energy_black", "1"])),
            Ok((Energy::Black, 1))
        );
        assert_eq!(
            pickup_args(&args(&["energy_red"])),
            Err(
                "needs a kind and a count, for example `pickup:energy_red:2`, found 1 parts".into()
            )
        );
        assert_eq!(
            pickup_args(&args(&["energy_red", "2", "3"])),
            Err(
                "needs a kind and a count, for example `pickup:energy_red:2`, found 3 parts".into()
            )
        );
        assert_eq!(
            pickup_args(&args(&["rock_0", "2"])),
            Err("`rock_0` is not a pickup kind".into())
        );
        for count in ["0", "-1", "two"] {
            assert_eq!(
                pickup_args(&args(&["energy_blue", count])),
                Err(format!("`{count}` is not a count above 0"))
            );
        }
    }

    #[test]
    fn a_script_pickup_adds_its_count_up_to_the_maximum_and_other_or_bad_commands_add_nothing() {
        let mut app = App::new();
        app.add_message::<Collected>()
            .add_message::<ScriptCommand>()
            .add_plugins(InventoryPlugin);
        for (name, args) in [
            ("pickup", vec!["energy_red", "2"]),
            ("pickup", vec!["energy_violet", "12"]),
            ("pickup", vec!["energy_blue", "zero"]),
            ("give", vec!["energy_blue", "3"]),
        ] {
            app.world_mut().write_message(ScriptCommand {
                name: name.to_string(),
                args: args.into_iter().map(String::from).collect(),
            });
        }
        app.update();
        assert_eq!(
            app.world().resource::<Inventory>().summary(),
            "B 0  R 2  V 10  K 0"
        );
    }

    #[test]
    fn a_collected_energy_is_counted_and_an_unknown_kind_is_not() {
        let mut app = App::new();
        app.add_message::<Collected>()
            .add_message::<ScriptCommand>()
            .add_plugins(InventoryPlugin);
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
