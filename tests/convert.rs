//! Aceite da TASK-E04-02-AC-2: PCM24/32 packing, bypass exato e dither só ao
//! reduzir precisão.
//!
//! As amostras inteiras vivem em container `i32` com os bits válidos nos mais
//! significativos — a convenção do `WAVEFORMATEXTENSIBLE` quando
//! `wValidBitsPerSample < wBitsPerSample`.

use vynamp_dsp::{Error, Tpdf, convert};

/// Rampa em que o frame `f` carrega `start + step*f`, o mesmo desenho de oráculo
/// que a evidência da TASK-E01-02 registrou. Aqui é construída localmente porque a
/// §2 da spec proíbe importar qualquer crate `vynamp-*`.
fn ramp(start: i32, step: i32, n: usize) -> Vec<i32> {
    (0..n as i64)
        .map(|f| (start as i64 + step as i64 * f).clamp(i32::MIN as i64, i32::MAX as i64) as i32)
        .collect()
}

// ------------------------------------------------------- packing e bypass

#[test]
fn same_depth_is_bit_exact_and_does_not_touch_the_dither() {
    for depth in [16u8, 24, 32] {
        let src = ramp(-1 << 20, 7919, 512);
        let mut dither = Tpdf::new(0x5EED);
        let before = dither.state();

        let mut with_dither = vec![0i32; src.len()];
        let mut plain = vec![0i32; src.len()];
        convert::integer(&src, depth, depth, Some(&mut dither), &mut with_dither).unwrap();
        convert::integer(&src, depth, depth, None, &mut plain).unwrap();

        for (i, (a, b)) in with_dither.iter().zip(plain.iter()).enumerate() {
            assert_eq!(a, b, "depth {depth}: amostra {i} divergiu");
        }
        assert_eq!(
            dither.state(),
            before,
            "depth {depth}: o gerador avançou sem haver precisão a reduzir"
        );
    }
}

#[test]
fn widening_zeroes_the_low_bits_and_never_dithers() {
    // 16 bits -> 24: o valor é reinterpretado, e os 8 bits baixos ficam zero.
    let src = ramp(-30_000 << 16, 37 << 16, 256);
    let mut dither = Tpdf::new(0xA11CE);
    let before = dither.state();
    let mut out = vec![0i32; src.len()];
    convert::integer(&src, 16, 24, Some(&mut dither), &mut out).unwrap();

    for (i, (s, o)) in src.iter().zip(out.iter()).enumerate() {
        assert_eq!(o & 0xFF, 0, "amostra {i}: bits baixos deveriam ser zero");
        assert_eq!(
            o & !0xFF,
            *s,
            "amostra {i}: os bits válidos têm de sobreviver"
        );
    }
    assert_eq!(
        dither.state(),
        before,
        "subir de precisão não consome dither"
    );
}

#[test]
fn reducing_depth_dithers_only_when_asked() {
    // 24 -> 16, com o dither pedido.
    let src = ramp(-1 << 22, 12345, 4_096);
    let mut a = Tpdf::new(0x1234);
    let mut b = Tpdf::new(0x1234);
    let mut d1 = vec![0i32; src.len()];
    let mut d2 = vec![0i32; src.len()];
    convert::integer(&src, 24, 16, Some(&mut a), &mut d1).unwrap();
    convert::integer(&src, 24, 16, Some(&mut b), &mut d2).unwrap();

    // Determinístico: mesma semente, mesma saída.
    assert_eq!(d1, d2, "o dither não é reprodutível");

    // Sem dither o resultado é o arredondamento simples, e difere em algum ponto.
    let mut plain = vec![0i32; src.len()];
    convert::integer(&src, 24, 16, None, &mut plain).unwrap();
    assert_ne!(d1, plain, "o dither não teve efeito nenhum");

    // A diferença fica dentro de ±1 LSB do arredondamento, e os 16 bits baixos
    // são zero: o container é i32 com os bits válidos no topo.
    for (i, (with, no)) in d1.iter().zip(plain.iter()).enumerate() {
        assert_eq!(with & 0xFFFF, 0, "amostra {i}: bits baixos não são zero");
        let diff = (*with as i64 - *no as i64).abs() >> 16;
        assert!(diff <= 1, "amostra {i}: desvio {diff} LSB, esperado ≤ 1");
    }

    // Média zero: a soma do erro em LSB é desprezível frente ao número de amostras.
    let sum: i64 = d1
        .iter()
        .zip(plain.iter())
        .map(|(w, n)| (*w as i64 - *n as i64) >> 16)
        .sum();
    let mean = sum as f64 / d1.len() as f64;
    assert!(mean.abs() < 0.05, "média do dither {mean}, esperado ~0");
}

