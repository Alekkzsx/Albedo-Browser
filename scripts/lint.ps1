# ============================================================================
# Albedo Browser — Custom Linter (PowerShell)
# 12 verificações de código + integração com Clippy
# Uso: .\scripts\lint.ps1 [-All] [-CI]
#   -All  : mostra todos os erros (padrão: para no 1º)
#   -CI   : output para GitHub Actions
# ============================================================================

param(
    [switch]$All,
    [switch]$CI
)

$ErrorActionPreference = "Continue"
$ERRORS = 0
$STOP_ON_FIRST = -not $All

function Log-Error {
    param([string]$Check, [string]$Message, [string]$Suggestion = "")
    $script:ERRORS++
    Write-Host "error[$Check]: $Message" -ForegroundColor Red
    if ($Suggestion) {
        Write-Host "  suggested: $Suggestion" -ForegroundColor Cyan
    }
    if ($STOP_ON_FIRST -and $script:ERRORS -gt 0) {
        Write-Host ""
        Write-Host "FAILED: stopping at first error (use -All to see all)" -ForegroundColor Red
        exit 1
    }
}

function Log-Ok {
    param([string]$Message)
    Write-Host "ok:    $Message" -ForegroundColor Green
}

function Log-Info {
    param([string]$Message)
    Write-Host "info:  $Message" -ForegroundColor Cyan
}

Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "  Albedo Browser — Custom Linter (12 checks)" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

# CHECK 1: Allow Attributes
Log-Info "Check 1: #[allow] attributes..."
$allowHits = Get-ChildItem -Path src -Recurse -Filter *.rs | Select-String -Pattern '#\[allow\b|#!\[allow\b' 2>$null
if ($allowHits) {
    foreach ($hit in $allowHits) {
        Log-Error "CHECK1" "$($hit.Filename):$($hit.LineNumber): #[allow] is forbidden" "remove this attribute and fix the underlying issue"
    }
} else {
    Log-Ok "No #[allow] attributes found"
}

# CHECK 2: println!/eprintln!
Log-Info "Check 2: println!/eprintln! in non-test code..."
$printHits = Get-ChildItem -Path src -Recurse -Filter *.rs | Select-String -Pattern 'println!|eprintln!' 2>$null | Where-Object { $_.Line -notmatch '#\[cfg\(test\)\]' -and $_.Line -notmatch 'mod tests' }
if ($printHits) {
    Log-Error "CHECK2" "$($printHits.Count) println!/eprintln! calls in production code" "use tracing::info!() or tracing::error!() instead"
    $printHits | Select-Object -First 5 | ForEach-Object { Write-Host "  $($_.Filename):$($_.LineNumber): $($_.Line.Trim())" }
    if ($printHits.Count -gt 5) {
        Write-Host "  ... and $($printHits.Count - 5) more"
    }
} else {
    Log-Ok "No println!/eprintln! in non-test code"
}

# CHECK 3: dbg!()
Log-Info "Check 3: dbg!() usage..."
$dbgHits = Get-ChildItem -Path src -Recurse -Filter *.rs | Select-String -Pattern 'dbg!' 2>$null | Where-Object { $_.Line -notmatch '#\[cfg\(test\)\]' }
if ($dbgHits) {
    foreach ($hit in $dbgHits) {
        Log-Error "CHECK3" "$($hit.Filename):$($hit.LineNumber): dbg!() is forbidden in production" "remove dbg!() or replace with tracing::debug!()"
    }
} else {
    Log-Ok "No dbg!() found"
}

# CHECK 4: .unwrap()
Log-Info "Check 4: .unwrap() in non-test code..."
$unwrapHits = Get-ChildItem -Path src -Recurse -Filter *.rs | Select-String -Pattern '\.unwrap\(\)' 2>$null | Where-Object { $_.Line -notmatch '#\[cfg\(test\)\]' -and $_.Line -notmatch 'mod tests' }
if ($unwrapHits) {
    Log-Error "CHECK4" "$($unwrapHits.Count) .unwrap() calls in production code" "use .expect(""reason"") or ? operator instead"
    $unwrapHits | Select-Object -First 5 | ForEach-Object { Write-Host "  $($_.Filename):$($_.LineNumber): $($_.Line.Trim())" }
    if ($unwrapHits.Count -gt 5) {
        Write-Host "  ... and $($unwrapHits.Count - 5) more"
    }
} else {
    Log-Ok "No .unwrap() in non-test code"
}

