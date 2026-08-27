# Fixture parity benchmark

Measured on 2026-08-23 on macOS. The C++ executable was built in Release mode
from `main` at `1fcb826`; Rust was built with `cargo build --release`. Each
meaningful fixture was run once, sequentially, with wall time measured by
`/usr/bin/time -p` and no competing BMAI processes.

| Fixture | C++ | Rust | Rust / C++ |
|---|---:|---:|---:|
| `Insult_in.txt` | <0.01s | <0.01s | n/a |
| `SurrenderDefault-Pass-in.txt` | <0.01s | <0.01s | n/a |
| `SurrenderOff-Attack-in.txt` | <0.01s | <0.01s | n/a |
| `SurrenderOff-Pass-in.txt` | <0.01s | <0.01s | n/a |
| `SurrenderOn-Attack-in.txt` | <0.01s | 0.01s | n/a |
| `SurrenderOn-Pass-in.txt` | <0.01s | <0.01s | n/a |
| `Value1_in.txt` | <0.01s | <0.01s | n/a |
| `Value2_in.txt` | <0.01s | <0.01s | n/a |
| `bmai_in.txt` | 5.99s | 20.42s | 3.41x |
| `bmsim_in.txt` | 17.19s | 71.96s | 4.19x |
| `bug11_in.txt` | 34.58s | 146.18s | 4.23x |
| `bug16_in.txt` | 163.89s | 677.68s | 4.13x |
| `bug55_a_in.txt` | <0.01s | <0.01s | n/a |
| `bug55_b_in.txt` | <0.01s | <0.01s | n/a |
| `test_in.txt` | <0.01s | <0.01s | n/a |

The four expensive rows execute the same search policies in both languages;
there are no fixture-specific direct-action paths. Material protocol output and
match results were identical. In addition, instrumented seeded comparisons
matched rollout boundaries, move counts, culling results, candidate scores, and
terminal RNG seeds. In the largest reserve search (`bug16_in.txt`), all five
candidates matched individually across 8,449,351 simulations.

The Rust port is currently about 3.4–4.2x slower on search-heavy fixtures.
Correctness and source-level parity were prioritized over representation-level
optimization; game clones and move/die vectors remain the clearest future
optimization targets.
