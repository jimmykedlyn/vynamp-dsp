//! Os dois modos de qualidade — `adr/ADR-017-resampler-e-conversao.md` §2.
//!
//! Os números daqui são os da ADR. Se um mudar no código, muda lá também: a ADR é
//! a fonte, esta tabela é a sua expressão executável.

/// Modo de qualidade do resampler.
///
/// Os dois modos diferem **na transição e na profundidade**, não só no comprimento:
/// `High` preserva mais do Nyquist e aprofunda 30 dB, a 2,69× o custo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    /// Passband até 0,44·Nyquist, 90 dB de atenuação de projeto.
    Balanced,
    /// Passband até 0,47·Nyquist, 120 dB de atenuação de projeto.
    High,
}

impl Quality {
    /// Meia-janela em amostras, no par de taxas mais próximo da igualdade.
    ///
    /// Em decimação o kernel alarga por `1/r`, para manter a seletividade relativa
    /// à taxa de saída — e a latência constante em tempo.
    #[must_use]
    pub const fn half_base(self) -> usize {
        match self {
            Self::Balanced => 48,
            Self::High => 130,
        }
    }

    /// Frequência de corte em ciclos por amostra, relativa à menor taxa.
    ///
    /// O corte fica no **meio** da banda de transição, não em 0,5: com o corte em
    /// 0,5 a atenuação em 0,5 seria metade do projeto, e o alvo de stopband
    /// começa exatamente ali.
    #[must_use]
    pub const fn cutoff(self) -> f64 {
        match self {
            Self::Balanced => 0.47,
            Self::High => 0.485,
        }
    }

    /// Parâmetro β da janela de Kaiser.
    #[must_use]
    pub const fn beta(self) -> f64 {
        match self {
            Self::Balanced => 8.96,
            Self::High => 12.27,
        }
    }

    /// Atenuação de projeto em dB, para documentação e evidência.
    #[must_use]
    pub const fn design_attenuation_db(self) -> f64 {
        match self {
            Self::Balanced => 90.0,
            Self::High => 120.0,
        }
    }
}
