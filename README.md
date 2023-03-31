Random number generation without state for games.

One way of thinking about a stateful pseudorandom number generator (RNGs) is as a list of numbers.
Each initial seed yields a different list.
Every time we generate a random number we are incrementing an index into this list. 

Suppose you are building a rougelike game where users can share seeds.
A design goal might be to keep the map generation consistent between updates.
With a stateful RNG you would have to be very careful about adding additional RNG calls.
This is because additional RNG calls will increment the index into the RNG's list, and will affect all RNG calls made after it.

This library solves this problem by removing the mutable state from the RNG.

---

For a concrete example, suppose we are filling a level with enemies at random positions.

```rust
let mut rng = ExampleRng::from_seed(seed);
let mut enemies = vec![];
for id in 0..100 {
  let x = rng.next();
  let y = rng.next();
  enemies.push(Enemy::new(id, x, y));
}
``` 

Now in an update we want to add variety, so we give enemies a choice of random weapons.
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

We load up the game with a known seed however the positions of all the enemies change!
What has happened is the additional rng calls have shifted all of the subsequent position generations.
One fix would be to initialize a new random number generator for the weapon type based on a seed generated from the initial, but this gets messy if we need a generator per property.

---

Another approach might be to be to embrace the "list of random numbers" view and transform the stateful RNG into an indexing function.

```rust
fn random(seed : SeedValue, i : usize) -> RandomValue
{ ... }
```

This would require the user to explicitly keep track of the index somewhere.
`FroggyRand` makes one more jump after this.
First it generates a hash value from its input argument, then it combines that with its seed to generate  an index into an RNG list.

Here is how we would use `FroggyRand` with the example above:

```rust
let froggy_rand = FroggyRand::new(seed);
let mut enemies = vec![];
for id in 0..100 {
  // We want the x position to be based on two things:
  //   The enemy id
  //   The hash of the string "enemy_x" to make sure its different to the y value
  let x = froggy_rand.gen(("enemy_x", id));
  let y = froggy_rand.gen(("enemy_y", id));
  let weapon_type = froggy_rand.gen(("weapon_type", id));
  enemies.push(Enemy::new(id, x, y, weapon_type));
}
``` 

Now we can add as many new parameters as we want without them effecting each other. 

For a more detailed explanation see
[this talk.](https://www.youtube.com/watch?v=e4b--cyXEsM)