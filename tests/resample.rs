//! Aceite da TASK-E04-02-AC-1: impulso, sweep, alias, latência e CPU.
//!
//! Os alvos vêm da `adr/ADR-017-resampler-e-conversao.md` do umbrella. Enquanto
//! `specs/dsp.md` §5 estiver vazia, é a ADR que fixa o número que estes testes
//! comparam — sem ela o teste não teria contra o que reprovar.

use std::f64::consts::PI;

use vynamp_dsp::{Error, Quality, Resampler, ResamplerConfig};

fn config(fs_in: u32, fs_out: u32, quality: Quality) -> ResamplerConfig {
    ResamplerConfig {
        fs_in,
        fs_out,
        quality,
        max_block_frames: 100_000,
    }
}

/// Roda o sinal inteiro em blocos, depois drena o atraso com `flush`.
fn run_all(input: &[f32], cfg: ResamplerConfig, block: usize) -> Vec<f32> {
    let mut resampler = Resampler::new(cfg).expect("construção");
    let mut out = Vec::new();
    let mut buf = vec![0.0f32; resampler.output_capacity_for(block).max(64)];
    let mut pos = 0;
    while pos < input.len() {
        let end = (pos + block).min(input.len());
        let n = resampler
            .process(&input[pos..end], &mut buf)
            .expect("process");
        out.extend_from_slice(&buf[..n]);
        pos = end;
    }
    let tail = resampler.taps().div_ceil(2);
    let mut fbuf = vec![0.0f32; resampler.output_capacity_for(tail).max(64)];
    let n = resampler.flush(&mut fbuf).expect("flush");
    out.extend_from_slice(&fbuf[..n]);
    out
}

