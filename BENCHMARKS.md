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

Those absolute Rust measurements predate the structural optimization work and
are retained as the original baseline. A same-machine paired study on
2026-08-27 measured the individual retained changes using user CPU time:

| Change | `bmai_in` | `bmsim_in` | `bug11_in` | `bug16_in` |
|---|---:|---:|---:|---:|
| stack-backed temporary die indices | about 21–22% faster | 13.6% faster | 17.2% faster | 30.8% faster |
| direct same-length simulation restoration | 9.1% faster | 3.2% faster | 3.4% faster | 10.0% faster |
| fat LTO versus Thin LTO | 7.1% faster | 0.5% slower | 0.6% slower | 19.2% faster |

Candidate-list reuse was tested and rejected because it regressed `bug16_in`
user CPU by about 18%. PGO improved its three training fixtures by roughly
5–9% but was also rejected because the untrained `bug16_in` case regressed by
about 1.4%. Exact all-fixture material output and RNG fingerprints passed after
the retained changes; see `PARITY.md` for the gate evidence.

## Preview-release artifact comparison

Measured on 2026-08-28 on an Intel Mac using the x86_64 slices from GitHub
Actions. The C++ reference was the universal macOS Release artifact from
Konstant PR #82 at `4813530`; Rust was the macOS x86_64 Release artifact at
`f2f56db`. All 24 `*in*.txt` fixtures had materially identical output.

| Fixture | C++ | Rust | Rust / C++ |
|---|---:|---:|---:|
| `bmai_in.txt` | 6.23s | 7.30s | 1.17x |
| `bmsim_in.txt` | 18.19s | 19.16s | 1.05x |
| `bug11_in.txt` | 43.94s | 46.13s | 1.05x |
| `bug16_in.txt` | 139.95s | 226.90s | 1.62x |
| all 24 fixtures | 209.00s | 300.16s | 1.44x |

The aggregate difference is dominated by `bug16_in.txt`; the other meaningful
searches are within 5–17% of C++. Sub-10ms protocol fixtures are omitted from
the table because process startup dominates their ratios.
