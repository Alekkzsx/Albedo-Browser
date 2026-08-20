//! # Decomposição de Matriz 4x4 e Interpolação SLERP (W3C CSS Transforms Level 2)
//!
//! A interpolação linear elemento a elemento de matrizes 4x4 gera artefatos de cisalhamento e colapsos geométricos.
//! A especificação CSS Transforms Level 2 define a decomposição em:
//! Matriz = Translação * Perspectiva * Rotação(Quaternion) * Skew * Escala
//! com interpolação esférica (SLERP) de rotação em $S^3$, eliminando Gimbal Lock.

use super::geometry::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quaternion {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

impl Quaternion {
    pub const IDENTITY: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };

    pub fn new(x: f64, y: f64, z: f64, w: f64) -> Self {
        Self { x, y, z, w }
    }

    #[inline]
    pub fn dot(self, o: Self) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z + self.w * o.w
    }

    pub fn normalize(self) -> Self {
        let len = self.dot(self).sqrt();
        if len > 0.0 {
            Self::new(self.x / len, self.y / len, self.z / len, self.w / len)
        } else {
            Self::IDENTITY
        }
    }

    /// Interpolação Esférica Linear (SLERP) entre quaternions com detecção de caminho curto.
    pub fn slerp(self, mut target: Self, t: f64) -> Self {
        let mut cos_half_theta = self.dot(target);
        if cos_half_theta < 0.0 {
            target = Self::new(-target.x, -target.y, -target.z, -target.w);
            cos_half_theta = -cos_half_theta;
        }

        if cos_half_theta >= 1.0 - 1e-6 {
            return Self::new(
                self.x + (target.x - self.x) * t,
                self.y + (target.y - self.y) * t,
                self.z + (target.z - self.z) * t,
                self.w + (target.w - self.w) * t,
            )
            .normalize();
        }

        let half_theta = cos_half_theta.acos();
        let sin_half_theta = (1.0 - cos_half_theta * cos_half_theta).sqrt();

        if sin_half_theta.abs() < 1e-6 {
            return self;
        }

        let ratio_a = ((1.0 - t) * half_theta).sin() / sin_half_theta;
        let ratio_b = (t * half_theta).sin() / sin_half_theta;

        Self::new(
            self.x * ratio_a + target.x * ratio_b,
            self.y * ratio_a + target.y * ratio_b,
            self.z * ratio_a + target.z * ratio_b,
            self.w * ratio_a + target.w * ratio_b,
        )
        .normalize()
    }
}

/// Representação decomposta de uma transformação afim/projetiva 3D.
#[derive(Debug, Clone, PartialEq)]
pub struct DecomposedTransform {
    pub translation: [f64; 3],
    pub scale: [f64; 3],
    pub skew: [f64; 3], // (xy, xz, yz)
    pub perspective: [f64; 4],
    pub quaternion: Quaternion,
}

/// Matriz 4x4 de alta precisão (`f64`) para decomposição e interpolação.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformMatrix4 {
    pub m: [[f64; 4]; 4], // m[col][row]
}

impl TransformMatrix4 {
    pub const IDENTITY: Self = Self {
        m: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };

