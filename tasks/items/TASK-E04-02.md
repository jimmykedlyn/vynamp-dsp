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
| Critérios da spec | pendente — `specs/dsp.md` §5 |
| Fase | M0 |
| Prioridade | P0 |
| Estado | planned |
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
  aprova o §5 é o mantenedor, não a evidência desta TASK.

## Antes de começar

- [ ] Spec do módulo aprovada e o critério de aceite citado já existe nela
- [ ] Worktree própria criada (`cargo xtask repo worktree task-e04-02-claude-<slug>`)
- [ ] Fixtures/hardware necessários disponíveis ou limitação registrada
- [ ] Nenhuma TASK em execução tocando os mesmos arquivos

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

- [ ] Todos os critérios de aceite com evidência
- [ ] `cargo fmt --check` e `cargo clippy --all-targets -- -D warnings` limpos
- [ ] `cargo test` verde; testes ignorados listados com motivo
- [ ] Spec do módulo atualizada se a implementação revelou algo novo
- [ ] `backlog.json` com o estado correto
- [ ] Revisão feita por contexto limpo, não pelo implementador
- [ ] PR aberto com template preenchido; CI Gate verde no SHA final
## Notas de execução

Medido em 2026-11-09. **Esta TASK não foi executada: não havia o que executar.**

| Medição | Resultado |
|---|---|
| Tamanho do repo | `wc -l src/*.rs` → **6 linha(s)**, contendo só `pub const VERSION` e o comentário de módulo. `src/` tem 1 arquivo(s) |
| Worktree suja? | Não: `git status --short` limpo antes da medição |
| Especificação do módulo | `libs/dsp/specs/dsp.md` — **esqueleto — §3 a §10 pendentes de autoria** |

O critério de aceite abaixo exige o spike do resampler próprio (modos balanced/high, razão inteira e não inteira, impulso e sweep com alias/CPU/latência medidos) e o caminho de bypass inteiro sample-exact contra o caminho com dither. Nenhum dos dois existe neste
repositório: não há código para compilar, medir, interromper ou comparar bit a bit.
Registrar uma nota de execução dizendo que o critério foi verificado seria falsear
a evidência — então a TASK permanece `planned` e esta nota registra o motivo
medido, não um resultado.

**Por que o motivo é o repo vazio e não uma limitação de ambiente:** uma limitação
de ambiente (hardware ausente, linker ausente) aparece como comando que falha. Aqui
os comandos não têm alvo: não há função a chamar, nenhum crate de terceiro no
`[dependencies]` para exercer. O que falta é implementação.

**O que destrava:** escrever o resampler e o dither em `libs/dsp` a partir de ADR-011 e do catálogo de DSP; a spec `specs/dsp.md` precisa sair do estado de esqueleto primeiro, porque §3 a §10 (contrato, formatos, limites) não existem para implementar contra

**Custo de registrar assim:** se o mantenedor esperava que o spike M0 desta TASK
fosse escrito nesta rodada do passo 42, ele não foi — o passo 42 mede e registra o
estado, não implementa. Nenhum estado mudou: `tasks/backlog.json` continua com esta
TASK em `planned`.
