//! # Comprimentos CSS, Porcentagens e Expressões Calc & Math W3C
//!
//! Tipos primitivos para comprimentos absolutos e relativos, porcentagens
//! e avaliação aritmética de `calc()`, `clamp()`, `min()`, `max()`, `round()`, `abs()`, `sign()`.

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

    /// Converte um comprimento em valor `f32` em pixels com referência de porcentagem opcional.
    #[inline]
    pub fn to_f32_px(
        &self,
        font_size_px: f32,
        root_font_size_px: f32,
        viewport_size: (f32, f32),
        ref_size: Option<f32>,
    ) -> Option<f32> {
        match self {
            Self::Px(val) => Some(*val),
            Self::Em(val) => Some(*val * font_size_px),
            Self::Rem(val) => Some(*val * root_font_size_px),
            Self::Vh(val) => Some(*val * viewport_size.1 / 100.0),
            Self::Vw(val) => Some(*val * viewport_size.0 / 100.0),
            Self::Pt(val) => Some(*val * (4.0 / 3.0)),
            Self::Ch(val) => Some(*val * font_size_px * 0.5),
            Self::Percent(pct) => ref_size.map(|r| *pct * r / 100.0),
            Self::Zero => Some(0.0),
            Self::Auto => None,
        }
    }

    /// Converte um comprimento em `LayoutUnit` com referência de porcentagem opcional.
    pub fn to_layout_unit_with_ref(
        &self,
        font_size_px: f32,
        root_font_size_px: f32,
        viewport_size: (f32, f32),
        ref_size: Option<f32>,
    ) -> Option<LayoutUnit> {
        self.to_f32_px(font_size_px, root_font_size_px, viewport_size, ref_size)
            .map(LayoutUnit::from_f32_px)
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

/// Estratégia de arredondamento para a função `round()` (W3C CSS Values 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoundStrategy {
    #[default]
    Nearest,
    Up,
    Down,
    ToZero,
}

impl RoundStrategy {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "nearest" => Some(Self::Nearest),
            "up" => Some(Self::Up),
            "down" => Some(Self::Down),
            "to-zero" => Some(Self::ToZero),
            _ => None,
        }
    }
}

/// Árvore de expressões de cálculo matemático CSS (`calc()`, `min()`, `max()`, `clamp()`, `round()`, `abs()`, `sign()`).
#[derive(Debug, Clone, PartialEq)]
pub enum CalcExpr {
    Value(Length),
    Number(f32),
    Add(Box<CalcExpr>, Box<CalcExpr>),
    Sub(Box<CalcExpr>, Box<CalcExpr>),
    Mul(Box<CalcExpr>, f32),
    Div(Box<CalcExpr>, f32),
    Min(Vec<CalcExpr>),
    Max(Vec<CalcExpr>),
    Clamp(Box<CalcExpr>, Box<CalcExpr>, Box<CalcExpr>),
    Round {
        strategy: RoundStrategy,
        val: Box<CalcExpr>,
        step: Option<Box<CalcExpr>>,
    },
    Abs(Box<CalcExpr>),
    Sign(Box<CalcExpr>),
}

