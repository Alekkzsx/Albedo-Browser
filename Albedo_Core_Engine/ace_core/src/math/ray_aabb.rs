//! # Interseção Raio-AABB Robusta IEEE 754 (Barnes Slab Algorithm)
//!
//! Algoritmo Slab para colisão de raios 3D contra caixas delimitadoras alinhadas aos eixos (AABB).
//! Trata divisões por zero ($D_i = \pm 0.0 \to \pm \infty$) e NaNs sem bifurcações condicionais (*branchless*),
//! garantindo precisão numérica para hit-testing, ray-casting de ponteiro e culling de camadas de renderização.

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray3D {
    pub origin: [f32; 3],
    pub dir: [f32; 3],
    pub inv_dir: [f32; 3],
    pub t_min: f32,
    pub t_max: f32,
}

impl Ray3D {
    pub fn new(origin: [f32; 3], dir: [f32; 3], t_min: f32, t_max: f32) -> Self {
        // IEEE 754 garante 1.0 / 0.0 = +inf e 1.0 / -0.0 = -inf
        let inv_dir = [1.0 / dir[0], 1.0 / dir[1], 1.0 / dir[2]];
        Self {
            origin,
            dir,
            inv_dir,
            t_min,
            t_max,
        }
    }
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb3D {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Aabb3D {
    pub const fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }

    /// Interseção robusta contra divisões por zero e NaNs. Retorna `Some((t_enter, t_exit))` se colidir.
    pub fn intersect(&self, ray: &Ray3D) -> Option<(f32, f32)> {
        let mut tmin = ray.t_min;
        let mut tmax = ray.t_max;

        for i in 0..3 {
            let t1 = (self.min[i] - ray.origin[i]) * ray.inv_dir[i];
            let t2 = (self.max[i] - ray.origin[i]) * ray.inv_dir[i];

            let t_near = t1.min(t2);
            let t_far = t1.max(t2);

            tmin = tmin.max(t_near);
            tmax = tmax.min(t_far);
        }

        if tmin <= tmax {
            Some((tmin, tmax))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ray_aabb_hit_and_miss() {
        let bbox = Aabb3D::new([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]);

        // Raio frontal atingindo o centro da caixa
        let ray_hit = Ray3D::new([0.0, 0.0, -5.0], [0.0, 0.0, 1.0], 0.0, 100.0);
        let hit = bbox.intersect(&ray_hit);
        assert!(hit.is_some());
        let (t_enter, t_exit) = hit.unwrap();
        assert_eq!(t_enter, 4.0);
        assert_eq!(t_exit, 6.0);

        // Raio paralelo passando fora da caixa
        let ray_miss = Ray3D::new([2.0, 0.0, -5.0], [0.0, 0.0, 1.0], 0.0, 100.0);
        assert!(bbox.intersect(&ray_miss).is_none());
    }
}
