use std::time::Instant;

#[test]
#[ignore]
fn benchmark_fibonacci() {
    let n = 35;
    let start = Instant::now();
    let res = fib(n);
    let elapsed = start.elapsed();
    assert_eq!(res, 9227465);
    println!("fib({n}) = {res} in {:?}", elapsed);
}

#[test]
#[ignore]
fn benchmark_nbody() {
    let mut system = nbody_system();
    let start = Instant::now();
    for _ in 0..100 {
        advance(&mut system, 0.01);
    }
    let e = energy(&system);
    let elapsed = start.elapsed();
    // Energia deve ser finita e estável dentro de um range aproximado.
    assert!(e.is_finite());
    println!("nbody energy = {e} in {:?}", elapsed);
}

#[test]
#[ignore]
fn benchmark_richards_like() {
    let start = Instant::now();
    let ops = richards_like(100_000);
    let elapsed = start.elapsed();
    assert!(ops > 0);
    println!("richards_like ops = {ops} in {:?}", elapsed);
}

#[test]
#[ignore]
fn benchmark_deltablue_like() {
    let start = Instant::now();
    let res = delta_blue_like(50_000);
    let elapsed = start.elapsed();
    assert!(res.is_finite());
    println!("deltablue_like result = {res} in {:?}", elapsed);
}

fn fib(n: i32) -> i32 {
    if n <= 1 { return n; }
    let mut a = 0;
    let mut b = 1;
    for _ in 2..=n {
        let c = a + b;
        a = b;
        b = c;
    }
    b
}

#[derive(Clone, Copy)]
struct Body {
    x: f64, y: f64, z: f64,
    vx: f64, vy: f64, vz: f64,
    mass: f64,
}

fn nbody_system() -> Vec<Body> {
    const PI: f64 = 3.141592653589793;
    const SOLAR_MASS: f64 = 4.0 * PI * PI;
    const DAYS_PER_YEAR: f64 = 365.24;

    let sun = Body { x: 0.0, y: 0.0, z: 0.0, vx: 0.0, vy: 0.0, vz: 0.0, mass: SOLAR_MASS };
    let jupiter = Body {
        x: 4.84143144246472090e+00,
        y: -1.16032004402742839e+00,
        z: -1.03622044471123109e-01,
        vx: 1.66007664274403694e-03 * DAYS_PER_YEAR,
        vy: 7.69901118419740425e-03 * DAYS_PER_YEAR,
        vz: -6.90460016972063023e-05 * DAYS_PER_YEAR,
        mass: 9.54791938424326609e-04 * SOLAR_MASS,
    };
    let saturn = Body {
        x: 8.34336671824457987e+00,
        y: 4.12479856412430479e+00,
        z: -4.03523417114321381e-01,
        vx: -2.76742510726862411e-03 * DAYS_PER_YEAR,
        vy: 4.99852801234917238e-03 * DAYS_PER_YEAR,
        vz: 2.30417297573763929e-05 * DAYS_PER_YEAR,
        mass: 2.85885980666130812e-04 * SOLAR_MASS,
    };
    let uranus = Body {
        x: 1.28943695621391310e+01,
        y: -1.51111514016986312e+01,
        z: -2.23307578892655734e-01,
        vx: 2.96460137564761618e-03 * DAYS_PER_YEAR,
        vy: 2.37847173959480950e-03 * DAYS_PER_YEAR,
        vz: -2.96589568540237556e-05 * DAYS_PER_YEAR,
        mass: 4.36624404335156298e-05 * SOLAR_MASS,
    };
    let neptune = Body {
        x: 1.53796971148509165e+01,
        y: -2.59193146099879641e+01,
        z: 1.79258772950371181e-01,
        vx: 2.68067772490389322e-03 * DAYS_PER_YEAR,
        vy: 1.62824170038242295e-03 * DAYS_PER_YEAR,
        vz: -9.51592254519715870e-05 * DAYS_PER_YEAR,
        mass: 5.15138902046611451e-05 * SOLAR_MASS,
    };

    let mut bodies = vec![sun, jupiter, saturn, uranus, neptune];
    // Ajustar momentum total para 0
    let mut px = 0.0;
    let mut py = 0.0;
    let mut pz = 0.0;
    for b in &bodies {
        px += b.vx * b.mass;
        py += b.vy * b.mass;
        pz += b.vz * b.mass;
    }
    bodies[0].vx = -px / SOLAR_MASS;
    bodies[0].vy = -py / SOLAR_MASS;
    bodies[0].vz = -pz / SOLAR_MASS;
    bodies
}

fn advance(bodies: &mut [Body], dt: f64) {
    let nb = bodies.len();
    for i in 0..nb {
        for j in (i + 1)..nb {
            let dx = bodies[i].x - bodies[j].x;
            let dy = bodies[i].y - bodies[j].y;
            let dz = bodies[i].z - bodies[j].z;
            let dist2 = dx * dx + dy * dy + dz * dz;
            let dist = dist2.sqrt();
            let mag = dt / (dist2 * dist);
            bodies[i].vx -= dx * bodies[j].mass * mag;
            bodies[i].vy -= dy * bodies[j].mass * mag;
            bodies[i].vz -= dz * bodies[j].mass * mag;
            bodies[j].vx += dx * bodies[i].mass * mag;
            bodies[j].vy += dy * bodies[i].mass * mag;
            bodies[j].vz += dz * bodies[i].mass * mag;
        }
    }
    for b in bodies {
        b.x += dt * b.vx;
        b.y += dt * b.vy;
        b.z += dt * b.vz;
    }
}

fn energy(bodies: &[Body]) -> f64 {
    let mut e = 0.0;
    let nb = bodies.len();
    for i in 0..nb {
        let bi = bodies[i];
        e += 0.5 * bi.mass * (bi.vx * bi.vx + bi.vy * bi.vy + bi.vz * bi.vz);
        for j in (i + 1)..nb {
            let bj = bodies[j];
            let dx = bi.x - bj.x;
            let dy = bi.y - bj.y;
            let dz = bi.z - bj.z;
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            e -= (bi.mass * bj.mass) / dist;
        }
    }
    e
}

fn richards_like(iterations: usize) -> usize {
    // Mini-scheduler com filas e work packets
    #[derive(Clone)]
    struct Packet { value: i32 }
    let mut queue: std::collections::VecDeque<Packet> = std::collections::VecDeque::new();
    let mut handled = 0usize;

    for i in 0..iterations {
        queue.push_back(Packet { value: i as i32 });
        if let Some(mut pkt) = queue.pop_front() {
            pkt.value += 1;
            handled += (pkt.value as usize) & 0xFF;
            if pkt.value % 2 == 0 {
                queue.push_back(pkt);
            }
        }
    }
    handled
}

fn delta_blue_like(iterations: usize) -> f64 {
    // Mini-solver de restrições: y = 2x + 1, z = y - 3
    let mut x = 0.0;
    let mut y = 1.0;
    let mut z = -2.0;
    for i in 0..iterations {
        x = i as f64 * 0.5;
        y = 2.0 * x + 1.0;
        z = y - 3.0;
    }
    x + y + z
}