impl CalcExpr {
    /// Avalia a expressão para um valor numérico em pixels.
    pub fn eval_px(
        &self,
        font_size_px: f32,
        root_font_size_px: f32,
        viewport_size: (f32, f32),
        ref_size: Option<f32>,
    ) -> Option<f32> {
        match self {
            Self::Value(len) => {
                len.to_f32_px(font_size_px, root_font_size_px, viewport_size, ref_size)
            }
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
            Self::Min(args) => {
                let mut min_val: Option<f32> = None;
                for arg in args {
                    let v =
                        arg.eval_px(font_size_px, root_font_size_px, viewport_size, ref_size)?;
                    min_val = Some(match min_val {
                        Some(cur) => cur.min(v),
                        None => v,
                    });
                }
                min_val
            }
            Self::Max(args) => {
                let mut max_val: Option<f32> = None;
                for arg in args {
                    let v =
                        arg.eval_px(font_size_px, root_font_size_px, viewport_size, ref_size)?;
                    max_val = Some(match max_val {
                        Some(cur) => cur.max(v),
                        None => v,
                    });
                }
                max_val
            }
            Self::Clamp(min, val, max) => {
                let min_v =
                    min.eval_px(font_size_px, root_font_size_px, viewport_size, ref_size)?;
                let v = val.eval_px(font_size_px, root_font_size_px, viewport_size, ref_size)?;
                let max_v =
                    max.eval_px(font_size_px, root_font_size_px, viewport_size, ref_size)?;
                // Conforme W3C CSS Values 4: clamp(MIN, VAL, MAX) = max(MIN, min(VAL, MAX))
                Some(min_v.max(v.min(max_v)))
            }
            Self::Round {
                strategy,
                val,
                step,
            } => {
                let v = val.eval_px(font_size_px, root_font_size_px, viewport_size, ref_size)?;
                let s = if let Some(step_expr) = step {
                    step_expr.eval_px(font_size_px, root_font_size_px, viewport_size, ref_size)?
                } else {
                    1.0
                };
                if s.abs() < f32::EPSILON {
                    return Some(v);
                }
                let ratio = v / s;
                let rounded = match strategy {
                    RoundStrategy::Nearest => ratio.round(),
                    RoundStrategy::Up => ratio.ceil(),
                    RoundStrategy::Down => ratio.floor(),
                    RoundStrategy::ToZero => ratio.trunc(),
                };
                Some(rounded * s)
            }
            Self::Abs(expr) => {
                let v = expr.eval_px(font_size_px, root_font_size_px, viewport_size, ref_size)?;
                Some(v.abs())
            }
            Self::Sign(expr) => {
                let v = expr.eval_px(font_size_px, root_font_size_px, viewport_size, ref_size)?;
                Some(if v > 0.0 {
                    1.0
                } else if v < 0.0 {
                    -1.0
                } else {
                    0.0
                })
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

    /// Analisa uma expressão CSS matemática (`calc`, `min`, `max`, `clamp`, `round`, `abs`, `sign`).
    pub fn parse(s: &str) -> Option<Self> {
        parse_calc_str(s.trim())
    }
}

/// Helper para verificar se a string é uma chamada de função completa que abrange todo o input.
fn extract_full_function_call<'a>(input: &'a str, func_name: &str) -> Option<&'a str> {
    let prefix = format!("{}(", func_name);
    if !input.to_ascii_lowercase().starts_with(&prefix) {
        return None;
    }
    // Verifica se o parêntese de fechamento do primeiro '(' fecha no último caractere
    let open_idx = input.find('(')?;
    let mut depth = 0;
    for (i, ch) in input[open_idx..].char_indices() {
        if ch == '(' {
            depth += 1;
        } else if ch == ')' {
            depth -= 1;
            if depth == 0 {
                if open_idx + i == input.len() - 1 {
                    return Some(&input[open_idx + 1..open_idx + i]);
                } else {
                    return None;
                }
            }
        }
    }
    None
}

/// Separa argumentos delimitados por vírgula no nível 0 de parênteses.
fn split_comma_args(s: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut curr = String::new();
    let mut depth = 0;

    for ch in s.chars() {
        if ch == '(' {
            depth += 1;
            curr.push(ch);
        } else if ch == ')' {
            if depth > 0 {
                depth -= 1;
            }
            curr.push(ch);
        } else if ch == ',' && depth == 0 {
            args.push(curr.trim().to_string());
            curr.clear();
        } else {
            curr.push(ch);
        }
    }
    if !curr.trim().is_empty() {
        args.push(curr.trim().to_string());
    }
    args
}

fn parse_calc_str(input: &str) -> Option<CalcExpr> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    // Se o input estiver totalmente envelopado por parênteses: ( ... )
    if input.starts_with('(') {
        let mut depth = 0;
        let mut full_enclosed = false;
        for (i, ch) in input.char_indices() {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
                if depth == 0 && i == input.len() - 1 {
                    full_enclosed = true;
                    break;
                }
            }
        }
        if full_enclosed {
            return parse_calc_str(&input[1..input.len() - 1]);
        }
    }

    // 1. Procura por operadores '+' ou '-' no nível 0 de parênteses
    let chars: Vec<char> = input.chars().collect();
    let mut paren_depth = 0;
    let mut op_idx = None;

    for i in (0..chars.len()).rev() {
        match chars[i] {
            ')' => paren_depth += 1,
            '(' => paren_depth -= 1,
            '+' | '-' if paren_depth == 0 => {
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

    // 2. Procura por operadores '*' ou '/' no nível 0 de parênteses
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

    // 3. Funções Matemáticas Normativas W3C (calc, min, max, clamp, round, abs, sign)
    if let Some(inner) = extract_full_function_call(input, "calc") {
        return parse_calc_str(inner);
    }

    if let Some(inner) = extract_full_function_call(input, "min") {
        let args = split_comma_args(inner);
        if !args.is_empty() {
            let mut exprs = Vec::new();
            for arg in args {
                exprs.push(parse_calc_str(&arg)?);
            }
            return Some(CalcExpr::Min(exprs));
        }
    }

    if let Some(inner) = extract_full_function_call(input, "max") {
        let args = split_comma_args(inner);
        if !args.is_empty() {
            let mut exprs = Vec::new();
            for arg in args {
                exprs.push(parse_calc_str(&arg)?);
            }
            return Some(CalcExpr::Max(exprs));
        }
    }

    if let Some(inner) = extract_full_function_call(input, "clamp") {
        let args = split_comma_args(inner);
        if args.len() == 3 {
            let min = parse_calc_str(&args[0])?;
            let val = parse_calc_str(&args[1])?;
            let max = parse_calc_str(&args[2])?;
            return Some(CalcExpr::Clamp(Box::new(min), Box::new(val), Box::new(max)));
        }
    }

    if let Some(inner) = extract_full_function_call(input, "round") {
        let args = split_comma_args(inner);
        match args.len() {
            1 => {
                let val = parse_calc_str(&args[0])?;
                return Some(CalcExpr::Round {
                    strategy: RoundStrategy::Nearest,
                    val: Box::new(val),
                    step: None,
                });
            }
            2 => {
                if let Some(strategy) = RoundStrategy::parse(&args[0]) {
                    let val = parse_calc_str(&args[1])?;
                    return Some(CalcExpr::Round {
                        strategy,
                        val: Box::new(val),
                        step: None,
                    });
                } else {
                    let val = parse_calc_str(&args[0])?;
                    let step = parse_calc_str(&args[1])?;
                    return Some(CalcExpr::Round {
                        strategy: RoundStrategy::Nearest,
                        val: Box::new(val),
                        step: Some(Box::new(step)),
                    });
                }
            }
            3 => {
                let strategy = RoundStrategy::parse(&args[0]).unwrap_or_default();
                let val = parse_calc_str(&args[1])?;
                let step = parse_calc_str(&args[2])?;
                return Some(CalcExpr::Round {
                    strategy,
                    val: Box::new(val),
                    step: Some(Box::new(step)),
                });
            }
            _ => {}
        }
    }

    if let Some(inner) = extract_full_function_call(input, "abs") {
        let val = parse_calc_str(inner)?;
        return Some(CalcExpr::Abs(Box::new(val)));
    }

    if let Some(inner) = extract_full_function_call(input, "sign") {
        let val = parse_calc_str(inner)?;
        return Some(CalcExpr::Sign(Box::new(val)));
    }

    // 4. Folha Terminal (Length ou Número puro)
    if let Some(len) = Length::parse(input) {
        return Some(CalcExpr::Value(len));
    }
    if let Ok(num) = input.parse::<f32>() {
        return Some(CalcExpr::Number(num));
    }

    None
}

/// Um valor que pode ser comprimento puro ou resultado de uma expressão matemática CSS.
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
            Self::Length(len) => len.to_layout_unit_with_ref(
                font_size_px,
                root_font_size_px,
                viewport_size,
                ref_size,
            ),
            Self::Calc(expr) => {
                expr.eval_layout_unit(font_size_px, root_font_size_px, viewport_size, ref_size)
            }
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("calc(")
            || lower.starts_with("min(")
            || lower.starts_with("max(")
            || lower.starts_with("clamp(")
            || lower.starts_with("round(")
            || lower.starts_with("abs(")
            || lower.starts_with("sign(")
        {
            CalcExpr::parse(trimmed).map(|e| Self::Calc(Box::new(e)))
        } else if let Some(len) = Length::parse(trimmed) {
            Some(Self::Length(len))
        } else {
            CalcExpr::parse(trimmed).map(|e| Self::Calc(Box::new(e)))
        }
    }
}
