# rcct contributor guidance

This repository is an early independent pure-Rust CAD-kernel project using
OCCT as a functional reference. It is licensed solely under Apache-2.0.
Do not introduce C++ bindings or native OCCT dependencies. Preserve explicit
numerical semantics and avoid unsupported compatibility claims.

Read README.md, CONTRIBUTING.md, and NOTICE.md before changing code. Run the
checks documented in README.md. Preserve the declared Rust minimum. Track
provenance without incorporating copied or translated LGPL-licensed OCCT code. The default
branch is `root`.

Only project-public material belongs in this repository. Review all outgoing
source, documentation, and history before publication. Do not commit credentials,
workstation paths, private project context, or unrelated content.
