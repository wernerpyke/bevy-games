//! How an enemy reacts to each weapon energy (`adr-lw43`, `docs/enemies.md`, "Reactions to weapon energy"). The
//! energy decides, not the weapon. Each enemy kind keeps its table as a constant next to its hit points.

use bevy::prelude::*;

use crate::inventory::Energy;

/// How an enemy reacts to the weapons of one energy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Reaction {
    /// The weapons of this energy do not see the enemy: no contact, no target, no explosion on it.
    PassThrough,
    /// The weapon touches the enemy.
    Hit(Hit),
}

/// What a touching weapon does. `damage` and `push` are shares of the weapon's own: 1 is all of it, 0 none.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub damage: f32,
    pub push: f32,
    /// Every hit sends `EnemyInterrupted`.
    pub interrupt: bool,
}

impl Hit {
    pub const FULL: Hit = Hit {
        damage: 1.0,
        push: 1.0,
        interrupt: false,
    };
}

/// One `Reaction` per energy. An enemy without its own table gets `FULL` (`EnemyHp` requires it).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Reactions {
    pub blue: Reaction,
    pub red: Reaction,
    pub violet: Reaction,
    pub black: Reaction,
}

impl Reactions {
    pub const FULL: Reactions = Reactions {
        blue: Reaction::Hit(Hit::FULL),
        red: Reaction::Hit(Hit::FULL),
        violet: Reaction::Hit(Hit::FULL),
        black: Reaction::Hit(Hit::FULL),
    };

    pub fn to(&self, energy: Energy) -> Reaction {
        match energy {
            Energy::Blue => self.blue,
            Energy::Red => self.red,
            Energy::Violet => self.violet,
            Energy::Black => self.black,
        }
    }

    /// The `Hit` of `energy`, or `None` when it passes through.
    pub fn hit(&self, energy: Energy) -> Option<Hit> {
        match self.to(energy) {
            Reaction::PassThrough => None,
            Reaction::Hit(hit) => Some(hit),
        }
    }
}

impl Default for Reactions {
    fn default() -> Self {
        Self::FULL
    }
}

/// On an enemy that follows a kit path (`ProjectilePath`, the volcano shot): a push also turns it, so it glances off
/// the weapon (`weapons::damage::deflected`). Without it a push only moves the enemy.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Deflects;

/// Sent for every weapon hit of an energy whose reaction interrupts. The enemy's own plugin decides what it does. Not the
/// kit's `Stunned`, a lasting state that a weapon adds and removes (`docs/enemies.md`, "Stun and interrupt").
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnemyInterrupted(pub Entity);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_energy_reads_its_own_reaction() {
        let hit = |damage| {
            Reaction::Hit(Hit {
                damage,
                push: 0.5,
                interrupt: true,
            })
        };
        let reactions = Reactions {
            blue: hit(0.25),
            red: Reaction::PassThrough,
            violet: hit(2.0),
            black: hit(3.0),
        };
        assert_eq!(reactions.to(Energy::Blue), hit(0.25));
        assert_eq!(reactions.to(Energy::Red), Reaction::PassThrough);
        assert_eq!(reactions.to(Energy::Violet), hit(2.0));
        assert_eq!(reactions.to(Energy::Black), hit(3.0));
        assert_eq!(reactions.hit(Energy::Red), None);
        assert_eq!(
            reactions.hit(Energy::Black),
            Some(Hit {
                damage: 3.0,
                push: 0.5,
                interrupt: true
            })
        );
    }
}
