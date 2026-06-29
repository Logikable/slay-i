use std::ops::{Deref, DerefMut};

use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};

/// Seedable, cloneable RNG. Replaces `ThreadRng` so a game can be forked
/// (cloned) for lookahead search and so rollouts can be made deterministic.
///
/// A newtype (rather than a bare `SmallRng` alias) lets us keep a `Default`
/// impl and add a seeding constructor. It transparently behaves as the inner
/// RNG: `DerefMut` gives it `rand_core::Rng` (and thus `RngExt`/`SliceRandom`)
/// for free, so all existing `rng.random_range(..)`/`shuffle(&mut rng)` call
/// sites keep working unchanged.
#[derive(Clone, Debug)]
pub struct Rand(SmallRng);

impl Rand {
    pub fn seed_from_u64(seed: u64) -> Self {
        Rand(SmallRng::seed_from_u64(seed))
    }
}

impl Default for Rand {
    fn default() -> Self {
        Rand(SmallRng::from_rng(&mut rand::rngs::ThreadRng::default()))
    }
}

impl Deref for Rand {
    type Target = SmallRng;
    fn deref(&self) -> &SmallRng {
        &self.0
    }
}

impl DerefMut for Rand {
    fn deref_mut(&mut self) -> &mut SmallRng {
        &mut self.0
    }
}

pub fn rand_slice<T: Copy>(rng: &mut Rand, slice: &[T]) -> T {
    let i = rng.random_range(0..slice.len());
    slice[i]
}

pub fn remove_random<T>(rng: &mut Rand, v: &mut Vec<T>) -> T {
    let i = rng.random_range(0..v.len());
    v.remove(i)
}
