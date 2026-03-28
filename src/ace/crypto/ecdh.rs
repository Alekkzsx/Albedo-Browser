use crate::ace::crypto::random::get_random_bytes;

const MASK51: u64 = (1u64 << 51) - 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EcdhCurve {
    X25519,
    P256,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct X25519SecretKey([u8; 32]);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct X25519PublicKey([u8; 32]);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct P256SecretKey([u8; 32]);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct P256PublicKey {
    x: U256,
    y: U256,
}

impl X25519SecretKey {
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        get_random_bytes(&mut bytes);
        clamp_scalar(&mut bytes);
        Self(bytes)
    }

    pub fn from_bytes(mut bytes: [u8; 32]) -> Self {
        clamp_scalar(&mut bytes);
        Self(bytes)
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }

    pub fn public_key(&self) -> X25519PublicKey {
        X25519PublicKey(x25519(
            self.0,
            [
                9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0,
            ],
        ))
    }

    pub fn diffie_hellman(&self, public: &X25519PublicKey) -> [u8; 32] {
        x25519(self.0, public.0)
    }
}

impl X25519PublicKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
}

impl P256SecretKey {
    pub fn generate() -> Self {
        loop {
            let mut bytes = [0u8; 32];
            get_random_bytes(&mut bytes);
            if let Ok(sk) = Self::from_bytes(bytes) {
                return sk;
            }
        }
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Result<Self, String> {
        let scalar = U256::from_be_bytes(bytes);
        if scalar.is_zero() || scalar.cmp(&p256_order()).is_ge() {
            return Err("invalid P-256 private key".to_string());
        }
        Ok(Self(bytes))
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }

    pub fn public_key(&self) -> Result<P256PublicKey, String> {
        let scalar = U256::from_be_bytes(self.0);
        let point = p256_scalar_mul_base(scalar);
        point.to_public()
    }

    pub fn diffie_hellman(&self, public: &P256PublicKey) -> Result<[u8; 32], String> {
        let scalar = U256::from_be_bytes(self.0);
        let point = p256_scalar_mul(scalar, public);
        let affine = point.to_public()?;
        Ok(affine.x.to_be_bytes())
    }
}

impl P256PublicKey {
    pub fn from_uncompressed_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != 65 || bytes[0] != 0x04 {
            return Err("invalid P-256 uncompressed public key".to_string());
        }
        let mut xb = [0u8; 32];
        let mut yb = [0u8; 32];
        xb.copy_from_slice(&bytes[1..33]);
        yb.copy_from_slice(&bytes[33..65]);
        let pk = Self {
            x: U256::from_be_bytes(xb),
            y: U256::from_be_bytes(yb),
        };
        if !pk.is_on_curve() {
            return Err("P-256 public key is not on curve".to_string());
        }
        Ok(pk)
    }

    pub fn to_uncompressed_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(65);
        out.push(0x04);
        out.extend_from_slice(&self.x.to_be_bytes());
        out.extend_from_slice(&self.y.to_be_bytes());
        out
    }

    fn is_on_curve(&self) -> bool {
        let x = P256Field(self.x);
        let y = P256Field(self.y);
        let lhs = y.square();
        let rhs = x
            .square()
            .mul(x)
            .add(P256Field(p256_a()).mul(x))
            .add(P256Field(p256_b()));
        lhs == rhs
    }
}

