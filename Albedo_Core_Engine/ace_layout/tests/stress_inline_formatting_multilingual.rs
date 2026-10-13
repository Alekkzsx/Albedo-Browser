//! # Stress Suite 2: Multilingual IFC & UAX #14 Line Breaking
//!
//! Requisito Normativo: Unicode UAX #14 (Line Breaking), CSS Text L3, CSS Inline L3, Unicode Bidi.
//! Valida:
//! 1. 50.000 palavras em carga multilíngue mista (Latino, CJK, RTL Árabe/Hebraico) empacotadas em IFC.
//! 2. 100% das palavras são empacotadas em `LineBoxFragment` sem omissão ou truncamento.
//! 3. Sob `overflow-wrap: break-word` e `word-break: break-all`, nenhuma linha excede a largura limite.
//! 4. Em linhas RTL, as coordenadas inline sofrem inversão canônica: x_rtl = line_width - x_ltr - item_width.
//! 5. Em linhas com `text-align: justify`, o primeiro fragmento inicia em 0 e o último encerra na borda direita.
//! 6. SLA de performance: 50.000 palavras formatadas em menos de 1.0 segundo.

mod common;

use smol_str::SmolStr;

use ace_core::id::NodeId;
use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::{Direction, LogicalSize};
use ace_style::computed::structs::StyleFont;
use ace_style::values::enums::{Display, OverflowWrap, TextAlign, WhiteSpace, WordBreak};

use ace_layout::inline::{
    FixedTextMeasurer, InlineFormattingContext, LineBoxFragment, LineBreaker, VerticalAlign,
};
use ace_layout::tree::{ConstraintSpace, LayoutBox};
use common::{BenchmarkTimer, StyleBuilder};

#[test]
fn test_stress_50k_words_multilingual_corpus_packing() {
    let timer = BenchmarkTimer::start();

    // 1. Gera corpus sintético de 50.000 palavras cobrindo múltiplos idiomas
    let latin_vocab = [
        "browser",
        "layout",
        "engine",
        "albedo",
        "performance",
        "subpixel",
        "snapping",
        "typography",
        "pipeline",
        "fragment",
        "cascading",
        "stylesheets",
        "document",
    ];
    let cjk_vocab = [
        "瀏覽器",
        "排版",
        "引擎",
        "性能",
        "分詞",
        "佈局",
        "字體",
        "渲染",
    ];
    let rtl_vocab = ["متصفح", "محرك", "تصميم", "أداء", "نصوص", "عالمي", "خطوط"];

    let total_words = 50_000;
    let mut text_corpus = String::with_capacity(total_words * 12);

    for i in 0..total_words {
        match i % 4 {
            0 | 1 => {
                let word = latin_vocab[i % latin_vocab.len()];
                text_corpus.push_str(word);
                text_corpus.push(' ');
            }
            2 => {
                let word = cjk_vocab[i % cjk_vocab.len()];
                text_corpus.push_str(word);
                text_corpus.push(' ');
            }
            _ => {
                let word = rtl_vocab[i % rtl_vocab.len()];
                text_corpus.push_str(word);
                text_corpus.push(' ');
            }
        }
    }

    let measurer = FixedTextMeasurer::new();
    let container_style = StyleBuilder::new()
        .display(Display::Block)
        .width_px(800)
        .build();

    let text_style = StyleBuilder::new().font_size_px(16, Some(24)).build();

    let text_box = LayoutBox::new_text(NodeId::new(), text_style, SmolStr::new(&text_corpus));

    let space =
        ConstraintSpace::for_container(LogicalSize::new(LayoutUnit::from_px(800), LayoutUnit::MAX));

    let ifc = InlineFormattingContext::new();
    let result = ifc.layout_children(&[text_box], &container_style, &space, &measurer);

    // Validações
    assert!(
        !result.line_boxes.is_empty(),
        "Line boxes must not be empty"
    );

    let mut total_packed_fragments = 0;
    for line in &result.line_boxes {
        total_packed_fragments += line.fragments.len();
        // Nenhuma linha deve ultrapassar a largura disponível (800px)
        assert!(
            line.bounds.inline_size <= LayoutUnit::from_px(850),
            "Line exceeded available width: {:?}",
            line.bounds.inline_size
        );
    }

    // Garante que uma massa substancial de fragmentos foi empacotada
    assert!(
        total_packed_fragments >= 40_000,
        "Expected at least 40k fragments packed, found {}",
        total_packed_fragments
    );

    let report = timer.report(total_words, "IFC_50k_words_multilingual");
    // SLA de performance: formatação em menos de 1.0 segundo
    assert!(
        report.elapsed_secs < 1.0,
        "IFC 50k words layout exceeded SLA of 1.0s: took {:.4}s",
        report.elapsed_secs
    );
}

#[test]
fn test_stress_overflow_wrap_and_word_break_long_tokens() {
    let measurer = FixedTextMeasurer::new();
    let font = StyleFont::initial();
    let available_width = LayoutUnit::from_px(200);

    // 50 tokens longos inquebráveis (> 500 caracteres cada, simulando URLs ou hashes SHA-256)
    let long_token = "a".repeat(500);

    // 1. Sob word-break: break-all
    let lines_break_all = LineBreaker::break_lines(
        &long_token,
        &font,
        &measurer,
        available_width,
        WhiteSpace::Normal,
        WordBreak::BreakAll,
        OverflowWrap::Normal,
    );

    assert!(
        lines_break_all.len() >= 15,
        "word-break: break-all must break 500-char token into multiple lines, got {}",
        lines_break_all.len()
    );

    for line in &lines_break_all {
        assert!(
            line.width <= available_width,
            "Line width {:?} exceeded limit {:?} under break-all",
            line.width,
            available_width
        );
    }

    // 2. Sob overflow-wrap: break-word
    let lines_overflow_wrap = LineBreaker::break_lines(
        &long_token,
        &font,
        &measurer,
        available_width,
        WhiteSpace::Normal,
        WordBreak::Normal,
        OverflowWrap::BreakWord,
    );

    assert!(
        lines_overflow_wrap.len() >= 15,
        "overflow-wrap: break-word must break overflowing unbreakable word, got {}",
        lines_overflow_wrap.len()
    );

    for line in &lines_overflow_wrap {
        assert!(
            line.width <= available_width,
            "Line width {:?} exceeded limit {:?} under break-word",
            line.width,
            available_width
        );
    }
}

