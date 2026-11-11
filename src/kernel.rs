//! Tabela de coeficientes por fase racional — `adr/ADR-017-resampler-e-conversao.md` §1.
//!
//! A posição da amostra de saída `m` é `m·a/b`, exata, com `a = fs_in/g` e
//! `b = fs_out/g`. Como `gcd(a,b) = 1`, o resto `(m·a) mod b` percorre todas as
//! `b` fases, então ele é o índice da linha da tabela: não há interpolação de
//! coeficiente nem erro de quantização de fase.

use std::sync::Arc;

use crate::error::Error;
use crate::quality::Quality;

/// Orçamento de fases. Acima disso a memória da tabela cresce sem limite útil, e a
/// construção devolve erro em vez de alocar.
pub const MAX_PHASES: u64 = 4096;

/// Kernel compartilhável: uma tabela por configuração, `Arc` entre os canais.
///
/// Um octeto carregaria oito cópias da mesma tabela sem isso.
#[derive(Debug)]
pub struct Kernel {
    /// Numerador da razão reduzida (`fs_in / gcd`).
    pub(crate) a: u64,
    /// Denominador da razão reduzida (`fs_out / gcd`), e o número de fases.
    pub(crate) b: u64,
    /// Coeficientes por fase.
    pub(crate) taps: usize,
    /// Meia-janela efetiva, em amostras de entrada.
    pub(crate) half: usize,
    /// Atraso em frames de saída, arredondado.
    pub(crate) latency: usize,
    /// `b` linhas de `taps` coeficientes.
    pub(crate) phases: Vec<f32>,
}

impl Kernel {
    /// Constrói a tabela para o par de taxas e o modo.
    ///
    /// # Errors
    ///
    /// [`Error::ZeroSampleRate`] se uma das taxas for zero, e
    /// [`Error::TooManyPhases`] se a razão reduzida pedir mais de [`MAX_PHASES`].
    #[allow(clippy::many_single_char_names)] // a, b, g, p, i, k espelham a ADR
    pub fn new(fs_in: u32, fs_out: u32, quality: Quality) -> Result<Arc<Self>, Error> {
        if fs_in == 0 || fs_out == 0 {
            return Err(Error::ZeroSampleRate);
        }
        let g = gcd(u64::from(fs_in), u64::from(fs_out));
        let a = u64::from(fs_in) / g;
        let b = u64::from(fs_out) / g;
        if b > MAX_PHASES {
            return Err(Error::TooManyPhases {
                phases: b,
                max: MAX_PHASES,
            });
        }

        let half_base = quality.half_base();
        // Alargamento por `1/r` em aritmética inteira: manter a seletividade
        // relativa à taxa de saída exige a janela em amostras de *entrada*, e
        // `ceil` em ponto flutuante decidiria 283 contra 284 por um bit.
        let half = if fs_out < fs_in {
            let num = half_base as u64 * u64::from(fs_in);
            let den = u64::from(fs_out);
            num.div_ceil(den) as usize
        } else {
            half_base
        };
        let taps = 2 * half + 1;

        // Corte relativo à **menor** taxa: em decimação isso o leva para dentro da
        // banda de entrada, que é o que impede o alias.
        let ratio = f64::from(fs_out) / f64::from(fs_in);
        let fc = quality.cutoff() * ratio.min(1.0);
        let beta = quality.beta();
        let win_norm = bessel_i0(beta);

        let mut phases = vec![0.0f32; b as usize * taps];
        for p in 0..b as usize {
            let frac = p as f64 / b as f64;
            let row = &mut phases[p * taps..(p + 1) * taps];
            for (i, slot) in row.iter_mut().enumerate() {
                let k = i as f64 - half as f64;
                let u = frac - k;
                let w =
                    bessel_i0(beta * (1.0 - (u / half as f64).powi(2)).max(0.0).sqrt()) / win_norm;
                *slot = (lowpass(fc, u) * w) as f32;
            }
            // Normalização por fase: ganho DC exatamente unitário em toda posição.
            // O resíduo do arredondamento para f32 é absorvido pelo tap central,
            // senão a soma da linha fica em 1 ± taps·eps e o DC desvia em ~1e-5.
            let sum: f64 = row.iter().map(|&c| f64::from(c)).sum();
            let residual = 1.0 - sum;
            row[half] = (f64::from(row[half]) + residual) as f32;
        }

        let latency = ((half as u64 * b + a / 2) / a) as usize;

        Ok(Arc::new(Self {
            a,
            b,
            taps,
            half,
            latency,
            phases,
        }))
    }

    /// Coeficientes de uma fase.
    #[must_use]
    pub fn phase(&self, p: usize) -> &[f32] {
        &self.phases[p * self.taps..(p + 1) * self.taps]
    }
}

fn gcd(mut x: u64, mut y: u64) -> u64 {
    while y != 0 {
        let t = x % y;
        x = y;
        y = t;
    }
    x
}

/// `sin(2π·fc·u)/(π·u)`, com o limite `2·fc` em `u = 0`.
fn lowpass(fc: f64, u: f64) -> f64 {
    if u.abs() < 1e-12 {
        2.0 * fc
    } else {
        (2.0 * std::f64::consts::PI * fc * u).sin() / (std::f64::consts::PI * u)
    }
}

/// Bessel modificada de primeira espécie, ordem zero — série de potências.
fn bessel_i0(x: f64) -> f64 {
    let y = x * x / 4.0;
    let mut sum = 1.0f64;
    let mut term = 1.0f64;
    for k in 1..80 {
        let kf = f64::from(k);
        term *= y / (kf * kf);
        sum += term;
        if term < sum * 1e-17 {
            break;
        }
    }
    sum
}
