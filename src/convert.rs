//! PCM24/32 packing, redução de profundidade e dither
//! — `adr/ADR-017-resampler-e-conversao.md` §4.
//!
//! As amostras inteiras vivem em container `i32` com os bits válidos nos mais
//! significativos: a convenção do `WAVEFORMATEXTENSIBLE` quando
//! `wValidBitsPerSample < wBitsPerSample`. 24 bits ocupam `31..8`; 16 bits, `31..16`.

use crate::dither::Tpdf;
use crate::error::Error;

/// Deslocamento dos bits válidos a partir do topo do `i32`.
#[must_use]
pub const fn shift_for(depth: u8) -> Option<u32> {
    match depth {
        32 => Some(0),
        24 => Some(8),
        16 => Some(16),
        _ => None,
    }
}

/// Desempacota um valor de 24 bits do container.
#[must_use]
pub fn unpack_24(packed: i32) -> i32 {
    packed >> 8
}

/// Desempacota um valor de 16 bits do container.
#[must_use]
pub fn unpack_16(packed: i32) -> i32 {
    packed >> 16
}

/// Converte `src` de `src_depth` para `dst_depth`, escrevendo em `out`.
///
/// Regras, todas fixadas pela ADR-017 §4:
///
/// - **Mesma profundidade**: cópia bit a bit. Não há o que arredondar, e a fonte de
///   dither não é consultada — o estado do gerador não avança.
/// - **Profundidade maior**: os bits válidos são preservados e os baixos ficam
///   zero. Dither nunca, porque adicionar ruído ao subir de precisão não compra
///   resolução nenhuma.
/// - **Profundidade menor**: deslocamento à direita com arredondamento, mais TPDF
///   se o chamador fornecer a fonte.
///
/// # Errors
///
/// [`Error::InvalidDepth`] para profundidade fora de `{16, 24, 32}`, e
/// [`Error::OutputTooSmall`] se `out` for menor que `src`.
pub fn integer(
    src: &[i32],
    src_depth: u8,
    dst_depth: u8,
    dither: Option<&mut Tpdf>,
    out: &mut [i32],
) -> Result<(), Error> {
    let src_shift = shift_for(src_depth).ok_or(Error::InvalidDepth { depth: src_depth })?;
    let dst_shift = shift_for(dst_depth).ok_or(Error::InvalidDepth { depth: dst_depth })?;
    if out.len() < src.len() {
        return Err(Error::OutputTooSmall {
            need: src.len(),
            have: out.len(),
        });
    }

    if dst_shift == src_shift {
        out[..src.len()].copy_from_slice(src);
        return Ok(());
    }

    if dst_shift < src_shift {
        // Profundidade maior: os bits válidos já estão no topo, então preservar a
        // amplitude é limpar o que sobra embaixo.
        let mask = !0i32 << dst_shift;
        for (o, &s) in out.iter_mut().zip(src) {
            *o = s & mask;
        }
        return Ok(());
    }

    let shift = dst_shift - src_shift;
    let half = 1i64 << (shift - 1);
    let step = 1i64 << shift;
    // Limites da profundidade de destino, em unidades pós-deslocamento. Arredondar
    // o pico para cima daria `max + 1`, que deslocado de volta transborda o `i32` e
    // vira o extremo oposto — um clique de escala inteira, não um LSB de erro.
    let max = i64::from(i32::MAX >> dst_shift);
    let min = i64::from(i32::MIN >> dst_shift);
    match dither {
        // Sem fonte de dither: arredondamento simples para o mais próximo.
        None => {
            for (o, &s) in out.iter_mut().zip(src) {
                let v = ((i64::from(s) + half) >> shift).clamp(min, max);
                *o = (v << dst_shift) as i32;
            }
        }
        // Com fonte: a perturbação TPDF é de ±1 LSB **na profundidade de destino**,
        // então em unidades da origem ela vale ±(1 << shift).
        Some(source) => {
            for (o, &s) in out.iter_mut().zip(src) {
                let perturbation = i64::from(source.next()) * step;
                let v = ((i64::from(s) + perturbation + half) >> shift).clamp(min, max);
                *o = (v << dst_shift) as i32;
            }
        }
    }
    Ok(())
}
