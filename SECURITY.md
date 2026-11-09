# Security

Report vulnerabilities privately through GitHub private vulnerability reporting
(Security -> Report a vulnerability) rather than in public issues. A public issue is
visible before a fix exists, and this repository takes untrusted input.

Design guarantees: no `unsafe` outside the exceptions this repository records in
`CONTRIBUTING.md`, explicit limits on every input this crate parses, and no network,
filesystem or thread dependency on the real-time path. Never commit a secret; the
local gate in `CONTRIBUTING.md` runs before every commit and the CI Gate before every
merge.
