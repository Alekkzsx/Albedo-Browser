use ace_core::math::{Matrix3x3, Rect, Vec2, Vec3};

/// Um Gerador Congruente Linear (LCG) hiper-rápido para geração de valores determinísticos pseudorandom.
/// Zero-dependency, perfeito para não poluir o projeto com bibliotecas pesadas de testes como `proptest` ou `rand`.
struct FastLCG {
    state: u64,
}

impl FastLCG {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.state >> 32) as u32
    }

    fn next_f32(&mut self, min: f32, max: f32) -> f32 {
        let f = (self.next_u32() as f32) / (u32::MAX as f32);
        min + f * (max - min)
    }
}

#[test]
fn vec2_length_squared_always_positive() {
    let mut rng = FastLCG::new(12345);
    for _ in 0..10_000 {
        let x = rng.next_f32(-1e6, 1e6);
        let y = rng.next_f32(-1e6, 1e6);
        let v = Vec2::new(x, y);
        assert!(v.length_squared() >= 0.0);
    }
}

#[test]
fn rect_union_commutative() {
    let mut rng = FastLCG::new(54321);
    for _ in 0..5_000 {
        let a = Rect::new(
            rng.next_f32(-1e3, 1e3),
            rng.next_f32(-1e3, 1e3),
            rng.next_f32(0.0, 1e3),
            rng.next_f32(0.0, 1e3),
        );
        let b = Rect::new(
            rng.next_f32(-1e3, 1e3),
            rng.next_f32(-1e3, 1e3),
            rng.next_f32(0.0, 1e3),
            rng.next_f32(0.0, 1e3),
        );

        let union_ab = a.union(&b);
        let union_ba = b.union(&a);

        assert!(union_ab.approx_eq(&union_ba, 1e-4));
    }
}

#[test]
fn matrix3x3_inverse_property() {
    let mut rng = FastLCG::new(9999);
    for _ in 0..5_000 {
        let m = Matrix3x3::new([
            rng.next_f32(-1e2, 1e2), rng.next_f32(-1e2, 1e2), rng.next_f32(-1e2, 1e2),
            rng.next_f32(-1e2, 1e2), rng.next_f32(-1e2, 1e2), rng.next_f32(-1e2, 1e2),
            rng.next_f32(-1e2, 1e2), rng.next_f32(-1e2, 1e2), rng.next_f32(-1e2, 1e2),
        ]);

        if let Some(inv) = m.inverse() {
            let identity = m * inv;
            assert!(identity.approx_eq(&Matrix3x3::identity(), 1e-1)); // floats grandes sofrem perda, 1e-1 é tolerável
        }
    }
}

#[test]
fn vec3_cross_orthogonal() {
    let mut rng = FastLCG::new(777);
    for _ in 0..5_000 {
        let u = Vec3::new(rng.next_f32(-1e2, 1e2), rng.next_f32(-1e2, 1e2), rng.next_f32(-1e2, 1e2));
        let v = Vec3::new(rng.next_f32(-1e2, 1e2), rng.next_f32(-1e2, 1e2), rng.next_f32(-1e2, 1e2));

        let cross = u.cross(&v);

        let dot_u = ace_core::math::abs(u.dot(&cross));
        let dot_v = ace_core::math::abs(v.dot(&cross));

        assert!(dot_u < 5.0); // Tolerância razoável para acúmulo float
        assert!(dot_v < 5.0);
    }
}
