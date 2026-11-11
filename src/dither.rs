//! Dither TPDF — `adr/ADR-017-resampler-e-conversao.md` §4.
//!
//! Triangular, pico a pico 2 LSB, média zero, e determinístico: um dither que não
//! se reproduz não se testa.

/// Gerador TPDF com semente do chamador.
///
/// O estado é público pela leitura de [`Tpdf::state`], para que um teste possa
/// fixar que a conversão **não** consome dither quando não há precisão a reduzir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tpdf {
    state: u64,
}

impl Tpdf {
    /// Cria o gerador com a semente dada. A mesma semente reproduz a mesma série.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        // A semente zero degeneraria o xorshift; qualquer outra serve.
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    /// Estado atual, para comparação em teste.
    #[must_use]
    pub const fn state(&self) -> u64 {
        self.state
    }

    /// Próxima amostra, em `{−1, 0, +1}`.
    ///
    /// É a diferença de duas uniformes em `{0, 1}`, o que dá ¼, ½, ¼ — triangular,
    /// média zero, e em aritmética inteira.
    #[must_use]
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> i32 {
        let a = self.bit();
        let b = self.bit();
        a - b
    }

    /// Um bit do xorshift64*, escolhido em `{0, 1}`.
    fn bit(&mut self) -> i32 {
        // xorshift64*
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        let out = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        // O bit menos significativo da saída mistura todos os bits do estado.
        (out & 1) as i32
    }
}
