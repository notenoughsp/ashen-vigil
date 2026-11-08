//! Mutators of the endless night: past dawn, every two and a half minutes the night takes on
//! another rule, and the same rule can come again and stack.
//!
//! The host draws them; clients learn the stacks from snapshots.

use super::*;

/// Seconds of endless night between two mutators.
pub const MUTATOR_EVERY: f32 = 150.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mutator {
    /// More foes at once.
    Swarm,
    /// Foes are tougher.
    Ironhide,
    /// Foes are faster.
    Hasted,
    /// Foes hit harder.
    Sharpened,
    /// Elites come more often.
    EliteTide,
    /// Souls are worth more: the night pays for itself.
    Gilded,
}

pub const ALL_MUTATORS: [Mutator; 6] = [Mutator::Swarm, Mutator::Ironhide, Mutator::Hasted, Mutator::Sharpened, Mutator::EliteTide, Mutator::Gilded];

impl Mutator {
    pub fn name(self) -> &'static str {
        match self {
            Mutator::Swarm => "Swarm",
            Mutator::Ironhide => "Ironhide",
            Mutator::Hasted => "Hasted",
            Mutator::Sharpened => "Sharpened",
            Mutator::EliteTide => "Elite tide",
            Mutator::Gilded => "Gilded",
        }
    }

    /// What one stack does.
    pub fn desc(self) -> &'static str {
        match self {
            Mutator::Swarm => "+25% foes",
            Mutator::Ironhide => "+30% foe health",
            Mutator::Hasted => "+10% foe speed",
            Mutator::Sharpened => "+20% foe damage",
            Mutator::EliteTide => "elites 30% more often",
            Mutator::Gilded => "+25% souls",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Mutator::Gilded => Color::from_rgba(240, 200, 90, 255),
            _ => Color::from_rgba(230, 110, 90, 255),
        }
    }
}

impl World {
    /// How many times a mutator is stacked.
    pub fn stacks(&self, m: Mutator) -> u8 {
        self.mutators[m as usize]
    }

    /// A factor for each stack of `m`: `per` raised to the number of stacks.
    pub(crate) fn mutated(&self, m: Mutator, per: f32) -> f32 {
        per.powi(self.stacks(m) as i32)
    }

    pub(crate) fn update_mutators(&mut self) {
        if !self.endless {
            return;
        }
        if self.next_mutator <= 0.0 {
            self.next_mutator = self.time + MUTATOR_EVERY;
            return;
        }
        if self.time < self.next_mutator {
            return;
        }
        self.next_mutator = self.time + MUTATOR_EVERY;
        let m = ALL_MUTATORS[gen_range(0, ALL_MUTATORS.len())];
        let n = &mut self.mutators[m as usize];
        *n = n.saturating_add(1);
        let n = *n;
        let title = if n > 1 { format!("{} x{}", m.name().to_uppercase(), n) } else { m.name().to_uppercase() };
        self.log_event(&format!("mutator: {} (x{})", m.name(), n));
        self.banner(&title, &format!("The night changes: {}", m.desc()), m.color(), 3.5);
        self.sfx("bell", 0.7);
    }
}
