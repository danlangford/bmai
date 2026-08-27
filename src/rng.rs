// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

/// The original BMAI generator. Keeping its integer operations exact is required
/// for seeded rollout parity with the C++ engine.
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Clone, Debug)]
pub struct BMC_RNG {
    m_seed: u32,
    m_trace_raw: bool,
    m_trace_hash: bool,
    m_trace_count: u64,
    m_trace_fingerprint: u64,
}

impl Default for BMC_RNG {
    fn default() -> Self {
        Self {
            m_seed: 78_904_497,
            m_trace_raw: std::env::var_os("BMAIR_TRACE_RAW_RNG").is_some(),
            m_trace_hash: std::env::var_os("BMAIR_TRACE_RNG_HASH").is_some(),
            m_trace_count: 0,
            m_trace_fingerprint: 0xcbf2_9ce4_8422_2325,
        }
    }
}

impl BMC_RNG {
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
        assert_eq!(rng.GetRand(), 1_150_470_880);
        assert_eq!(rng.GetRand(), 21_322_572);
        assert_eq!(rng.GetRand(), 1_886_182_202);
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
