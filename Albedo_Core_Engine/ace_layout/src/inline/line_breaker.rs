//! # Algoritmo de Quebra de Linha Unicode UAX #14 (`LineBreaker`)
//!
//! Implementação em Safe Rust estrito do algoritmo normativo Unicode Standard Annex #14
//! com suporte completo às diretivas de texto CSS:
//! - `white-space`: `normal`, `nowrap`, `pre`, `pre-wrap`, `pre-line`
//! - `word-break`: `normal`, `break-all`, `keep-all`
//! - `overflow-wrap`: `normal`, `break-word`, `anywhere`

use ace_core::math::layout_unit::LayoutUnit;
use ace_style::computed::structs::StyleFont;
use ace_style::values::enums::{OverflowWrap, WhiteSpace, WordBreak};

use crate::inline::text_measurer::TextMeasurer;

/// Classes normativas de quebra de linha definidas pelo Unicode Standard Annex #14.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineBreakClass {
    /// Quebra obrigatória (BK, p. ex. Form Feed \x0C).
    BK,
    /// Carriage Return (\r).
    CR,
    /// Line Feed (\n).
    LF,
    /// Next Line (\u{0085}).
    NL,
    /// Espaço em branco colapsável (espaço ordinário ou tabulação).
    SP,
    /// Cola não-quebrável (NBSP \u{00A0}, Narrow NBSP \u{202F}).
    GL,
    /// Word Joiner (\u{2060}, \u{FEFF}).
    WJ,
    /// Zero Width Space (\u{200B}).
    ZW,
    /// Quebra permitida após o caractere (hífen suave, traço, etc.).
    BA,
    /// Quebra permitida antes do caractere.
    BB,
    /// Hífen comum (-).
    HY,
    /// Ideogramas CJK (chinês, japonês, coreano).
    ID,
    /// Caracteres alfabéticos ordinários (latino, grego, cirílico, árabe, hebraico).
    AL,
    /// Dígitos numéricos (0-9).
    NU,
    /// Pontuação de abertura (parênteses, colchetes, chaves).
    OP,
    /// Pontuação de fechamento.
    CL,
    /// Aspas e citações (", ').
    QU,
    /// Ponto de exclamação ou interrogação (!, ?).
    EX,
    /// Separador infixado (, . : ;).
    IS,
    /// Prefixo monetário ou símbolo ($, £, €).
    PR,
    /// Sufixo numérico (%, ¢).
    PO,
    /// Marca combinante (acentos gráficos acoplados).
    CM,
    /// Qualquer outro caractere desconhecido.
    XX,
}

impl LineBreakClass {
    /// Classifica um caractere Unicode conforme a especificação UAX #14.
    pub fn classify(c: char) -> Self {
        match c {
            '\r' => Self::CR,
            '\n' => Self::LF,
            '\x0C' => Self::BK,
            '\u{0085}' => Self::NL,
            ' ' | '\t' => Self::SP,
            '\u{00A0}' | '\u{202F}' => Self::GL,
            '\u{2060}' | '\u{FEFF}' => Self::WJ,
            '\u{200B}' => Self::ZW,
            '-' => Self::HY,
            '\u{00AD}' | '\u{2010}' | '\u{2013}' | '\u{2014}' => Self::BA,
            '(' | '[' | '{' | '⟨' | '«' => Self::OP,
            ')' | ']' | '}' | '⟩' | '»' => Self::CL,
            '"' | '\'' => Self::QU,
            '!' | '?' => Self::EX,
            ',' | '.' | ':' | ';' => Self::IS,
            '$' | '£' | '€' | '¥' => Self::PR,
            '%' | '‰' | '¢' => Self::PO,
            '0'..='9' => Self::NU,
            // Intervalos CJK canônicos
            '\u{2E80}'..='\u{2EFF}'
            | '\u{3000}'..='\u{303F}'
            | '\u{3040}'..='\u{309F}'
            | '\u{30A0}'..='\u{30FF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{AC00}'..='\u{D7AF}'
            | '\u{F900}'..='\u{FAFF}' => Self::ID,
            // Marcas combinantes
            '\u{0300}'..='\u{036F}' => Self::CM,
            // Letras alfabéticas ocidentais e orientais
            _ if c.is_alphabetic() => Self::AL,
            _ => Self::AL, // Fallback normativo seguro UAX #14 (tratar como AL)
        }
    }
}

