//! O caminho quente não aloca — ADR-017 §5.
//!
//! Conta alocações **por thread**, com um alocador global. A evidência da
//! TASK-E01-02 registrou por que: a primeira versão daquele teste media o processo
//! inteiro e falhava 5 vezes em 30 porque o escritor de outra thread alocava
//! enquanto o leitor media.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use vynamp_dsp::{Quality, Resampler, ResamplerConfig};

thread_local! {
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
    static ARMED: Cell<bool> = const { Cell::new(false) };
}

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: `try_with` devolve Err só durante o destrutor do thread-local,
        // quando não há como incrementar — e nesse caso não há o que contar.
        let _ = ARMED.try_with(|armed| {
            if armed.get() {
                let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let _ = ARMED.try_with(|armed| {
            if armed.get() {
                let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
            }
        });
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

fn measure_allocs<F: FnOnce()>(f: F) -> usize {
    ALLOCS.with(|c| c.set(0));
    ARMED.with(|a| a.set(true));
    f();
    ARMED.with(|a| a.set(false));
    ALLOCS.with(|c| c.get())
}

#[test]
fn process_does_not_allocate_after_construction() {
    for (fs_in, fs_out) in [(44_100u32, 48_000u32), (96_000, 44_100), (48_000, 48_000)] {
        for quality in [Quality::Balanced, Quality::High] {
            let cfg = ResamplerConfig {
                fs_in,
                fs_out,
                quality,
                max_block_frames: 4_096,
            };
            // A construção aloca a tabela de fases — é justamente o que ela tem de
            // fazer. Tudo o que o caminho quente precisa sai reservado daqui.
            let mut resampler = Resampler::new(cfg).unwrap();
            let input = vec![0.1f32; 4_096];
            let mut out = vec![0.0f32; resampler.output_capacity_for(4_096)];
            let mut tail = vec![0.0f32; resampler.output_capacity_for(4_096)];

            let allocs = measure_allocs(|| {
                let n = resampler.process(&input, &mut out).unwrap();
                assert!(n > 0);
                let _ = resampler.flush(&mut tail).unwrap();
            });
            assert_eq!(
                allocs, 0,
                "{fs_in}->{fs_out} {quality:?}: {allocs} alocações no caminho quente"
            );
        }
    }
}

#[test]
fn the_probe_can_actually_see_an_allocation() {
    // Sem este controle, um alocador mal instrumentado passaria no teste acima
    // por não contar nada — e o teste acima viraria decoração.
    let allocs = measure_allocs(|| {
        let v: Vec<u8> = Vec::with_capacity(4_096);
        std::hint::black_box(&v);
    });
    assert!(allocs > 0, "o contador não viu a alocação de controle");
}
