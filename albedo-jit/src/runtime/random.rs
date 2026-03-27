use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static RNG_STATE: AtomicU64 = AtomicU64::new(0);

pub fn next_f64() -> f64 {
    // [0, 1)
    let v = next_u64() >> 11;
    (v as f64) * (1.0 / ((1u64 << 53) as f64))
}

fn next_u64() -> u64 {
    let mut current = RNG_STATE.load(Ordering::Relaxed);
    if current == 0 {
        current = initial_seed();
        RNG_STATE.store(current, Ordering::Relaxed);
    }

    loop {
        let mut x = current;
        // xorshift64*
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        let next = x.wrapping_mul(0x2545F4914F6CDD1D);

        match RNG_STATE.compare_exchange(current, next, Ordering::SeqCst, Ordering::Relaxed) {
            Ok(_) => return next,
            Err(observed) => current = observed,
        }
    }
}

fn initial_seed() -> u64 {
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let addr = (&RNG_STATE as *const AtomicU64 as usize) as u64;
    let seed = t ^ addr.rotate_left(17) ^ 0x9E37_79B9_7F4A_7C15;
    if seed == 0 {
        0xA076_1D64_78BD_642F
    } else {
        seed
    }
}
