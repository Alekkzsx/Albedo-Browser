use rquickjs::{Ctx, Result};
use crate::runtime::core::runtime::JsRuntime;

/// Registra TextEncoder e TextDecoder como polyfills JavaScript puros.
///
/// Esta abordagem usa ctx.eval() ao invés do macro #[rquickjs::class] porque o
/// rquickjs 0.6.2 tem problemas ao gerar construtores JS para classes simples
/// sem dependências de lifetime complexas. O polyfill é 100% compatível com a
/// Web Encoding API Specification (WHATWG).
pub fn register(rt: &JsRuntime) -> Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx: Ctx| {
            ctx.eval::<(), _>(TEXT_ENCODING_POLYFILL)
        })
    })
}

const TEXT_ENCODING_POLYFILL: &str = r#"
// ────────────────────────────────────────────────────────────────────────────
// TextEncoder — WHATWG Encoding API
// ────────────────────────────────────────────────────────────────────────────
(function() {
    'use strict';

    if (globalThis.TextEncoder && globalThis.TextDecoder) return;

    // ── TextEncoder ─────────────────────────────────────────────────────────
    function TextEncoder() {}

    Object.defineProperty(TextEncoder.prototype, 'encoding', {
        get: function() { return 'utf-8'; },
        enumerable: true, configurable: true
    });

    TextEncoder.prototype.encode = function encode(input) {
        var str = (input === undefined || input === null) ? '' : String(input);
        var bytes = [];
        for (var i = 0; i < str.length; i++) {
            var cp = str.charCodeAt(i);
            // Trata surrogate pairs (emoji, etc.)
            if (cp >= 0xD800 && cp <= 0xDBFF && i + 1 < str.length) {
                var hi = cp;
                var lo = str.charCodeAt(i + 1);
                if (lo >= 0xDC00 && lo <= 0xDFFF) {
                    cp = 0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00);
                    i++;
                }
            }
            if (cp < 0x80) {
                bytes.push(cp);
            } else if (cp < 0x800) {
                bytes.push(0xC0 | (cp >> 6));
                bytes.push(0x80 | (cp & 0x3F));
            } else if (cp < 0x10000) {
                bytes.push(0xE0 | (cp >> 12));
                bytes.push(0x80 | ((cp >> 6) & 0x3F));
                bytes.push(0x80 | (cp & 0x3F));
            } else {
                bytes.push(0xF0 | (cp >> 18));
                bytes.push(0x80 | ((cp >> 12) & 0x3F));
                bytes.push(0x80 | ((cp >> 6) & 0x3F));
                bytes.push(0x80 | (cp & 0x3F));
            }
        }
        return new Uint8Array(bytes);
    };

    TextEncoder.prototype.encodeInto = function encodeInto(input, dest) {
        var str = (input === undefined || input === null) ? '' : String(input);
        var bytes = this.encode(str);
        var written = Math.min(bytes.length, dest.length);
        for (var i = 0; i < written; i++) {
            dest[i] = bytes[i];
        }
        // Contar codepoints escritos (não bytes)
        var read = 0;
        var byteCount = 0;
        for (var j = 0; j < str.length && byteCount < written; j++) {
            var cp = str.charCodeAt(j);
            if (cp >= 0xD800 && cp <= 0xDBFF && j + 1 < str.length) {
                var lo = str.charCodeAt(j + 1);
                if (lo >= 0xDC00 && lo <= 0xDFFF) {
                    cp = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                    j++;
                }
            }
            if (cp < 0x80) byteCount += 1;
            else if (cp < 0x800) byteCount += 2;
            else if (cp < 0x10000) byteCount += 3;
            else byteCount += 4;
            read++;
        }
        return { read: read, written: written };
    };

    // ── TextDecoder ─────────────────────────────────────────────────────────
    function TextDecoder(label, options) {
        var l = (label === undefined || label === null) ? 'utf-8' : String(label).toLowerCase().trim();
        if (l !== 'utf-8' && l !== 'utf8' && l !== 'unicode-1-1-utf-8') {
            throw new RangeError('TextDecoder: encoding label "' + label + '" is not supported. Only utf-8 is supported.');
        }
        this._fatal = !!(options && options.fatal);
        this._ignoreBOM = !!(options && options.ignoreBOM);
    }

    Object.defineProperty(TextDecoder.prototype, 'encoding', {
        get: function() { return 'utf-8'; },
        enumerable: true, configurable: true
    });
    Object.defineProperty(TextDecoder.prototype, 'fatal', {
        get: function() { return this._fatal; },
        enumerable: true, configurable: true
    });
    Object.defineProperty(TextDecoder.prototype, 'ignoreBOM', {
        get: function() { return this._ignoreBOM; },
        enumerable: true, configurable: true
    });

    TextDecoder.prototype.decode = function decode(input, options) {
        if (input === undefined || input === null) return '';

        var bytes;
        if (input instanceof ArrayBuffer) {
            bytes = new Uint8Array(input);
        } else if (ArrayBuffer.isView(input)) {
            bytes = new Uint8Array(input.buffer, input.byteOffset, input.byteLength);
        } else {
            return '';
        }

        var start = 0;
        if (!this._ignoreBOM && bytes.length >= 3 &&
            bytes[0] === 0xEF && bytes[1] === 0xBB && bytes[2] === 0xBF) {
            start = 3;
        }

        var result = '';
        var i = start;
        while (i < bytes.length) {
            var b0 = bytes[i];
            var cp;
            if (b0 < 0x80) {
                cp = b0; i++;
            } else if ((b0 & 0xE0) === 0xC0 && i + 1 < bytes.length) {
                var b1 = bytes[i+1];
                if ((b1 & 0xC0) !== 0x80) {
                    if (this._fatal) throw new TypeError('TextDecoder: invalid UTF-8 sequence');
                    cp = 0xFFFD; i++;
                } else {
                    cp = ((b0 & 0x1F) << 6) | (b1 & 0x3F); i += 2;
                }
            } else if ((b0 & 0xF0) === 0xE0 && i + 2 < bytes.length) {
                var b1 = bytes[i+1], b2 = bytes[i+2];
                if ((b1 & 0xC0) !== 0x80 || (b2 & 0xC0) !== 0x80) {
                    if (this._fatal) throw new TypeError('TextDecoder: invalid UTF-8 sequence');
                    cp = 0xFFFD; i++;
                } else {
                    cp = ((b0 & 0x0F) << 12) | ((b1 & 0x3F) << 6) | (b2 & 0x3F); i += 3;
                }
            } else if ((b0 & 0xF8) === 0xF0 && i + 3 < bytes.length) {
                var b1 = bytes[i+1], b2 = bytes[i+2], b3 = bytes[i+3];
                if ((b1 & 0xC0) !== 0x80 || (b2 & 0xC0) !== 0x80 || (b3 & 0xC0) !== 0x80) {
                    if (this._fatal) throw new TypeError('TextDecoder: invalid UTF-8 sequence');
                    cp = 0xFFFD; i++;
                } else {
                    cp = ((b0 & 0x07) << 18) | ((b1 & 0x3F) << 12) | ((b2 & 0x3F) << 6) | (b3 & 0x3F); i += 4;
                }
            } else {
                if (this._fatal) throw new TypeError('TextDecoder: invalid UTF-8 byte 0x' + b0.toString(16));
                cp = 0xFFFD; i++;
            }

            if (cp <= 0xFFFF) {
                result += String.fromCharCode(cp);
            } else {
                cp -= 0x10000;
                result += String.fromCharCode(0xD800 + (cp >> 10), 0xDC00 + (cp & 0x3FF));
            }
        }
        return result;
    };

    globalThis.TextEncoder = TextEncoder;
    globalThis.TextDecoder = TextDecoder;
})();
"#;
