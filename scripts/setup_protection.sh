#!/usr/bin/env bash
# Script para blindar a branch 'main' usando o GitHub CLI (gh) com suporte a Merge Commit Padrão

echo "🛡️ Habilitando Merge Commit padrão nas configurações do repositório..."
gh api \
  -X PATCH \
  -H "Accept: application/vnd.github+json" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  /repos/Alekkzsx/Albedo-Browser \
  -f "allow_merge_commit=true" \
  -f "allow_squash_merge=true" \
  -f "allow_rebase_merge=true"

echo "🛡️ Configurando a Proteção Enterprise na branch 'main'..."

# Criando o payload JSON estrito (com linear history desativado para permitir Merge Commits)
cat << 'EOF' > payload.json
{
  "required_status_checks": {
    "strict": true,
    "contexts": [
      "🧹 Linter & Code Formatting",
      "🛡️ Security & Licensing Audit",
      "🏗️ Build & Test (ubuntu-latest)",
      "🏗️ Build & Test (windows-latest)",
      "🏗️ Build & Test (macos-latest)",
      "🧠 Enterprise Gemini PR Review"
    ]
  },
  "enforce_admins": true,
  "required_pull_request_reviews": {
    "dismiss_stale_reviews": true,
    "require_code_owner_reviews": true,
    "required_approving_review_count": 1
  },
  "restrictions": null,
  "required_linear_history": false,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "required_conversation_resolution": true
}
EOF

# Aplicando a regra na API do GitHub
gh api \
  -X PUT \
  -H "Accept: application/vnd.github+json" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  /repos/Alekkzsx/Albedo-Browser/branches/main/protection \
  --input payload.json

if [ $? -eq 0 ]; then
    echo "✅ Proteção e Merge Commit ativados com sucesso! A 'main' agora aceita Merge normal."
else
    echo "❌ Falha ao configurar proteção."
fi

rm -f payload.json
