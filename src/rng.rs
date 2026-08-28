// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

/// Stable identifier for a replayable RNG implementation.
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BME_RNG_ALGORITHM {
    /// BMAI's Park-Miller minimal-standard LCG plus its custom seed expansion.
    LEGACY_PARK_MILLER_V1,
}

impl BME_RNG_ALGORITHM {
    pub const fn ReplayId(self) -> &'static str {
        match self {
            Self::LEGACY_PARK_MILLER_V1 => "bmai-park-miller-16807-v1",
        }
    }

    pub fn Parse(value: &str) -> Option<Self> {
        match value {
            "legacy" | "park-miller" | "bmai-park-miller-16807-v1" => {
                Some(Self::LEGACY_PARK_MILLER_V1)
            }
            _ => None,
        }
    }
}

/// Versioned RNG dispatcher. The closed enum keeps dispatch statically
/// optimizable in hot search loops while leaving a deliberate seam for native
/// generators with different state and stream-splitting behavior.
///
/// The initial implementation is the original BMAI Park-Miller generator.
/// Keeping its integer operations exact is required for seeded C++ parity.
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Clone, Debug)]
pub struct BMC_RNG {
    m_algorithm: BME_RNG_ALGORITHM,
    m_seed: u32,
    m_trace_raw: bool,
    m_trace_hash: bool,
    m_trace_count: u64,
    m_trace_fingerprint: u64,
}

impl Default for BMC_RNG {
    fn default() -> Self {
        Self {
            m_algorithm: BME_RNG_ALGORITHM::LEGACY_PARK_MILLER_V1,
            m_seed: 78_904_497,
            m_trace_raw: std::env::var_os("BMAIR_TRACE_RAW_RNG").is_some(),
            m_trace_hash: std::env::var_os("BMAIR_TRACE_RNG_HASH").is_some(),
            m_trace_count: 0,
            m_trace_fingerprint: 0xcbf2_9ce4_8422_2325,
        }
    }
}

impl BMC_RNG {
    pub const fn Algorithm(&self) -> BME_RNG_ALGORITHM {
        self.m_algorithm
    }

    pub const fn ReplayId(&self) -> &'static str {
        self.m_algorithm.ReplayId()
    }

    pub fn SetAlgorithm(&mut self, algorithm: BME_RNG_ALGORITHM) {
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
        match self.m_algorithm {
            BME_RNG_ALGORITHM::LEGACY_PARK_MILLER_V1 => self.GetLegacyParkMillerRand(),
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
        self.GetRand() % upper
    }

    pub fn GetFRand(&mut self) -> f32 {
        self.GetRand() as f32 / 0x8000_0000u32 as f32
    }
}

impl Drop for BMC_RNG {
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
        let mut rng = BMC_RNG::default();
        assert_eq!(rng.Algorithm(), BME_RNG_ALGORITHM::LEGACY_PARK_MILLER_V1);
        assert_eq!(rng.ReplayId(), "bmai-park-miller-16807-v1");
        assert_eq!(rng.GetRand(), 1_150_470_880);
        assert_eq!(rng.GetRand(), 21_322_572);
        assert_eq!(rng.GetRand(), 1_886_182_202);
    }

    #[test]
    fn legacy_rng_names_select_the_same_versioned_stream_without_reseeding() {
        for name in ["legacy", "park-miller", "bmai-park-miller-16807-v1"] {
            assert_eq!(
                BME_RNG_ALGORITHM::Parse(name),
                Some(BME_RNG_ALGORITHM::LEGACY_PARK_MILLER_V1)
            );
        }
        assert_eq!(BME_RNG_ALGORITHM::Parse("unknown"), None);

        let mut rng = BMC_RNG::default();
        rng.SRand(17);
        let first = rng.GetRand();
        rng.SetAlgorithm(BME_RNG_ALGORITHM::LEGACY_PARK_MILLER_V1);
        let second = rng.GetRand();
        let mut uninterrupted = BMC_RNG::default();
        uninterrupted.SRand(17);
        assert_eq!(
            (first, second),
            (uninterrupted.GetRand(), uninterrupted.GetRand())
        );
    }

    /// Port of LegacyMembers.TestRNG. The C++ test is statistical rather than
    /// sequence-based, so retain its sample count and tolerances verbatim.
    #[test]
    fn cpp_legacy_rng_distribution() {
        const SAMPLES: usize = 1_000_000;
        let mut rng = BMC_RNG::default();
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
