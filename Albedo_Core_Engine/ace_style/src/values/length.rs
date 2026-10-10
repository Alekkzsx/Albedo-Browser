//! # Comprimentos CSS, Porcentagens e Expressões Calc
//!
//! Tipos primitivos para comprimentos absolutos e relativos, porcentagens
//! e avaliação aritmética de `calc()` em ponto fixo (`LayoutUnit`).

use ace_core::math::LayoutUnit;

/// Unidades de comprimento CSS (CSS Values and Units Module Level 3 & 4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    /// Pixels CSS (1px = 1/96th de polegada)
    Px(f32),
    /// Relativo ao tamanho da fonte do elemento atual
    Em(f32),
    /// Relativo ao tamanho da fonte do elemento raiz (`<html>`)
    Rem(f32),
    /// Relativo a 1% da altura da viewport
    Vh(f32),
    /// Relativo a 1% da largura da viewport
    Vw(f32),
    /// Pontos tipográficos (1pt = 4/3 px)
    Pt(f32),
    /// Largura do caractere '0' na fonte atual (aproximado como 0.5em)
    Ch(f32),
    /// Porcentagem relativa ao tamanho do contêiner pai
    Percent(f32),
    /// Dimensão automática
    Auto,
    /// Zero absoluto (0px)
    Zero,
}

impl Length {
    /// Converte um comprimento em `LayoutUnit` de ponto fixo.
    /// Retorna `None` se o comprimento for `Auto` ou uma porcentagem dependente de contexto não resolvido.
    #[inline]
    pub fn to_layout_unit(
        &self,
        font_size_px: f32,
        root_font_size_px: f32,
        viewport_size: (f32, f32),
    ) -> Option<LayoutUnit> {
        self.to_layout_unit_with_ref(font_size_px, root_font_size_px, viewport_size, None)
    }

    /// Converte um comprimento em `LayoutUnit` com referência de porcentagem opcional.
    pub fn to_layout_unit_with_ref(
        &self,
        font_size_px: f32,
        root_font_size_px: f32,
        viewport_size: (f32, f32),
        ref_size: Option<f32>,
    ) -> Option<LayoutUnit> {
        match self {
            Self::Px(val) => Some(LayoutUnit::from_f32_px(*val)),
            Self::Em(val) => Some(LayoutUnit::from_f32_px(*val * font_size_px)),
            Self::Rem(val) => Some(LayoutUnit::from_f32_px(*val * root_font_size_px)),
            Self::Vh(val) => Some(LayoutUnit::from_f32_px(*val * viewport_size.1 / 100.0)),
            Self::Vw(val) => Some(LayoutUnit::from_f32_px(*val * viewport_size.0 / 100.0)),
            Self::Pt(val) => Some(LayoutUnit::from_f32_px(*val * (4.0 / 3.0))),
            Self::Ch(val) => Some(LayoutUnit::from_f32_px(*val * font_size_px * 0.5)),
            Self::Percent(pct) => ref_size.map(|r| LayoutUnit::from_f32_px(*pct * r / 100.0)),
            Self::Zero => Some(LayoutUnit::ZERO),
            Self::Auto => None,
        }
    }

    /// Analisa uma string CSS contendo comprimento ou porcentagem.
    pub fn parse(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return None;
        }

        if trimmed.eq_ignore_ascii_case("auto") {
            return Some(Self::Auto);
        }

        if trimmed == "0" || trimmed == "0.0" {
            return Some(Self::Zero);
        }

        if let Some(num) = trimmed.strip_suffix('%') {
            return num.trim().parse::<f32>().ok().map(Self::Percent);
        }

        let lower = trimmed.to_ascii_lowercase();
        if let Some(num) = lower.strip_suffix("px") {
            num.trim().parse::<f32>().ok().map(Self::Px)
        } else if let Some(num) = lower.strip_suffix("rem") {
            num.trim().parse::<f32>().ok().map(Self::Rem)
        } else if let Some(num) = lower.strip_suffix("em") {
            num.trim().parse::<f32>().ok().map(Self::Em)
        } else if let Some(num) = lower.strip_suffix("vh") {
            num.trim().parse::<f32>().ok().map(Self::Vh)
        } else if let Some(num) = lower.strip_suffix("vw") {
            num.trim().parse::<f32>().ok().map(Self::Vw)
        } else if let Some(num) = lower.strip_suffix("pt") {
            num.trim().parse::<f32>().ok().map(Self::Pt)
        } else if let Some(num) = lower.strip_suffix("ch") {
            num.trim().parse::<f32>().ok().map(Self::Ch)
        } else {
            None
        }
    }
}

/// Árvore de expressões de cálculo CSS `calc()`.
#[derive(Debug, Clone, PartialEq)]
pub enum CalcExpr {
    Value(Length),
    Number(f32),
    Add(Box<CalcExpr>, Box<CalcExpr>),
    Sub(Box<CalcExpr>, Box<CalcExpr>),
    Mul(Box<CalcExpr>, f32),
    Div(Box<CalcExpr>, f32),
}

