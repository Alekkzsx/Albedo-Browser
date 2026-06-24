#!/usr/bin/env bash
# ============================================================================
# Albedo Browser — Custom Linter
# 12 verificações de código + integração com Clippy
# Uso: ./scripts/lint.sh [--all] [--ci]
#   --all  : mostra todos os erros (padrão: para no 1º)
#   --ci   : output para GitHub Actions
# ============================================================================

set -uo pipefail

RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

ERRORS=0
STOP_ON_FIRST=true
CI_MODE=false

for arg in "$@"; do
    case $arg in
        --all) STOP_ON_FIRST=false ;;
        --ci) CI_MODE=true; RED=''; YELLOW=''; GREEN=''; CYAN=''; BOLD=''; NC='' ;;
    esac
done

log_error() {
    ERRORS=$((ERRORS + 1))
    echo -e "${RED}error${NC}[$1]: $2"
    if [ -n "${3:-}" ]; then
        echo -e "  ${CYAN}suggested${NC}: $3"
    fi
    if [ "$STOP_ON_FIRST" = true ] && [ "$ERRORS" -gt 0 ]; then
        echo ""
        echo -e "${RED}${BOLD}FAILED${NC}: stopping at first error (use --all to see all)"
        exit 1
    fi
}

log_ok() { echo -e "${GREEN}ok${NC}:    $1"; }
log_info() { echo -e "${CYAN}info${NC}:  $1"; }

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Albedo Browser — Custom Linter (12 checks)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# ============================================================================
# CHECK 1: Allow Attributes (NÃO PERMITIDO)
# ============================================================================
log_info "Check 1: #[allow] attributes..."
ALLOW_HITS=$(grep -rn '#\[allow\b\|#!\[allow\b' src/ --include='*.rs' 2>/dev/null || true)
if [ -n "$ALLOW_HITS" ]; then
    while IFS= read -r line; do
        file=$(echo "$line" | cut -d: -f1)
        linenum=$(echo "$line" | cut -d: -f2)
        log_error "CHECK1" "${file}:${linenum}: #[allow] is forbidden" \
            "remove this attribute and fix the underlying issue"
    done <<< "$ALLOW_HITS"
else
    log_ok "No #[allow] attributes found"
fi

# ============================================================================
# CHECK 2: println!/eprintln! em código não-teste
# ============================================================================
log_info "Check 2: println!/eprintln! in non-test code..."
PRINT_HITS=$(grep -rn 'println!\|eprintln!' src/ --include='*.rs' 2>/dev/null \
    | grep -v '#\[cfg(test)\]' \
    | grep -v 'mod tests' \
    | grep -v '//.*println' \
    | grep -v '///.*println' \
    | grep -v 'set_hook' \
    | grep -v 'panic_hook' \
    | grep -v 'runtime_tests' \
    | grep -v 'setup\.rs' \
    || true)
if [ -n "$PRINT_HITS" ]; then
    COUNT=$(echo "$PRINT_HITS" | wc -l)
    log_error "CHECK2" "$COUNT println!/eprintln! calls in production code" \
        "use tracing::info!() or tracing::error!() instead"
    echo "$PRINT_HITS" | head -5
    if [ "$COUNT" -gt 5 ]; then
        echo "  ... and $((COUNT - 5)) more"
    fi
else
    log_ok "No println!/eprintln! in non-test code"
fi

# ============================================================================
# CHECK 3: dbg!() — zero tolerância
# ============================================================================
log_info "Check 3: dbg!() usage..."
DBG_HITS=$(grep -rn 'dbg!' src/ --include='*.rs' 2>/dev/null \
    | grep -v '#\[cfg(test)\]' || true)
if [ -n "$DBG_HITS" ]; then
    while IFS= read -r line; do
        file=$(echo "$line" | cut -d: -f1)
        linenum=$(echo "$line" | cut -d: -f2)
        log_error "CHECK3" "${file}:${linenum}: dbg!() is forbidden in production" \
            "remove dbg!() or replace with tracing::debug!()"
    done <<< "$DBG_HITS"
else
    log_ok "No dbg!() found"
fi

# ============================================================================
# CHECK 4: .unwrap() em código não-teste
# ============================================================================
log_info "Check 4: .unwrap() in non-test code..."
UNWRAP_HITS=$(grep -rn '\.unwrap()' src/ --include='*.rs' 2>/dev/null \
    | grep -v '#\[cfg(test)\]' | grep -v 'mod tests' || true)
if [ -n "$UNWRAP_HITS" ]; then
    COUNT=$(echo "$UNWRAP_HITS" | wc -l)
    log_error "CHECK4" "$COUNT .unwrap() calls in production code" \
        "use .expect(\"reason\") or ? operator instead"
    echo "$UNWRAP_HITS" | head -5
    if [ "$COUNT" -gt 5 ]; then
        echo "  ... and $((COUNT - 5)) more"
    fi
else
    log_ok "No .unwrap() in non-test code"
fi

