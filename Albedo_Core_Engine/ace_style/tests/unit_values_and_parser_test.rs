//! # Testes Unitários de Valores Primitivos e Parser L3 (Tier 1 & Tier 2)

use ace_core::math::layout_unit::LayoutUnit;
use ace_style::parser::component_value::{ComponentValueParser, MAX_NESTING_DEPTH};
use ace_style::parser::declaration_parser::parse_declarations;
use ace_style::parser::token::Token;
use ace_style::parser::tokenizer::Tokenizer;
use ace_style::values::color::CssColor;
use ace_style::values::enums::*;
use ace_style::values::length::{CalcExpr, Length, LengthPercentage};

#[test]
fn test_length_conversions() {
    let font_size = 16.0;
    let root_font_size = 16.0;
    let viewport = (1920.0, 1080.0);

    let px = Length::Px(32.0);
    assert_eq!(px.to_layout_unit(font_size, root_font_size, viewport), Some(LayoutUnit::from_px(32)));

    let em = Length::Em(2.0);
    assert_eq!(em.to_layout_unit(font_size, root_font_size, viewport), Some(LayoutUnit::from_px(32)));

    let rem = Length::Rem(1.5);
    assert_eq!(rem.to_layout_unit(font_size, root_font_size, viewport), Some(LayoutUnit::from_px(24)));

    let vh = Length::Vh(10.0);
    assert_eq!(vh.to_layout_unit(font_size, root_font_size, viewport), Some(LayoutUnit::from_px(108)));

    let vw = Length::Vw(10.0);
    assert_eq!(vw.to_layout_unit(font_size, root_font_size, viewport), Some(LayoutUnit::from_px(192)));

    let auto = Length::Auto;
    assert_eq!(auto.to_layout_unit(font_size, root_font_size, viewport), None);

    let zero = Length::Zero;
    assert_eq!(zero.to_layout_unit(font_size, root_font_size, viewport), Some(LayoutUnit::ZERO));
}

#[test]
fn test_calc_expression_evaluation() {
    let font_size = 16.0;
    let root_font_size = 16.0;
    let viewport = (1000.0, 1000.0);

    let calc = CalcExpr::parse("calc(100px + 20px)").expect("parse calc");
    let result = calc.eval_layout_unit(font_size, root_font_size, viewport, None);
    assert_eq!(result, Some(LayoutUnit::from_px(120)));

    let calc_sub = CalcExpr::parse("calc(10rem - 2rem)").expect("parse calc");
    let result_sub = calc_sub.eval_layout_unit(font_size, root_font_size, viewport, None);
    assert_eq!(result_sub, Some(LayoutUnit::from_px(128)));

    let calc_mul = CalcExpr::parse("calc(10px * 3)").expect("parse calc");
    let result_mul = calc_mul.eval_layout_unit(font_size, root_font_size, viewport, None);
    assert_eq!(result_mul, Some(LayoutUnit::from_px(30)));

    let calc_div = CalcExpr::parse("calc(100px / 4)").expect("parse calc");
    let result_div = calc_div.eval_layout_unit(font_size, root_font_size, viewport, None);
    assert_eq!(result_div, Some(LayoutUnit::from_px(25)));
}

#[test]
fn test_css_colors() {
    let hex = CssColor::parse("#ff0000").expect("parse hex");
    let resolved = hex.resolve(ace_style::Color::BLACK);
    assert_eq!(resolved.r, 255);
    assert_eq!(resolved.g, 0);
    assert_eq!(resolved.b, 0);

    let current = CssColor::parse("currentColor").expect("parse currentColor");
    let resolved_current = current.resolve(ace_style::Color::from_rgb(0, 255, 0));
    assert_eq!(resolved_current.g, 255);

    let transparent = CssColor::parse("transparent").expect("parse transparent");
    let resolved_trans = transparent.resolve(ace_style::Color::WHITE);
    assert_eq!(resolved_trans.a, 0);
}

#[test]
fn test_tokenizer_normative() {
    let css = r#"
        /* Comment */
        @media screen {
            #main.active {
                display: flex;
                margin: 10px 20px;
                color: "hello \"world\"";
            }
        }
    "#;

    let mut tokenizer = Tokenizer::new(css);
    let tokens = tokenizer.tokenize_all();

    assert!(tokens.iter().any(|t| matches!(t, Token::AtKeyword(k) if k == "media")));
    assert!(tokens.iter().any(|t| matches!(t, Token::Hash { value, .. } if value == "main")));
    assert!(tokens.iter().any(|t| matches!(t, Token::Ident(i) if i == "active")));
    assert!(tokens.iter().any(|t| matches!(t, Token::Dimension { value, unit } if *value == 10.0 && unit == "px")));
    assert!(tokens.iter().any(|t| matches!(t, Token::String(s) if s == "hello \"world\"")));
}

#[test]
fn test_shorthand_expansions() {
    let decls = parse_declarations("margin: 10px 20px 30px 40px !important;");
    assert_eq!(decls.len(), 4);
    assert!(decls.iter().all(|d| d.important));
    assert_eq!(decls[0].property.as_str(), "margin-top");
    assert_eq!(decls[0].value.as_str(), "10px");
    assert_eq!(decls[1].property.as_str(), "margin-right");
    assert_eq!(decls[1].value.as_str(), "20px");
    assert_eq!(decls[2].property.as_str(), "margin-bottom");
    assert_eq!(decls[2].value.as_str(), "30px");
    assert_eq!(decls[3].property.as_str(), "margin-left");
    assert_eq!(decls[3].value.as_str(), "40px");

    let flex = parse_declarations("flex: 2 1 100px;");
    assert_eq!(flex.len(), 3);
    assert_eq!(flex[0].property.as_str(), "flex-grow");
    assert_eq!(flex[0].value.as_str(), "2");
    assert_eq!(flex[1].property.as_str(), "flex-shrink");
    assert_eq!(flex[1].value.as_str(), "1");
    assert_eq!(flex[2].property.as_str(), "flex-basis");
    assert_eq!(flex[2].value.as_str(), "100px");
}