#[test]
fn dither_distribution_is_tpdf() {
    // {−1, 0, +1} com probabilidades ¼, ½, ¼.
    let mut dither = Tpdf::new(0xC0FFEE);
    let n = 400_000usize;
    let mut counts = [0usize; 3];
    for _ in 0..n {
        let v = dither.next();
        match v {
            -1 => counts[0] += 1,
            0 => counts[1] += 1,
            1 => counts[2] += 1,
            other => panic!("TPDF em LSB só pode dar -1, 0 ou 1; veio {other}"),
        }
    }
    let p: Vec<f64> = counts.iter().map(|&c| c as f64 / n as f64).collect();
    assert!((p[0] - 0.25).abs() < 0.005, "P(-1) = {}", p[0]);
    assert!((p[1] - 0.50).abs() < 0.005, "P(0)  = {}", p[1]);
    assert!((p[2] - 0.25).abs() < 0.005, "P(+1) = {}", p[2]);
}

#[test]
fn packing_24bit_occupies_the_top_of_the_container() {
    // Um valor de 24 bits, empacotado, tem os 8 bits baixos livres.
    let case = [
        (0i32, 0i32),
        (1, 1 << 8),
        (-1, -1 << 8),
        (8_388_607, 8_388_607 << 8),   // máximo positivo de 24 bits
        (-8_388_608, -8_388_608 << 8), // mínimo de 24 bits
    ];
    for (sample, packed) in case {
        let mut out = vec![0i32; 1];
        let src = [sample << 8];
        convert::integer(&src, 24, 24, None, &mut out).unwrap();
        assert_eq!(out[0], packed, "packing de {sample} em 24 bits");
        // E desempacotar devolve o original.
        assert_eq!(convert::unpack_24(packed), sample, "unpack de {packed}");
    }
}

#[test]
fn packing_16bit_occupies_the_top_of_the_container() {
    for sample in [0i32, 1, -1, 32_767, -32_768] {
        let src = [sample << 16];
        let mut out = vec![0i32; 1];
        convert::integer(&src, 16, 16, None, &mut out).unwrap();
        assert_eq!(out[0], sample << 16);
        assert_eq!(convert::unpack_16(out[0]), sample);
    }
}

#[test]
fn invalid_depth_is_an_error_not_a_panic() {
    let src = [0i32; 4];
    let mut out = [0i32; 4];
    for depth in [0u8, 1, 15, 17, 25, 31, 33, 64] {
        assert_eq!(
            convert::integer(&src, depth, 24, None, &mut out).unwrap_err(),
            Error::InvalidDepth { depth },
            "depth {depth} deveria ser recusada"
        );
    }
    assert_eq!(
        convert::integer(&src, 24, 24, None, &mut out[..2]).unwrap_err(),
        Error::OutputTooSmall { need: 4, have: 2 }
    );
}

#[test]
fn full_scale_narrows_without_wrapping_to_the_opposite_sign() {
    // O defeito que este teste existe para impedir: `(2^31−1 + 2^15) >> 16` dá
    // 32768, um a mais que o máximo de 16 bits, e `32768 << 16` transborda para
    // −2^31 — o pico positivo viraria o pico negativo, um clique de escala inteira.
    for depth in [16u8, 24] {
        let shift = convert::shift_for(depth).unwrap();
        let top = i32::MAX >> shift;

        let wide = [i32::MAX, i32::MIN, 0i32];
        let mut narrowed = vec![0i32; 3];
        convert::integer(&wide, 32, depth, None, &mut narrowed).unwrap();

        assert!(
            narrowed[0] > 0,
            "depth {depth}: o pico positivo trocou de sinal"
        );
        assert_eq!(
            narrowed[0],
            top << shift,
            "depth {depth}: saturação positiva"
        );
        assert_eq!(narrowed[1], i32::MIN, "depth {depth}: −1,0 é exato");
        assert_eq!(narrowed[2], 0);

        // E o pico negativo nunca vira positivo — o mesmo defeito no outro extremo.
        let mut neg = vec![0i32; 1];
        convert::integer(&[i32::MIN], 32, depth, None, &mut neg).unwrap();
        assert!(neg[0] < 0, "depth {depth}: o pico negativo trocou de sinal");

        // Mesma profundidade continua bit a bit, inclusive nos extremos.
        let mut same = vec![0i32; 3];
        convert::integer(&wide, depth, depth, None, &mut same).unwrap();
        assert_eq!(
            same, wide,
            "depth {depth}: mesma profundidade tem de copiar"
        );
    }
}

#[test]
fn narrowing_is_monotonic_across_the_whole_range() {
    // Sem histerese: um sinal crescente não pode ter um ponto onde a saída cai.
    // Isso pega saturação mal colocada e sinal invertido em qualquer profundidade.
    for (src_depth, dst_depth) in [(32u8, 16u8), (24, 16), (32, 24)] {
        let src: Vec<i32> = (0..2_000).map(|i| (i - 1_000) * 1_048_576).collect();
        let mut out = vec![0i32; src.len()];
        convert::integer(&src, src_depth, dst_depth, None, &mut out).unwrap();
        for w in out.windows(2) {
            assert!(
                w[0] <= w[1],
                "{src_depth}->{dst_depth}: a saída não é monótona em {:?}",
                w
            );
        }
    }
}
