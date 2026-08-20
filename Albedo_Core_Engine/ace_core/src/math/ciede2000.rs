//! # Diferença Perceptual de Cores CIE ΔE2000 (CIEDE2000)
//!
//! A fórmula padrão-ouro internacional (ISO/CIE 11664-6 / Sharma et al., 2005) para quantificação
//! da distância perceptual entre cores no espaço Lab. Usada para anti-aliasing de fontes,
//! otimizações de rendering de imagens e cálculo de fidelidade em espaços de cores CSS Color 4/5.

/// Calcula a distância perceptual de cores CIEDE2000 entre duas cores no espaço CIE L*a*b*.
pub fn ciede2000(l1: f64, a1: f64, b1: f64, l2: f64, a2: f64, b2: f64) -> f64 {
    let k_l = 1.0;
    let k_c = 1.0;
    let k_h = 1.0;

    let c1_star = (a1 * a1 + b1 * b1).sqrt();
    let c2_star = (a2 * a2 + b2 * b2).sqrt();
    let mean_c_star = (c1_star + c2_star) * 0.5;

    let mean_c_pow7 = mean_c_star.powi(7);
    let g = 0.5 * (1.0 - (mean_c_pow7 / (mean_c_pow7 + 6103515625.0)).sqrt()); // 25^7 = 6103515625

    let a1_prime = (1.0 + g) * a1;
    let a2_prime = (1.0 + g) * a2;

    let c1_prime = (a1_prime * a1_prime + b1 * b1).sqrt();
    let c2_prime = (a2_prime * a2_prime + b2 * b2).sqrt();
    let mean_c_prime = (c1_prime + c2_prime) * 0.5;

    let mut h1_prime = b1.atan2(a1_prime).to_degrees();
    if h1_prime < 0.0 {
        h1_prime += 360.0;
    }

    let mut h2_prime = b2.atan2(a2_prime).to_degrees();
    if h2_prime < 0.0 {
        h2_prime += 360.0;
    }

    let delta_l_prime = l2 - l1;
    let delta_c_prime = c2_prime - c1_prime;

    let delta_h_prime = if c1_prime * c2_prime == 0.0 {
        0.0
    } else if (h2_prime - h1_prime).abs() <= 180.0 {
        h2_prime - h1_prime
    } else if (h2_prime - h1_prime) > 180.0 {
        (h2_prime - h1_prime) - 360.0
    } else {
        (h2_prime - h1_prime) + 360.0
    };

    let delta_cap_h_prime =
        2.0 * (c1_prime * c2_prime).sqrt() * (delta_h_prime.to_radians() * 0.5).sin();

    let mean_l_prime = (l1 + l2) * 0.5;

    let mean_h_prime = if c1_prime * c2_prime == 0.0 {
        h1_prime + h2_prime
    } else if (h1_prime - h2_prime).abs() <= 180.0 {
        (h1_prime + h2_prime) * 0.5
    } else if (h1_prime + h2_prime) < 360.0 {
        (h1_prime + h2_prime + 360.0) * 0.5
    } else {
        (h1_prime + h2_prime - 360.0) * 0.5
    };

    let t = 1.0 - 0.17 * (mean_h_prime - 30.0).to_radians().cos()
        + 0.24 * (2.0 * mean_h_prime).to_radians().cos()
        + 0.32 * (3.0 * mean_h_prime + 6.0).to_radians().cos()
        - 0.20 * (4.0 * mean_h_prime - 63.0).to_radians().cos();

    let delta_theta = 30.0 * (-((mean_h_prime - 275.0) / 25.0).powi(2)).exp();

    let mean_c_prime_pow7 = mean_c_prime.powi(7);
    let r_c = 2.0 * (mean_c_prime_pow7 / (mean_c_prime_pow7 + 6103515625.0)).sqrt();

    let l_sub_50_sq = (mean_l_prime - 50.0).powi(2);
    let s_l = 1.0 + (0.015 * l_sub_50_sq) / (20.0 + l_sub_50_sq).sqrt();
    let s_c = 1.0 + 0.045 * mean_c_prime;
    let s_h = 1.0 + 0.015 * mean_c_prime * t;

    let r_t = -(2.0 * delta_theta.to_radians()).sin() * r_c;

    let term_l = delta_l_prime / (k_l * s_l);
    let term_c = delta_c_prime / (k_c * s_c);
    let term_h = delta_cap_h_prime / (k_h * s_h);

    (term_l * term_l + term_c * term_c + term_h * term_h + r_t * term_c * term_h)
        .max(0.0)
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ciede2000_reference_values() {
        // Par de teste canônico de Sharma (2005)
        let de = ciede2000(50.0, 2.6772, -79.7751, 50.0, 0.0, -82.7485);
        assert!((de - 2.0425).abs() < 1e-3);
    }
}
