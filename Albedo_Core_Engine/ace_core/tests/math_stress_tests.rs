use ace_core::math::{Color, Matrix3x3, Rect};

#[test]
fn stress_matrix_multiplication() {
    let mut m = Matrix3x3::identity();
    let rot = Matrix3x3::rotation(0.01);
    
    // Simula pesada carga de transformações encadeadas (ex: nós filhos de CSS profundamente aninhados)
    for _ in 0..100_000 {
        m = m * rot;
    }
    
    // Garante que o motor de ponto flutuante não degenerou em NaN após milhares de operações cumulativas
    assert!(!m.data[0].is_nan());
}

#[test]
fn stress_color_blending() {
    let mut c = Color::new(255, 0, 0, 10);
    let bg = Color::new(0, 0, 255, 255);
    
    // Simula a repintura massiva (Painter's Algorithm) de 100.000 camadas de transparência
    for _ in 0..100_000 {
        c = c.blend_source_over(&bg);
    }
    
    // Eventualmente, a saturação de cor deve normalizar (sem estourar bounds de u8)
    assert_eq!(c.a, 255);
}

#[test]
fn stress_rect_union_many() {
    let mut r = Rect::new(0.0, 0.0, 1.0, 1.0);
    
    // Simula o cálculo da Bounding Box Total de uma página web contendo 100.000 elementos aninhados
    for i in 0..100_000 {
        let expand = Rect::new(i as f32, i as f32, 10.0, 10.0);
        r = r.union(&expand);
    }
    
    assert!(r.width() > 99_000.0);
    assert!(r.height() > 99_000.0);
}
