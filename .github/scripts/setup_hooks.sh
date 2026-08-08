#!/usr/bin/env bash
# Script de configuração dos Git Hooks para Linux / macOS - Albedo Browser

set -e

echo "⚙️ Configurando Git Hooks para o Albedo Browser..."

# Garante permissão de execução nos hooks nativos
chmod +x .github/.githooks/* 2>/dev/null || true

# Caso o lefthook esteja instalado, usa o lefthook
if command -v lefthook >/dev/null 2>&1; then
    echo "⚡ Lefthook detectado! Instalando hooks via Lefthook..."
    lefthook install
    echo "✅ Hooks ativados via Lefthook!"
else
    echo "📌 Configurando hooks nativos do Git em .github/.githooks..."
    git config core.hooksPath .github/.githooks
    echo "✅ Git Hooks nativos configurados com sucesso!"
    echo "💡 Dica: Instale 'lefthook' ou 'prek' via cargo para ter checagens paralelas ultrarrápidas:"
    echo "   cargo install lefthook"
fi