impl CalcExpr {
    /// Avalia a expressão `calc()` para um valor numérico em pixels.
    pub fn eval_px(
        &self,
        font_size_px: f32,
        root_font_size_px: f32,
        viewport_size: (f32, f32),
        ref_size: Option<f32>,
    ) -> Option<f32> {
        match self {
            Self::Value(len) => len
                .to_layout_unit_with_ref(font_size_px, root_font_size_px, viewport_size, ref_size)
                .map(|u| u.to_f32_px()),
            Self::Number(n) => Some(*n),
            Self::Add(lhs, rhs) => {
                let l = lhs.eval_px(font_size_px, root_font_size_px, viewport_size, ref_size)?;
                let r = rhs.eval_px(font_size_px, root_font_size_px, viewport_size, ref_size)?;
                Some(l + r)
            }
            Self::Sub(lhs, rhs) => {
                let l = lhs.eval_px(font_size_px, root_font_size_px, viewport_size, ref_size)?;
                let r = rhs.eval_px(font_size_px, root_font_size_px, viewport_size, ref_size)?;
                Some(l - r)
            }
            Self::Mul(lhs, factor) => {
                let l = lhs.eval_px(font_size_px, root_font_size_px, viewport_size, ref_size)?;
                Some(l * factor)
            }
            Self::Div(lhs, divisor) => {
                if divisor.abs() < f32::EPSILON {
                    return None;
                }
                let l = lhs.eval_px(font_size_px, root_font_size_px, viewport_size, ref_size)?;
                Some(l / divisor)
            }
        }
    }

    /// Avalia a expressão para `LayoutUnit`.
    pub fn eval_layout_unit(
        &self,
        font_size_px: f32,
        root_font_size_px: f32,
        viewport_size: (f32, f32),
        ref_size: Option<f32>,
    ) -> Option<LayoutUnit> {
        self.eval_px(font_size_px, root_font_size_px, viewport_size, ref_size)
            .map(LayoutUnit::from_f32_px)
    }

    /// Faz o parse simples de uma expressão `calc(...)`.
    pub fn parse(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        let content = if let Some(inner) = trimmed.strip_prefix("calc(").and_then(|t| t.strip_suffix(')')) {
            inner.trim()
        } else {
            trimmed
        };
        parse_calc_str(content)
    }
}

fn parse_calc_str(input: &str) -> Option<CalcExpr> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    // Procura por operadores '+' ou '-' no nível de parênteses zero
    let mut paren_depth = 0;
    let chars: Vec<char> = input.chars().collect();
    let mut op_idx = None;

    // Scan da direita para esquerda para respeitar associatividade à esquerda
    for i in (0..chars.len()).rev() {
        match chars[i] {
            ')' => paren_depth += 1,
            '(' => paren_depth -= 1,
            '+' | '-' if paren_depth == 0 => {
                // CSS calc requires whitespace around + and -
                let is_surrounded_by_ws = (i > 0 && chars[i - 1].is_whitespace())
                    && (i + 1 < chars.len() && chars[i + 1].is_whitespace());
                if is_surrounded_by_ws {
                    op_idx = Some((i, chars[i]));
                    break;
                }
            }
            _ => {}
        }
    }

    if let Some((idx, op)) = op_idx {
        let left_str: String = chars[0..idx].iter().collect();
        let right_str: String = chars[idx + 1..].iter().collect();
        let left = parse_calc_str(&left_str)?;
        let right = parse_calc_str(&right_str)?;
        return match op {
            '+' => Some(CalcExpr::Add(Box::new(left), Box::new(right))),
            '-' => Some(CalcExpr::Sub(Box::new(left), Box::new(right))),
            _ => unreachable!(),
        };
    }

    // Procura por operadores '*' ou '/'
    paren_depth = 0;
    for i in (0..chars.len()).rev() {
        match chars[i] {
            ')' => paren_depth += 1,
            '(' => paren_depth -= 1,
            '*' | '/' if paren_depth == 0 => {
                op_idx = Some((i, chars[i]));
                break;
            }
            _ => {}
        }
    }

    if let Some((idx, op)) = op_idx {
        let left_str: String = chars[0..idx].iter().collect();
        let right_str: String = chars[idx + 1..].iter().collect();
        let left = parse_calc_str(&left_str)?;
        let factor = right_str.trim().parse::<f32>().ok()?;
        return match op {
            '*' => Some(CalcExpr::Mul(Box::new(left), factor)),
            '/' => Some(CalcExpr::Div(Box::new(left), factor)),
            _ => unreachable!(),
        };
    }

    // Sub-expressão entre parênteses: ( ... )
    if input.starts_with('(') && input.ends_with(')') {
        return parse_calc_str(&input[1..input.len() - 1]);
    }

    // Folha (Length ou Número)
    if let Some(len) = Length::parse(input) {
        return Some(CalcExpr::Value(len));
    }
    if let Ok(num) = input.parse::<f32>() {
        return Some(CalcExpr::Number(num));
    }

    None
}

/// Um valor que pode ser comprimento puro ou resultado de `calc()`.
#[derive(Debug, Clone, PartialEq)]
pub enum LengthPercentage {
    Length(Length),
    Calc(Box<CalcExpr>),
}

impl LengthPercentage {
    pub fn resolve(
        &self,
        font_size_px: f32,
        root_font_size_px: f32,
        viewport_size: (f32, f32),
        ref_size: Option<f32>,
    ) -> Option<LayoutUnit> {
        match self {
            Self::Length(len) => len.to_layout_unit_with_ref(font_size_px, root_font_size_px, viewport_size, ref_size),
            Self::Calc(expr) => expr.eval_layout_unit(font_size_px, root_font_size_px, viewport_size, ref_size),
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        if trimmed.starts_with("calc(") {
            CalcExpr::parse(trimmed).map(|e| Self::Calc(Box::new(e)))
        } else {
            Length::parse(trimmed).map(Self::Length)
        }
    }
}
