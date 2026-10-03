// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RngAlgorithm {
    LegacyParkMillerV1,
}

impl RngAlgorithm {
    pub const fn ReplayId(self) -> &'static str {
        match self {
            Self::LegacyParkMillerV1 => "bmai-park-miller-16807-v1",
        }
    }

    pub fn Parse(value: &str) -> Option<Self> {
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
    m_algorithm: RngAlgorithm,
    m_seed: u32,
    m_trace_raw: bool,
    m_trace_hash: bool,
    m_trace_count: u64,
    m_trace_fingerprint: u64,
    m_native_stratum: Option<crate::native::NativeStratum>,
}

impl Default for Rng {
    fn default() -> Self {
        Self {
            m_algorithm: RngAlgorithm::LegacyParkMillerV1,
            m_seed: 78_904_497,
            m_trace_raw: std::env::var_os("BMAIR_TRACE_RAW_RNG").is_some(),
            m_trace_hash: std::env::var_os("BMAIR_TRACE_RNG_HASH").is_some(),
            m_trace_count: 0,
            m_trace_fingerprint: 0xcbf2_9ce4_8422_2325,
            m_native_stratum: None,
        }
    }
}

impl Rng {
    pub(crate) fn UntracedDefault() -> Self {
        Self {
            m_algorithm: RngAlgorithm::LegacyParkMillerV1,
            m_seed: 78_904_497,
            m_trace_raw: false,
            m_trace_hash: false,
            m_trace_count: 0,
            m_trace_fingerprint: 0xcbf2_9ce4_8422_2325,
            m_native_stratum: None,
        }
    }

    pub(crate) fn FromNativeStream(
        algorithm: RngAlgorithm,
        seed: crate::native::NativeStreamSeed,
        stratum: Option<crate::native::NativeStratum>,
    ) -> Self {
        Self {
            m_algorithm: algorithm,
            m_seed: seed.legacy_park_miller_state(),
            m_trace_raw: false,
            m_trace_hash: false,
            m_trace_count: 0,
            m_trace_fingerprint: 0xcbf2_9ce4_8422_2325,
            m_native_stratum: stratum,
        }
    }

    pub const fn Algorithm(&self) -> RngAlgorithm {
        self.m_algorithm
    }

    pub const fn ReplayId(&self) -> &'static str {
        self.m_algorithm.ReplayId()
    }

    pub fn SetAlgorithm(&mut self, algorithm: RngAlgorithm) {
        self.m_algorithm = algorithm;
    }

    pub(crate) fn DebugSeed(&self) -> u32 {
        self.m_seed
    }

    pub fn SRand(&mut self, seed: u32) {
        // A zero seed is time-based in C++. Callers that need reproducibility
        // must resolve it at the I/O boundary before invoking this method.
        self.m_seed = if seed >> 16 == 0 {
            seed | seed << 16
        } else {
            seed
        };
    }

    pub fn GetRand(&mut self) -> u32 {
        self.m_native_stratum = None;
        match self.m_algorithm {
            RngAlgorithm::LegacyParkMillerV1 => self.GetLegacyParkMillerRand(),
        }
    }

    fn GetLegacyParkMillerRand(&mut self) -> u32 {
        let mut lo = i64::from(self.m_seed & 0xffff) * 16_807;
        let mut hi = i64::from(self.m_seed >> 16) * 16_807 + (lo >> 16);
        lo = (lo & 0xffff) + (hi >> 15);
        hi = (hi & 0x7fff) + (lo >> 16);
        lo = (lo & 0xffff) + (hi >> 15);
        hi = ((hi & 0x7fff) << 16) + lo;
        self.m_seed = hi as u32;
        if self.m_trace_hash {
            self.m_trace_count += 1;
            self.m_trace_fingerprint ^= u64::from(self.m_seed);
            self.m_trace_fingerprint = self.m_trace_fingerprint.wrapping_mul(0x0000_0100_0000_01b3);
        }
        if self.m_trace_raw {
            eprintln!("RNG {}", self.m_seed);
        }
        self.m_seed
    }

    pub fn GetRandMax(&mut self, upper: u32) -> u32 {
        assert!(upper > 0, "GetRandMax requires a nonzero upper bound");
        let random = match self.m_algorithm {
            RngAlgorithm::LegacyParkMillerV1 => self.GetLegacyParkMillerRand(),
        };
        let Some(stratum) = self.m_native_stratum.as_mut() else {
            return random % upper;
        };

        let upper_u64 = u64::from(upper);
        let Some(next_radix) = stratum.radix.checked_mul(upper_u64) else {
            self.m_native_stratum = None;
            return random % upper;
        };
        let position = ((u128::from(stratum.index % next_radix)
            + u128::from(stratum.offset % next_radix))
            % u128::from(next_radix)) as u64;
        let base_digit = position / stratum.radix % upper_u64;
        let lower_cell = position % stratum.radix;
        // A permutation of the faces, so full blocks stay exhaustive.
        let value = (base_digit + lower_cell % upper_u64) % upper_u64;
        stratum.radix = next_radix;
        value as u32
    }

    pub fn GetFRand(&mut self) -> f32 {
        self.GetRand() as f32 / 0x8000_0000u32 as f32
    }
}

