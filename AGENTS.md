# Rust port parity contract

The `rust` branch is a behavioral port of the C++ implementation on `main`.
Do not call the port complete merely because the shipped fixtures select the
same final actions.

Full parity requires all of the following:

1. Every `tests/fixtures/*in*.txt` input produces materially identical C++ and
   Rust output after removing only timestamps, elapsed-time fields, throughput,
   and other timing-only diagnostics.
2. Every meaningful upstream C++ test has an explicitly mapped Rust test. The
   Rust test must assert the same behavior and run by default whenever its cost
   is practical. Any deliberate exclusion must be named and justified.
3. Every externally reachable C++ parser command and public game/AI behavior is
   either implemented compatibly in Rust or listed as an explicit, approved
   exclusion. Do not silently omit commands or replace algorithms with direct
   policies.
4. The source-level feature matrix in `PARITY.md` maps C++ mechanics and search
   paths to Rust code and evidence. Untested code-path inspection is required;
   fixture coverage alone is insufficient.
5. Mechanics absent from the shipped fixtures have focused Rust tests and,
   where observable through the protocol, seeded C++/Rust differential cases.
6. Search parity includes candidate enumeration/order, simulation counts,
   culling, scores, state transitions, and RNG consumption—not only final
   actions.
7. Structural parity uses Rust-native equivalents of meaningful C++ design:
   align state ownership and boundedness, reuse simulation storage, enumerate
   candidates directly in C++ order, preserve cached-state update points, and
   retain corresponding search control flow. Do not require unsafe Rust,
   C++ syntax, ABI compatibility, unions, raw pointers, or literal `memcpy`
   where safe idiomatic Rust expresses the same lifecycle efficiently.
8. A behaviorally equivalent reconstruction is not enough when it adds a
   materially different hot-path algorithm (for example generate/sort/filter/
   regenerate instead of direct ordered enumeration). Such differences must be
   ported or explicitly justified in `PARITY.md`.

Keep `PARITY.md` current as work is completed. A checked item must cite concrete
evidence. Preserve C++ naming and control-flow correspondence where practical;
optimization must not weaken parity evidence. Local commits are allowed. Never
push unless the user explicitly asks.