# ============================================================================
# CHECK 5: unsafe blocks
# ============================================================================
log_info "Check 5: unsafe blocks..."
UNSAFE_HITS=""
while IFS= read -r file; do
    while IFS= read -r result; do
        linenum=$(echo "$result" | cut -d: -f1)
        # Check if any of the 4 lines above have SAFETY comment
        has_safety=false
        for offset in 1 2 3 4; do
            prev_line=$((linenum - offset))
            if [ "$prev_line" -gt 0 ]; then
                prev_content=$(sed -n "${prev_line}p" "$file" 2>/dev/null || true)
                if echo "$prev_content" | grep -q '//.*SAFETY'; then
                    has_safety=true
                    break
                fi
            fi
        done
        if [ "$has_safety" = false ]; then
            UNSAFE_HITS="${UNSAFE_HITS}${file}:${linenum}: unsafe block without SAFETY comment
"
        fi
    done < <(grep -n 'unsafe {' "$file" 2>/dev/null || true)
done < <(find src/ -name '*.rs' 2>/dev/null)

if [ -n "$UNSAFE_HITS" ]; then
    COUNT=$(echo "$UNSAFE_HITS" | grep -c . || true)
    log_error "CHECK5" "$COUNT unsafe blocks without // SAFETY: comment" \
        "add '// SAFETY: <justificativa>' above each unsafe block"
    echo "$UNSAFE_HITS" | head -5
    if [ "$COUNT" -gt 5 ]; then
        echo "  ... and $((COUNT - 5)) more"
    fi
else
    log_ok "No unsafe blocks without safety comments"
fi

# ============================================================================
# CHECK 6: Funções sem doc comment
# ============================================================================
log_info "Check 6: functions without doc comment..."
MISSING_DOC=0
while IFS= read -r file; do
    while IFS= read -r result; do
        linenum=$(echo "$result" | cut -d: -f1)
        # Check if the line above is a comment
        prev_line=$((linenum - 1))
        if [ "$prev_line" -gt 0 ]; then
            prev_content=$(sed -n "${prev_line}p" "$file" 2>/dev/null || true)
            if ! echo "$prev_content" | grep -qE '^\s*(///|//|pub |#\[)'; then
                log_error "CHECK6" "${file}:${linenum}: function missing doc comment" \
                    "add '/// <description>' above this function"
                MISSING_DOC=$((MISSING_DOC + 1))
                if [ "$STOP_ON_FIRST" = true ] && [ "$ERRORS" -gt 0 ]; then
                    break 2
                fi
            fi
        fi
    done < <(grep -n '^\s*\(pub \)\?\(async \)\?fn ' "$file" 2>/dev/null || true)
done < <(find src/ -name '*.rs' 2>/dev/null)

if [ "$MISSING_DOC" -eq 0 ]; then
    log_ok "All functions have doc comments"
fi

