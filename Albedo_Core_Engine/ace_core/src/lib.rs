/// Adiciona dois inteiros de 64 bits sem sinal utilizando adição saturada para prevenir overflow.
pub fn add(left: u64, right: u64) -> u64 {
    left.saturating_add(right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