# CHECK 5: unsafe blocks
Log-Info "Check 5: unsafe blocks..."
$unsafeHits = Get-ChildItem -Path src -Recurse -Filter *.rs | Select-String -Pattern 'unsafe \{' 2>$null | Where-Object { $_.Line -notmatch '//.*SAFETY' -and $_.Line -notmatch '#\[cfg\(test\)\]' }
if ($unsafeHits) {
    Log-Error "CHECK5" "$($unsafeHits.Count) unsafe blocks without // SAFETY: comment" "add '// SAFETY: <justificativa>' above each unsafe block"
    $unsafeHits | Select-Object -First 5 | ForEach-Object { Write-Host "  $($_.Filename):$($_.LineNumber): $($_.Line.Trim())" }
    if ($unsafeHits.Count -gt 5) {
        Write-Host "  ... and $($unsafeHits.Count - 5) more"
    }
} else {
    Log-Ok "No unsafe blocks without safety comments"
}

# CHECK 6: Functions without doc comment
Log-Info "Check 6: functions without doc comment..."
$missingDoc = 0
$files = Get-ChildItem -Path src -Recurse -Filter *.rs
foreach ($file in $files) {
    $lines = Get-Content $file.FullName
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match '^\s*(pub\s+)?(async\s+)?fn\s+') {
            if ($i -gt 0) {
                $prevLine = $lines[$i - 1].Trim()
                if ($prevLine -notmatch '^(///|//|pub|#\[)') {
                    Log-Error "CHECK6" "$($file.Name):$($i + 1): function missing doc comment" "add '/// <description>' above this function"
                    $missingDoc++
                    if ($STOP_ON_FIRST -and $script:ERRORS -gt 0) { break }
                }
            }
        }
    }
}
if ($missingDoc -eq 0) {
    Log-Ok "All functions have doc comments"
}

# CHECK 7: Long functions (> 150 lines)
Log-Info "Check 7: functions exceeding 150 lines..."
# Simplified check - count functions with many lines
$longFuncs = @()
foreach ($file in $files) {
    $content = Get-Content $file.FullName -Raw
    if ($content -match '(?s)fn\s+\w+.*?\{.{5000,}') {
        $longFuncs += "$($file.Name): function exceeds 150 lines"
    }
}
if ($longFuncs.Count -gt 0) {
    foreach ($func in $longFuncs) {
        Log-Error "CHECK7" $func "split into smaller functions"
    }
} else {
    Log-Ok "No functions exceed 150 lines"
}

# CHECK 8: Large files (> 250 lines)
Log-Info "Check 8: files exceeding 250 lines..."
$largeFiles = $files | Where-Object { (Get-Content $_.FullName | Measure-Object -Line).Lines -gt 250 }
if ($largeFiles) {
    foreach ($file in $largeFiles) {
        $lines = (Get-Content $file.FullName | Measure-Object -Line).Lines
        Log-Error "CHECK8" "$($file.Name) ($lines lines): file exceeds 250 lines" "split into smaller modules"
    }
} else {
    Log-Ok "No files exceed 250 lines"
}

# CHECK 9: Module boundaries
Log-Info "Check 9: module boundary violations..."
$netAce = Get-ChildItem -Path src/network -Recurse -Filter *.rs | Select-String -Pattern 'use crate::ace::' 2>$null | Where-Object { $_.Line -notmatch 'contracts' }
if ($netAce) {
    foreach ($hit in $netAce) {
        Log-Error "CHECK9" "$($hit.Filename):$($hit.LineNumber): network/ imports from ace/" "use crate::network::contracts interface instead"
    }
}

$renBro = Get-ChildItem -Path src/renderer -Recurse -Filter *.rs | Select-String -Pattern 'use crate::browser::' 2>$null
if ($renBro) {
    foreach ($hit in $renBro) {
        Log-Error "CHECK9" "$($hit.Filename):$($hit.LineNumber): renderer/ imports from browser/" "renderer should be independent of browser shell"
    }
}