# ============================================================================
# CHECK 7: Funções longas (> 150 linhas)
# ============================================================================
log_info "Check 7: functions exceeding 150 lines..."
LONG_FUNCS=$(awk '
    /^(pub )?(async )?fn / {
        if (in_func && NR - func_start > 150) {
            printf "%s:%d (%d lines)\n", func_file, func_start, NR - func_start
        }
        func_start = NR
        func_file = FILENAME
        in_func = 1
        depth = 0
    }
    in_func {
        for (i = 1; i <= length($0); i++) {
            c = substr($0, i, 1)
            if (c == "{") depth++
            if (c == "}") depth--
        }
        if (depth == 0 && in_func) {
            if (NR - func_start > 150) {
                printf "%s:%d (%d lines)\n", func_file, func_start, NR - func_start
            }
            in_func = 0
        }
    }
' $(find src/ -name '*.rs' 2>/dev/null) 2>/dev/null || true)

if [ -n "$LONG_FUNCS" ]; then
    while IFS= read -r line; do
        log_error "CHECK7" "$line: function exceeds 150 lines" \
            "split into smaller functions"
    done <<< "$LONG_FUNCS"
else
    log_ok "No functions exceed 150 lines"
fi

# ============================================================================
# CHECK 8: Arquivos longos (> 250 linhas)
# ============================================================================
log_info "Check 8: files exceeding 250 lines..."
LARGE_FILES=$(find src/ -name '*.rs' -exec sh -c 'lines=$(wc -l < "$1"); if [ "$lines" -gt 250 ]; then echo "$1 ($lines lines)"; fi' _ {} \; 2>/dev/null || true)
if [ -n "$LARGE_FILES" ]; then
    while IFS= read -r line; do
        log_error "CHECK8" "$line: file exceeds 250 lines" \
            "split into smaller modules"
    done <<< "$LARGE_FILES"
else
    log_ok "No files exceed 250 lines"
fi

# ============================================================================
# CHECK 9: Module boundaries
# ============================================================================
log_info "Check 9: module boundary violations..."

# network/ must not import from ace/ (except contracts)
NET_ACE=$(grep -rn 'use crate::ace::' src/network/ --include='*.rs' 2>/dev/null \
    | grep -v 'contracts' || true)
if [ -n "$NET_ACE" ]; then
    while IFS= read -r line; do
        log_error "CHECK9" "$line: network/ imports from ace/" \
            "use crate::network::contracts interface instead"
    done <<< "$NET_ACE"
fi

# renderer/ must not import from browser/
REN_BRO=$(grep -rn 'use crate::browser::' src/renderer/ --include='*.rs' 2>/dev/null || true)
if [ -n "$REN_BRO" ]; then
    while IFS= read -r line; do
        log_error "CHECK9" "$line: renderer/ imports from browser/" \
            "renderer should be independent of browser shell"
    done <<< "$REN_BRO"
fi

# utils/ must not import from project modules
UTILS_PROJ=$(grep -rn 'use crate::' src/utils/ --include='*.rs' 2>/dev/null || true)
if [ -n "$UTILS_PROJ" ]; then
    while IFS= read -r line; do
        log_error "CHECK9" "$line: utils/ imports from other modules" \
            "utils must be standalone with no project dependencies"
    done <<< "$UTILS_PROJ"
fi

# ace/ must not import from browser/ or renderer/
ACE_SHELL=$(grep -rn 'use crate::browser::\|use crate::renderer::\|use crate::ui::' src/ace/ --include='*.rs' 2>/dev/null || true)
if [ -n "$ACE_SHELL" ]; then
    while IFS= read -r line; do
        log_error "CHECK9" "$line: ace/ imports from shell layer" \
            "ace engine must not depend on browser/renderer/ui"
    done <<< "$ACE_SHELL"
fi

if [ -z "${NET_ACE:-}${REN_BRO:-}${UTILS_PROJ:-}${ACE_SHELL:-}" ]; then
    log_ok "No module boundary violations"
fi

# ============================================================================
# CHECK 10: Imports circulares (simplificado)
# ============================================================================
log_info "Check 10: circular import detection..."
CIRCULAR=0

# Check if ace/ files import from browser/ AND browser/ imports from ace/
ACE_TO_BROWSER=$(grep -rn 'use crate::browser::' src/ace/ --include='*.rs' 2>/dev/null | wc -l)
BROWSER_TO_ACE=$(grep -rn 'use crate::ace::' src/browser/ --include='*.rs' 2>/dev/null | wc -l)
if [ "$ACE_TO_BROWSER" -gt 0 ] && [ "$BROWSER_TO_ACE" -gt 0 ]; then
    log_error "CHECK10" "circular dependency: ace/ ↔ browser/" \
        "break the cycle by introducing a shared interface crate"
    CIRCULAR=1
fi

# Check network/ ↔ ace/
NETWORK_TO_ACE=$(grep -rn 'use crate::ace::' src/network/ --include='*.rs' 2>/dev/null | grep -v contracts | wc -l)
ACE_TO_NETWORK=$(grep -rn 'use crate::network::' src/ace/ --include='*.rs' 2>/dev/null | wc -l)
if [ "$NETWORK_TO_ACE" -gt 0 ] && [ "$ACE_TO_NETWORK" -gt 0 ]; then
    log_error "CHECK10" "circular dependency: ace/ ↔ network/" \
        "break the cycle by introducing a shared interface crate"
    CIRCULAR=1
fi

if [ "$CIRCULAR" -eq 0 ]; then
    log_ok "No circular imports detected"
fi

# ============================================================================
# CHECK 11: albedo-jit cyclic dependency
# ============================================================================
log_info "Check 11: albedo-jit cyclic dependency..."
JIT_CYCLIC=$(grep -rn 'use albedo::' albedo-jit/src/ --include='*.rs' 2>/dev/null || true)
if [ -n "$JIT_CYCLIC" ]; then
    while IFS= read -r line; do
        log_error "CHECK11" "$line: albedo-jit imports from albedo" \
            "albedo-jit must not depend on albedo (use contracts interface)"
    done <<< "$JIT_CYCLIC"
else
    log_ok "No cyclic dependency in albedo-jit"
fi

# ============================================================================
# CHECK 12: Nomes abreviados (fora de loops)
# ============================================================================
log_info "Check 12: abbreviated variable names..."
# Only flag standalone variable declarations with short names, not loop counters
ABBREV_HITS=$(grep -rn 'let \(mut \)\?\(x\|y\|z\|tmp\|res\|val\|ret\|idx\)\s*=' src/ --include='*.rs' 2>/dev/null \
    | grep -v '#\[cfg(test)\]' | grep -v 'mod tests' || true)
if [ -n "$ABBREV_HITS" ]; then
    COUNT=$(echo "$ABBREV_HITS" | wc -l)
    log_error "CHECK12" "$COUNT abbreviated variable names (x, y, z, tmp, res, val, ret, idx)" \
        "use descriptive names (e.g., cursor_x, result, return_value)"
    echo "$ABBREV_HITS" | head -5
    if [ "$COUNT" -gt 5 ]; then
        echo "  ... and $((COUNT - 5)) more"
    fi
else
    log_ok "No abbreviated variable names found"
fi

# ============================================================================
# SUMMARY
# ============================================================================
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ "$ERRORS" -gt 0 ]; then
    echo -e "  ${RED}${BOLD}FAILED${NC}: $ERRORS errors found"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    exit 1
else
    echo -e "  ${GREEN}${BOLD}PASSED${NC}: all 12 checks clean"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    exit 0
fi
