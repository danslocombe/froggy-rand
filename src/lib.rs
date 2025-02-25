//! Random number generation without state for games.
//!
//! One way of thinking about a stateful pseudorandom number generator (RNGs) is as a list of numbers.
//! Each initial seed yields a different list.
//! Every time we generate a random number we are incrementing an index into this list. 
//! 
//! Suppose you are building a rougelike game where users can share seeds.
//! A design goal might be to keep the map generation consistent between updates.
//! With a stateful RNG you would have to be very careful about adding additional RNG calls.
//! This is because additional RNG calls will increment the index into the RNG's list, and will affect all RNG calls made after it.
//! 
//! This library solves this problem by removing the mutable state from the RNG.
//! 
//! ---
//! 
//! For a concrete example, suppose we are filling a level with enemies at random positions.
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
//! Now in an update we want to add variety, so we give enemies a choice of random weapons.
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
//! We load up the game with a known seed however the positions of all the enemies change!
//! What has happened is the additional rng calls have shifted all of the subsequent position generations.
//! One fix would be to initialize a new random number generator for the weapon type based on a seed generated from the initial, but this gets messy if we need a generator per property.
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
//! This would require the user to explicitly keep track of the index somewhere.
//! `FroggyRand` makes one more jump after this.
//! First it generates a hash value from its input argument, then it combines that with its seed to generate  an index into an RNG list.
//! 
//! Here is how we would use `FroggyRand` with the example above:
//! 
//! ```
//! let froggy_rand = FroggyRand::new(seed);
//! let mut enemies = vec![];
//! for id in 0..100 {
//!   // We want the x position to be based on two things:
//!   //   The enemy id
//!   //   The hash of the string "enemy_x" to make sure its different to the y value
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

mod hasher;

#[derive(Debug, Copy, Clone)]
pub struct FroggyRand {
    pub seed : u64,
}

fn split_mix_64(index : u64) -> u64 {
    let mut z = Wrapping(index) + Wrapping(0x9E3779B97F4A7C15);
    z = (z ^ (z >> 30)) * Wrapping(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)) * Wrapping(0x94D049BB133111EB);
    (z ^ (z >> 31)).0
}

#[inline]
fn hash<T : Hash>(x : T) -> u64 {
    let mut hasher = hasher::Lookup3Hasher::default();
    x.hash(&mut hasher);
    hasher.finish()
}

impl FroggyRand {
    #[inline]
    pub fn new(seed : u64) -> Self {
        Self {seed}
    }

    #[inline]
    pub fn from_hash<T : Hash>(x : T) -> Self {
        Self::new(hash(x))
    }

    #[inline]
    pub fn subrand<T : Hash>(&self, x : T) -> Self {
        Self::from_hash((x, self.seed))
    }

    /// Should be uniform over all u64 values
    #[inline]
    pub fn gen<T : Hash>(&self, x : T) -> u64 {
        let hash = hash(x);
        let index = self.seed.wrapping_add(hash);
        split_mix_64(index)
    }

    /// Should be uniform in [0, 1]
    #[inline]
    pub fn gen_unit<T : Hash>(&self, x : T) -> f32 {
        // Should be enough precision for a game
        (self.gen(x) % 1_000_000) as f32 / 1_000_000.0
    }

    /// Should be uniform in [min, max]
    #[inline]
    pub fn gen_range<T : Hash>(&self, x : T, min : f32, max : f32) -> f32 {
        min + self.gen_unit(x) * (max - min)
    }

    /// Should give a uniform random element of the slice choices. 
    #[inline]
    pub fn choose<'a, T : Hash, X>(&self, x : T, choices : &'a [X]) -> &'a X {
        // usize can be aliased to u32 or u64 in wasm based on the compilation
        // for safety we restrict to u32 range.
        let index = self.gen(x) as u64 % u32::MAX as u64;
        let i = index as usize % choices.len();
        &choices[i]
    }

    /// I dont know what a statistic is
    /// Approx normal dist https://en.wikipedia.org/wiki/Irwin%E2%80%93Hall_distribution
    #[inline]
    pub fn gen_froggy<T : Hash>(&self, x : T, min : f32, max : f32, n : u32) -> f32 {
        let mut sum = 0.;
        let gen_min = min / n as f32;
        let gen_max = max / n as f32;

        for i in 0..n {
            sum += self.gen_range((&x, i), gen_min, gen_max);
        }

        sum
    }

    #[inline]
    pub fn gen_usize_range<T : Hash>(&self, x : T, min : usize, max : usize) -> usize {
        let range = 1 + max - min;
        min + ((self.gen(x) as usize) % range)
    }

    #[inline]
    pub fn shuffle<T : Hash, X>(&self, x : T, xs : &mut [X]) {
        // Fisher-yates
        // See https://en.wikipedia.org/wiki/Fisher%E2%80%93Yates_shuffle#The_modern_algorithm
        for i in 0..=xs.len()-2 {
            let j = self.gen_usize_range((&x, i), i, xs.len() - 1);
            xs.swap(i, j);
        }
    }

    /// Should be uniform in [0, 255]
    #[inline]
    pub fn gen_byte<T : Hash>(&self, x : T) -> u8 {
        (self.gen(x) % 255) as u8
    }

    /// More performant gen() if the only control parameter you need is a single int.
    #[inline]
    pub fn gen_perf(&self, seed: i32) -> u64 {
        let index = (Wrapping(self.seed) + Wrapping(seed as u64)).0;
        split_mix_64(index)
    }

    /// More performant gen_unit() if the only control parameter you need is a single int.
    #[inline]
    pub fn gen_unit_perf(&self, seed: i32) -> f32 {
        (self.gen_perf(seed) % 1_000_000) as f32 / 1_000_000.0
    }
}


#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn same_hashes() {
        let froggy_rand = FroggyRand::new(100);
        let val0 = froggy_rand.gen(("test", 0));
        let val1 = froggy_rand.gen(("test", 0));
        assert_eq!(val0, val1);
    }

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