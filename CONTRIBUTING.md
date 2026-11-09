# Contributing

## Global rules

- No crates.io dependencies at all, development dependencies included. Only `std`.
- `#![forbid(unsafe_code)]` everywhere; an exception has to be argued in the PR and
  recorded here before it lands.
- Licence: `MIT OR Apache-2.0` (see [LICENSE](LICENSE)).
- Commits end with the `Co-Authored-By:` trailer of the agent that authored them.
- Conventional Commits (`feat`, `fix`, `test`, `docs`, `chore`, `build`, `ci`).
- Branch names follow the umbrella's rule: `{feature,fix,chore,hotfix}/task-eNN-NN-<agent>-<slug>`.
  `develop` and `main` are protected; a direct push is refused by the server, not by
  convention.

## Boundary -- what this repository is not

Primitivas de sinal. Sem estado de aplicacao, sem I/O, sem dispositivo.

That boundary is not advisory: a change that crosses it belongs in the repository that
owns that concern, and the umbrella `jimmykedlyn/vynamp` is where the decision to move
it is recorded.

## Clean-room

Anything reimplemented from a public specification is recorded under `docs/sources/`.
Never copy code from another implementation. Permissively licensed sources may be
studied with credit; copyleft sources follow the umbrella's `LEGAL.md`.

## TDD

Write the failing test first, watch it fail for the expected reason, then implement.
Bit-exact claims are verified bit-exact; anything measured is compared against an
independent oracle, never against "it looks right".

## Local gate (required before every commit)

```
cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked
```

`Cargo.lock` is committed.

## Security

Report privately through GitHub Security Advisories; see [SECURITY.md](SECURITY.md).
