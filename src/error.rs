//! Erros do módulo. Erro é dado, não `panic!` — `specs/dsp.md` §6.

use core::fmt;

/// Falha de construção ou de chamada, devolvida ao chamador.
///
/// Nenhum destes casos interrompe o processo: a instância que devolveu um erro de
/// chamada continua utilizável no estado em que estava.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// `fs_in` ou `fs_out` é zero.
    ZeroSampleRate,
    /// Profundidade fora de `{16, 24, 32}`.
    InvalidDepth {
        /// Profundidade recebida.
        depth: u8,
    },
    /// A razão de taxas pediria mais fases do que o orçamento de memória permite.
    TooManyPhases {
        /// Fases que a razão exigiria (`fs_out / gcd`).
        phases: u64,
        /// Orçamento configurado.
        max: u64,
    },
    /// O bloco de entrada passa do `max_block_frames` declarado na construção.
    BlockTooLarge {
        /// Frames recebidos.
        requested: usize,
        /// Limite declarado.
        max: usize,
    },
    /// O buffer de saída é menor do que o necessário para o bloco recebido.
    OutputTooSmall {
        /// Frames necessários, por [`crate::Resampler::output_capacity_for`].
        need: usize,
        /// Frames disponíveis.
        have: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroSampleRate => f.write_str("taxa de amostragem zero"),
            Self::InvalidDepth { depth } => {
                write!(f, "profundidade {depth} inválida: só 16, 24 e 32")
            }
            Self::TooManyPhases { phases, max } => write!(
                f,
                "razão de taxas exige {phases} fases, o orçamento é {max}"
            ),
            Self::BlockTooLarge { requested, max } => {
                write!(f, "bloco de {requested} frames excede o máximo {max}")
            }
            Self::OutputTooSmall { need, have } => {
                write!(f, "buffer de saída precisa de {need} frames, tem {have}")
            }
        }
    }
}

impl core::error::Error for Error {}