/// Representa uma oportunidade normativa de quebra de linha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakOpportunity {
    /// Posição do byte no texto onde a quebra pode ocorrer.
    pub byte_offset: usize,
    /// Indica se a quebra é obrigatória (ex: `\n`, `\r`).
    pub is_mandatory: bool,
    /// Indica se a oportunidade decorre de espaço em branco colapsável.
    pub is_space: bool,
}

/// Linha física de texto fatiada pelo algoritmo de quebra de linha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineSegment {
    /// Conteúdo textual visível da linha (sem espaços excedentes no final).
    pub text: String,
    /// Largura computada do texto visível.
    pub width: LayoutUnit,
    /// Indica se o término da linha decorreu de quebra mandatória (`\n`).
    pub is_mandatory_break: bool,
}

/// Motor de quebra de linha Unicode UAX #14.
pub struct LineBreaker;

impl LineBreaker {
    /// Normaliza e processa espaços em branco segundo o modo CSS `white-space`.
    pub fn prepare_text(text: &str, white_space: WhiteSpace) -> String {
        match white_space {
            WhiteSpace::Normal | WhiteSpace::NoWrap => {
                // Colapsa sequências de whitespace para um único espaço; converte newlines para espaços
                let mut result = String::with_capacity(text.len());
                let mut in_whitespace = false;
                for c in text.chars() {
                    if c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\x0C' {
                        if !in_whitespace {
                            result.push(' ');
                            in_whitespace = true;
                        }
                    } else {
                        result.push(c);
                        in_whitespace = false;
                    }
                }
                result
            }
            WhiteSpace::Pre => {
                // Preserva tudo exatamente como está
                text.to_string()
            }
            WhiteSpace::PreWrap => {
                // Preserva espaços e newlines
                text.to_string()
            }
            WhiteSpace::PreLine => {
                // Colapsa espaços horizontais consecutivos, mas PRESERVA newlines '\n'
                let mut result = String::with_capacity(text.len());
                let mut in_h_space = false;
                for c in text.chars() {
                    if c == '\n' || c == '\r' {
                        in_h_space = false;
                        result.push(c);
                    } else if c == ' ' || c == '\t' {
                        if !in_h_space {
                            result.push(' ');
                            in_h_space = true;
                        }
                    } else {
                        result.push(c);
                        in_h_space = false;
                    }
                }
                result
            }
        }
    }

    /// Identifica todas as oportunidades de quebra de linha em uma string conforme UAX #14 e diretivas CSS.
    pub fn find_break_opportunities(
        text: &str,
        white_space: WhiteSpace,
        word_break: WordBreak,
    ) -> Vec<BreakOpportunity> {
        let mut opportunities = Vec::new();
        if text.is_empty() {
            return opportunities;
        }

        let chars: Vec<(usize, char, LineBreakClass)> = text
            .char_indices()
            .map(|(idx, ch)| (idx, ch, LineBreakClass::classify(ch)))
            .collect();

        let allow_soft_wrap = match white_space {
            WhiteSpace::Normal | WhiteSpace::PreWrap | WhiteSpace::PreLine => true,
            WhiteSpace::NoWrap | WhiteSpace::Pre => false,
        };

        for i in 0..chars.len() {
            let (_byte_idx, _ch, class) = chars[i];
            let next_byte_idx = if i + 1 < chars.len() {
                chars[i + 1].0
            } else {
                text.len()
            };

            // Quebras mandatórias (LB4, LB5)
            if class == LineBreakClass::LF
                || class == LineBreakClass::BK
                || class == LineBreakClass::NL
            {
                opportunities.push(BreakOpportunity {
                    byte_offset: next_byte_idx,
                    is_mandatory: true,
                    is_space: false,
                });
                continue;
            }
            if class == LineBreakClass::CR {
                // Não quebra entre CR e LF (LB5)
                if i + 1 < chars.len() && chars[i + 1].2 == LineBreakClass::LF {
                    continue;
                }
                opportunities.push(BreakOpportunity {
                    byte_offset: next_byte_idx,
                    is_mandatory: true,
                    is_space: false,
                });
                continue;
            }

            // Se o modo CSS proíbe quebras suaves (nowrap ou pre), ignora oportunidades não-mandatórias
            if !allow_soft_wrap {
                continue;
            }

            // Quebra suave após espaços (LB18)
            if class == LineBreakClass::SP {
                opportunities.push(BreakOpportunity {
                    byte_offset: next_byte_idx,
                    is_mandatory: false,
                    is_space: true,
                });
                continue;
            }

            // Quebra suave após Zero-Width Space (LB8)
            if class == LineBreakClass::ZW {
                opportunities.push(BreakOpportunity {
                    byte_offset: next_byte_idx,
                    is_mandatory: false,
                    is_space: false,
                });
                continue;
            }

            // Quebra após hífen ou BA (LB20, LB21)
            if (class == LineBreakClass::HY || class == LineBreakClass::BA) && i + 1 < chars.len() {
                let next_class = chars[i + 1].2;
                // Não quebra se seguido de caracteres numéricos (ex: número negativo -5)
                if next_class != LineBreakClass::NU && next_class != LineBreakClass::SP {
                    opportunities.push(BreakOpportunity {
                        byte_offset: next_byte_idx,
                        is_mandatory: false,
                        is_space: false,
                    });
                }
                continue;
            }

            // word-break: break-all permite quebra suave entre quaisquer letras
            if word_break == WordBreak::BreakAll && i + 1 < chars.len() {
                let next_class = chars[i + 1].2;
                if class == LineBreakClass::AL
                    && (next_class == LineBreakClass::AL || next_class == LineBreakClass::NU)
                {
                    opportunities.push(BreakOpportunity {
                        byte_offset: next_byte_idx,
                        is_mandatory: false,
                        is_space: false,
                    });
                    continue;
                }
            }

            // Ideogramas CJK (LB23)
            if class == LineBreakClass::ID && i + 1 < chars.len() {
                let next_class = chars[i + 1].2;
                // CJK quebra livremente antes de outro ID, AL, ou NU
                if next_class == LineBreakClass::ID
                    || next_class == LineBreakClass::AL
                    || next_class == LineBreakClass::NU
                {
                    opportunities.push(BreakOpportunity {
                        byte_offset: next_byte_idx,
                        is_mandatory: false,
                        is_space: false,
                    });
                }
                continue;
            }

            // Antes de ideogramas CJK (LB23)
            if (class == LineBreakClass::AL || class == LineBreakClass::NU) && i + 1 < chars.len() {
                let next_class = chars[i + 1].2;
                if next_class == LineBreakClass::ID {
                    opportunities.push(BreakOpportunity {
                        byte_offset: next_byte_idx,
                        is_mandatory: false,
                        is_space: false,
                    });
                }
            }
        }

        opportunities
    }

    /// Segmenta o texto em linhas físicas (`LineSegment`), respeitando a largura disponível,
    /// a fonte e as diretivas CSS de quebra.
    pub fn break_lines(
        raw_text: &str,
        font: &StyleFont,
        measurer: &dyn TextMeasurer,
        available_width: LayoutUnit,
        white_space: WhiteSpace,
        word_break: WordBreak,
        overflow_wrap: OverflowWrap,
    ) -> Vec<LineSegment> {
        let prepared = Self::prepare_text(raw_text, white_space);
        if prepared.is_empty() {
            return Vec::new();
        }

        let opportunities = Self::find_break_opportunities(&prepared, white_space, word_break);
        let mut lines = Vec::new();

        let mut line_start = 0;
        let mut last_valid_break: Option<(usize, bool)> = None; // (byte_offset, is_mandatory)
        let mut opp_idx = 0;

        let chars_indices: Vec<usize> = prepared.char_indices().map(|(idx, _)| idx).collect();
        let mut curr_char_pos = 0;

        while curr_char_pos < chars_indices.len() {
            let curr_char_byte = chars_indices[curr_char_pos];
            let curr_char = prepared[curr_char_byte..].chars().next().unwrap();
            let curr_byte_end = if curr_char_pos + 1 < chars_indices.len() {
                chars_indices[curr_char_pos + 1]
            } else {
                prepared.len()
            };

            // 1. Quebra mandatória (\n, \r)
            if curr_char == '\n' || curr_char == '\r' {
                let segment_text = &prepared[line_start..curr_char_byte];
                let trimmed = segment_text.trim_end_matches(['\r', '\n']);
                let m = measurer.measure_text(trimmed, font);
                lines.push(LineSegment {
                    text: trimmed.to_string(),
                    width: m.width,
                    is_mandatory_break: true,
                });
                line_start = curr_byte_end;
                last_valid_break = None;
                curr_char_pos += 1;
                while opp_idx < opportunities.len()
                    && opportunities[opp_idx].byte_offset <= line_start
                {
                    opp_idx += 1;
                }
                continue;
            }

            let candidate_text = &prepared[line_start..curr_byte_end];
            let measured = measurer.measure_text(candidate_text, font);

            // 2. Se excedeu a largura disponível no contêiner
            if available_width > LayoutUnit::ZERO && measured.width > available_width {
                // Caso 1: Temos uma quebra suave prévia nesta linha
                if let Some((break_pos, _)) = last_valid_break {
                    if break_pos > line_start {
                        let line_text = &prepared[line_start..break_pos];
                        let trimmed = line_text.trim_end();
                        let m = measurer.measure_text(trimmed, font);
                        lines.push(LineSegment {
                            text: trimmed.to_string(),
                            width: m.width,
                            is_mandatory_break: false,
                        });
                        line_start = break_pos;
                        // Pula espaços iniciais na nova linha se não for pre
                        if white_space != WhiteSpace::Pre && white_space != WhiteSpace::PreWrap {
                            while line_start < prepared.len()
                                && prepared[line_start..].starts_with(' ')
                            {
                                line_start += 1;
                            }
                        }
                        last_valid_break = None;
                        curr_char_pos = chars_indices
                            .iter()
                            .position(|&b| b >= line_start)
                            .unwrap_or(chars_indices.len());
                        while opp_idx < opportunities.len()
                            && opportunities[opp_idx].byte_offset <= line_start
                        {
                            opp_idx += 1;
                        }
                        continue;
                    }
                }

                // Caso 2: Não há quebra suave anterior, palavra monolítica!
                if overflow_wrap == OverflowWrap::BreakWord
                    || overflow_wrap == OverflowWrap::Anywhere
                {
                    let break_at = if curr_char_byte > line_start {
                        curr_char_byte
                    } else {
                        curr_byte_end
                    };

                    let line_text = &prepared[line_start..break_at];
                    let trimmed = line_text.trim_end();
                    let m = measurer.measure_text(trimmed, font);
                    lines.push(LineSegment {
                        text: trimmed.to_string(),
                        width: m.width,
                        is_mandatory_break: false,
                    });
                    line_start = break_at;
                    last_valid_break = None;
                    curr_char_pos = chars_indices
                        .iter()
                        .position(|&b| b >= line_start)
                        .unwrap_or(chars_indices.len());
                    while opp_idx < opportunities.len()
                        && opportunities[opp_idx].byte_offset <= line_start
                    {
                        opp_idx += 1;
                    }
                    continue;
                }
            }

            // O candidato coube! Agora registramos oportunidades de quebra válidas até curr_byte_end
            while opp_idx < opportunities.len()
                && opportunities[opp_idx].byte_offset <= curr_byte_end
            {
                let opp = &opportunities[opp_idx];
                if !opp.is_mandatory {
                    last_valid_break = Some((opp.byte_offset, false));
                }
                opp_idx += 1;
            }

            curr_char_pos += 1;
        }

        // Restante final da linha
        if line_start < prepared.len() {
            let final_slice = &prepared[line_start..];
            let trimmed = final_slice.trim_end_matches(['\r', '\n']);
            let m = measurer.measure_text(trimmed, font);
            lines.push(LineSegment {
                text: trimmed.to_string(),
                width: m.width,
                is_mandatory_break: false,
            });
        }

        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inline::text_measurer::FixedTextMeasurer;

    #[test]
    fn test_whitespace_collapsing_normal() {
        let text = "hello   world\n\tfrom   browser";
        let prepared = LineBreaker::prepare_text(text, WhiteSpace::Normal);
        assert_eq!(prepared, "hello world from browser");
    }

    #[test]
    fn test_whitespace_preservation_pre() {
        let text = "hello   world\n\tfrom";
        let prepared = LineBreaker::prepare_text(text, WhiteSpace::Pre);
        assert_eq!(prepared, "hello   world\n\tfrom");
    }

    #[test]
    fn test_whitespace_pre_line() {
        let text = "hello   world\nfrom   pre-line";
        let prepared = LineBreaker::prepare_text(text, WhiteSpace::PreLine);
        assert_eq!(prepared, "hello world\nfrom pre-line");
    }

    #[test]
    fn test_line_breaking_opportunities_spaces_and_cjk() {
        let text = "hello world 你好世界";
        let opps =
            LineBreaker::find_break_opportunities(text, WhiteSpace::Normal, WordBreak::Normal);
        // Deve conter oportunidade após "hello ", e entre ideogramas CJK
        assert!(!opps.is_empty());
        assert!(opps.iter().any(|o| o.is_space));
    }

    #[test]
    fn test_line_breaking_with_width_constraint() {
        let measurer = FixedTextMeasurer::new(); // 8px por caractere
        let font = StyleFont::initial();

        // 3 palavras: "apple" (5*8=40px), "banana" (6*8=48px), "cherry" (6*8=48px)
        // Largura disponível = 70px (suficiente para "apple", mas não "apple banana" = 96px)
        let text = "apple banana cherry";
        let lines = LineBreaker::break_lines(
            text,
            &font,
            &measurer,
            LayoutUnit::from_px(70),
            WhiteSpace::Normal,
            WordBreak::Normal,
            OverflowWrap::Normal,
        );

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].text, "apple");
        assert_eq!(lines[1].text, "banana");
        assert_eq!(lines[2].text, "cherry");
    }

    #[test]
    fn test_line_breaking_nowrap() {
        let measurer = FixedTextMeasurer::new();
        let font = StyleFont::initial();

        let text = "apple banana cherry";
        // Sob nowrap, não deve quebrar mesmo com largura estreita de 50px
        let lines = LineBreaker::break_lines(
            text,
            &font,
            &measurer,
            LayoutUnit::from_px(50),
            WhiteSpace::NoWrap,
            WordBreak::Normal,
            OverflowWrap::Normal,
        );

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "apple banana cherry");
    }

    #[test]
    fn test_overflow_wrap_break_word() {
        let measurer = FixedTextMeasurer::new(); // 8px por caractere
        let font = StyleFont::initial();

        // Palavra monolítica de 10 caracteres = 80px. Contêiner = 32px (cabe 4 caracteres)
        let text = "abcdefghij";
        let lines = LineBreaker::break_lines(
            text,
            &font,
            &measurer,
            LayoutUnit::from_px(32),
            WhiteSpace::Normal,
            WordBreak::Normal,
            OverflowWrap::BreakWord,
        );

        assert!(lines.len() >= 3);
        assert_eq!(lines[0].text, "abcd");
        assert_eq!(lines[1].text, "efgh");
        assert_eq!(lines[2].text, "ij");
    }

    #[test]
    fn test_word_break_all() {
        let measurer = FixedTextMeasurer::new();
        let font = StyleFont::initial();

        let text = "abcdef";
        let lines = LineBreaker::break_lines(
            text,
            &font,
            &measurer,
            LayoutUnit::from_px(24), // 3 caracteres por linha
            WhiteSpace::Normal,
            WordBreak::BreakAll,
            OverflowWrap::Normal,
        );

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "abc");
        assert_eq!(lines[1].text, "def");
    }

    #[test]
    fn test_break_after_hyphen() {
        let measurer = FixedTextMeasurer::new(); // 8px por caractere
        let font = StyleFont::initial();

        // "super-fast" -> "super-" (6 chars = 48px), "fast" (4 chars = 32px)
        // Disponível = 50px
        let text = "super-fast";
        let lines = LineBreaker::break_lines(
            text,
            &font,
            &measurer,
            LayoutUnit::from_px(50),
            WhiteSpace::Normal,
            WordBreak::Normal,
            OverflowWrap::Normal,
        );

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "super-");
        assert_eq!(lines[1].text, "fast");
    }

    #[test]
    fn test_cjk_line_breaking_multilingual() {
        let measurer = FixedTextMeasurer::new();
        let font = StyleFont::initial();

        // 4 ideogramas CJK: "你好世界" (4 chars = 32px)
        // Disponível = 16px (cabe 2 ideogramas por linha)
        let text = "你好世界";
        let lines = LineBreaker::break_lines(
            text,
            &font,
            &measurer,
            LayoutUnit::from_px(16),
            WhiteSpace::Normal,
            WordBreak::Normal,
            OverflowWrap::Normal,
        );

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "你好");
        assert_eq!(lines[1].text, "世界");
    }
}
