//! # Utilitários de Leitura e Parsing para Cursores
//!
//! Funções de conversão numérica de alta performance e salto de espaços em branco diretamente sobre `CharCursor`.

use super::CharCursor;

/// Pula todos os espaços em branco ASCII contíguos no cursor. Retorna a quantidade de caracteres pulados.
pub fn skip_ascii_whitespace(cursor: &mut CharCursor) -> usize {
    let mut count = 0;
    while let Some(c) = cursor.peek() {
        if c.is_ascii_whitespace() {
            cursor.advance();
            count += 1;
        } else {
            break;
        }
    }
    count
}

/// Extrai e consome um número inteiro `i32` diretamente do fluxo do cursor, com suporte a sinal negativo/positivo.
pub fn parse_i32(cursor: &mut CharCursor) -> Option<i32> {
    let mut is_negative = false;
    if let Some(c) = cursor.peek() {
        if c == '-' {
            is_negative = true;
            cursor.advance();
        } else if c == '+' {
            cursor.advance();
        }
    }

    let mut has_digits = false;
    let mut value: i64 = 0;

    while let Some(c) = cursor.peek() {
        if let Some(digit) = c.to_digit(10) {
            has_digits = true;
            value = value.saturating_mul(10).saturating_add(digit as i64);
            cursor.advance();
        } else {
            break;
        }
    }

    if has_digits {
        let final_val = if is_negative { -value } else { value };
        Some(final_val.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
    } else {
        None
    }
}

/// Extrai e consome um número float `f32` diretamente do fluxo do cursor (ex: `12.5`, `-0.75`).
pub fn parse_f32(cursor: &mut CharCursor) -> Option<f32> {
    let mut is_negative = false;

    if let Some(c) = cursor.peek() {
        if c == '-' {
            is_negative = true;
            cursor.advance();
        } else if c == '+' {
            cursor.advance();
        }
    }

    let mut integer_digits = 0;
    let mut int_part: f64 = 0.0;

    while let Some(c) = cursor.peek() {
        if let Some(digit) = c.to_digit(10) {
            integer_digits += 1;
            int_part = int_part * 10.0 + digit as f64;
            cursor.advance();
        } else {
            break;
        }
    }

    let mut frac_part: f64 = 0.0;
    let mut frac_divisor: f64 = 1.0;
    let mut has_dot = false;

    if let Some('.') = cursor.peek() {
        // Verifica se há dígitos após o ponto para evitar consumir '.' solto
        if let Some(next_c) = cursor.peek_at(1) {
            if next_c.is_ascii_digit() {
                has_dot = true;
                cursor.advance(); // consome '.'

                while let Some(c) = cursor.peek() {
                    if let Some(digit) = c.to_digit(10) {
                        frac_divisor *= 10.0;
                        frac_part += digit as f64 / frac_divisor;
                        cursor.advance();
                    } else {
                        break;
                    }
                }
            }
        }
    }

    if integer_digits == 0 && !has_dot {
        None
    } else {
        let mut total = int_part + frac_part;
        if is_negative {
            total = -total;
        }
        Some(total as f32)
    }
}

/// Extrai e consome um valor hexadecimal de até `max_digits` dígitos (ex: para cores hex ou escapes unicode).
pub fn parse_hex_u32(cursor: &mut CharCursor, max_digits: usize) -> Option<u32> {
    let mut value: u32 = 0;
    let mut digits_read = 0;

    while digits_read < max_digits {
        if let Some(c) = cursor.peek() {
            if let Some(digit) = c.to_digit(16) {
                value = (value << 4) | digit;
                digits_read += 1;
                cursor.advance();
            } else {
                break;
            }
        } else {
            break;
        }
    }

    if digits_read > 0 {
        Some(value)
    } else {
        None
    }
}
