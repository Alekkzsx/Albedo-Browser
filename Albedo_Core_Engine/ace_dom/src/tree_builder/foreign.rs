//! # Ajustes de Conteúdo Estrangeiro (SVG & MathML — WHATWG §12.2.6.5)
//!
//! Tabelas estáticas de alto desempenho ($O(1)$) para ajuste de Case-Sensitivity
//! de tags e atributos SVG/MathML, e detecção de pontos de integração HTML/MathML.

use ace_core::intern::Atom;
use phf::phf_map;

/// Mapa estático de ajuste normativo de tags SVG (lowercase -> CamelCase).
static SVG_TAG_NAME_FIXUPS: phf::Map<&'static str, &'static str> = phf_map! {
    "altglyph" => "altGlyph",
    "altglyphdef" => "altGlyphDef",
    "altglyphitem" => "altGlyphItem",
    "animatecolor" => "animateColor",
    "animatemotion" => "animateMotion",
    "animatetransform" => "animateTransform",
    "clippath" => "clipPath",
    "feblend" => "feBlend",
    "fecolormatrix" => "feColorMatrix",
    "fecomponenttransfer" => "feComponentTransfer",
    "fecomposite" => "feComposite",
    "feconvolvematrix" => "feConvolveMatrix",
    "fediffuselighting" => "feDiffuseLighting",
    "fedisplacementmap" => "feDisplacementMap",
    "fedistantlight" => "feDistantLight",
    "fedropshadow" => "feDropShadow",
    "feflood" => "feFlood",
    "fefunca" => "feFuncA",
    "fefuncb" => "feFuncB",
    "fefuncg" => "feFuncG",
    "fefuncr" => "feFuncR",
    "fegaussianblur" => "feGaussianBlur",
    "feimage" => "feImage",
    "femerge" => "feMerge",
    "femergenode" => "feMergeNode",
    "femorphology" => "feMorphology",
    "feoffset" => "feOffset",
    "fepointlight" => "fePointLight",
    "fespecularlighting" => "feSpecularLighting",
    "fespotlight" => "feSpotLight",
    "fetile" => "feTile",
    "feturbulence" => "feTurbulence",
    "foreignobject" => "foreignObject",
    "glyphref" => "glyphRef",
    "lineargradient" => "linearGradient",
    "radialgradient" => "radialGradient",
    "textpath" => "textPath",
};

/// Mapa estático de ajuste normativo de atributos SVG (lowercase -> CamelCase).
static SVG_ATTRIBUTE_FIXUPS: phf::Map<&'static str, &'static str> = phf_map! {
    "attributename" => "attributeName",
    "attributetype" => "attributeType",
    "basefrequency" => "baseFrequency",
    "baseprofile" => "baseProfile",
    "calcmode" => "calcMode",
    "clippathunits" => "clipPathUnits",
    "diffuseconstant" => "diffuseConstant",
    "edgemode" => "edgeMode",
    "filterunits" => "filterUnits",
    "glyphref" => "glyphRef",
    "gradienttransform" => "gradientTransform",
    "gradientunits" => "gradientUnits",
    "kernelmatrix" => "kernelMatrix",
    "kernelunitlength" => "kernelUnitLength",
    "keypoints" => "keyPoints",
    "keysplines" => "keySplines",
    "keytimes" => "keyTimes",
    "lengthadjust" => "lengthAdjust",
    "limitingconeangle" => "limitingConeAngle",
    "markerheight" => "markerHeight",
    "markerunits" => "markerUnits",
    "markerwidth" => "markerWidth",
    "maskcontentunits" => "maskContentUnits",
    "maskunits" => "maskUnits",
    "numoctaves" => "numOctaves",
    "pathlength" => "pathLength",
    "patterncontentunits" => "patternContentUnits",
    "patterntransform" => "patternTransform",
    "patternunits" => "patternUnits",
    "pointsatx" => "pointsAtX",
    "pointsaty" => "pointsAtY",
    "pointsatz" => "pointsAtZ",
    "preservealpha" => "preserveAlpha",
    "preserveaspectratio" => "preserveAspectRatio",
    "primitiveunits" => "primitiveUnits",
    "refx" => "refX",
    "refy" => "refY",
    "repeatcount" => "repeatCount",
    "repeatdur" => "repeatDur",
    "requiredextensions" => "requiredExtensions",
    "requiredfeatures" => "requiredFeatures",
    "specularconstant" => "specularConstant",
    "specularexponent" => "specularExponent",
    "spreadmethod" => "spreadMethod",
    "startoffset" => "startOffset",
    "stddeviation" => "stdDeviation",
    "stitchtiles" => "stitchTiles",
    "surfacescale" => "surfaceScale",
    "systemlanguage" => "systemLanguage",
    "tablevalues" => "tableValues",
    "targetx" => "targetX",
    "targety" => "targetY",
    "textlength" => "textLength",
    "viewbox" => "viewBox",
    "viewtarget" => "viewTarget",
    "xchannelselector" => "xChannelSelector",
    "ychannelselector" => "yChannelSelector",
    "zoomandpan" => "zoomAndPan",
};

/// Ajusta o nome de uma tag SVG para a grafia CamelCase normativa.
#[inline]
pub fn adjust_svg_tag_name(tag: &str) -> Atom {
    let lower = tag.to_ascii_lowercase();
    if let Some(&adjusted) = SVG_TAG_NAME_FIXUPS.get(lower.as_str()) {
        Atom::new(adjusted)
    } else {
        Atom::new(&lower)
    }
}

/// Ajusta o nome de um atributo SVG para a grafia CamelCase normativa.
#[inline]
pub fn adjust_svg_attribute_name(attr: &str) -> Atom {
    let lower = attr.to_ascii_lowercase();
    if let Some(&adjusted) = SVG_ATTRIBUTE_FIXUPS.get(lower.as_str()) {
        Atom::new(adjusted)
    } else {
        Atom::new(&lower)
    }
}

/// Retorna `true` se a tag for um ponto de integração HTML dentro de SVG (WHATWG §12.2.6.5).
#[inline]
pub fn is_html_integration_point_in_svg(tag: &str) -> bool {
    matches!(tag, "foreignObject" | "desc" | "title")
}

/// Retorna `true` se a tag for um ponto de integração HTML dentro de MathML.
#[inline]
pub fn is_html_integration_point_in_mathml(tag: &str) -> bool {
    matches!(tag, "annotation-xml" | "mi" | "mo" | "mn" | "ms" | "mtext")
}
