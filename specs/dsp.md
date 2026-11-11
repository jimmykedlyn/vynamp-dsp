# Spec — vynamp-dsp · dsp

| Campo | Valor |
|---|---|
| Módulo | `vynamp-dsp` |
| Crate(s) | a definir na autoria |
| Tipo | lib |
| Versão da spec | 0.2.0 |
| Data | 2026-11-11 |
| Estado | **rascunho autorado pelo spike, não aprovado** — §3, §5, §6, §9 e §10 escritos a partir da [ADR-017](https://github.com/jimmykedlyn/vynamp/blob/develop/adr/ADR-017-resampler-e-conversao.md) e do código da TASK-E04-02; §4 continua a autorar. A aprovação é do mantenedor, por `GOVERNANCE.md` |
| Dono | a definir |
| Depende de | nada — esta lib e a base da pilha |
| É dependido por | engine (obrigatorio); output e encode (opcionais, feature `resample`); restore |

> **Esqueleto gerado pelo passo 37, preenchido em parte pelo spike de 2026-11-11.**
> §1, §2 e §7 foram decididos em `01-arvore-final.md` §2 e valem como norma.
> §3, §5, §6, §9 e §10 foram escritos a partir do que a TASK-E04-02 **mediu**, e a
> fonte dos números é a [ADR-017](https://github.com/jimmykedlyn/vynamp/blob/develop/adr/ADR-017-resampler-e-conversao.md) —
> esta spec cita a ADR e não repete os valores, para não haver duas verdades.
> **Estar escrito não é estar aprovado:** a Boundary da TASK-E04-02 proíbe o spike de
> aprovar esta spec, e quem move o estado para `aprovada` é o mantenedor, por
> `GOVERNANCE.md` §"Changing this document". §4 continua a autorar.

## 1. Propósito

Primitivas de processamento de amostras: resampling, dither, mapa de canais, mixer, ganho, fade e crossfade.

**Não faz:** Nao conhece arquivo, container, dispositivo, formato de midia nem qualquer tipo de dominio.

## 2. Fronteira

| Permitido | Proibido |
|---|---|
| std; nenhuma dependencia de outro crate, nem opcional | qualquer crate `vynamp-*`, inclusive o proprio engine; qualquer tipo de dominio |

## 3. API pública

> Autorado pelo spike da TASK-E04-02 em 2026-11-11 e **não aprovado**. O que está
> aqui é o que o código implementa hoje; os valores de qualidade são os da
> [ADR-017](https://github.com/jimmykedlyn/vynamp/blob/develop/adr/ADR-017-resampler-e-conversao.md) §2.

Três tipos e dois submódulos. Nada além disso é contrato; o resto é privado.

### 3.1 `Resampler` — conversão de taxa

```rust
pub struct ResamplerConfig {
    pub fs_in: u32,
    pub fs_out: u32,
    pub quality: Quality,
    pub max_block_frames: usize,
}

impl Resampler {
    pub fn new(config: ResamplerConfig) -> Result<Self, Error>;
    pub fn taps(&self) -> usize;
    pub fn latency_frames(&self) -> usize;
    pub fn output_capacity_for(&self, frames: usize) -> usize;
    pub fn process(&mut self, input: &[f32], out: &mut [f32]) -> Result<usize, Error>;
    pub fn flush(&mut self, out: &mut [f32]) -> Result<usize, Error>;
}
```

- **Uma instância por canal.** A biblioteca não tem o conceito de canal: ela processa
  `&[f32]`. Quem tem um octeto mantém oito instâncias, todas sobre a mesma `Kernel`.
- **`Kernel` é compartilhada.** `Kernel::new` constrói a tabela de coeficientes uma
  vez e devolve `Arc<Kernel>`; o custo em memória é `fases · taps · 4` bytes por
  configuração, não por canal.
- **Sem alocação depois de `new`.** `max_block_frames` declara o maior bloco que a
  instância aceita e é a origem de toda reserva. Um bloco maior devolve
  `Error::BlockTooLarge` em vez de alocar.
- **`output_capacity_for(n)` é o tamanho que o chamador precisa reservar** para
  processar `n` frames de entrada sem perder amostra. Buffer menor devolve
  `Error::OutputTooSmall`, e o estado não é corrompido pela chamada que falhou.
- **`flush` drena a antecipação** no fim do fluxo, injetando zeros.
- **`fs_in == fs_out` é bypass bit a bit**, com `latency_frames() == 0` e `taps() == 1`.
  Não é filtro unitário: filtrar em taxa igual atenuaria o topo do espectro sem
  conversão pedida.

### 3.2 `Quality` — os dois modos

```rust
pub enum Quality { Balanced, High }
```

| Modo | Passband até | Stopband de | Taps em 48 kHz |
|---|---|---|---|
| `Balanced` | 0,44·min(fs_in, fs_out) | 0,50·min | 97 |
| `High` | 0,47·min(fs_in, fs_out) | 0,50·min | 261 |

`High` custa 2,69× em MAC e em latência. Não existe modo abaixo de `Balanced`: um
filtro que só existiria para reprovar o `DSP-T-13` não é um modo.

### 3.3 `convert` — profundidade e packing

```rust
pub const fn shift_for(depth: u8) -> Option<u32>;   // 32 → 0, 24 → 8, 16 → 16
pub fn unpack_24(packed: i32) -> i32;
pub fn unpack_16(packed: i32) -> i32;
pub fn integer(
    src: &[i32], src_depth: u8, dst_depth: u8,
    dither: Option<&mut Tpdf>, out: &mut [i32],
) -> Result<(), Error>;
```

Amostras inteiras vivem em container `i32` com os bits válidos nos **mais
significativos** — a convenção do `WAVEFORMATEXTENSIBLE` quando
`wValidBitsPerSample < wBitsPerSample`. 24 bits ocupam `31..8`; 16 bits, `31..16`.

| De → para | O que acontece | Dither |
|---|---|---|
| mesma profundidade | cópia bit a bit | impossível — não há o que arredondar |
| maior profundidade | valor reinterpretado, bits baixos ficam zero | nunca |
| menor profundidade | deslocamento à direita com arredondamento e saturação | TPDF, se o chamador fornecer a fonte |

**Dither só ao reduzir precisão** é propriedade do código, não recomendação: quando
`dst_depth >= src_depth` a função não toca na fonte de dither e o estado do gerador
não avança.

### 3.4 `Tpdf` — o gerador de dither

```rust
impl Tpdf {
    pub const fn new(seed: u64) -> Self;
    pub const fn state(&self) -> u64;
    pub fn next(&mut self) -> i32;   // −1, 0 ou +1
}
```

Determinístico e semeado pelo chamador: um dither que não se reproduz não se testa.
A distribuição é triangular de pico a pico 2 LSB — `{−1, 0, +1}` com ¼, ½, ¼ —, em
aritmética inteira, e a média é zero.

## 4. Requisitos

PENDENTE — cada requisito com ID estável `DSP-REQ-nn` e REQ de produto,
rastreável até uma TASK e um teste.

## 5. Critérios de aceite

> Autorado pelo spike da TASK-E04-02 em 2026-11-11, com os números que o código
> entregou. **Não aprovado:** quem aprova esta seção é o mantenedor.

Cada critério é verificável por um teste automatizado que já existe em `tests/` e roda
em qualquer máquina, sem hardware de áudio.

| ID | Given | When | Then | Teste |
|---|---|---|---|---|
| `DSP-AC-01` | `fs_in == fs_out` | um sinal passa pelo `Resampler` | a saída é **bit a bit** igual à entrada e `latency_frames()` é 0 | `bypass_at_equal_rate_is_bit_exact` |
| `DSP-AC-02` | uma entrada constante | ela atravessa qualquer par de taxas | o ganho DC fica dentro de 4·ε de f32 fora das bordas | `constant_input_keeps_dc_gain` |
| `DSP-AC-03` | um impulso unitário em índice conhecido | ele atravessa o resampler | o pico da saída cai a menos de 1,5 frame da posição ideal `n0·fs_out/fs_in` | `impulse_peak_aligns_with_input_position` |
| `DSP-AC-04` | um tom acima do Nyquist de saída | ele é decimado | a energia no bin dobrado fica ≥ 80 dB abaixo da referência em `Balanced` e ≥ 110 dB em `High` | `alias_below_the_target_passes_and_above_fails` |
| `DSP-AC-05` | tons de 100 Hz até a borda de passband | eles atravessam o resampler | o ripple fica ≤ 0,1 dB em toda a banda | `passband_ripple_is_within_the_target` |
| `DSP-AC-06` | taps e latência publicados | a instância é construída em cada par de taxas da ADR-017 §2 | os valores batem com a tabela da ADR | `latency_frames_match_the_adr_table` |
| `DSP-AC-07` | a latência publicada | um fluxo de N frames é processado | a saída produzida é exatamente a das janelas que fecharam, e o pendente bate com a latência | `latency_is_what_the_stream_actually_holds_back` |
| `DSP-AC-08` | o mesmo sinal | ele é processado em um bloco e em blocos de 7 | as duas saídas são idênticas amostra a amostra — o estado entre blocos é obrigatório | `block_splitting_matches_single_block` |
| `DSP-AC-09` | a mesma profundidade | a conversão roda com e sem fonte de dither | as saídas são idênticas e o estado do gerador **não avançou** | `same_depth_is_bit_exact_and_does_not_touch_the_dither` |
| `DSP-AC-10` | 16 bits → 24 bits | a conversão roda com uma fonte de dither | os 8 bits baixos ficam zero e o dither não é consumido | `widening_zeroes_the_low_bits_and_never_dithers` |
| `DSP-AC-11` | um sinal de 24 bits | ele desce para 16 bits com e sem dither | a saída difere em no máximo 1 LSB, com média zero e 16 bits baixos zerados | `reducing_depth_dithers_only_when_asked` |
| `DSP-AC-12` | o gerador TPDF | 400 000 amostras são sorteadas | a distribuição é ¼, ½, ¼ em {−1, 0, +1} com desvio ≤ 0,005 | `dither_distribution_is_tpdf` |
| `DSP-AC-13` | os extremos de escala inteira | a profundidade é reduzida | o pico positivo satura no máximo do destino e **nunca** troca de sinal | `full_scale_narrows_without_wrapping_to_the_opposite_sign` |
| `DSP-AC-14` | uma rampa crescente | a profundidade é reduzida | a saída é monótona — não há ponto onde ela cai | `narrowing_is_monotonic_across_the_whole_range` |
| `DSP-AC-15` | uma profundidade inválida | a conversão é chamada | devolve `Error::InvalidDepth`, não `panic!` | `invalid_depth_is_an_error_not_a_panic` |
| `DSP-AC-16` | uma instância já construída | `process` roda em regime | **zero** alocações no caminho quente | `process_does_not_allocate_after_construction` |
| `DSP-AC-17` | qualquer condição de erro | a chamada falha | a instância continua utilizável e o estado não é corrompido | `errors_are_data_and_the_instance_stays_usable` |

`DSP-AC-16` é acompanhado por `the_probe_can_actually_see_an_allocation`, um teste de
controle que prova que o contador de alocações funciona: sem ele, um contador quebrado
passaria o critério anterior sem medir nada.

### Critérios que esta spec **não** fixa

- **Métricas de CPU e latência em tempo.** A ADR-017 §2 registra o que foi medido, mas
  nenhum teste compara CPU: o número depende da máquina e reprovaria em CI por
  variabilidade, não por regressão.
- **Validação acústica.** Nenhum DAC é tocado aqui; é a TASK-E04-10.

## 6. Erros e cancelamento

**Erro é dado, não `panic!`.** A biblioteca devolve `Result` e o chamador decide. Nenhum
caminho de entrada do usuário chama `panic!`, `unwrap` ou `expect`.

```rust
pub enum Error {
    ZeroSampleRate,
    InvalidDepth { depth: u8 },
    TooManyPhases { phases: u64, max: u64 },
    BlockTooLarge { requested: usize, max: usize },
    OutputTooSmall { need: usize, have: usize },
}
```

O enum é `Copy`, `PartialEq` e `Eq`, então o chamador pode comparar sem `matches!`.
Ele implementa `Display` e `core::error::Error`.

| Variante | Quando | O que o chamador faz |
|---|---|---|
| `ZeroSampleRate` | `fs_in` ou `fs_out` é 0 | erro de configuração: não há conversão possível |
| `InvalidDepth` | profundidade fora de 16, 24 ou 32 | erro de configuração do chamador |
| `TooManyPhases { phases, max }` | a razão reduzida exige mais fases que o orçamento | a ADR-017 §Lacunas registra: a construção recusa em vez de alocar sem limite |
| `BlockTooLarge { .. }` | `input.len() > max_block_frames` | dividir o bloco; a reserva é do tamanho declarado |
| `OutputTooSmall { .. }` | o buffer de saída não comporta o que a entrada pode produzir | reservar `output_capacity_for(n)` |

**Depois de um erro a instância continua utilizável** — é o que `DSP-AC-17` fixa. Uma
chamada que falha na validação não chega a tocar no estado.

**Cancelamento não existe nesta camada.** O resampler processa blocos finitos que o
chamador entrega; parar é o chamador parar de chamar. Um timeout aqui seria esconder
que o bloco é grande demais — e o bloco declarado é o contrato.

## 7. Fora de escopo

DSP musical (EQ, dynamics, reverb, tempo/pitch) — isso e do engine; formatos de arquivo — isso e do decode/encode

## 8. Dependências

Depende de: nada — esta lib e a base da pilha

É dependido por: engine (obrigatorio); output e encode (opcionais, feature `resample`); restore

## 9. Testes e fixtures

**Nenhuma fixture externa.** Todo sinal é gerado no teste: senoides, rampas e impulsos
construídos em memória. Não há arquivo de áudio, corpus nem `#[ignore]`. A suíte inteira
roda em ~6 s e não precisa de variável de ambiente.

**Sem hardware.** Nenhum dispositivo de áudio é tocado. As alegações sobre CPU e
latência da ADR-017 são wall-clock de bancada sobre o buffer, estão registradas com essa
ressalva, e **não** são testadas em CI — o teste que as usaria reprovaria por
variabilidade da máquina.

Os dois oráculos numéricos que os testes usam:

- **Goertzel no bin exato.** A resposta em frequência é medida por Goertzel na
  frequência do tom, com o número de amostras escolhido para que a frequência caia
  exatamente num bin (`46 080` amostras a 44,1 kHz dão 25/24 Hz por bin, e 1 000 e
  14 100 são múltiplos inteiros disso). Amostrar fora do bin mediria vazamento
  espectral em vez do filtro.
- **Rampa inteira.** Para a conversão, o frame `f` carrega `start + step·f`: um bypass
  que borre ou reordene uma amostra aparece como valor fora da rampa. É a mesma ideia
  do `ChannelRamp` de `vynamp-output`, construída localmente porque o §2 proíbe
  importar qualquer crate `vynamp-*`.

**Testes de borda que existem porque a borda já falhou uma vez:**

- `full_scale_narrows_without_wrapping_to_the_opposite_sign` — o pico positivo virava
  o pico negativo na redução para 16 bits.
- `latency_is_what_the_stream_actually_holds_back` — a antecipação não era pré-carregada
  e a contagem de frames saía abaixo do que as janelas fechadas permitiam.

**Teste de controle.** `the_probe_can_actually_see_an_allocation` prova que o alocador
de contagem funciona, para que `DSP-AC-16` não passe por um contador quebrado.

## 10. Limitações conhecidas

Limitação declarada não reprova a entrega; limitação omitida reprova.

| Limitação | Efeito | Quem cobre |
|---|---|---|
| **Um modo por vez, fixo na construção.** Não há troca de qualidade em tempo real | Mudar de `Balanced` para `High` exige construir outra instância, e a troca tem de ser no ponto de silêncio ou com transição no chamador | TASK-E04-03, que escolhe o modo por dispositivo |
| **Modo `low` não existe** | Aparelhos que só comportariam um filtro mais barato não são atendidos pelo `Balanced` | Decisão da ADR-017 §2: filtro que reprovaria o `DSP-T-13` não é modo |
| **Orçamento de fases.** A tabela é pré-computada e limitada a 4 096 fases | Razões cujo denominador reduzido estoura o orçamento são **recusadas na construção**, não servidas com menos qualidade | A ADR-017 registra o custo como lacuna |
| **Custo cresce com a razão de taxas**, sem decimação em cascata | 44,1k↔384k custa ~32 % de um núcleo em `High`, contra ~4 % dos pares próximos | Lacuna registrada na ADR-017; a cascata é otimização |
| **Latência de `half_base / min(fs_in, fs_out)`** por instância | ~2,95 ms em `High`. Precisa entrar na compensação de clock do grafo | TASK-E04-03 |
| **Sem noise shaping** | O TPDF é o único dither; shaping fica para M5 | M5, como a referência já diz |
| **Sem hardware de áudio** | CPU e latência são wall-clock de bancada; nenhum DAC, nenhuma callback RT foi medida nesta lib | TASK-E04-10 |
| **Sem mapa de canais, mixer, ganho, fade nem crossfade** | O §1 da spec de módulo lista essas primitivas, mas esta entrega é só resample, packing e dither | TASK-E01-09 (mapa), E01-05/E04-03 (mixer), E04-01 (ganho) |
| **Sem decodificar nem escrever container** | A lib trabalha em `f32` e `i32`; byte order e header são do `decode`/`output` | TASK-E04-03, output |