impl Drop for Rng {
    fn drop(&mut self) {
        if self.m_trace_hash {
            eprintln!(
                "RNG_HASH {} {}",
                self.m_trace_count, self.m_trace_fingerprint
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sequence_is_stable() {
        let mut rng = Rng::default();
        assert_eq!(rng.Algorithm(), RngAlgorithm::LegacyParkMillerV1);
        assert_eq!(rng.ReplayId(), "bmai-park-miller-16807-v1");
        assert_eq!(rng.GetRand(), 1_150_470_880);
        assert_eq!(rng.GetRand(), 21_322_572);
        assert_eq!(rng.GetRand(), 1_886_182_202);
    }

    #[test]
    fn legacy_rng_names_select_the_same_versioned_stream_without_reseeding() {
        for name in ["legacy", "park-miller", "bmai-park-miller-16807-v1"] {
            assert_eq!(
                RngAlgorithm::Parse(name),
                Some(RngAlgorithm::LegacyParkMillerV1)
            );
        }
        assert_eq!(RngAlgorithm::Parse("unknown"), None);

        let mut rng = Rng::default();
        rng.SRand(17);
        let first = rng.GetRand();
        rng.SetAlgorithm(RngAlgorithm::LegacyParkMillerV1);
        let second = rng.GetRand();
        let mut uninterrupted = Rng::default();
        uninterrupted.SRand(17);
        assert_eq!(
            (first, second),
            (uninterrupted.GetRand(), uninterrupted.GetRand())
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
                let mut rng = Rng::FromNativeStream(
                    RngAlgorithm::LegacyParkMillerV1,
                    seed,
                    Some(crate::native::NativeStratum {
                        index,
                        offset,
                        radix: 1,
                    }),
                );
                counts[rng.GetRandMax(20) as usize] += 1;
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
                    let mut rng = Rng::FromNativeStream(
                        RngAlgorithm::LegacyParkMillerV1,
                        seed,
                        Some(crate::native::NativeStratum {
                            index: index as u64,
                            offset,
                            radix: 1,
                        }),
                    );
                    let first = rng.GetRandMax(first_bound as u32) as usize;
                    let second = rng.GetRandMax(second_bound as u32) as usize;
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
        let mut stratified = Rng::FromNativeStream(
            RngAlgorithm::LegacyParkMillerV1,
            seed,
            Some(crate::native::NativeStratum {
                index: 999,
                offset: 999,
                radix: 1,
            }),
        );
        let mut ordinary = Rng::FromNativeStream(RngAlgorithm::LegacyParkMillerV1, seed, None);

        assert_eq!(stratified.GetRand(), ordinary.GetRand());
        assert_eq!(stratified.GetRandMax(20), ordinary.GetRandMax(20));
    }

    /// The C++ test is statistical, so its sample count and tolerances stay.
    #[test]
    fn cpp_legacy_rng_distribution() {
        const SAMPLES: usize = 1_000_000;
        let mut rng = Rng::default();
        let mut bins = [0usize; 10];
        for _ in 0..SAMPLES {
            let sample = rng.GetFRand();
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
