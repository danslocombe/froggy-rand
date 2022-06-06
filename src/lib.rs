//! Random number generation without state for games.
//!
//! One way of thinking about a pseudorandom number generator (RNG) is as a list of numbers defined by an initial seed.
//! Every time we generate a random number we increment the index into this list. 
//! Suppose you are building a rougelike game where users can share seeds. A design goal might be to keep the map generation consistent between updates.
//! With a traditional RNG you would have to be very careful about adding additional RNG calls as doing so would affect all RNG calls made after it.
//! 
//! This library solves this problem by removing the mutable state from the RNG.
//! 
//! ---
//! 
//! For a concrete example, suppose we are filling a level with enemies.
//! 
//! ```
//! let mut rng = ExampleRng::from_seed(seed);
//! let mut enemies = vec![];
//! for id in 0..100 {
//!   let x = rng.next();
//!   let y = rng.next();
//!   enemies.push(Enemy::new(id, x, y));
//! }
//! ``` 
//! 
//! Now suppose in an update we want to add variety, so give enemies a choice of random weapons.
//! We might do that as follows:
//! 
//! ```
//! let mut rng = ExampleRng::from_seed(seed);
//! let mut enemies = vec![];
//! for id in 0..100 {
//!   let x = rng.next();
//!   let y = rng.next();
//!   let weapon_type = rng.next();
//!   enemies.push(Enemy::new(id, x, y, weapon_type));
//! }
//! ``` 
//! 
//! However we have just changed the positions of all enemies past the first!
//! One fix would be to initialize a new random number generator for the weapon type based on a seed generated from the initial, but this gets messy.
//! 
//! ---
//!
//! Another approach might be to be to embrace the "list of random numbers" view and transform the stateful RNG into an indexing function.
//! 
//! ```
//! fn random(seed : SeedValue, i : usize) -> RandomValue
//! { ... }
//! ```
//! 
//! But this would require the user to explicitly keep track of the index somewhere.
//! `FroggyRand` uses a two stage approach, first it generates a hash value from its input argument.
//! Then it combines that with its seed to generate and index.
//! 
//! Here is how we would use `FroggyRand` with the example above:
//! 
//! ```
//! let froggy_rand = FroggyRand::new(seed);
//! let mut enemies = vec![];
//! for id in 0..100 {
//!   // We want the x position to be based on two things:
//!   //   The hash of the string "enemy_x" to make sure its different to the y value
//!   //   The enemy id
//!   let x = froggy_rand.gen(("enemy_x", id));
//!   let y = froggy_rand.gen(("enemy_y", id));
//!   let weapon_type = froggy_rand.gen(("weapon_type", id));
//!   enemies.push(Enemy::new(id, x, y, weapon_type));
//! }
//! ``` 
//! 
//! Now we can add as many new parameters as we want without them effecting each other. 
//! 
//! For a more detailed explanation see
//! [this talk.](https://www.youtube.com/watch?v=e4b--cyXEsM)

#![no_std]

use core::hash::{Hash, Hasher};
use core::num::Wrapping;

#[derive(Debug, Copy, Clone)]
pub struct FroggyRand {
    seed : u64,
}

fn split_mix_64(index : u64) -> u64 {
    let mut z = Wrapping(index) + Wrapping(0x9E3779B97F4A7C15);
    z = (z ^ (z >> 30)) * Wrapping(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)) * Wrapping(0x94D049BB133111EB);
    (z ^ (z >> 31)).0
}

#[inline]
fn hash<T : Hash>(x : T) -> u64 {
    let mut hasher = deterministic_hash::DeterministicHasher::new(hashers::jenkins::Lookup3Hasher::default());
    x.hash(&mut hasher);
    hasher.finish()
}

impl FroggyRand {
    pub fn new(seed : u64) -> Self {
        Self {seed}
    }

    pub fn from_hash<T : Hash>(x : T) -> Self {
        Self::new(hash(x))
    }

    pub fn subrand<T : Hash>(&self, x : T) -> Self {
        Self::from_hash((x, self.seed))
    }

    pub fn get_seed(&self) -> u64 {
        self.seed
    }

    /// Should be uniform over all u64 values
    pub fn gen<T : Hash>(&self, x : T) -> u64 {
        let hash = hash(x);
        let index = (Wrapping(self.seed) + Wrapping(hash)).0;
        split_mix_64(index)
    }

    /// Should be uniform in [0, 1]
    pub fn gen_unit<T : Hash>(&self, x : T) -> f64 {
        // Should be enough precision for a game
        (self.gen(x) % 1_000_000) as f64 / 1_000_000.0
    }

    /// Should be uniform in [min, max]
    pub fn gen_range<T : Hash>(&self, x : T, min : f64, max : f64) -> f64 {
        min + self.gen_unit(x) * (max - min)
    }

    /// Should give a uniform random element of the slice choices. 
    pub fn choose<'a, T : Hash, X>(&self, x : T, choices : &'a [X]) -> &'a X {
        // usize can be aliased to u32 or u64 in wasm based on the compilation
        // for safety we restrict to u32 range.
        let index = self.gen(x) as u64 % u32::MAX as u64;
        let i = index as usize % choices.len();
        &choices[i]
    }

    /// I dont know what a statistic is
    /// Approx normal dist https://en.wikipedia.org/wiki/Irwin%E2%80%93Hall_distribution
    pub fn gen_froggy<T : Hash>(&self, x : T, min : f64, max : f64, n : u32) -> f64 {
        let mut sum = 0.;
        let gen_min = min / n as f64;
        let gen_max = max / n as f64;

        for i in 0..n {
            sum += self.gen_range((&x, i), gen_min, gen_max);
        }

        sum
    }

    pub fn gen_usize_range<T : Hash>(&self, x : T, min : usize, max : usize) -> usize {
        let range = 1 + max - min;
        min + ((self.gen(x) as usize) % range)
    }

    pub fn shuffle<T : Hash, X>(&self, x : T, xs : &mut [X]) {
        // Fisher-yates
        // See https://en.wikipedia.org/wiki/Fisher%E2%80%93Yates_shuffle#The_modern_algorithm
        for i in 0..=xs.len()-2 {
            let j = self.gen_usize_range((&x, i), i, xs.len() - 1);
            xs.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn different_hashes() {
        let froggy_rand = FroggyRand::new(100);
        let val0 = froggy_rand.gen(("test", 0));
        let val1 = froggy_rand.gen(("test", 1));
        let val2 = froggy_rand.gen(("test_other", 0));
        let val3 = froggy_rand.gen(("test_other", 1));

        assert_ne!(val0, val1);
        assert_ne!(val0, val2);
        assert_ne!(val0, val3);

        assert_ne!(val1, val0);
        assert_ne!(val1, val2);
        assert_ne!(val1, val3);

        assert_ne!(val2, val0);
        assert_ne!(val2, val1);
        assert_ne!(val2, val3);

        assert_ne!(val3, val0);
        assert_ne!(val3, val1);
        assert_ne!(val3, val2);
    }
}