# Working on BMAIR

BMAIR plays Button Men by ButtonWeavers' rules: the engine in
[danlangford/buttonmen](https://github.com/danlangford/buttonmen)
(`src/engine/`, with button recipes in `deploy/database/data.button.sql`) and
its skills page. When BMAIR and ButtonWeavers disagree, BMAIR changes; a
correct skill matters more than matching BMAIR's past output. The C++ BMAI
that BMAIR was ported from is history, not a reference.

Write a comment only when a reader needs a reason the code cannot give; keep
it short and explain why the code is there, never what it does or how it
works.

A rule change needs a focused test that names the ButtonWeavers behavior it
pins, and a row in `RULES.md` citing it. Check real buttons in
`data.button.sql` to judge whether an edge case matters; one no button reaches
goes under Known gaps in `RULES.md` rather than into the code.

The golden outputs in `tests/golden/` are change detectors, not a rules
oracle. An intentional change may regenerate them
(`BMAIR_UPDATE_GOLDEN=1 cargo test --release --test fixture_golden -- --include-ignored`),
and the diff belongs in review.

Search must stay deterministic for a complete replay key and must not depend
on the worker count. Top-level decisions draw native replay keys; inner levels
draw from their simulation's own generator. Scenario tests run at more than
one worker count.

Use Rust naming and idioms. Keep `RULES.md` and `CHANGELOG.md` current as work
is completed. Local commits are allowed. Never push unless the user explicitly
asks.