pub fn p256_generate_keypair() -> Result<([u8; 32], Vec<u8>), String> {
    let sk = P256SecretKey::generate();
    let pk = sk.public_key()?;
    Ok((sk.to_bytes(), pk.to_uncompressed_bytes()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FieldElement([u64; 5]);

impl FieldElement {
    fn zero() -> Self {
        Self([0; 5])
    }

    fn one() -> Self {
        Self([1, 0, 0, 0, 0])
    }

    fn from_bytes(bytes: [u8; 32]) -> Self {
        let mut t = [0u64; 4];
        for i in 0..4 {
            let mut chunk = [0u8; 8];
            chunk.copy_from_slice(&bytes[i * 8..(i + 1) * 8]);
            t[i] = u64::from_le_bytes(chunk);
        }
        let h0 = t[0] & MASK51;
        let h1 = ((t[0] >> 51) | (t[1] << 13)) & MASK51;
        let h2 = ((t[1] >> 38) | (t[2] << 26)) & MASK51;
        let h3 = ((t[2] >> 25) | (t[3] << 39)) & MASK51;
        let h4 = (t[3] >> 12) & MASK51;
        Self([h0, h1, h2, h3, h4]).normalize()
    }

    fn to_bytes(self) -> [u8; 32] {
        let mut h = self.normalize();
        let p = [MASK51 - 18, MASK51, MASK51, MASK51, MASK51];
        if ge_limbs(&h.0, &p) {
            h = h.sub_raw(&FieldElement(p));
        }

        let h0 = h.0[0] as u128;
        let h1 = h.0[1] as u128;
        let h2 = h.0[2] as u128;
        let h3 = h.0[3] as u128;
        let h4 = h.0[4] as u128;

        let t0 = h0 | (h1 << 51);
        let t1 = (h1 >> 13) | (h2 << 38);
        let t2 = (h2 >> 26) | (h3 << 25);
        let t3 = (h3 >> 39) | (h4 << 12);

        let mut out = [0u8; 32];
        out[0..8].copy_from_slice(&(t0 as u64).to_le_bytes());
        out[8..16].copy_from_slice(&(t1 as u64).to_le_bytes());
        out[16..24].copy_from_slice(&(t2 as u64).to_le_bytes());
        out[24..32].copy_from_slice(&(t3 as u64).to_le_bytes());
        out
    }

    fn normalize(mut self) -> Self {
        let mut carry = self.0[0] >> 51;
        self.0[0] &= MASK51;
        self.0[1] += carry;

        carry = self.0[1] >> 51;
        self.0[1] &= MASK51;
        self.0[2] += carry;

        carry = self.0[2] >> 51;
        self.0[2] &= MASK51;
        self.0[3] += carry;

        carry = self.0[3] >> 51;
        self.0[3] &= MASK51;
        self.0[4] += carry;

        carry = self.0[4] >> 51;
        self.0[4] &= MASK51;
        self.0[0] += carry * 19;

        carry = self.0[0] >> 51;
        self.0[0] &= MASK51;
        self.0[1] += carry;
        self
    }

    fn add(self, rhs: &Self) -> Self {
        let mut out = [0u64; 5];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = self.0[i] + rhs.0[i];
        }
        Self(out).normalize()
    }

    fn sub(self, rhs: &Self) -> Self {
        let b = [
            (MASK51 - 18) * 4,
            MASK51 * 4,
            MASK51 * 4,
            MASK51 * 4,
            MASK51 * 4,
        ];
        let mut out = [0u64; 5];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = self.0[i] + b[i] - rhs.0[i];
        }
        Self(out).normalize()
    }

    fn sub_raw(self, rhs: &Self) -> Self {
        let mut borrow = 0i128;
        let mut out = [0u64; 5];
        for (i, slot) in out.iter_mut().enumerate() {
            let lhs = self.0[i] as i128;
            let rhs = rhs.0[i] as i128;
            let mut val = lhs - rhs - borrow;
            if val < 0 {
                val += 1i128 << 51;
                borrow = 1;
            } else {
                borrow = 0;
            }
            *slot = val as u64;
        }
        Self(out)
    }

    fn mul(self, rhs: &Self) -> Self {
        let a = self.0;
        let b = rhs.0;
        let a1_19 = (a[1] as u128) * 19;
        let a2_19 = (a[2] as u128) * 19;
        let a3_19 = (a[3] as u128) * 19;
        let a4_19 = (a[4] as u128) * 19;

        let mut c = [0u128; 5];
        c[0] = (a[0] as u128 * b[0] as u128)
            + a1_19 * b[4] as u128
            + a2_19 * b[3] as u128
            + a3_19 * b[2] as u128
            + a4_19 * b[1] as u128;
        c[1] = (a[0] as u128 * b[1] as u128)
            + (a[1] as u128 * b[0] as u128)
            + a2_19 * b[4] as u128
            + a3_19 * b[3] as u128
            + a4_19 * b[2] as u128;
        c[2] = (a[0] as u128 * b[2] as u128)
            + (a[1] as u128 * b[1] as u128)
            + (a[2] as u128 * b[0] as u128)
            + a3_19 * b[4] as u128
            + a4_19 * b[3] as u128;
        c[3] = (a[0] as u128 * b[3] as u128)
            + (a[1] as u128 * b[2] as u128)
            + (a[2] as u128 * b[1] as u128)
            + (a[3] as u128 * b[0] as u128)
            + a4_19 * b[4] as u128;
        c[4] = (a[0] as u128 * b[4] as u128)
            + (a[1] as u128 * b[3] as u128)
            + (a[2] as u128 * b[2] as u128)
            + (a[3] as u128 * b[1] as u128)
            + (a[4] as u128 * b[0] as u128);

        let mut out = [0u64; 5];
        let mut carry = 0u128;
        for i in 0..5 {
            c[i] += carry;
            out[i] = (c[i] & MASK51 as u128) as u64;
            carry = c[i] >> 51;
        }
        out[0] = out[0].wrapping_add((carry * 19) as u64);
        Self(out).normalize()
    }

    fn square(self) -> Self {
        self.mul(&self)
    }

    fn invert(self) -> Self {
        // z^(p-2) where p = 2^255 - 19
        let z2 = self.square();
        let z4 = z2.square();
        let z8 = z4.square();
        let z9 = z8.mul(&self);
        let z11 = z9.mul(&z2);
        let z2_5_0 = z11.mul(&z9);

        let z2_10_0 = pow2k(z2_5_0, 5).mul(&z2_5_0);
        let z2_20_0 = pow2k(z2_10_0, 10).mul(&z2_10_0);
        let z2_40_0 = pow2k(z2_20_0, 20).mul(&z2_20_0);
        let z2_50_0 = pow2k(z2_40_0, 10).mul(&z2_10_0);
        let z2_100_0 = pow2k(z2_50_0, 50).mul(&z2_50_0);
        let z2_200_0 = pow2k(z2_100_0, 100).mul(&z2_100_0);
        let z2_250_0 = pow2k(z2_200_0, 50).mul(&z2_50_0);
        pow2k(z2_250_0, 5).mul(&z11)
    }

    fn cswap(a: &mut Self, b: &mut Self, swap: u8) {
        let mask = 0u64.wrapping_sub(swap as u64);
        for i in 0..5 {
            let t = mask & (a.0[i] ^ b.0[i]);
            a.0[i] ^= t;
            b.0[i] ^= t;
        }
    }
}

fn pow2k(mut z: FieldElement, k: usize) -> FieldElement {
    for _ in 0..k {
        z = z.square();
    }
    z
}

fn ge_limbs(a: &[u64; 5], b: &[u64; 5]) -> bool {
    for i in (0..5).rev() {
        if a[i] > b[i] {
            return true;
        }
        if a[i] < b[i] {
            return false;
        }
    }
    true
}

fn clamp_scalar(s: &mut [u8; 32]) {
    s[0] &= 248;
    s[31] &= 127;
    s[31] |= 64;
}

pub fn x25519(mut scalar: [u8; 32], point: [u8; 32]) -> [u8; 32] {
    clamp_scalar(&mut scalar);
    let x1 = FieldElement::from_bytes(point);
    let mut x2 = FieldElement::one();
    let mut z2 = FieldElement::zero();
    let mut x3 = x1;
    let mut z3 = FieldElement::one();
    let mut swap = 0u8;

    for t in (0..255).rev() {
        let k_t = (scalar[t / 8] >> (t & 7)) & 1;
        swap ^= k_t;
        FieldElement::cswap(&mut x2, &mut x3, swap);
        FieldElement::cswap(&mut z2, &mut z3, swap);
        swap = k_t;

        let a = x2.add(&z2);
        let aa = a.square();
        let b = x2.sub(&z2);
        let bb = b.square();
        let e = aa.sub(&bb);
        let c = x3.add(&z3);
        let d = x3.sub(&z3);
        let da = d.mul(&a);
        let cb = c.mul(&b);
        x3 = da.add(&cb).square();
        z3 = x1.mul(&da.sub(&cb).square());
        x2 = aa.mul(&bb);
        let a24e = FieldElement([121666, 0, 0, 0, 0]).mul(&e);
        z2 = e.mul(&aa.add(&a24e));
    }

    FieldElement::cswap(&mut x2, &mut x3, swap);
    FieldElement::cswap(&mut z2, &mut z3, swap);

    x2.mul(&z2.invert()).to_bytes()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct U256([u64; 4]);

impl U256 {
    fn zero() -> Self {
        Self([0; 4])
    }

    fn one() -> Self {
        Self([1, 0, 0, 0])
    }

    fn from_be_bytes(bytes: [u8; 32]) -> Self {
        let mut limbs = [0u64; 4];
        for i in 0..4 {
            let start = 24 - (i * 8);
            let mut chunk = [0u8; 8];
            chunk.copy_from_slice(&bytes[start..start + 8]);
            limbs[i] = u64::from_be_bytes(chunk);
        }
        Self(limbs)
    }

    fn to_be_bytes(self) -> [u8; 32] {
        let mut out = [0u8; 32];
        for i in 0..4 {
            let start = 24 - (i * 8);
            out[start..start + 8].copy_from_slice(&self.0[i].to_be_bytes());
        }
        out
    }

    fn is_zero(&self) -> bool {
        self.0.iter().all(|&x| x == 0)
    }

    fn cmp(&self, rhs: &Self) -> std::cmp::Ordering {
        for i in (0..4).rev() {
            if self.0[i] < rhs.0[i] {
                return std::cmp::Ordering::Less;
            }
            if self.0[i] > rhs.0[i] {
                return std::cmp::Ordering::Greater;
            }
        }
        std::cmp::Ordering::Equal
    }

    fn bit(&self, idx: usize) -> bool {
        let limb = idx / 64;
        let bit = idx % 64;
        ((self.0[limb] >> bit) & 1) == 1
    }

    fn add_raw(self, rhs: Self) -> (Self, bool) {
        let mut out = [0u64; 4];
        let mut carry = 0u128;
        for (i, slot) in out.iter_mut().enumerate() {
            let sum = self.0[i] as u128 + rhs.0[i] as u128 + carry;
            *slot = sum as u64;
            carry = sum >> 64;
        }
        (Self(out), carry != 0)
    }

    fn sub_raw(self, rhs: Self) -> (Self, bool) {
        let mut out = [0u64; 4];
        let mut borrow = 0u128;
        for (i, slot) in out.iter_mut().enumerate() {
            let lhs = self.0[i] as u128;
            let rhs = rhs.0[i] as u128 + borrow;
            if lhs >= rhs {
                *slot = (lhs - rhs) as u64;
                borrow = 0;
            } else {
                *slot = ((1u128 << 64) + lhs - rhs) as u64;
                borrow = 1;
            }
        }
        (Self(out), borrow != 0)
    }

    fn add_mod(self, rhs: Self, modulus: Self) -> Self {
        let (sum, carry) = self.add_raw(rhs);
        if carry || sum.cmp(&modulus).is_ge() {
            sum.sub_raw(modulus).0
        } else {
            sum
        }
    }

    fn sub_mod(self, rhs: Self, modulus: Self) -> Self {
        let (diff, borrow) = self.sub_raw(rhs);
        if borrow {
            diff.add_raw(modulus).0
        } else {
            diff
        }
    }

    fn mul_mod(self, rhs: Self, modulus: Self) -> Self {
        let mut result = U256::zero();
        let mut base = if self.cmp(&modulus).is_ge() {
            reduce_once(self, modulus)
        } else {
            self
        };

        for i in 0..256 {
            if rhs.bit(i) {
                result = result.add_mod(base, modulus);
            }
            base = base.add_mod(base, modulus);
        }

        result
    }

    fn pow_mod(self, exp: Self, modulus: Self) -> Self {
        let mut result = U256::one();
        let mut base = self;
        for i in 0..256 {
            if exp.bit(i) {
                result = result.mul_mod(base, modulus);
            }
            base = base.mul_mod(base, modulus);
        }
        result
    }
}

fn reduce_once(value: U256, modulus: U256) -> U256 {
    if value.cmp(&modulus).is_ge() {
        value.sub_raw(modulus).0
    } else {
        value
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct P256Field(U256);

impl P256Field {
    fn zero() -> Self {
        Self(U256::zero())
    }

    fn one() -> Self {
        Self(U256::one())
    }

    fn add(self, rhs: Self) -> Self {
        Self(self.0.add_mod(rhs.0, p256_prime()))
    }

    fn sub(self, rhs: Self) -> Self {
        Self(self.0.sub_mod(rhs.0, p256_prime()))
    }

    fn mul(self, rhs: Self) -> Self {
        Self(self.0.mul_mod(rhs.0, p256_prime()))
    }

    fn square(self) -> Self {
        self.mul(self)
    }

    fn invert(self) -> Self {
        let (exp, _) = p256_prime().sub_raw(U256([2, 0, 0, 0]));
        Self(self.0.pow_mod(exp, p256_prime()))
    }

    fn mul_small(self, n: u32) -> Self {
        let mut out = Self::zero();
        for _ in 0..n {
            out = out.add(self);
        }
        out
    }

    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

#[derive(Clone, Copy, Debug)]
struct P256JacobianPoint {
    x: P256Field,
    y: P256Field,
    z: P256Field,
}

impl P256JacobianPoint {
    fn infinity() -> Self {
        Self {
            x: P256Field::zero(),
            y: P256Field::one(),
            z: P256Field::zero(),
        }
    }

    fn from_affine(point: &P256PublicKey) -> Self {
        Self {
            x: P256Field(point.x),
            y: P256Field(point.y),
            z: P256Field::one(),
        }
    }

    fn is_infinity(&self) -> bool {
        self.z.is_zero()
    }

    fn double(self) -> Self {
        if self.is_infinity() || self.y.is_zero() {
            return Self::infinity();
        }

        let delta = self.z.square();
        let gamma = self.y.square();
        let beta = self.x.mul(gamma);
        let alpha = self.x.sub(delta).mul(self.x.add(delta)).mul_small(3);
        let x3 = alpha.square().sub(beta.mul_small(8));
        let z3 = self.y.add(self.z).square().sub(gamma).sub(delta);
        let y3 = alpha
            .mul(beta.mul_small(4).sub(x3))
            .sub(gamma.square().mul_small(8));

        Self {
            x: x3,
            y: y3,
            z: z3,
        }
    }

    fn add_mixed(self, point: &P256PublicKey) -> Self {
        if self.is_infinity() {
            return Self::from_affine(point);
        }

        let z1z1 = self.z.square();
        let u2 = P256Field(point.x).mul(z1z1);
        let s2 = P256Field(point.y).mul(z1z1.mul(self.z));

        if u2 == self.x {
            if s2 == self.y {
                return self.double();
            }
            return Self::infinity();
        }

        let h = u2.sub(self.x);
        let hh = h.square();
        let i = hh.mul_small(4);
        let j = h.mul(i);
        let r = s2.sub(self.y).mul_small(2);
        let v = self.x.mul(i);

        let x3 = r.square().sub(j).sub(v.mul_small(2));
        let y3 = r.mul(v.sub(x3)).sub(self.y.mul(j).mul_small(2));
        let z3 = self.z.add(h).square().sub(z1z1).sub(hh);

        Self {
            x: x3,
            y: y3,
            z: z3,
        }
    }

    fn to_public(self) -> Result<P256PublicKey, String> {
        if self.is_infinity() {
            return Err("P-256 point at infinity".to_string());
        }
        let z_inv = self.z.invert();
        let z2 = z_inv.square();
        let x = self.x.mul(z2);
        let y = self.y.mul(z2).mul(z_inv);
        Ok(P256PublicKey { x: x.0, y: y.0 })
    }
}

fn p256_scalar_mul_base(scalar: U256) -> P256JacobianPoint {
    let base = P256PublicKey {
        x: p256_gx(),
        y: p256_gy(),
    };
    p256_scalar_mul(scalar, &base)
}

fn p256_scalar_mul(scalar: U256, point: &P256PublicKey) -> P256JacobianPoint {
    let mut acc = P256JacobianPoint::infinity();
    for bit in (0..256).rev() {
        acc = acc.double();
        if scalar.bit(bit) {
            acc = acc.add_mixed(point);
        }
    }
    acc
}

fn p256_prime() -> U256 {
    U256([
        0xffff_ffff_ffff_ffff,
        0x0000_0000_ffff_ffff,
        0x0000_0000_0000_0000,
        0xffff_ffff_0000_0001,
    ])
}

fn p256_order() -> U256 {
    U256([
        0xf3b9_cac2_fc63_2551,
        0xbce6_faad_a717_9e84,
        0xffff_ffff_ffff_ffff,
        0xffff_ffff_0000_0000,
    ])
}

fn p256_a() -> U256 {
    let (a, _) = p256_prime().sub_raw(U256([3, 0, 0, 0]));
    a
}

fn p256_b() -> U256 {
    U256::from_be_bytes([
        0x5a, 0xc6, 0x35, 0xd8, 0xaa, 0x3a, 0x93, 0xe7, 0xb3, 0xeb, 0xbd, 0x55, 0x76, 0x98, 0x86,
        0xbc, 0x65, 0x1d, 0x06, 0xb0, 0xcc, 0x53, 0xb0, 0xf6, 0x3b, 0xce, 0x3c, 0x3e, 0x27, 0xd2,
        0x60, 0x4b,
    ])
}

fn p256_gx() -> U256 {
    U256::from_be_bytes([
        0x6b, 0x17, 0xd1, 0xf2, 0xe1, 0x2c, 0x42, 0x47, 0xf8, 0xbc, 0xe6, 0xe5, 0x63, 0xa4, 0x40,
        0xf2, 0x77, 0x03, 0x7d, 0x81, 0x2d, 0xeb, 0x33, 0xa0, 0xf4, 0xa1, 0x39, 0x45, 0xd8, 0x98,
        0xc2, 0x96,
    ])
}

fn p256_gy() -> U256 {
    U256::from_be_bytes([
        0x4f, 0xe3, 0x42, 0xe2, 0xfe, 0x1a, 0x7f, 0x9b, 0x8e, 0xe7, 0xeb, 0x4a, 0x7c, 0x0f, 0x9e,
        0x16, 0x2b, 0xce, 0x33, 0x57, 0x6b, 0x31, 0x5e, 0xce, 0xcb, 0xb6, 0x40, 0x68, 0x37, 0xbf,
        0x51, 0xf5,
    ])
}

#[cfg(test)]
mod tests {
    use super::{x25519, P256SecretKey, X25519PublicKey, X25519SecretKey};
    use crate::ace::util::hex::{decode, encode};

    #[test]
    fn x25519_rfc7748_vector() {
        let scalar: [u8; 32] = decode(
            "77076d0a7318a57d3c16c17251b26645\
             df4c2f87ebc0992ab177fba51db92c2a",
        )
        .unwrap()
        .try_into()
        .unwrap();
        let point: [u8; 32] = decode(
            "09000000000000000000000000000000\
             00000000000000000000000000000000",
        )
        .unwrap()
        .try_into()
        .unwrap();
        let expected = "8520f0098930a754748b7ddcb43ef75a\
                        0dbf3a0d26381af4eba4a98eaa9b4e6a"
            .replace(' ', "");

        assert_eq!(encode(&x25519(scalar, point)), expected);
    }

    #[test]
    fn x25519_ecdh_is_symmetric() {
        let alice = X25519SecretKey::from_bytes([1u8; 32]);
        let bob = X25519SecretKey::from_bytes([2u8; 32]);

        let alice_pub = alice.public_key();
        let bob_pub = bob.public_key();

        let s1 = alice.diffie_hellman(&bob_pub);
        let s2 = bob.diffie_hellman(&alice_pub);
        assert_eq!(s1, s2);
    }

    #[test]
    fn public_key_round_trip() {
        let sk = X25519SecretKey::generate();
        let pk = sk.public_key();
        let round = X25519PublicKey::from_bytes(pk.to_bytes());
        assert_eq!(pk, round);
    }

    #[test]
    fn p256_public_key_round_trip() {
        let sk = P256SecretKey::from_bytes({
            let mut bytes = [0u8; 32];
            bytes[31] = 1;
            bytes
        })
        .unwrap();
        let pk = sk.public_key().unwrap();
        let encoded = pk.to_uncompressed_bytes();
        let round = super::P256PublicKey::from_uncompressed_bytes(&encoded).unwrap();
        assert_eq!(pk, round);
    }

    #[test]
    fn p256_ecdh_is_symmetric() {
        let alice = P256SecretKey::from_bytes({
            let mut bytes = [0u8; 32];
            bytes[31] = 3;
            bytes
        })
        .unwrap();
        let bob = P256SecretKey::from_bytes({
            let mut bytes = [0u8; 32];
            bytes[31] = 7;
            bytes
        })
        .unwrap();

        let alice_pub = alice.public_key().unwrap();
        let bob_pub = bob.public_key().unwrap();

        let s1 = alice.diffie_hellman(&bob_pub).unwrap();
        let s2 = bob.diffie_hellman(&alice_pub).unwrap();
        assert_eq!(s1, s2);
    }
}
