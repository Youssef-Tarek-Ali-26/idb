use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatencyStats {
    pub count: u64,
    pub total_micros: u128,
    pub max_micros: u128,
}

impl Default for LatencyStats {
    fn default() -> Self {
        Self {
            count: 0,
            total_micros: 0,
            max_micros: 0,
        }
    }
}

impl LatencyStats {
    pub fn record(&mut self, micros: u128) {
        self.count += 1;
        self.total_micros += micros;
        self.max_micros = self.max_micros.max(micros);
    }

    pub fn avg_micros(&self) -> u128 {
        if self.count == 0 {
            return 0;
        }
        self.total_micros / self.count as u128
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EngineMetrics {
    pub ingest_latency: LatencyStats,
    pub query_latency: LatencyStats,
    pub wal_bytes_written: u64,
    pub wal_entries: u64,
    pub hot_record_count: u64,
    pub cold_record_count: u64,
}

impl EngineMetrics {
    pub fn record_ingest(&mut self, micros: u128, wal_bytes: u64, wal_entries: u64) {
        self.ingest_latency.record(micros);
        self.wal_bytes_written += wal_bytes;
        self.wal_entries += wal_entries;
    }

    pub fn record_query(&mut self, micros: u128) {
        self.query_latency.record(micros);
    }

    pub fn update_record_counts(&mut self, hot: u64, cold: u64) {
        self.hot_record_count = hot;
        self.cold_record_count = cold;
    }

    pub fn storage_amplification(&self) -> f64 {
        if self.hot_record_count == 0 {
            return 1.0;
        }
        self.cold_record_count as f64 / self.hot_record_count as f64
    }
}