/// Amplitude de um tom, por Goertzel, no bin exato `freq * len / fs`.
fn goertzel(samples: &[f32], freq: f64, fs: f64) -> f64 {
    let n = samples.len();
    let k = freq * n as f64 / fs;
    let w = 2.0 * PI * k / n as f64;
    let (cw, sw) = (w.cos(), w.sin());
    let coeff = 2.0 * cw;
    let (mut s1, mut s2) = (0.0f64, 0.0f64);
    for &v in samples {
        let s0 = v as f64 + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    let re = s1 - s2 * cw;
    let im = s2 * sw;
    (re * re + im * im).sqrt() * 2.0 / n as f64
}

/// Meia-janela derivada dos taps publicados.
fn half_taps(resampler_taps: usize) -> usize {
    (resampler_taps - 1) / 2
}

// ---------------------------------------------------------------- bypass

#[test]
fn bypass_at_equal_rate_is_bit_exact() {
    for quality in [Quality::Balanced, Quality::High] {
        let cfg = config(48_000, 48_000, quality);
        let mut resampler = Resampler::new(cfg).unwrap();
        assert_eq!(
            resampler.latency_frames(),
            0,
            "{quality:?}: bypass não tem atraso"
        );

        let input: Vec<f32> = (0..1024)
            .map(|i| ((i as f32) * 0.37).sin() * 0.8 - 0.05)
            .collect();
        let mut out = vec![0.0f32; 2048];
        let n = resampler.process(&input, &mut out).unwrap();
        assert_eq!(n, input.len());
        // Comparação por bits, não por epsilon: filtrar em taxa igual atenuaria o
        // topo do espectro sem que ninguém tivesse pedido conversão.
        for (i, (a, b)) in input.iter().zip(out.iter()).enumerate() {
            assert_eq!(
                a.to_bits(),
                b.to_bits(),
                "{quality:?}: amostra {i} divergiu no bypass"
            );
        }
    }
}

// ---------------------------------------------------------------- ganho DC

#[test]
fn constant_input_keeps_dc_gain() {
    for (fs_in, fs_out) in [(44_100u32, 48_000u32), (96_000, 44_100), (44_100, 96_000)] {
        for quality in [Quality::Balanced, Quality::High] {
            let cfg = config(fs_in, fs_out, quality);
            let input = vec![0.25f32; 20_000];
            let out = run_all(&input, cfg, 20_000);
            let half = 600;
            assert!(
                out.len() > 2 * half,
                "{fs_in}->{fs_out}: saída curta demais"
            );
            // Fora das bordas o ganho tem de ser unitário: o kernel é normalizado
            // por fase, então a soma dos coeficientes é 1 em toda posição.
            for &v in &out[half..out.len() - half] {
                assert!(
                    (v - 0.25).abs() < 4.0 * f32::EPSILON,
                    "{fs_in}->{fs_out} {quality:?}: ganho DC {v}, esperado 0.25"
                );
            }
        }
    }
}

// ---------------------------------------------------------------- impulso

#[test]
fn impulse_peak_aligns_with_input_position() {
    // O kernel é simétrico e centrado na posição de saída: a amostra de saída `m`
    // corresponde à posição `m·a/b` da entrada, então o pico do impulso cai perto
    // de `n0·b/a` — não deslocado pela meia-janela.
    let pairs = [
        (44_100u32, 48_000u32),
        (96_000, 44_100),
        (44_100, 96_000),
        (48_000, 192_000),
    ];
    for (fs_in, fs_out) in pairs {
        for quality in [Quality::Balanced, Quality::High] {
            let cfg = config(fs_in, fs_out, quality);
            let resampler = Resampler::new(cfg).unwrap();
            let half = half_taps(resampler.taps());

            let n0 = 4_000usize;
            let len = n0 + 4 * half + 100;
            let mut input = vec![0.0f32; len];
            input[n0] = 1.0;

            let out = run_all(&input, cfg, len);
            let (peak_idx, peak_val) = out
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap())
                .unwrap();
            // Na decimação a janela é larga em amostras de entrada, então mesmo a
            // saída mais próxima do impulso é uma amostra do lóbulo, não do topo —
            // 96 000 → 44 100 dá ~0,29. O piso tem de refletir isso ou o teste
            // reprova um filtro correto.
            let floor = if fs_out < fs_in { 0.15 } else { 0.5 };
            assert!(
                peak_val.abs() > floor,
                "{fs_in}->{fs_out}: pico {peak_val} abaixo do piso {floor}"
            );

            let g = gcd(fs_in as u64, fs_out as u64);
            let (a, b) = (fs_in as u64 / g, fs_out as u64 / g);
            let ideal = n0 as f64 * b as f64 / a as f64;
            assert!(
                (peak_idx as f64 - ideal).abs() <= 1.5,
                "{fs_in}->{fs_out} {quality:?}: pico em {peak_idx}, ideal {ideal:.2}"
            );
        }
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

#[test]
fn latency_frames_match_the_adr_table() {
    // Números da tabela da ADR-017 §2. Se um mudar no código, o outro muda aqui.
    let expected = [
        (44_100u32, 48_000u32, Quality::High, 261usize, 141usize),
        (48_000, 44_100, Quality::High, 285, 130),
        (44_100, 96_000, Quality::High, 261, 283),
        (96_000, 44_100, Quality::High, 567, 130),
        (192_000, 44_100, Quality::High, 1_133, 130),
        (48_000, 192_000, Quality::High, 261, 520),
        (44_100, 384_000, Quality::High, 261, 1_132),
        (384_000, 44_100, Quality::High, 2_265, 130),
        (44_100, 48_000, Quality::Balanced, 97, 52),
    ];
    for (fs_in, fs_out, quality, taps, latency) in expected {
        let resampler = Resampler::new(config(fs_in, fs_out, quality)).unwrap();
        assert_eq!(
            resampler.taps(),
            taps,
            "{fs_in}->{fs_out} {quality:?}: taps"
        );
        assert_eq!(
            resampler.latency_frames(),
            latency,
            "{fs_in}->{fs_out} {quality:?}: latência em frames de saída"
        );
    }
}

#[test]
fn latency_is_what_the_stream_actually_holds_back() {
    // A latência publicada tem de ser o atraso observável: alimentando `N`
    // amostras, a saída produzida é exatamente a dos índices `m` cuja janela já
    // fechou — `floor(m·a/b) + half <= N-1`.
    for (fs_in, fs_out) in [(44_100u32, 48_000u32), (96_000, 44_100), (48_000, 192_000)] {
        let cfg = config(fs_in, fs_out, Quality::High);
        let mut resampler = Resampler::new(cfg).unwrap();
        let half = half_taps(resampler.taps());
        let g = gcd(fs_in as u64, fs_out as u64);
        let (a, b) = (fs_in as u64 / g, fs_out as u64 / g);

        let n = 5_000usize;
        let input = vec![0.0f32; n];
        let mut out = vec![0.0f32; resampler.output_capacity_for(n)];
        let produced = resampler.process(&input, &mut out).unwrap();

        let mut expected = 0usize;
        loop {
            let pos = expected as u64 * a / b;
            if pos + half as u64 > n as u64 - 1 {
                break;
            }
            expected += 1;
        }
        assert_eq!(
            produced, expected,
            "{fs_in}->{fs_out}: produzidas {produced}, janelas fechadas {expected}"
        );

        let pending = (n as u64 * b / a) as usize - produced;
        assert!(
            pending.abs_diff(resampler.latency_frames()) <= 1,
            "{fs_in}->{fs_out}: pendente {pending} vs latência publicada {}",
            resampler.latency_frames()
        );
    }
}

// ---------------------------------------------------------------- estado

#[test]
fn block_splitting_matches_single_block() {
    // Um resampler que reinicia o filtro a cada bloco falha aqui na primeira
    // amostra da segunda janela.
    let cfg = config(44_100, 48_000, Quality::High);
    let input: Vec<f32> = (0..30_000)
        .map(|i| {
            let t = i as f64 / 44_100.0;
            (0.5 * (2.0 * PI * 997.0 * t).sin() + 0.3 * (2.0 * PI * 63.0 * t).sin()) as f32
        })
        .collect();

    let one = run_all(&input, cfg, input.len());
    let many = run_all(&input, cfg, 7);

    assert_eq!(one.len(), many.len(), "contagem de frames divergiu");
    for (i, (x, y)) in one.iter().zip(many.iter()).enumerate() {
        assert_eq!(
            x.to_bits(),
            y.to_bits(),
            "amostra {i} divergiu entre bloco único e blocos de 7"
        );
    }
}

#[test]
fn frame_count_follows_the_rate_ratio() {
    // 48k -> 96k dobra, 48k -> 192k quadruplica; 44.1k -> 48k é não inteira.
    for (fs_in, fs_out, factor) in [
        (48_000u32, 96_000u32, 2.0f64),
        (48_000, 192_000, 4.0),
        (44_100, 48_000, 48_000.0 / 44_100.0),
    ] {
        let cfg = config(fs_in, fs_out, Quality::Balanced);
        let input = vec![0.1f32; 9_600];
        let out = run_all(&input, cfg, 1_024);
        // `flush` fecha a cauda; a contagem total é a razão de taxas.
        let expected = (out.len() as f64) / (9_600.0 * factor);
        assert!(
            (expected - 1.0).abs() < 0.02,
            "{fs_in}->{fs_out}: {} frames para {factor:.4}x de 9600",
            out.len()
        );
    }
}

// ---------------------------------------------------------------- alias

#[test]
fn alias_below_the_target_passes_and_above_fails() {
    // 96k -> 44.1k. Um tom de 30 kHz está acima do Nyquist de saída (22.05 kHz)
    // e dobra para 44 100 - 30 000 = 14 100 Hz. A energia lá é o alias.
    let fs_in = 96_000u32;
    let fs_out = 44_100u32;
    let alias_freq = 14_100.0f64;
    let ref_freq = 1_000.0f64;

    // Análise em bins exatos: 46 080 amostras a 44.1 kHz -> 25/24 Hz por bin, e
    // 1 000 e 14 100 são múltiplos inteiros disso.
    let analyze = 46_080usize;
    let skip = 2_000usize;
    let total_in = 110_000usize;

    for (quality, target_db) in [(Quality::Balanced, 80.0f64), (Quality::High, 110.0)] {
        let tone = |freq: f64| -> Vec<f32> {
            (0..total_in)
                .map(|n| (0.5 * (2.0 * PI * freq * n as f64 / fs_in as f64).sin()) as f32)
                .collect()
        };

        let cfg = config(fs_in, fs_out, quality);
        let pass = run_all(&tone(ref_freq), cfg, 4_096);
        let stop = run_all(&tone(30_000.0), cfg, 4_096);
        assert!(pass.len() > skip + analyze, "{quality:?}: saída curta");

        let ref_amp = goertzel(&pass[skip..skip + analyze], ref_freq, fs_out as f64);
        let alias_amp = goertzel(&stop[skip..skip + analyze], alias_freq, fs_out as f64);
        let db = 20.0 * (alias_amp / ref_amp).log10();

        assert!(
            db <= -target_db,
            "{quality:?}: rejeição de alias {db:.1} dB, alvo -{target_db} dB"
        );
        // Sanidade do próprio oráculo: o tom de passagem tem de estar em ~0,5.
        assert!(
            (ref_amp - 0.5).abs() < 0.05,
            "{quality:?}: referência {ref_amp} fora do esperado"
        );
    }
}

#[test]
fn passband_ripple_is_within_the_target() {
    // A borda de passband vale 0,44·min para `balanced`; acima disso a resposta
    // já está caindo de propósito.
    let fs_in = 44_100u32;
    let fs_out = 48_000u32;
    let analyze = 46_080usize;
    let skip = 2_000usize;
    let total_in = 100_000usize;
    let bin = fs_out as f64 / analyze as f64;

    let resampler_probe = Resampler::new(config(fs_in, fs_out, Quality::Balanced)).unwrap();
    assert!(resampler_probe.taps() > 0);

    let edge = 0.44 * (fs_in.min(fs_out)) as f64;
    let mut worst = 0.0f64;
    let mut worst_f = 0.0f64;

    for step in 0..24 {
        let frac = step as f64 / 23.0;
        let freq = 100.0 * (edge / 100.0f64).powf(frac);
        // Arredonda para o bin exato mais próximo para não medir vazamento.
        let freq = ((freq / bin).round() * bin).max(bin);
        let cfg = config(fs_in, fs_out, Quality::Balanced);
        let input: Vec<f32> = (0..total_in)
            .map(|n| (0.5 * (2.0 * PI * freq * n as f64 / fs_in as f64).sin()) as f32)
            .collect();
        let out = run_all(&input, cfg, 8_192);
        assert!(out.len() > skip + analyze);
        let amp = goertzel(&out[skip..skip + analyze], freq, fs_out as f64);
        let db = 20.0 * (amp / 0.5).log10();
        if db.abs() > worst {
            worst = db.abs();
            worst_f = freq;
        }
    }
    assert!(
        worst <= 0.1,
        "ripple de passband {worst:.4} dB em {worst_f:.0} Hz, alvo 0,1 dB"
    );
}

// ---------------------------------------------------------------- erros

#[test]
fn errors_are_data_and_the_instance_stays_usable() {
    // Taxa zero.
    assert_eq!(
        Resampler::new(ResamplerConfig {
            fs_in: 0,
            fs_out: 48_000,
            quality: Quality::Balanced,
            max_block_frames: 64,
        })
        .unwrap_err(),
        Error::ZeroSampleRate
    );
    assert_eq!(
        Resampler::new(ResamplerConfig {
            fs_in: 48_000,
            fs_out: 0,
            quality: Quality::Balanced,
            max_block_frames: 64,
        })
        .unwrap_err(),
        Error::ZeroSampleRate
    );

    // Orçamento de fases: 44 100 e 44 101 são coprimos, então b = 44 101 fases.
    match Resampler::new(ResamplerConfig {
        fs_in: 44_100,
        fs_out: 44_101,
        quality: Quality::Balanced,
        max_block_frames: 64,
    }) {
        Err(Error::TooManyPhases { phases, max }) => {
            assert_eq!(phases, 44_101);
            assert!(max < phases);
        }
        other => panic!("esperava TooManyPhases, veio {other:?}"),
    }

    // Bloco maior que o declarado, e buffer de saída curto — sem `panic!`, e a
    // instância continua utilizável depois dos dois erros.
    let cfg = ResamplerConfig {
        fs_in: 48_000,
        fs_out: 96_000,
        quality: Quality::Balanced,
        max_block_frames: 128,
    };
    let mut resampler = Resampler::new(cfg).unwrap();
    let big = vec![0.0f32; 129];
    let mut out = vec![0.0f32; 1_024];
    assert_eq!(
        resampler.process(&big, &mut out).unwrap_err(),
        Error::BlockTooLarge {
            requested: 129,
            max: 128
        }
    );

    let small_in = vec![0.0f32; 128];
    let need = resampler.output_capacity_for(128);
    assert!(need > 8);
    let mut tiny = vec![0.0f32; 8];
    assert_eq!(
        resampler.process(&small_in, &mut tiny).unwrap_err(),
        Error::OutputTooSmall { need, have: 8 }
    );

    // Depois dos dois erros, o mesmo sinal ainda produz o que deveria: o estado
    // não foi corrompido por uma chamada que falhou.
    let mut good = vec![0.0f32; need];
    let n = resampler.process(&small_in, &mut good).unwrap();
    assert!(n > 128, "esperava ~256 frames de saída, veio {n}");
}
