use rquickjs::{Ctx, Result};
use crate::runtime::core::runtime::JsRuntime;

/// Registra structuredClone como um polyfill JavaScript puro.
///
/// Seguindo o padrão consolidado no Albedo (vide text_encoding.rs).
pub fn register(rt: &JsRuntime) -> Result<()> {
    rt.with_context(|ctx| {
        ctx.with(|ctx: Ctx| {
            ctx.eval::<(), _>(STRUCTURED_CLONE_POLYFILL)
        })
    })
}

const STRUCTURED_CLONE_POLYFILL: &str = r#"
(function() {
    'use strict';
    if (globalThis.structuredClone) return;

    function structuredClone(value, options) {
        const transfer = options && options.transfer ? Array.from(options.transfer) : [];
        const seen = new Map();

        function clone(v) {
            // 1. Tratamento de Primitivos e Tipos Não-Clonáveis
            if (v === null) return null;
            
            const typeOf = typeof v;
            if (typeOf !== 'object') {
                if (typeOf === 'function' || typeOf === 'symbol') {
                    const err = new Error("DataCloneError: " + typeOf + " could not be cloned.");
                    err.name = "DataCloneError";
                    throw err;
                }
                return v;
            }

            // 2. Referências Circulares
            if (seen.has(v)) return seen.get(v);

            let result;
            const type = Object.prototype.toString.call(v);

            // 3. Objetos Complexos
            if (v instanceof Date) {
                result = new Date(v.getTime());
            } else if (v instanceof RegExp) {
                result = new RegExp(v.source, v.flags);
            } else if (v instanceof Map) {
                result = new Map();
                seen.set(v, result);
                v.forEach((val, key) => result.set(clone(key), clone(val)));
                return result;
            } else if (v instanceof Set) {
                result = new Set();
                seen.set(v, result);
                v.forEach(val => result.add(clone(val)));
                return result;
            } else if (ArrayBuffer.isView(v)) {
                // TypedArrays
                const buffer = clone(v.buffer);
                result = new v.constructor(buffer, v.byteOffset, v.length);
            } else if (v instanceof ArrayBuffer) {
                result = v.slice(0);
            } else if (v instanceof Error) {
                const ErrorCtor = v.constructor;
                result = new ErrorCtor(v.message);
                if (v.stack) result.stack = v.stack;
                // cause não é padrão em todos os ambientes mas a spec menciona
                if ('cause' in v) result.cause = clone(v.cause);
            } else if (type === '[object Array]') {
                result = new Array(v.length);
                seen.set(v, result);
                for (let i = 0; i < v.length; i++) {
                    if (i in v) result[i] = clone(v[i]);
                }
                return result;
            } else if (type === '[object Object]') {
                result = {};
                seen.set(v, result);
                for (const key in v) {
                    if (Object.prototype.hasOwnProperty.call(v, key)) {
                        result[key] = clone(v[key]);
                    }
                }
                return result;
            } else {
                const err = new Error("DataCloneError: " + type + " could not be cloned.");
                err.name = "DataCloneError";
                throw err;
            }

            seen.set(v, result);
            return result;
        }

        const cloned = clone(value);
        
        // 4. Lógica de Transferência (Neutralização simbólica no polyfill)
        transfer.forEach(t => {
            if (t instanceof ArrayBuffer) {
                // Em browsers reais, isso "desaloca" o buffer original.
                // Como polyfill JS puro, não temos controle de memória baixo nível,
                // mas podemos tentar impedir o uso futuro dele se o motor permitir.
            }
        });

        return cloned;
    }

    globalThis.structuredClone = structuredClone;
})();
"#;