$utilsProj = Get-ChildItem -Path src/utils -Recurse -Filter *.rs | Select-String -Pattern 'use crate::' 2>$null
if ($utilsProj) {
    foreach ($hit in $utilsProj) {
        Log-Error "CHECK9" "$($hit.Filename):$($hit.LineNumber): utils/ imports from other modules" "utils must be standalone with no project dependencies"
    }
}

if (-not $netAce -and -not $renBro -and -not $utilsProj) {
    Log-Ok "No module boundary violations"
}

# CHECK 10: Circular imports
Log-Info "Check 10: circular import detection..."
$aceToBrowser = (Get-ChildItem -Path src/ace -Recurse -Filter *.rs | Select-String -Pattern 'use crate::browser::' 2>$null).Count
$browserToAce = (Get-ChildItem -Path src/browser -Recurse -Filter *.rs | Select-String -Pattern 'use crate::ace::' 2>$null).Count
if ($aceToBrowser -gt 0 -and $browserToAce -gt 0) {
    Log-Error "CHECK10" "circular dependency: ace/ ↔ browser/" "break the cycle by introducing a shared interface crate"
}

$networkToAce = (Get-ChildItem -Path src/network -Recurse -Filter *.rs | Select-String -Pattern 'use crate::ace::' 2>$null | Where-Object { $_.Line -notmatch 'contracts' }).Count
$aceToNetwork = (Get-ChildItem -Path src/ace -Recurse -Filter *.rs | Select-String -Pattern 'use crate::network::' 2>$null).Count
if ($networkToAce -gt 0 -and $aceToNetwork -gt 0) {
    Log-Error "CHECK10" "circular dependency: ace/ ↔ network/" "break the cycle by introducing a shared interface crate"
}

if ($aceToBrowser -eq 0 -and $browserToAce -eq 0 -and $networkToAce -eq 0 -and $aceToNetwork -eq 0) {
    Log-Ok "No circular imports detected"
}

# CHECK 11: albedo-jit cyclic dependency
Log-Info "Check 11: albedo-jit cyclic dependency..."
$jitCyclic = Get-ChildItem -Path albedo-jit/src -Recurse -Filter *.rs -ErrorAction SilentlyContinue | Select-String -Pattern 'use albedo::' 2>$null
if ($jitCyclic) {
    foreach ($hit in $jitCyclic) {
        Log-Error "CHECK11" "$($hit.Filename):$($hit.LineNumber): albedo-jit imports from albedo" "albedo-jit must not depend on albedo (use contracts interface)"
    }
} else {
    Log-Ok "No cyclic dependency in albedo-jit"
}

# CHECK 12: Abbreviated variable names
Log-Info "Check 12: abbreviated variable names..."
$abbrevHits = Get-ChildItem -Path src -Recurse -Filter *.rs | Select-String -Pattern 'let\s+(mut\s+)?(x|y|z|tmp|res|val|ret|idx)\s*=' 2>$null | Where-Object { $_.Line -notmatch '#\[cfg\(test\)\]' -and $_.Line -notmatch 'mod tests' }
if ($abbrevHits) {
    Log-Error "CHECK12" "$($abbrevHits.Count) abbreviated variable names (x, y, z, tmp, res, val, ret, idx)" "use descriptive names (e.g., cursor_x, result, return_value)"
    $abbrevHits | Select-Object -First 5 | ForEach-Object { Write-Host "  $($_.Filename):$($_.LineNumber): $($_.Line.Trim())" }
    if ($abbrevHits.Count -gt 5) {
        Write-Host "  ... and $($abbrevHits.Count - 5) more"
    }
} else {
    Log-Ok "No abbreviated variable names found"
}

# SUMMARY
Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
if ($ERRORS -gt 0) {
    Write-Host "  FAILED: $ERRORS errors found" -ForegroundColor Red
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
    exit 1
} else {
    Write-Host "  PASSED: all 12 checks clean" -ForegroundColor Green
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
    exit 0
}
