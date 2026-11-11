//! Medição de CPU e latência para a evidência da TASK-E04-02-AC-1.
//!
//! Uso: `cargo run --release --bin measure -- [comentário]`
//!
//! Escreve um JSON na saída padrão. Nenhum dispositivo de áudio é tocado: as
//! medidas são de parede de relógio sobre o buffer, como a TASK declara.

use std::f64::consts::PI;
use std::io::Write;
use std::time::Instant;

use vynamp_dsp::{Quality, Resampler, ResamplerConfig};

const BLOCK: usize = 1024;
const SECONDS: f64 = 5.0;

/// Roda o sinal inteiro em blocos e devolve a saída, drenando a cauda.
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

/// Amplitude de um tom por Goertzel, no bin exato.
fn goertzel(samples: &[f32], freq: f64, fs: f64) -> f64 {
    let n = samples.len();
    let k = freq * n as f64 / fs;
    let w = 2.0 * PI * k / n as f64;
    let (cw, sw) = (w.cos(), w.sin());
    let coeff = 2.0 * cw;
    let (mut s1, mut s2) = (0.0f64, 0.0f64);
    for &v in samples {
        let s0 = f64::from(v) + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    let re = s1 - s2 * cw;
    let im = s2 * sw;
    (re * re + im * im).sqrt() * 2.0 / n as f64
}

/// Rejeição de alias medida para um par de taxas, em dB abaixo da referência.
///
/// Um tom acima do Nyquist de saída dobra para `|fs_out − f|`; a energia lá é o
/// alias. A referência é um tom de 1 kHz, para o número não depender do nível.
fn alias_rejection_db(fs_in: u32, fs_out: u32, quality: Quality) -> f64 {
    let analyze = 46_080usize;
    let skip = 2_000usize;
    // A entrada é dimensionada pela saída, não o contrário: em 192k → 44.1k são
    // necessários ~4,4 amostras de entrada por amostra analisada.
    let needed_out = skip + analyze + 4_096;
    let total = (needed_out as u64 * u64::from(fs_in)).div_ceil(u64::from(fs_out)) as usize + 4_096;
    // Escolhe o tom de stopband na metade da banda proibida, arredondado para um
    // múltiplo do bin, para não medir vazamento espectral no próprio oráculo.
    let bin = f64::from(fs_out) / analyze as f64;
    let raw = (f64::from(fs_out) + 0.5 * (f64::from(fs_in) - f64::from(fs_out))) / 2.0;
    let stop_freq = (raw / bin).round() * bin;
    let alias_freq = (f64::from(fs_out) - stop_freq).abs();
    let alias_freq = (alias_freq / bin).round() * bin;

    let tone = |freq: f64| -> Vec<f32> {
        (0..total)
            .map(|n| (0.5 * (2.0 * PI * freq * n as f64 / f64::from(fs_in)).sin()) as f32)
            .collect()
    };
    let cfg = ResamplerConfig {
        fs_in,
        fs_out,
        quality,
        max_block_frames: 4096,
    };
    let pass = run_all(&tone(1_000.0), cfg, 4096);
    let stop = run_all(&tone(stop_freq), cfg, 4096);
    let reference = goertzel(&pass[skip..skip + analyze], 1_000.0, f64::from(fs_out));
    let alias = goertzel(&stop[skip..skip + analyze], alias_freq, f64::from(fs_out));
    20.0 * (alias / reference).log10()
}

fn main() {
    let pairs = [
        (44_100u32, 48_000u32),
        (48_000, 44_100),
        (44_100, 96_000),
        (96_000, 44_100),
        (44_100, 192_000),
        (192_000, 44_100),
        (48_000, 192_000),
        (44_100, 384_000),
        (384_000, 44_100),
    ];

    let mut rows = Vec::new();
    for (fs_in, fs_out) in pairs {
        for quality in [Quality::Balanced, Quality::High] {
            let cfg = ResamplerConfig {
                fs_in,
                fs_out,
                quality,
                max_block_frames: BLOCK,
            };
            let mut resampler = Resampler::new(cfg).expect("construção");

            let input: Vec<f32> = (0..BLOCK)
                .map(|i| (0.4 * ((i as f32) * 0.031).sin()) + 0.1)
                .collect();
            let capacity = resampler.output_capacity_for(BLOCK);
            let mut out = vec![0.0f32; capacity];

            // Aquecimento: a primeira passada paga as faltas de cache da tabela, e
            // contá-la mediria a inicialização, não o regime.
            for _ in 0..200 {
                resampler.process(&input, &mut out).expect("process");
            }

            let produced_per_block = resampler.process(&input, &mut out).expect("process");
            let blocks = ((SECONDS * f64::from(fs_in)) / BLOCK as f64).ceil() as usize;

            let start = Instant::now();
            let mut total_out = 0usize;
            for _ in 0..blocks {
                total_out += resampler.process(&input, &mut out).expect("process");
            }
            let elapsed = start.elapsed().as_secs_f64();

            // Custo por amostra de saída, em nanossegundos, e o MAC/s que isso
            // implica — o número que a ADR precisa para o orçamento de CPU.
            let per_out_ns = elapsed * 1e9 / total_out as f64;
            let in_seconds = (blocks * BLOCK) as f64 / f64::from(fs_in);
            let cpu_ratio = elapsed / in_seconds;

            rows.push(format!(
                concat!(
                    "    {{\"fs_in\": {fs_in}, \"fs_out\": {fs_out}, ",
                    "\"quality\": \"{quality}\", \"taps\": {taps}, ",
                    "\"latency_frames\": {lat}, \"latency_ms\": {lat_ms:.4}, ",
                    "\"block_frames\": {block}, \"out_per_block\": {opb}, ",
                    "\"elapsed_s\": {elapsed:.4}, ",
                    "\"ns_per_out_frame\": {per_out_ns:.2}, ",
                    "\"cpu_ratio_of_realtime\": {cpu_ratio:.4}, ",
                    "\"mac_per_second\": {mac:.0}}}"
                ),
                fs_in = fs_in,
                fs_out = fs_out,
                quality = if quality == Quality::High {
                    "high"
                } else {
                    "balanced"
                },
                taps = resampler.taps(),
                lat = resampler.latency_frames(),
                lat_ms = resampler.latency_frames() as f64 * 1000.0 / f64::from(fs_out),
                block = BLOCK,
                opb = produced_per_block,
                elapsed = elapsed,
                per_out_ns = per_out_ns,
                cpu_ratio = cpu_ratio,
                mac = resampler.taps() as f64 * total_out as f64 / elapsed,
            ));
        }
    }

    println!("{{");
    println!("  \"note\": \"wall-clock sobre o buffer; nenhum DAC tocado\",");
    println!("  \"block_frames\": {BLOCK},");
    println!("  \"seconds_of_input_per_row\": {SECONDS},");
    println!("  \"rows\": [");
    println!("{}", rows.join(",\n"));
    println!("  ],");

    // Rejeição de alias medida, só na decimação — é onde o alias existe.
    let mut alias_rows = Vec::new();
    for (fs_in, fs_out) in [(96_000u32, 44_100u32), (192_000, 44_100), (48_000, 44_100)] {
        for quality in [Quality::Balanced, Quality::High] {
            let db = alias_rejection_db(fs_in, fs_out, quality);
            alias_rows.push(format!(
                "    {{\"fs_in\": {fs_in}, \"fs_out\": {fs_out}, \"quality\": \"{}\", \"alias_rejection_db\": {db:.2}}}",
                if quality == Quality::High { "high" } else { "balanced" }
            ));
        }
    }
    println!("  \"alias_rejection\": [");
    println!("{}", alias_rows.join(",\n"));
    println!("  ]");
    println!("}}");
    std::io::stdout().flush().ok();
}
