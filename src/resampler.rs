//! Resampler por canal — `adr/ADR-017-resampler-e-conversao.md` §1 e §5.

use std::sync::Arc;

use crate::error::Error;
use crate::kernel::Kernel;
use crate::quality::Quality;

/// Configuração de uma instância.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResamplerConfig {
    /// Taxa de entrada, em Hz.
    pub fs_in: u32,
    /// Taxa de saída, em Hz.
    pub fs_out: u32,
    /// Modo de qualidade.
    pub quality: Quality,
    /// Maior bloco que [`Resampler::process`] vai receber, em frames.
    pub max_block_frames: usize,
}

/// Conversor de taxa com estado, uma instância por canal.
///
/// O kernel é compartilhado (`Arc`): um octeto aponta oito instâncias para a mesma
/// tabela, e nada no caminho quente aloca.
#[derive(Debug)]
pub struct Resampler {
    kernel: Arc<Kernel>,
    /// Amostras de zero à frente do sinal, para alinhar o índice de saída.
    pad: usize,
    /// Amostras aceitas até agora, contando o `pad`.
    written: usize,
    /// Capacidade do histórico circular.
    capacity: usize,
    /// Histórico de entrada, circular.
    history: Vec<f32>,
    /// Índice de saída que a próxima chamada vai produzir.
    next_out: u64,
    /// Maior bloco aceito por `process`.
    max_block: usize,
    bypass: bool,
}

impl Resampler {
    /// Constrói a instância e a tabela de fases.
    ///
    /// # Errors
    ///
    /// [`Error::ZeroSampleRate`] para taxa zero, [`Error::TooManyPhases`] quando a
    /// razão reduzida passa de [`crate::kernel::MAX_PHASES`].
    pub fn new(cfg: ResamplerConfig) -> Result<Self, Error> {
        let kernel = Kernel::new(cfg.fs_in, cfg.fs_out, cfg.quality)?;
        let bypass = cfg.fs_in == cfg.fs_out;
        let pad = if bypass { 0 } else { kernel.half };
        // O histórico precisa guardar a janela inteira mais o maior bloco: o bloco
        // é escrito de uma vez e depois produzido, então a janela antiga não pode
        // ser sobrescrita antes de ser lida.
        let capacity = if bypass {
            0
        } else {
            kernel.taps + cfg.max_block_frames.max(1)
        };
        Ok(Self {
            kernel,
            pad,
            // A antecipação é pré-carregada com zero: as primeiras `pad` posições do
            // histórico são o "antes" do sinal, e a entrada `i` mora em `pad + i`.
            // Começar em zero deslocaria tudo em `half` amostras.
            written: pad,
            capacity,
            history: vec![0.0; capacity],
            next_out: 0,
            max_block: cfg.max_block_frames.max(1),
            bypass,
        })
    }

    /// Atraso em frames de saída — `half_base / min(fs_in, fs_out)`, arredondado.
    ///
    /// Zero no bypass em taxa igual.
    #[must_use]
    pub fn latency_frames(&self) -> usize {
        if self.bypass { 0 } else { self.kernel.latency }
    }

    /// Coeficientes por fase.
    #[must_use]
    pub fn taps(&self) -> usize {
        if self.bypass { 1 } else { self.kernel.taps }
    }

    /// Frames de saída que um bloco de `frames` de entrada pode gerar.
    #[must_use]
    pub fn output_capacity_for(&self, frames: usize) -> usize {
        if self.bypass {
            return frames;
        }
        // Dois a mais cobrem a fronteira do bloco: o acumulador de fase inteiro
        // pode emitir uma amostra que só seria possível no bloco seguinte.
        (frames as u64 * self.kernel.b).div_ceil(self.kernel.a) as usize + 2
    }

    /// Converte um bloco de `f32` e devolve quantos frames escreveu em `out`.
    ///
    /// # Errors
    ///
    /// [`Error::BlockTooLarge`] e [`Error::OutputTooSmall`]. Depois de qualquer um
    /// deles a instância continua utilizável no estado em que estava.
    pub fn process(&mut self, input: &[f32], out: &mut [f32]) -> Result<usize, Error> {
        if self.bypass {
            if out.len() < input.len() {
                return Err(Error::OutputTooSmall {
                    need: input.len(),
                    have: out.len(),
                });
            }
            out[..input.len()].copy_from_slice(input);
            return Ok(input.len());
        }

        if input.len() > self.max_block {
            return Err(Error::BlockTooLarge {
                requested: input.len(),
                max: self.max_block,
            });
        }
        let need = self.output_capacity_for(input.len());
        if out.len() < need {
            return Err(Error::OutputTooSmall {
                need,
                have: out.len(),
            });
        }

        for &s in input {
            self.history[self.written % self.capacity] = s;
            self.written += 1;
        }
        Ok(self.produce(out))
    }

    /// Esvazia o atraso injetando `half` amostras de zero, e devolve a cauda.
    ///
    /// Sem isto, as últimas `half` amostras de entrada nunca alcançam a saída.
    ///
    /// # Errors
    ///
    /// [`Error::OutputTooSmall`].
    pub fn flush(&mut self, out: &mut [f32]) -> Result<usize, Error> {
        if self.bypass {
            return Ok(0);
        }
        let need = self.output_capacity_for(self.pad);
        if out.len() < need {
            return Err(Error::OutputTooSmall {
                need,
                have: out.len(),
            });
        }
        for _ in 0..self.pad {
            self.history[self.written % self.capacity] = 0.0;
            self.written += 1;
        }
        Ok(self.produce(out))
    }

    /// Quantas saídas têm a janela inteira disponível.
    ///
    /// A amostra de saída `m` usa as entradas `floor(m·a/b) + k` para `k` em
    /// `-half..=half`, então a maior entrada tocada é `floor(m·a/b) + half` e ela
    /// precisa existir: `floor(m·a/b) <= avail - 1 - half`. Com `usable = avail -
    /// half`, isso é `m·a/b < usable`, ou seja `m < usable·b/a`.
    fn ready_frames(&self) -> u64 {
        let avail = self.written.saturating_sub(self.pad);
        if avail <= self.kernel.half {
            return 0;
        }
        let usable = (avail - self.kernel.half) as u64;
        usable.saturating_mul(self.kernel.b).div_ceil(self.kernel.a)
    }

    fn produce(&mut self, out: &mut [f32]) -> usize {
        let ready = self.ready_frames();
        let a = self.kernel.a;
        let b = self.kernel.b;
        let mut count = 0usize;

        while self.next_out < ready && count < out.len() {
            let phase = (self.next_out * a % b) as usize;
            let base = (self.next_out * a / b) as usize;
            let coefs = self.kernel.phase(phase);
            // Com `pad == half`, a entrada `base + i - half` mora no índice
            // `base + i` do histórico: o zero de antecipação cancela o `-half`.
            let start = base;
            let mut acc = 0.0f32;
            for (i, &c) in coefs.iter().enumerate() {
                acc += self.history[(start + i) % self.capacity] * c;
            }
            out[count] = acc;
            count += 1;
            self.next_out += 1;
        }
        count
    }
}
