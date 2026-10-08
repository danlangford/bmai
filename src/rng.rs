// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RngAlgorithm {
    LegacyParkMillerV1,
}

impl RngAlgorithm {
    pub const fn replay_id(self) -> &'static str {
        match self {
            Self::LegacyParkMillerV1 => "bmai-park-miller-16807-v1",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "legacy" | "park-miller" | "bmai-park-miller-16807-v1" => {
                Some(Self::LegacyParkMillerV1)
            }
            _ => None,
        }
    }
}

/// A closed enum keeps hot-loop dispatch static; exact integer operations keep
/// seeded legacy replays reproducible.
#[derive(Clone, Debug)]
pub struct Rng {
    algorithm: RngAlgorithm,
    seed: u32,
    trace_raw: bool,
    trace_hash: bool,
    trace_count: u64,
    trace_fingerprint: u64,
    native_stratum: Option<crate::native::NativeStratum>,
    script: Option<Box<DrawScript>>,
}

/// Replays chosen faces for each draw and records each draw's range, so a
/// caller can walk every outcome of an attack with its exact probability.
#[derive(Clone, Debug, Default)]
struct DrawScript {
    faces: Vec<u32>,
    ranges: Vec<u32>,
}

impl Default for Rng {
    fn default() -> Self {
        Self {
            algorithm: RngAlgorithm::LegacyParkMillerV1,
            seed: 78_904_497,
            trace_raw: std::env::var_os("BMAIR_TRACE_RAW_RNG").is_some(),
            trace_hash: std::env::var_os("BMAIR_TRACE_RNG_HASH").is_some(),
            trace_count: 0,
            trace_fingerprint: 0xcbf2_9ce4_8422_2325,
            native_stratum: None,
            script: None,
        }
    }
}

impl Rng {
    #[cfg(test)]
    pub(crate) fn untraced_default() -> Self {
        Self {
            algorithm: RngAlgorithm::LegacyParkMillerV1,
            seed: 78_904_497,
            trace_raw: false,
            trace_hash: false,
            trace_count: 0,
            trace_fingerprint: 0xcbf2_9ce4_8422_2325,
            native_stratum: None,
            script: None,
        }
    }

    pub(crate) fn from_native_stream(
        algorithm: RngAlgorithm,
        seed: crate::native::NativeStreamSeed,
        stratum: Option<crate::native::NativeStratum>,
    ) -> Self {
        Self {
            algorithm,
            seed: seed.legacy_park_miller_state(),
            trace_raw: false,
            trace_hash: false,
            trace_count: 0,
            trace_fingerprint: 0xcbf2_9ce4_8422_2325,
            native_stratum: stratum,
            script: None,
        }
    }

    /// Draws the given faces in order, then the lowest face of each later draw.
    pub(crate) fn scripted(faces: Vec<u32>) -> Self {
        Self {
            script: Some(Box::new(DrawScript {
                faces,
                ranges: Vec::new(),
            })),
            ..Self::default()
        }
    }

    /// Each draw's number of faces, in order, for a scripted generator.
    pub(crate) fn scripted_ranges(&self) -> Option<&[u32]> {
        self.script
            .as_deref()
            .map(|script| script.ranges.as_slice())
    }

    pub const fn algorithm(&self) -> RngAlgorithm {
        self.algorithm
    }

