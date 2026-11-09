# vynamp-dsp

**English** · [Português](#português--vynamp-dsp)

Signal primitives in Rust, `std` only, with no crates.io dependencies: resampler,
dither, channel mapping, mixer, gain and crossfade.

> **Status: specification.** This library is not implemented. The repository holds the
> module specification, its task list and the tooling both are built with. It ships no
> running code and makes no measured claim.

Licence: `MIT OR Apache-2.0` — see [LICENSE](LICENSE), [LICENSE-MIT](LICENSE-MIT) and
[LICENSE-APACHE](LICENSE-APACHE).
Security: [GitHub Security Advisories](https://github.com/jimmykedlyn/vynamp-dsp/security/advisories/new)
— see [SECURITY.md](SECURITY.md).
Contributing: [CONTRIBUTING.md](CONTRIBUTING.md).

## Scope

Primitives. It knows nothing about a file, a device or a container — the same
category as `std`, so it depends on no other `vynamp-*` crate and is optional to
`decode`, `encode` and `output`.

## Builds on its own

```bash
cargo test
```

The agent contract is the umbrella's:
[AGENTS.md](https://github.com/jimmykedlyn/vynamp/blob/develop/AGENTS.md).

## CI

GitHub Actions is the only CI (ADR-014).
[`.github/workflows/ci.yml`](.github/workflows/ci.yml) runs `fmt`, `clippy`, `test` and
`machete` on every pull request to `develop`, and the job `gate` publishes the required
status **`CI Gate`**. It runs with `if: always()` and reads `needs.*.result`, so a dependency
that fails, is cancelled or never reports fails the gate — there is no path to a green gate
without the real checks. Actions are pinned by full commit SHA; a floating tag is an
unreviewed dependency.

# Português — vynamp-dsp

[English](#vynamp-dsp) · **Português**

Primitivas de sinal em Rust, só `std`, sem dependências do crates.io: resampler,
dither, mapeamento de canal, mixer, ganho e crossfade.

> **Estado: especificação.** Esta biblioteca não está implementada. O repositório contém
> a especificação do módulo, a lista de tarefas dele e o tooling com que os dois são
> construídos. Não entrega código em execução e não afirma resultado medido.

Licença: `MIT OR Apache-2.0` — ver [LICENSE](LICENSE), [LICENSE-MIT](LICENSE-MIT) e
[LICENSE-APACHE](LICENSE-APACHE).
Segurança: [GitHub Security Advisories](https://github.com/jimmykedlyn/vynamp-dsp/security/advisories/new)
— ver [SECURITY.md](SECURITY.md).
Contribuições: [CONTRIBUTING.md](CONTRIBUTING.md).

## Escopo

Primitivas. Não conhece arquivo, device nem container — a mesma categoria de
`std`, portanto não depende de nenhum crate `vynamp-*` e é opcional para
`decode`, `encode` e `output`.

## Compila sozinho

```bash
cargo test
```

O contrato do agente é o do umbrella:
[AGENTS.md](https://github.com/jimmykedlyn/vynamp/blob/develop/AGENTS.md).

## CI

O GitHub Actions é a única CI (ADR-014). O
[`.github/workflows/ci.yml`](.github/workflows/ci.yml) roda `fmt`, `clippy`, `test` e
`machete` em todo pull request para `develop`, e o job `gate` publica o status exigido
**`CI Gate`**. Ele roda com `if: always()` e lê `needs.*.result`, então uma dependência que
falha, é cancelada ou nunca reporta reprova o gate — não há caminho para um gate verde sem
as verificações reais. As actions são fixadas por SHA completo de commit; uma tag flutuante
é dependência não revisada.
