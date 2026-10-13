//! # Rayon Multithreading Concurrency (`rayon_parallel.rs`)
//!
//! Provides parallel execution across independent formatting contexts,
//! utilizing Rayon's work-stealing thread pool for independent subtrees.

use rayon::prelude::*;

/// Minimum item count threshold to justify spawning work across thread pools.
pub const RAYON_PARALLEL_THRESHOLD: usize = 4;

/// Executes a layout function over a collection of independent subtrees in parallel if
/// the item count reaches the threshold, falling back to sequential processing otherwise.
pub fn par_layout_subtrees<T, R, F>(items: Vec<T>, f: F) -> Vec<R>
where
    T: Send,
    R: Send,
    F: Fn(T) -> R + Sync + Send,
{
    if items.len() >= RAYON_PARALLEL_THRESHOLD {
        items.into_par_iter().map(f).collect()
    } else {
        items.into_iter().map(f).collect()
    }
}

/// Executes two independent layout computations concurrently using `rayon::join`.
pub fn par_join_subtrees<A, B, RA, RB>(task_a: A, task_b: B) -> (RA, RB)
where
    A: FnOnce() -> RA + Send,
    B: FnOnce() -> RB + Send,
    RA: Send,
    RB: Send,
{
    rayon::join(task_a, task_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_par_layout_subtrees_below_and_above_threshold() {
        let small = vec![1, 2, 3];
        let res_small = par_layout_subtrees(small, |x| x * 2);
        assert_eq!(res_small, vec![2, 4, 6]);

        let large: Vec<i32> = (0..20).collect();
        let res_large = par_layout_subtrees(large, |x| x * 10);
        assert_eq!(res_large.len(), 20);
        assert_eq!(res_large[5], 50);
    }

    #[test]
    fn test_par_join_subtrees() {
        let (a, b) = par_join_subtrees(|| 10 + 20, || "hello".to_uppercase());
        assert_eq!(a, 30);
        assert_eq!(b, "HELLO");
    }
}
