//! vynamp-dsp
//!
//! Primitivas de processamento de amostras: resampling e conversão de profundidade
//! com dither. Não conhece arquivo, container, dispositivo, formato de mídia nem
//! qualquer tipo de domínio — `specs/dsp.md` §2.
//!
//! As metas numéricas que este módulo persegue estão na
//! `adr/ADR-017-resampler-e-conversao.md` do umbrella, que é a fonte: se um número
//! mudar lá, muda aqui.

#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::module_name_repetitions)]

pub mod convert;
pub mod dither;
mod error;
mod kernel;
mod quality;
mod resampler;

pub use dither::Tpdf;
pub use error::Error;
pub use kernel::{Kernel, MAX_PHASES};
pub use quality::Quality;
pub use resampler::{Resampler, ResamplerConfig};

/// Versao do modulo. O contrato de release do umbrella fixa a tag.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
