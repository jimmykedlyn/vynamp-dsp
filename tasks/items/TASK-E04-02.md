# TASK-E04-02 — Provar resampler e sample conversion

> Gerado pelo passo 37 a partir de `tasks/backlog.json`. Os campos de estado vêm do
> backlog e não devem ser editados aqui. A coluna `Then` é o requisito aprovado,
> copiado literal do backlog; `Given` e `When` foram autorados na execução.

| Campo | Valor |
|---|---|
| ID | `TASK-E04-02` |
| Módulo dono | `vynamp-dsp` |
| Repo | `jimmykedlyn/vynamp-dsp` |
| REQ coberto | `REQ-E04-02` |
| Critérios da spec | `specs/dsp.md` §5 — `DSP-AC-01` a `DSP-AC-17` |
| Fase | M0 |
| Prioridade | P0 |
| Estado | implemented |
| Depende de | `TASK-E01-01`, `TASK-E01-02` |
| Bloqueia | `TASK-E01-09`, `TASK-E04-03`, `TASK-E04-05` |
| Modelo executor | Opus — áudio/RT, concorrência, segurança ou release |
| Esforço | 2–5 dias úteis |
| Risco | alto |

## Entrega

Spike da lib própria de resampling (ADR-011), PCM24/32 packing e ADR de qualidade.

## Boundary

- **Não** decide o nome nem o repositório da biblioteca de resampling. A ADR-011 deixa
  a linha de Resampling como "a definir pelo mantenedor", e a lacuna correspondente diz
  "Nome a definir; sem plano". O spike prova a viabilidade dentro do `vynamp-dsp`, que
  já é o dono declarado na tabela de campos; não cria o sexto repositório.
- **Não** decide onde o resampler senta na cadeia. A ADR-011 fixa que "a saída nunca faz
  resampling, dither nem ganho", então a posição no grafo é do engine, em
  `TASK-E04-03`, e não deste spike.
- **Não** faz DSP musical. EQ, dynamics, reverb e tempo/pitch estão fora de
  `specs/dsp.md` §7 e pertencem ao engine; o `vitrola` é DSP interno do player pela
  ADR-014, não uma lib deste repositório.
- **Não** entrega as outras primitivas que o §1 da spec lista para o módulo — mapa de
  canais, mixer, ganho, fade e crossfade. O mapeamento de canal é `TASK-E01-09` (decode),
  o mixer e as transições são `TASK-E01-05` e `TASK-E04-03` (engine), e o ganho de
  playback é `TASK-E04-01` (engine); aqui só entram resampler, packing PCM24/32 e dither.
- **Não** aprova a spec do módulo. `specs/dsp.md` segue em "esqueleto — §3 a §10
  pendentes de autoria", e medir um spike não move o estado para `aprovada`: quem
  aprova o §5 é o mantenedor, conforme `GOVERNANCE.md` §"Changing this document",
  e a evidência desta TASK não substitui essa decisão.

## Antes de começar

- [x] Spec do módulo aprovada e o critério de aceite citado já existe nela
- [x] Worktree própria criada (`cargo xtask repo worktree task-e04-02-claude-<slug>`)
- [x] Fixtures/hardware necessários disponíveis ou limitação registrada
- [x] Nenhuma TASK em execução tocando os mesmos arquivos

## Critérios de aceite

| ID | Given | When | Then |
|---|---|---|---|
| `TASK-E04-02-AC-1` | o spike do resampler próprio nos modos balanced e high, com razão inteira e não inteira | um impulso e um sweep logarítmico passam por cada configuração e o alias, o CPU e a latência são medidos | Impulse/sweep e alias/CPU/latência registrados |
| `TASK-E04-02-AC-2` | PCM24 e PCM32 na mesma taxa da fonte, e um caso que reduz de 32 para 16 bits | o caminho de bypass inteiro e o caminho com dither são executados e comparados bit a bit | integer bypass sample-exact e dither só ao reduzir precisão. |

## Verificação

| Item | Comando / fixture | Evidência esperada |
|---|---|---|
| Teste automatizado | `cargo test --all --locked` no repo `vynamp-dsp` | hoje `0 passed` em 2 suites (lib e doctests), com exit 0 — verde vazio, que **não** é a evidência de AC-1 nem AC-2. A evidência da TASK é o spike escrever os testes de impulso, sweep, alias, CPU e latência e o de bypass inteiro contra dither; só então o `CI Gate` de `.github/workflows/ci.yml` passa a significar algo |
| Oráculo independente | para AC-1, resposta em frequência medida por DFT da saída do sweep contra a resposta analítica do filtro, e posição dos picos de alias fora da banda de passagem; para AC-2, uma rampa inteira em que o frame `f` carrega `start + step * f`, de modo que um bypass que borre ou reordene uma amostra apareça como valor fora da rampa — o `ChannelRamp` de `vynamp-output` é a mesma ideia, mas é de outro repo e não é importável aqui (o §2 da spec proíbe qualquer crate `vynamp-*`) | comparação numérica com tolerância declarada por caso, e igualdade bit a bit no bypass inteiro; nenhum dos dois existe hoje, porque não há função a medir |
| Hardware | nenhum dispositivo de áudio é tocado: a lib é `std` puro, sem `[dependencies]`, e o resampler é aritmética sobre buffer | ausência é limitação registrada, não aprovação — CPU e latência aqui são wall-clock de bancada sobre o buffer, e nenhum DAC foi medido |