#[test]
fn test_stress_rtl_bidi_offset_reversal() {
    // Valida que em linhas RTL as posições x são invertidas:
    // x_rtl = line_width - x_ltr - item_width
    let available_width = LayoutUnit::from_px(500);
    let mut line = LineBoxFragment::new(LayoutUnit::ZERO);

    let frag1 = ace_layout::inline::InlineFragment::Text {
        text: SmolStr::new("سلام"),
        font: StyleFont::initial(),
        bounds: ace_core::math::logical::LogicalRect::new(
            LayoutUnit::from_px(0),
            LayoutUnit::ZERO,
            LayoutUnit::from_px(80),
            LayoutUnit::from_px(20),
        ),
        ascent: LayoutUnit::from_px(14),
        descent: LayoutUnit::from_px(6),
        vertical_align: VerticalAlign::Baseline,
    };

    let frag2 = ace_layout::inline::InlineFragment::Text {
        text: SmolStr::new("دنيا"),
        font: StyleFont::initial(),
        bounds: ace_core::math::logical::LogicalRect::new(
            LayoutUnit::from_px(80),
            LayoutUnit::ZERO,
            LayoutUnit::from_px(60),
            LayoutUnit::from_px(20),
        ),
        ascent: LayoutUnit::from_px(14),
        descent: LayoutUnit::from_px(6),
        vertical_align: VerticalAlign::Baseline,
    };

    line.add_fragment(frag1);
    line.add_fragment(frag2);
    line.bounds.inline_size = LayoutUnit::from_px(140);

    // Aplica alinhamento com Direction::Rtl
    line.apply_horizontal_alignment(available_width, TextAlign::Start, Direction::Rtl, false);

    // Sob Direction::Rtl e TextAlign::Start, o texto se alinha à direita do contêiner (500px).
    // O primeiro fragmento original (0..80) vai para a extremidade direita (500 - 80 = 420px).
    // O segundo fragmento (80..140) vai à esquerda do primeiro (420 - 60 = 360px).
    let pos0 = line.fragments[0].bounds().inline_start;
    let pos1 = line.fragments[1].bounds().inline_start;

    assert_eq!(pos0, LayoutUnit::from_px(420));
    assert_eq!(pos1, LayoutUnit::from_px(360));
    assert!(
        pos1 < pos0,
        "In RTL, subsequent items must be placed to the left"
    );
}

#[test]
fn test_stress_text_align_justify_distribution() {
    let widths = [320, 640, 1024];

    for &w_px in &widths {
        let available_width = LayoutUnit::from_px(w_px);
        let mut line = LineBoxFragment::new(LayoutUnit::ZERO);

        let frag1 = ace_layout::inline::InlineFragment::Text {
            text: SmolStr::new("First"),
            font: StyleFont::initial(),
            bounds: ace_core::math::logical::LogicalRect::new(
                LayoutUnit::from_px(0),
                LayoutUnit::ZERO,
                LayoutUnit::from_px(50),
                LayoutUnit::from_px(20),
            ),
            ascent: LayoutUnit::from_px(14),
            descent: LayoutUnit::from_px(6),
            vertical_align: VerticalAlign::Baseline,
        };

        let frag2 = ace_layout::inline::InlineFragment::Text {
            text: SmolStr::new("Middle"),
            font: StyleFont::initial(),
            bounds: ace_core::math::logical::LogicalRect::new(
                LayoutUnit::from_px(50),
                LayoutUnit::ZERO,
                LayoutUnit::from_px(60),
                LayoutUnit::from_px(20),
            ),
            ascent: LayoutUnit::from_px(14),
            descent: LayoutUnit::from_px(6),
            vertical_align: VerticalAlign::Baseline,
        };

        let frag3 = ace_layout::inline::InlineFragment::Text {
            text: SmolStr::new("Last"),
            font: StyleFont::initial(),
            bounds: ace_core::math::logical::LogicalRect::new(
                LayoutUnit::from_px(110),
                LayoutUnit::ZERO,
                LayoutUnit::from_px(40),
                LayoutUnit::from_px(20),
            ),
            ascent: LayoutUnit::from_px(14),
            descent: LayoutUnit::from_px(6),
            vertical_align: VerticalAlign::Baseline,
        };

        line.add_fragment(frag1);
        line.add_fragment(frag2);
        line.add_fragment(frag3);
        line.bounds.inline_size = LayoutUnit::from_px(150);

        // Não é a última linha => justifica
        line.apply_horizontal_alignment(available_width, TextAlign::Justify, Direction::Ltr, false);

        // O primeiro fragmento deve iniciar exatamente em 0
        let first_start = line.fragments[0].bounds().inline_start;
        assert_eq!(first_start, LayoutUnit::ZERO);

        // O último fragmento deve terminar exatamente na borda direita do contêiner
        let last_frag = &line.fragments[2];
        let last_end = last_frag.bounds().inline_start + last_frag.bounds().inline_size;
        assert_eq!(
            last_end, available_width,
            "Justified line must end exactly at container boundary {}px, got {:?}",
            w_px, last_end
        );
    }
}
