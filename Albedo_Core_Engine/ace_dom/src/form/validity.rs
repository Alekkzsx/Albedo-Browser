//! # Estado de Validade de Controles de Formulário (WHATWG HTML §4.10.21.2)
//!
//! Representa as 10 causas normativas de invalidade de um controle em relação a restrições.

use smol_str::SmolStr;

/// Objeto `ValidityState` normativo contendo os indicadores de conformidade de restrições.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ValidityState {
    /// O campo é obrigatório (`required`) mas não possui valor preenchido.
    pub value_missing: bool,
    /// O valor não corresponde à sintaxe esperada para o tipo (ex: `type="email"` inválido).
    pub type_mismatch: bool,
    /// O valor não coincide com a expressão regular definida no atributo `pattern`.
    pub pattern_mismatch: bool,
    /// O tamanho do valor excede o limite máximo permitido por `maxlength`.
    pub too_long: bool,
    /// O tamanho do valor é inferior ao limite mínimo exigido por `minlength`.
    pub too_short: bool,
    /// O valor numérico/temporal é inferior ao mínimo permitido por `min`.
    pub range_underflow: bool,
    /// O valor numérico/temporal é superior ao máximo permitido por `max`.
    pub range_overflow: bool,
    /// O valor numérico/temporal não se ajusta ao múltiplo do passo configurado em `step`.
    pub step_mismatch: bool,
    /// O valor fornecido pelo usuário não pôde ser convertido pelo navegador.
    pub bad_input: bool,
    /// Foi definida uma mensagem de erro customizada via `setCustomValidity()`.
    pub custom_error: bool,
    /// Mensagem customizada de validação ativa.
    pub custom_message: Option<SmolStr>,
}

impl ValidityState {
    /// Retorna `true` se nenhuma das condições de erro estiver ativa (o campo é plenamente válido).
    #[inline]
    pub fn valid(&self) -> bool {
        !self.value_missing
            && !self.type_mismatch
            && !self.pattern_mismatch
            && !self.too_long
            && !self.too_short
            && !self.range_underflow
            && !self.range_overflow
            && !self.step_mismatch
            && !self.bad_input
            && !self.custom_error
    }

    /// Define ou limpa a mensagem de erro customizada (`setCustomValidity`).
    pub fn set_custom_validity(&mut self, message: &str) {
        if message.is_empty() {
            self.custom_error = false;
            self.custom_message = None;
        } else {
            self.custom_error = true;
            self.custom_message = Some(SmolStr::new(message));
        }
    }

    /// Retorna a mensagem de erro apropriada para o estado atual.
    pub fn validation_message(&self) -> SmolStr {
        if let Some(ref custom) = self.custom_message {
            return custom.clone();
        }

        if self.value_missing {
            SmolStr::new("Por favor, preencha este campo obrigatório.")
        } else if self.type_mismatch {
            SmolStr::new("Por favor, insira um valor no formato correto.")
        } else if self.pattern_mismatch {
            SmolStr::new("O valor inserido não corresponde ao padrão solicitado.")
        } else if self.too_long {
            SmolStr::new("O texto inserido ultrapassa o limite máximo de caracteres.")
        } else if self.too_short {
            SmolStr::new("O texto inserido é menor que o tamanho mínimo exigido.")
        } else if self.range_underflow {
            SmolStr::new("O valor deve ser maior ou igual ao limite mínimo.")
        } else if self.range_overflow {
            SmolStr::new("O valor deve ser menor ou igual ao limite máximo.")
        } else if self.step_mismatch {
            SmolStr::new("O valor deve seguir o incremento de passo válido.")
        } else if self.bad_input {
            SmolStr::new("Entrada inválida.")
        } else {
            SmolStr::default()
        }
    }
}
