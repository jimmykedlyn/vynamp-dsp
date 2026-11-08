# Spec — vynamp-dsp · dsp

| Campo | Valor |
|---|---|
| Módulo | `vynamp-dsp` |
| Crate(s) | a definir na autoria |
| Tipo | lib |
| Versão da spec | 0.1.0 |
| Data | 2026-11-08 |
| Estado | esqueleto — §3 a §10 pendentes de autoria |
| Dono | a definir |
| Depende de | nada — esta lib e a base da pilha |
| É dependido por | engine (obrigatorio); output e encode (opcionais, feature `resample`); restore |

> **Esqueleto gerado pelo passo 37.** §1, §2 e §7 foram decididos em
> `01-arvore-final.md` §2 e valem como norma. §3 a §6 e §8 a §10 exigem projeto de
> API e **não** foram escritos: inventá-los aqui seria fabricar a autoridade que o SDD
> exige. Estado só passa a `aprovada` com §5 preenchida.

## 1. Propósito

Primitivas de processamento de amostras: resampling, dither, mapa de canais, mixer, ganho, fade e crossfade.

**Não faz:** Nao conhece arquivo, container, dispositivo, formato de midia nem qualquer tipo de dominio.

## 2. Fronteira

| Permitido | Proibido |
|---|---|
| std; nenhuma dependencia de outro crate, nem opcional | qualquer crate `vynamp-*`, inclusive o proprio engine; qualquer tipo de dominio |

## 3. API pública

PENDENTE — projeto de API. Só o que estiver aqui é contrato.

## 4. Requisitos

PENDENTE — cada requisito com ID estável `DSP-REQ-nn` e REQ de produto,
rastreável até uma TASK e um teste.

## 5. Critérios de aceite

PENDENTE — Given/When/Then, coerente de fora. Enquanto esta seção estiver vazia,
nenhuma TASK deste módulo pode sair de `planned`.

## 6. Erros e cancelamento

PENDENTE — erro é dado, não `panic!`. Cancelamento é explícito, não timeout.

## 7. Fora de escopo

DSP musical (EQ, dynamics, reverb, tempo/pitch) — isso e do engine; formatos de arquivo — isso e do decode/encode

## 8. Dependências

Depende de: nada — esta lib e a base da pilha

É dependido por: engine (obrigatorio); output e encode (opcionais, feature `resample`); restore

## 9. Testes e fixtures

PENDENTE — oráculo numérico onde couber. Teste que depende de arquivo grande vive
atrás de `#[ignore]` com motivo escrito e variável de ambiente documentada.

## 10. Limitações conhecidas

PENDENTE — limitação declarada não reprova a entrega; limitação omitida reprova.
