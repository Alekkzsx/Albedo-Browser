# Script de configuração dos Git Hooks para Windows PowerShell - Albedo Browser
Write-Host "⚙️ Configurando Git Hooks para o Albedo Browser..." -ForegroundColor Cyan

if (Get-Command lefthook -ErrorAction SilentlyContinue) {
    Write-Host "⚡ Lefthook detectado! Instalando hooks via Lefthook..." -ForegroundColor Yellow
    lefthook install
    Write-Host "✅ Hooks ativados via Lefthook com sucesso!" -ForegroundColor Green
} else {
    git config core.hooksPath .githooks
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ Git Hooks nativos configurados com sucesso! Seu 'git commit' passará por verificações automáticas." -ForegroundColor Green
        Write-Host "💡 Dica: Instale o 'lefthook' via cargo para checagens mais rápidas: cargo install lefthook" -ForegroundColor Gray
    } else {
        Write-Host "❌ Falha ao configurar Git Hooks." -ForegroundColor Red
    }
}
