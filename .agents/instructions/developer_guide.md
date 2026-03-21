# Guia do Desenvolvedor de Elite (AlbedoBrowser)

Este guia acompanha as regras do agente AI para garantir consistência no projeto.

## Diretrizes de Código
- **Rust**: Use `clippy` agressivamente. Deny `warnings` na medida do possível.
- **UI**: Mantenha a estética premium e as micro-animações do Slint.
- **Segurança**: O sandbox é a parte mais importante. Audite cada entrada de JS para Rust.

## Fluxos de Trabalho
Use os workflows em `.agents/workflows/` para manter o padrão de maestria.
- `feature_mastery.md`: Para novas funcionalidades.