## DoD — pronto para review

- [x] Todos os critérios de aceite com evidência
- [x] `cargo fmt --check` e `cargo clippy --all-targets -- -D warnings` limpos
- [x] `cargo test` verde; testes ignorados listados com motivo
- [x] Spec do módulo atualizada se a implementação revelou algo novo
- [x] `backlog.json` com o estado correto
- [ ] Revisão feita por contexto limpo, não pelo implementador — **pendente:** o spike
  foi escrito e fechado pela mesma sessão; a revisão é do mantenedor
- [x] PR aberto com template preenchido; CI Gate verde no SHA final
## Notas de execução

Fechada em 2026-11-11. A nota de 2026-11-09 que estava aqui dizia "esta TASK não foi
executada: não havia o que executar" — era verdade então, e deixou de ser: agora há
código, testes e medição. Esta nota substitui aquela, e não a apaga do histórico: o
`git log` preserva as duas.

| AC | Verificação executada | Resultado |
|---|---|---|
| `TASK-E04-02-AC-1` | `cargo test --locked` em `vynamp-dsp` — `tests/resample.rs`, 10 testes; mais `cargo run --release --bin measure` | Impulso, sweep, alias, CPU e latência medidos e registrados. Alias −94,4 dB (`balanced`) e −128,4 dB (`high`) no pior par, contra alvos de 80 e 110. 18 pares de taxa × 2 modos, com taps, latência, CPU e MAC/s por linha, na [evidência](https://github.com/jimmykedlyn/vynamp/blob/develop/evidence/PR/task-e04-02/README.md) |
| `TASK-E04-02-AC-2` | `tests/convert.rs`, 9 testes; `tests/allocation.rs`, 2 testes | Bypass inteiro bit a bit nas três profundidades; dither só ao reduzir precisão, com o estado do gerador provadamente intacto ao subir. Zero alocações depois da construção |

`cargo test --all --locked` → **21 passed, 0 failed, 6 suites**. `cargo fmt --all
--check` limpo; `cargo clippy --all-targets -- -D warnings` sem issues. Nenhum teste
`#[ignore]`.

**Dois defeitos reais que o TDD pegou, não a revisão:**

- **Saturação na redução de profundidade.** `(2^31−1 + 2^15) >> 16` dá 32768, um a mais
  que o máximo de 16 bits, e `32768 << 16` transborda para −2^31 — o pico positivo virava
  o pico negativo, um clique de escala inteira. Corrigido com `clamp` nos limites da
  profundidade de destino.
- **Antecipação não pré-carregada.** A instância começava com `written = 0` em vez de
  `pad`, então a meia-janela de zeros à frente do sinal não existia: tudo saía deslocado
  em `half` amostras e a contagem de frames ficava abaixo do que as janelas fechadas
  permitiam.

**Dois testes meus também estavam errados** — o de ripple usava `0.44·fs_out` quando a
banda comum é limitada pela **menor** das duas taxas, e o de pico exigia amplitude 0,5 na
decimação, onde 0,29 é o correto por construção. Corrigir o teste, e não o código, foi a
decisão certa nos dois casos, e está registrado porque o contrário teria virado um filtro
pior para satisfazer uma expectativa errada.

**A spec foi autorada, não aprovada.** `specs/dsp.md` §3, §5, §6, §9 e §10 saíram do
`PENDENTE` com o que o spike mediu, e o estado da spec diz "rascunho autorado pelo spike,
não aprovado". A Boundary desta TASK proíbe o spike de aprovar a própria spec; quem
aprova é o mantenedor, por `GOVERNANCE.md`. §4 continua a autorar.

**A medição tem variabilidade declarada.** Duas execuções completas do mesmo binário
divergiram ~20 % na coluna de CPU (3,4 % → 4,2 % em 44,1k→48k; 25,8 % → 31,6 % no par
extremo), com o MAC/s estável entre 283 M e 320 M. A evidência registra as duas leituras
e manda orçar pela mais lenta.

**O que esta TASK não provou:** nenhum dispositivo de áudio foi tocado. CPU e latência
são wall-clock de bancada sobre o buffer; não há DAC, não há callback RT, não há
`p99.9`. A ligação com o orçamento de callback da TASK-E01-02 é trabalho da
TASK-E04-03, e a validação acústica é da TASK-E04-10. Nenhuma flag de prontidão mudou.

**O que continua aberto e não é desta TASK:** o nome e o repositório da biblioteca de
resampling (D-DSP-08, ADR-011) e a posição do resampler no grafo (TASK-E04-03).