    pub const fn replay_id(&self) -> &'static str {
        self.algorithm.replay_id()
    }

    pub fn set_algorithm(&mut self, algorithm: RngAlgorithm) {
        self.algorithm = algorithm;
    }

    pub(crate) fn debug_seed(&self) -> u32 {
        self.seed
    }

    pub fn reseed(&mut self, seed: u32) {
        // A zero seed is time-based in C++. Callers that need reproducibility
        // must resolve it at the I/O boundary before invoking this method.
        self.seed = if seed >> 16 == 0 {
            seed | seed << 16
        } else {
            seed
        };
    }

    pub fn rand(&mut self) -> u32 {
        // A script enumerates rand_below's faces; any other draw breaks exactness.
        assert!(
            self.script.is_none(),
            "a scripted Rng only answers rand_below"
        );
        self.native_stratum = None;
        match self.algorithm {
            RngAlgorithm::LegacyParkMillerV1 => self.legacy_park_miller_rand(),
        }
    }

    fn legacy_park_miller_rand(&mut self) -> u32 {
        let mut lo = i64::from(self.seed & 0xffff) * 16_807;
        let mut hi = i64::from(self.seed >> 16) * 16_807 + (lo >> 16);
        lo = (lo & 0xffff) + (hi >> 15);
        hi = (hi & 0x7fff) + (lo >> 16);
        lo = (lo & 0xffff) + (hi >> 15);
        hi = ((hi & 0x7fff) << 16) + lo;
        self.seed = hi as u32;
        if self.trace_hash {
            self.trace_count += 1;
            self.trace_fingerprint ^= u64::from(self.seed);
            self.trace_fingerprint = self.trace_fingerprint.wrapping_mul(0x0000_0100_0000_01b3);
        }
        if self.trace_raw {
            eprintln!("RNG {}", self.seed);
        }
        self.seed
    }

    pub fn rand_below(&mut self, upper: u32) -> u32 {
        assert!(upper > 0, "rand_below requires a nonzero upper bound");
        if let Some(script) = self.script.as_mut() {
            let face = script.faces.get(script.ranges.len()).copied().unwrap_or(0);
            assert!(face < upper, "scripted face {face} is outside 0..{upper}");
            script.ranges.push(upper);
            return face;
        }
        let random = match self.algorithm {
            RngAlgorithm::LegacyParkMillerV1 => self.legacy_park_miller_rand(),
        };
        let Some(stratum) = self.native_stratum.as_mut() else {
            return random % upper;
        };

        let upper_u64 = u64::from(upper);
        let Some(next_radix) = stratum.radix.checked_mul(upper_u64) else {
            self.native_stratum = None;
            return random % upper;
        };
        let position = add_mod(
            stratum.index % next_radix,
            stratum.offset % next_radix,
            next_radix,
        );
        // position < radix * upper, so this digit is already below upper.
        let base_digit = position / stratum.radix;
        let lower_cell = position % stratum.radix;
        // A permutation of the faces, so full blocks stay exhaustive.
        let value = add_mod(base_digit, lower_cell % upper_u64, upper_u64);
        stratum.radix = next_radix;
        value as u32
    }

    pub fn rand_f32(&mut self) -> f32 {
        self.rand() as f32 / 0x8000_0000u32 as f32
    }
}

impl Drop for Rng {
    fn drop(&mut self) {
        if self.trace_hash {
            eprintln!("RNG_HASH {} {}", self.trace_count, self.trace_fingerprint);
        }
    }
}

