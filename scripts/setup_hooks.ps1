# Script de configuração dos Git Hooks para Windows PowerShell
Write-Host "⚙️ Configurando Git Hooks para o Albedo Browser..." -ForegroundColor Cyan

git config core.hooksPath .githooks
if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ Git Hooks configurados com sucesso! Seu 'git commit' agora passará por verificações automáticas." -ForegroundColor Green
} else {
    Write-Host "❌ Falha ao configurar Git Hooks." -ForegroundColor Red
}