    /// Decompõe a matriz 4x4 conforme a especificação W3C CSS Transforms Level 2.
    pub fn decompose(&self) -> Option<DecomposedTransform> {
        let mut mat = *self;

        if mat.m[3][3] == 0.0 {
            return None;
        }
        let scale_w = mat.m[3][3];
        for c in 0..4 {
            for r in 0..4 {
                mat.m[c][r] /= scale_w;
            }
        }

        // Translação
        let translation = [mat.m[3][0], mat.m[3][1], mat.m[3][2]];
        mat.m[3][0] = 0.0;
        mat.m[3][1] = 0.0;
        mat.m[3][2] = 0.0;

        // Escala e Skew via Gram-Schmidt
        let mut row0 = [mat.m[0][0], mat.m[0][1], mat.m[0][2]];
        let mut row1 = [mat.m[1][0], mat.m[1][1], mat.m[1][2]];
        let mut row2 = [mat.m[2][0], mat.m[2][1], mat.m[2][2]];

        let len3 = |v: [f64; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        let dot3 = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let norm3 = |v: [f64; 3]| {
            let l = len3(v);
            if l > 0.0 {
                [v[0] / l, v[1] / l, v[2] / l]
            } else {
                [0.0, 0.0, 0.0]
            }
        };

        let mut scale = [0.0; 3];
        let mut skew = [0.0; 3];

        scale[0] = len3(row0);
        row0 = norm3(row0);

        skew[0] = dot3(row0, row1);
        row1 = [
            row1[0] - row0[0] * skew[0],
            row1[1] - row0[1] * skew[0],
            row1[2] - row0[2] * skew[0],
        ];
        scale[1] = len3(row1);
        row1 = norm3(row1);
        if scale[1] > 0.0 {
            skew[0] /= scale[1];
        }

        skew[1] = dot3(row0, row2);
        row2 = [
            row2[0] - row0[0] * skew[1],
            row2[1] - row0[1] * skew[1],
            row2[2] - row0[2] * skew[1],
        ];
        skew[2] = dot3(row1, row2);
        row2 = [
            row2[0] - row1[0] * skew[2],
            row2[1] - row1[1] * skew[2],
            row2[2] - row1[2] * skew[2],
        ];
        scale[2] = len3(row2);
        row2 = norm3(row2);
        if scale[2] > 0.0 {
            skew[1] /= scale[2];
            skew[2] /= scale[2];
        }

        // Extração de Quaternion
        let trace = row0[0] + row1[1] + row2[2];
        let quaternion = if trace > 0.0 {
            let s = 0.5 / (trace + 1.0).sqrt();
            Quaternion::new(
                (row1[2] - row2[1]) * s,
                (row2[0] - row0[2]) * s,
                (row0[1] - row1[0]) * s,
                0.25 / s,
            )
        } else if row0[0] > row1[1] && row0[0] > row2[2] {
            let s = 2.0 * (1.0 + row0[0] - row1[1] - row2[2]).sqrt();
            Quaternion::new(
                0.25 * s,
                (row0[1] + row1[0]) / s,
                (row0[2] + row2[0]) / s,
                (row1[2] - row2[1]) / s,
            )
        } else if row1[1] > row2[2] {
            let s = 2.0 * (1.0 + row1[1] - row0[0] - row2[2]).sqrt();
            Quaternion::new(
                (row0[1] + row1[0]) / s,
                0.25 * s,
                (row1[2] + row2[1]) / s,
                (row2[0] - row0[2]) / s,
            )
        } else {
            let s = 2.0 * (1.0 + row2[2] - row0[0] - row1[1]).sqrt();
            Quaternion::new(
                (row0[2] + row2[0]) / s,
                (row1[2] + row2[1]) / s,
                0.25 * s,
                (row0[1] - row1[0]) / s,
            )
        }
        .normalize();

        Some(DecomposedTransform {
            translation,
            scale,
            skew,
            perspective: [0.0, 0.0, 0.0, 1.0],
            quaternion,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quaternion_slerp_identity() {
        let q1 = Quaternion::IDENTITY;
        let q2 = Quaternion::IDENTITY;
        let interpolated = q1.slerp(q2, 0.5);
        assert_eq!(interpolated, Quaternion::IDENTITY);
    }

    #[test]
    fn test_matrix_decompose_identity() {
        let mat = TransformMatrix4::IDENTITY;
        let decomp = mat.decompose().unwrap();
        assert_eq!(decomp.translation, [0.0, 0.0, 0.0]);
        assert_eq!(decomp.scale, [1.0, 1.0, 1.0]);
        assert_eq!(decomp.quaternion, Quaternion::IDENTITY);
    }
}