/// `(a + b) % modulus` for `a, b < modulus`, without a 128-bit division.
fn add_mod(a: u64, b: u64, modulus: u64) -> u64 {
    debug_assert!(a < modulus && b < modulus);
    if a >= modulus - b {
        a - (modulus - b)
    } else {
        a + b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_mod_matches_a_128_bit_reference() {
        let reference =
            |a: u64, b: u64, m: u64| ((u128::from(a) + u128::from(b)) % u128::from(m)) as u64;
        for m in [1, 2, 3, 7, 1 << 32, (1 << 32) + 1, u64::MAX - 1, u64::MAX] {
            let near = |x: u64| [0, 1, x / 2, x.saturating_sub(2), x.saturating_sub(1)];
            for a in near(m).into_iter().filter(|&a| a < m) {
                // Includes the sum that lands exactly on the modulus.
                let edges = [m - 1 - a, (m - a) % m];
                for b in near(m).into_iter().chain(edges).filter(|&b| b < m) {
                    assert_eq!(add_mod(a, b, m), reference(a, b, m), "{a} + {b} mod {m}");
                }
            }
        }
        // A sum landing exactly on the modulus wraps to zero.
        assert_eq!(add_mod(3, 4, 7), 0);
    }

    #[test]
    fn default_sequence_is_stable() {
        let mut rng = Rng::default();
        assert_eq!(rng.algorithm(), RngAlgorithm::LegacyParkMillerV1);
        assert_eq!(rng.replay_id(), "bmai-park-miller-16807-v1");
        assert_eq!(rng.rand(), 1_150_470_880);
        assert_eq!(rng.rand(), 21_322_572);
        assert_eq!(rng.rand(), 1_886_182_202);
    }

    #[test]
    fn legacy_rng_names_select_the_same_versioned_stream_without_reseeding() {
        for name in ["legacy", "park-miller", "bmai-park-miller-16807-v1"] {
            assert_eq!(
                RngAlgorithm::parse(name),
                Some(RngAlgorithm::LegacyParkMillerV1)
            );
        }
        assert_eq!(RngAlgorithm::parse("unknown"), None);

        let mut rng = Rng::default();
        rng.reseed(17);
        let first = rng.rand();
        rng.set_algorithm(RngAlgorithm::LegacyParkMillerV1);
        let second = rng.rand();
        let mut uninterrupted = Rng::default();
        uninterrupted.reseed(17);
        assert_eq!(
            (first, second),
            (uninterrupted.rand(), uninterrupted.rand())
        );
    }

    #[test]
    fn native_strata_balance_the_first_bounded_sample() {
        let seed = crate::native::NativeStreamSeed {
            state: 123,
            stream: 456,
        };
        for offset in [0, 7, u64::MAX] {
            let mut counts = [0usize; 20];
            for index in 0..100 {
                let mut rng = Rng::from_native_stream(
                    RngAlgorithm::LegacyParkMillerV1,
                    seed,
                    Some(crate::native::NativeStratum {
                        index,
                        offset,
                        radix: 1,
                    }),
                );
                counts[rng.rand_below(20) as usize] += 1;
            }
            assert_eq!(counts, [5; 20]);
        }
    }

    #[test]
    fn native_strata_enumerate_two_die_outcomes_before_repeating() {
        let seed = crate::native::NativeStreamSeed {
            state: 123,
            stream: 456,
        };
        for (first_bound, second_bound) in [(6usize, 6usize), (4, 6), (6, 8)] {
            for offset in [0, 17, u64::MAX] {
                let mut outcomes = vec![vec![0usize; second_bound]; first_bound];
                for index in 0..(first_bound * second_bound) {
                    let mut rng = Rng::from_native_stream(
                        RngAlgorithm::LegacyParkMillerV1,
                        seed,
                        Some(crate::native::NativeStratum {
                            index: index as u64,
                            offset,
                            radix: 1,
                        }),
                    );
                    let first = rng.rand_below(first_bound as u32) as usize;
                    let second = rng.rand_below(second_bound as u32) as usize;
                    outcomes[first][second] += 1;
                }
                assert!(
                    outcomes.iter().flatten().all(|count| *count == 1),
                    "{first_bound}x{second_bound} outcomes were not exhaustive at offset {offset}"
                );
            }
        }
    }

    #[test]
    fn an_unbounded_native_draw_consumes_the_first_sample_stratum() {
        let seed = crate::native::NativeStreamSeed {
            state: 123,
            stream: 456,
        };
        let mut stratified = Rng::from_native_stream(
            RngAlgorithm::LegacyParkMillerV1,
            seed,
            Some(crate::native::NativeStratum {
                index: 999,
                offset: 999,
                radix: 1,
            }),
        );
        let mut ordinary = Rng::from_native_stream(RngAlgorithm::LegacyParkMillerV1, seed, None);

        assert_eq!(stratified.rand(), ordinary.rand());
        assert_eq!(stratified.rand_below(20), ordinary.rand_below(20));
    }

    /// The C++ test is statistical, so its sample count and tolerances stay.
    #[test]
    fn cpp_legacy_rng_distribution() {
        const SAMPLES: usize = 1_000_000;
        let mut rng = Rng::default();
        let mut bins = [0usize; 10];
        for _ in 0..SAMPLES {
            let sample = rng.rand_f32();
            assert!((0.0..1.0).contains(&sample));
            bins[(sample / 0.1) as usize] += 1;
        }
        let errors = bins.map(|count| (count as f64 / SAMPLES as f64 - 0.1).abs());
        let maximum_error = errors.into_iter().fold(0.0_f64, f64::max) / 0.1;
        let average_error = errors.into_iter().sum::<f64>() / bins.len() as f64;
        let stddev = (errors.into_iter().map(|error| error * error).sum::<f64>()
            / (average_error * average_error))
            .sqrt();
        assert!(maximum_error * 100.0 < 0.3, "maximum error {maximum_error}");
        assert!(stddev < 3.8, "stddev {stddev}");
    }
}
