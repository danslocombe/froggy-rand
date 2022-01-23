Random number generation without state for games.

One way of thinking about a pseudorandom number generator (RNG) is as a list of numbers defined by an initial seed.
Every time we generate a random number we increment the index into this list. 
Suppose you are building a rougelike game where users can share seeds. A design goal might be to keep the map generation consistent between updates.
With a traditional RNG you would have to be very careful about adding additional RNG calls as doing so would affect all RNG calls made after it.

This library solves this problem by removing the mutable state from the RNG.

---

For a concrete example, suppose we are filling a level with enemies.

```rust
let mut rng = ExampleRng::from_seed(seed);
let mut enemies = vec![];
for id in 0..100 {
  let x = rng.next();
  let y = rng.next();
  enemies.push(Enemy::new(id, x, y));
}
``` 

Now suppose in an update we want to add variety, so give enemies a choice of random weapons.
We might do that as follows:

```rust
let mut rng = ExampleRng::from_seed(seed);
let mut enemies = vec![];
for id in 0..100 {
  let x = rng.next();
  let y = rng.next();
  let weapon_type = rng.next();
  enemies.push(Enemy::new(id, x, y, weapon_type));
}
``` 

However we have just changed the positions of all enemies past the first!
One fix would be to initialize a new random number generator for the weapon type based on a seed generated from the initial, but this gets messy.

---

Another approach might be to be to embrace the "list of random numbers" view and transform the stateful RNG into an indexing function.

```rust
fn random(seed : SeedValue, i : usize) -> RandomValue
{ ... }
```

But this would require the user to explicitly keep track of the index somewhere.
`FroggyRand` uses a two stage approach, first it generates a hash value from its input argument.
Then it combines that with its seed to generate and index.

Here is how we would use `FroggyRand` with the example above:

```rust
let froggy_rand = FroggyRand::new(seed);
let mut enemies = vec![];
for id in 0..100 {
  // We want the x position to be based on two things:
  //   The hash of the string "enemy_x" to make sure its different to the y value
  //   The enemy id
  let x = froggy_rand.gen(("enemy_x", id));
  let y = froggy_rand.gen(("enemy_y", id));
  let weapon_type = froggy_rand.gen(("weapon_type", id));
  enemies.push(Enemy::new(id, x, y, weapon_type));
}
``` 

Now we can add as many new parameters as we want without them effecting each other. 

For a more detailed explanation see
[this talk.](https://www.youtube.com/watch?v=e4b--cyXEsM)