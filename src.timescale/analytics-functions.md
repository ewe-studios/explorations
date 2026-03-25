# TimescaleDB Toolkit: Analytics Functions Deep Dive

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.timescale/timescaledb-toolkit/`

---

## Table of Contents

1. [Overview](#overview)
2. [Time-Weighted Average](#time-weighted-average)
3. [UddSketch (Percentile Approximation)](#uddsketch-percentile-approximation)
4. [HyperLogLog (Count Distinct)](#hyperloglog-count-distinct)
5. [Statistical Aggregates](#statistical-aggregates)
6. [LTTB Downsampling](#lttb-downsampling)
7. [Gap Filling and Interpolation](#gap-filling-and-interpolation)
8. [Counter Aggregation](#counter-aggregation)
9. [State Aggregation](#state-aggregation)
10. [Rust Implementation Guide](#rust-implementation-guide)

---

## Overview

### What is TimescaleDB Toolkit?

TimescaleDB Toolkit is a PostgreSQL extension providing specialized analytics functions for time-series data:

```
┌────────────────────────────────────────────────────────────┐
│                  TIMESCALEDB TOOLKIT                         │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  TIME-SERIES ANALYTICS                                      │
│  - Time-weighted average (LOCF, Linear)                    │
│  - Counter aggregation (rate of change)                    │
│  - State aggregation (state timeline)                      │
│  - Heartbeat aggregation (liveness detection)              │
│                                                             │
│  STATISTICAL FUNCTIONS                                      │
│  - Stats agg (1D and 2D regression)                        │
│  - UddSketch (percentile approximation)                    │
│  - T-Digest (percentile approximation)                     │
│  - HyperLogLog (count distinct)                            │
│                                                             │
│  DOWNSAMPLING                                               │
│  - LTTB (Largest Triangle Three Buckets)                   │
│  - ASAP (Automatic Smoothing and Pattern detection)        │
│                                                             │
│  UTILITY FUNCTIONS                                          │
│  - Gap filling and interpolation                           │
│  - Time bucketing                                          │
│  - Candlestick/OHLC aggregation                            │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

### Two-Step Aggregation Pattern

Toolkit uses a two-step aggregation pattern for efficiency:

```sql
-- Step 1: Aggregate into intermediate state
SELECT time_weight('LOCF', time, value) as tw_state
FROM metrics;

-- Step 2: Extract final value
SELECT average(tw_state) as time_weighted_avg
FROM (
    SELECT time_weight('LOCF', time, value) as tw_state
    FROM metrics
) subquery;

-- Combined (most common)
SELECT average(time_weight('LOCF', time, value))
FROM metrics;
```

**Benefits:**
- **Composability**: Intermediate states can be combined with `rollup()`
- **Continuous Aggregates**: States can be stored and re-aggregated
- **Memory Efficiency**: Compact state representation

---

## Time-Weighted Average

### Problem Statement

Traditional averages are misleading for irregularly sampled data:

```
Traditional Average:
  Values: [10, 20, 10, 20, 15]
  Average: 15

But if timestamps are:
  Time:   [0, 1, 2, 3, 4] minutes  (evenly spaced)
  -> Average 15 is correct

If timestamps are:
  Time:   [0, 1, 2, 3, 10] minutes  (uneven!)
  -> Value 15 persisted for 7 minutes
  -> Traditional average overweights the transient
```

### Solution: Time-Weighted Average

```
Time-Weighted Average (LOCF method):

  Value
   │
20 │       ┌───┐           ┌───┐
   │       │   │           │   │
15 │       │   │           │   ├──────
   │       │   │       ┌───┘   │
10 │   ┌───┘   │   ┌───┘       │
   │   │       │   │           │
   └───┴───────┴───┴───────────┴──────> Time
       0   1   2   3       10

  Area = 10*1 + 20*1 + 10*1 + 20*1 + 15*7
       = 10 + 20 + 10 + 20 + 105
       = 165

  Time-Weighted Avg = 165 / 10 = 16.5
```

### Implementation

```rust
/// Time-weighted average state
#[derive(Debug, Clone)]
pub struct TimeWeightSummary {
    /// First observed point
    first: TimeValue,
    /// Last observed point
    last: TimeValue,
    /// Accumulated weighted sum
    weighted_sum: f64,
    /// Interpolation method
    method: TimeWeightMethod,
}

#[derive(Debug, Clone, Copy)]
pub struct TimeValue {
    pub time: i64,  // Microseconds since epoch
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimeWeightMethod {
    Linear,
    Locf,
}

impl TimeWeightSummary {
    pub fn new(method: TimeWeightMethod) -> Self {
        Self {
            first: TimeValue { time: 0, value: 0.0 },
            last: TimeValue { time: 0, value: 0.0 },
            weighted_sum: 0.0,
            method,
        }
    }

    pub fn push(&mut self, time: i64, value: f64) {
        if self.first.time == 0 {
            // First value
            self.first = TimeValue { time, value };
            self.last = TimeValue { time, value };
            return;
        }

        // Calculate weighted contribution
        let duration = time - self.last.time;
        let contribution = match self.method {
            TimeWeightMethod::Linear => {
                // Trapezoidal area
                (self.last.value + value) * duration as f64 / 2.0
            }
            TimeWeightMethod::Locf => {
                // Rectangle area (last value carried forward)
                self.last.value * duration as f64
            }
        };

        self.weighted_sum += contribution;
        self.last = TimeValue { time, value };
    }

    pub fn average(&self) -> Option<f64> {
        let total_duration = self.last.time - self.first.time;
        if total_duration <= 0 {
            return None;
        }

        Some(self.weighted_sum / total_duration as f64)
    }

    /// Combine two states (for parallel aggregation)
    pub fn combine(&mut self, other: &TimeWeightSummary) {
        assert_eq!(self.method, other.method);

        // Add a contribution for the gap between states
        let gap_duration = other.first.time - self.last.time;
        let gap_contribution = match self.method {
            TimeWeightMethod::Linear => {
                (self.last.value + other.first.value) * gap_duration as f64 / 2.0
            }
            TimeWeightMethod::Locf => {
                self.last.value * gap_duration as f64
            }
        };

        self.weighted_sum += gap_contribution + other.weighted_sum;
        self.last = other.last;
    }
}
```

### SQL Usage

```sql
-- Basic usage
SELECT
  measure_id,
  average(time_weight('LOCF', ts, val)) as tw_avg
FROM sensor_data
GROUP BY measure_id;

-- With time buckets
SELECT
  time_bucket('5 min', ts) as bucket,
  measure_id,
  average(time_weight('LOCF', ts, val)) as tw_avg
FROM sensor_data
GROUP BY 1, 2
ORDER BY 1, 2;

-- With continuous aggregates
CREATE MATERIALIZED VIEW sensor_tw_5min
WITH (timescaledb.continuous) AS
SELECT
  measure_id,
  time_bucket('5 min', ts) as bucket,
  time_weight('LOCF', ts, val) as tw_state
FROM sensor_data
GROUP BY 1, 2;

-- Re-aggregate from continuous aggregate
SELECT
  measure_id,
  time_bucket('1 hour', bucket) as hour,
  average(rollup(tw_state)) as hourly_tw_avg
FROM sensor_tw_5min
GROUP BY 1, 2;
```

### Accessor Functions

```sql
-- Get the weighted sum
SELECT weighted_sum(time_weight('LOCF', ts, val)) FROM data;

-- Get duration covered
SELECT duration(time_weight('LOCF', ts, val)) FROM data;

-- Get first/last values
SELECT first_val(time_weight('LOCF', ts, val)) FROM data;
SELECT last_val(time_weight('LOCF', ts, val)) FROM data;

-- Get method
SELECT method(time_weight('LOCF', ts, val)) FROM data;
```

---

## UddSketch (Percentile Approximation)

### Algorithm Overview

UddSketch is an adaptive histogram for percentile estimation:

```
┌────────────────────────────────────────────────────────────┐
│                    UDDSKETCH ALGORITHM                       │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  Key Insight:                                               │
│  - Use logarithmically-sized buckets                       │
│  - Guarantees max relative error for percentile estimates  │
│  - Adapts when too many buckets needed                     │
│                                                             │
│  Bucket Structure:                                          │
│  ┌─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┐        │
│  │  0  │  1  │  2  │  3  │  4  │  5  │ ... │  N  │        │
│  └─────┴─────┴─────┴─────┴─────┴─────┴─────┴─────┘        │
│    │     │     │                       │                   │
│    v     v     v                       v                   │
│   [0,1) [1,2) [2,4)                   [2^N,2^N+1)          │
│                                                             │
│  Logarithmic sizing: each bucket covers [γ^i, γ^i+1)        │
│  where γ = 1 + max_error                                   │
│                                                             │
│  When buckets exceed max:                                   │
│  - Combine adjacent buckets (double their width)           │
│  - Error bound increases but all percentiles estimable     │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

### Implementation

```rust
/// UddSketch state
pub struct UddSketch {
    /// Logarithmic bucket size factor
    gamma: f64,
    /// Maximum number of buckets
    max_buckets: usize,
    /// Bucket counts (index -> count)
    buckets: BTreeMap<i32, u64>,
    /// Total count of values
    count: u64,
    /// Sum of all values (for mean)
    sum: f64,
    /// Current error bound (may increase if buckets combined)
    current_error: f64,
}

impl UddSketch {
    pub fn new(max_buckets: usize, max_error: f64) -> Self {
        let gamma = 1.0 + max_error;
        Self {
            gamma,
            max_buckets,
            buckets: BTreeMap::new(),
            count: 0,
            sum: 0.0,
            current_error: max_error,
        }
    }

    /// Get bucket index for a value
    fn get_bucket_index(&self, value: f64) -> i32 {
        if value <= 0.0 {
            return i32::MIN;
        }
        // bucket_index = floor(log_gamma(value))
        (value.ln() / self.gamma.ln()).floor() as i32
    }

    /// Add a value to the sketch
    pub fn add(&mut self, value: f64) {
        if value < 0.0 {
            // Handle negative values (implementation detail)
            return;
        }

        let idx = self.get_bucket_index(value);
        *self.buckets.entry(idx).or_insert(0) += 1;
        self.count += 1;
        self.sum += value;

        // Check if we need to combine buckets
        if self.buckets.len() > self.max_buckets {
            self.conservative_collapse();
        }
    }

    /// Combine buckets to reduce count
    fn conservative_collapse(&mut self) {
        // Combine pairs of adjacent buckets
        let mut new_buckets = BTreeMap::new();

        let mut prev_idx: Option<i32> = None;
        for (&idx, &count) in &self.buckets {
            if let Some(prev) = prev_idx {
                if idx == prev + 1 {
                    // Combine with previous bucket
                    let combined_count = new_buckets.remove(&prev).unwrap_or(0) + count;
                    new_buckets.insert(prev, combined_count);
                    prev_idx = None;
                    continue;
                }
            }
            prev_idx = Some(idx);
            *new_buckets.entry(idx).or_insert(0) += count;
        }

        self.buckets = new_buckets;
        self.current_error *= 2.0;  // Error doubles when combining
    }

    /// Estimate percentile
    pub fn approx_percentile(&self, percentile: f64) -> f64 {
        if self.count == 0 {
            return f64::NAN;
        }

        let target_rank = percentile * self.count as f64;
        let mut cumulative = 0u64;

        for (&idx, &count) in &self.buckets {
            cumulative += count;
            if cumulative >= target_rank as u64 {
                // Return bucket midpoint
                let bucket_start = self.gamma.powi(idx);
                let bucket_end = self.gamma.powi(idx + 1);
                return (bucket_start + bucket_end) / 2.0;
            }
        }

        // Return last bucket
        let (&last_idx, _) = self.buckets.last().unwrap();
        self.gamma.powi(last_idx)
    }

    /// Get estimated rank of a value
    pub fn approx_percentile_rank(&self, value: f64) -> f64 {
        if self.count == 0 {
            return f64::NAN;
        }

        let idx = self.get_bucket_index(value);
        let mut cumulative = 0u64;

        for (&bucket_idx, &count) in &self.buckets {
            if bucket_idx >= idx {
                break;
            }
            cumulative += count;
        }

        cumulative as f64 / self.count as f64
    }

    /// Combine two sketches
    pub fn combine(&mut self, other: &UddSketch) {
        assert_eq!(self.max_buckets, other.max_buckets);

        for (&idx, &count) in &other.buckets {
            *self.buckets.entry(idx).or_insert(0) += count;
        }
        self.count += other.count;
        self.sum += other.sum;

        if self.buckets.len() > self.max_buckets {
            self.conservative_collapse();
        }
    }
}
```

### SQL Usage

```sql
-- Basic percentile estimation
SELECT
  approx_percentile(0.50, uddsketch(100, 0.005, value)) as p50,
  approx_percentile(0.95, uddsketch(100, 0.005, value)) as p95,
  approx_percentile(0.99, uddsketch(100, 0.005, value)) as p99
FROM metrics;

-- With grouping
SELECT
  sensor_id,
  approx_percentile(0.95, uddsketch(100, 0.01, response_time)) as p95_latency
FROM api_metrics
GROUP BY sensor_id;

-- Get sketch statistics
SELECT
  num_vals(uddsketch(100, 0.01, value)) as count,
  mean(uddsketch(100, 0.01, value)) as mean,
  error(uddsketch(100, 0.01, value)) as max_error
FROM metrics;

-- Estimate rank of a value
SELECT approx_percentile_rank(100.0, uddsketch(100, 0.01, value))
FROM metrics;
```

### Parameter Selection

| Parameter | Effect | Recommendation |
|-----------|--------|----------------|
| `max_buckets` | Memory vs accuracy | 100-1000 |
| `max_error` | Initial error bound | 0.001-0.01 (0.1%-1%) |

**Error Formula:**
```
std_error ≈ 1.04 / sqrt(max_buckets)
```

For 100 buckets: std_error ≈ 1.04 / 10 = 10.4%

---

## HyperLogLog (Count Distinct)

### Algorithm Overview

HyperLogLog provides approximate count distinct:

```
┌────────────────────────────────────────────────────────────┐
│                    HYPERLOGLOG ALGORITHM                     │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  Key Insight:                                               │
│  - Hash values uniformly distribute bits                   │
│  - Leading zeros in hash follow geometric distribution     │
│  - Max leading zeros observed ~ log2(cardinality)          │
│                                                             │
│  Structure:                                                 │
│  ┌─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┐        │
│  │  0  │  1  │  2  │  3  │  4  │  5  │ ... │  63 │        │
│  └─────┴─────┴─────┴─────┴─────┴─────┴─────┴─────┘        │
│    │     │     │                       │                   │
│    v     v     v                       v                   │
│   max   max   max                   max                   │
│   zeros zeros zeros                 zeros                 │
│                                                             │
│  With 64 registers (precision=6):                          │
│  - Use first 6 bits of hash to select register             │
│  - Count leading zeros in remaining bits                   │
│  - Store max zeros seen for each register                  │
│                                                             │
│  Final estimate:                                            │
│  cardinality ≈ α * m^2 / (Σ 2^(-register[i]))              │
│  where m = num_registers, α = bias constant                │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

### Implementation

```rust
/// HyperLogLog state
pub struct HyperLogLog {
    /// Number of registers (must be power of 2)
    num_registers: usize,
    /// Precision (log2 of num_registers)
    precision: u8,
    /// Register array
    registers: Vec<u8>,
    /// Cached estimate
    cached_estimate: Option<u64>,
}

impl HyperLogLog {
    pub fn new(precision: u8) -> Self {
        assert!(precision >= 4 && precision <= 18);
        let num_registers = 1 << precision;
        Self {
            num_registers,
            precision,
            registers: vec![0; num_registers],
            cached_estimate: None,
        }
    }

    /// Hash a value and get 64-bit hash
    fn hash(&self, value: &[u8]) -> u64 {
        // Use xxHash or similar
        xxhash::xxh64(value, 0)
    }

    /// Add a value
    pub fn add(&mut self, value: &[u8]) {
        let hash = self.hash(value);

        // Use first `precision` bits to select register
        let register_idx = (hash >> (64 - self.precision)) as usize;

        // Count leading zeros in remaining bits
        let remaining = (hash << self.precision) | (1 << (self.precision - 1));
        let leading_zeros = remaining.leading_zeros() as u8;

        // Update register if we found more zeros
        if leading_zeros > self.registers[register_idx] {
            self.registers[register_idx] = leading_zeros;
            self.cached_estimate = None;
        }
    }

    /// Estimate cardinality
    pub fn count(&self) -> u64 {
        if let Some(estimate) = self.cached_estimate {
            return estimate;
        }

        // Harmonic mean of 2^(-register[i])
        let sum: f64 = self.registers.iter()
            .map(|&r| (1.0 / (1u64 << r)) as f64)
            .sum();

        let alpha = self.get_alpha();
        let raw_estimate = alpha * (self.num_registers as f64).powi(2) / sum;

        // Apply corrections
        let estimate = self.apply_corrections(raw_estimate);

        self.cached_estimate = Some(estimate);
        estimate
    }

    fn get_alpha(&self) -> f64 {
        match self.num_registers {
            16 => 0.673,
            32 => 0.697,
            64 => 0.709,
            _ => 0.7213 / (1.0 + 1.079 / self.num_registers as f64),
        }
    }

    fn apply_corrections(&self, estimate: f64) -> u64 {
        // Small range correction
        if estimate <= 2.5 * self.num_registers as f64 {
            let zeros = self.registers.iter().filter(|&&r| r == 0).count();
            if zeros > 0 {
                return (self.num_registers as f64 * (self.num_registers as f64 / zeros as f64).ln()) as u64;
            }
        }

        // Large range correction (for 32-bit hashes)
        if estimate > (1.0 / 30.0) * (1u64 << 32) as f64 {
            return (-((1u64 << 32) as f64) * (1.0 - estimate / (1u64 << 32) as f64).ln()) as u64;
        }

        estimate as u64
    }

    /// Combine two HLLs
    pub fn combine(&mut self, other: &HyperLogLog) {
        assert_eq!(self.num_registers, other.num_registers);

        for i in 0..self.num_registers {
            self.registers[i] = self.registers[i].max(other.registers[i]);
        }
        self.cached_estimate = None;
    }
}
```

### SQL Usage

```sql
-- Basic count distinct
SELECT distinct_count(hyperloglog(64, user_id))
FROM events;

-- Compare with exact count
SELECT
  COUNT(DISTINCT user_id) as exact,
  distinct_count(hyperloglog(64, user_id)) as approx,
  stderror(hyperloglog(64, user_id)) as error
FROM events;

-- Union of multiple HLLs
SELECT distinct_count(rollup(hll))
FROM (
    SELECT hyperloglog(64, user_id) as hll
    FROM events
    WHERE date = '2024-01-01'
    UNION ALL
    SELECT hyperloglog(64, user_id)
    FROM events
    WHERE date = '2024-01-02'
) subquery;

-- With continuous aggregates
CREATE MATERIALIZED VIEW daily_users
WITH (timescaledb.continuous) AS
SELECT
  time_bucket('1 day', ts) as day,
  hyperloglog(1024, user_id) as hll
FROM events
GROUP BY 1;

-- Query weekly unique users
SELECT
  time_bucket('7 day', day) as week,
  distinct_count(rollup(hll)) as weekly_users
FROM daily_users
GROUP BY 1;
```

### Error Characteristics

| Registers | Precision | Std Error | Memory |
|-----------|-----------|-----------|--------|
| 16 | 4 | 26.0% | 12 bytes |
| 64 | 6 | 13.0% | 48 bytes |
| 256 | 8 | 6.5% | 192 bytes |
| 1024 | 10 | 3.25% | 768 bytes |
| 4096 | 12 | 1.63% | 3072 bytes |

---

## Statistical Aggregates

### 1D Statistics

```rust
/// Running statistics for 1D data
pub struct StatsAgg1D {
    count: u64,
    mean: f64,
    m2: f64,   // Sum of squared differences from mean
    m3: f64,   // For skewness
    m4: f64,   // For kurtosis
    min: f64,
    max: f64,
    sum: f64,
}

impl StatsAgg1D {
    pub fn new() -> Self {
        Self {
            count: 0,
            mean: 0.0,
            m2: 0.0,
            m3: 0.0,
            m4: 0.0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
            sum: 0.0,
        }
    }

    pub fn add(&mut self, value: f64) {
        self.count += 1;
        self.sum += value;
        self.min = self.min.min(value);
        self.max = self.max.max(value);

        // Welford's online algorithm for variance
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;

        self.m2 += delta * delta2;
        self.m3 += delta * delta2 * delta2;
        self.m4 += delta * delta2 * delta2 * delta2;
    }

    pub fn variance_population(&self) -> f64 {
        if self.count < 1 {
            return f64::NAN;
        }
        self.m2 / self.count as f64
    }

    pub fn variance_sample(&self) -> f64 {
        if self.count < 2 {
            return f64::NAN;
        }
        self.m2 / (self.count - 1) as f64
    }

    pub fn stddev_population(&self) -> f64 {
        self.variance_population().sqrt()
    }

    pub fn stddev_sample(&self) -> f64 {
        self.variance_sample().sqrt()
    }

    pub fn skewness(&self) -> f64 {
        if self.count < 3 || self.m2 == 0.0 {
            return f64::NAN;
        }
        let n = self.count as f64;
        (n * (n - 1.0)).sqrt() / (n - 2.0) * self.m3 / self.m2.powf(1.5)
    }

    pub fn kurtosis(&self) -> f64 {
        if self.count < 4 || self.m2 == 0.0 {
            return f64::NAN;
        }
        let n = self.count as f64;
        (n - 1.0) / ((n - 2.0) * (n - 3.0)) * ((n + 1.0) * n * self.m4 / (self.m2 * self.m2) - 3.0 * (n - 1.0))
    }
}
```

### 2D Regression Statistics

```rust
/// Running statistics for 2D regression (y on x)
pub struct StatsAgg2D {
    count: u64,
    mean_x: f64,
    mean_y: f64,
    cxx: f64,  // Sum of (x - mean_x)^2
    cyy: f64,  // Sum of (y - mean_y)^2
    cxy: f64,  // Sum of (x - mean_x)(y - mean_y)
}

impl StatsAgg2D {
    pub fn add(&mut self, x: f64, y: f64) {
        self.count += 1;

        let dx = x - self.mean_x;
        self.mean_x += dx / self.count as f64;

        let dy = y - self.mean_y;
        self.mean_y += dy / self.count as f64;

        let dx2 = x - self.mean_x;
        let dy2 = y - self.mean_y;

        self.cxx += dx * dx2;
        self.cyy += dy * dy2;
        self.cxy += dx * dy2;
    }

    pub fn slope(&self) -> f64 {
        if self.cxx == 0.0 {
            return f64::NAN;
        }
        self.cxy / self.cxx
    }

    pub fn intercept(&self) -> f64 {
        self.mean_y - self.slope() * self.mean_x
    }

    pub fn x_intercept(&self) -> f64 {
        if self.slope() == 0.0 {
            return f64::NAN;
        }
        -self.intercept() / self.slope()
    }

    pub fn correlation(&self) -> f64 {
        if self.cxx == 0.0 || self.cyy == 0.0 {
            return f64::NAN;
        }
        self.cxy / (self.cxx * self.cyy).sqrt()
    }

    pub fn covariance_population(&self) -> f64 {
        if self.count == 0 {
            return f64::NAN;
        }
        self.cxy / self.count as f64
    }

    pub fn r_squared(&self) -> f64 {
        self.correlation().powi(2)
    }
}
```

### SQL Usage

```sql
-- 1D statistics
SELECT
  average(stats_agg(value)) as avg,
  stddev(stats_agg(value)) as stddev,
  variance(stats_agg(value)) as variance,
  skewness(stats_agg(value)) as skewness,
  kurtosis(stats_agg(value)) as kurtosis,
  min_val(stats_agg(value)) as min,
  max_val(stats_agg(value)) as max
FROM metrics;

-- 2D regression (y vs x)
SELECT
  slope(stats_agg(y, x)) as slope,
  intercept(stats_agg(y, x)) as intercept,
  x_intercept(stats_agg(y, x)) as x_intercept,
  corr(stats_agg(y, x)) as correlation,
  covar_pop(stats_agg(y, x)) as covariance,
  determination_coeff(stats_agg(y, x)) as r_squared
FROM paired_data;

-- With time buckets
SELECT
  time_bucket('1 hour', ts) as hour,
  slope(stats_agg(temperature, humidity)) as temp_humidity_slope
FROM sensor_data
GROUP BY 1;
```

---

## LTTB Downsampling

### Algorithm Overview

Largest Triangle Three Buckets preserves visual appearance:

```
┌────────────────────────────────────────────────────────────┐
│                    LTTB ALGORITHM                           │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  Goal: Downsample N points to M points while preserving    │
│  visual appearance of the line chart.                      │
│                                                             │
│  Algorithm:                                                 │
│  1. Keep first and last points                             │
│  2. Divide remaining points into (M-2) buckets             │
│  3. For each bucket:                                        │
│     a. Calculate average of points in bucket               │
│     b. Find point that forms largest triangle              │
│        with previous selected point and average            │
│                                                             │
│  Triangle Area:                                             │
│                    C (candidate point)                     │
│                   / \                                      │
│                  /   \                                     │
│                 /     \                                    │
│                /       \                                   │
│               /         \                                  │
│              /           \                                 │
│             /             \                                │
│            A───────────────B                               │
│         (prev point)   (avg point)                         │
│                                                             │
│  Area = |Ax(By - Cy) + Bx(Cy - Ay) + Cx(Ay - By)| / 2      │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

### Implementation

```rust
/// Largest Triangle Three Buckets downsampling
pub fn lttb_downsample(
    times: &[i64],
    values: &[f64],
    resolution: usize,
) -> Vec<(i64, f64)> {
    let n = times.len();
    assert_eq!(n, values.len());

    if resolution >= n {
        return (0..n).map(|i| (times[i], values[i])).collect();
    }

    if resolution <= 2 {
        return vec![(times[0], values[0]), (times[n - 1], values[n - 1])];
    }

    let mut result = Vec::with_capacity(resolution);
    result.push((times[0], values[0]));

    let bucket_size = (n - 2) as f64 / (resolution - 2) as f64;

    let mut prev_idx = 0;

    for bucket in 0..(resolution - 2) {
        let bucket_start = (1 + bucket as f64 * bucket_size) as usize;
        let bucket_end = (1 + (bucket + 1) as f64 * bucket_size) as usize;
        let bucket_end = bucket_end.min(n - 1);

        // Calculate average of points in bucket
        let mut avg_x = 0.0;
        let mut avg_y = 0.0;
        let bucket_count = bucket_end - bucket_start;

        for i in bucket_start..bucket_end {
            avg_x += times[i] as f64;
            avg_y += values[i];
        }
        avg_x /= bucket_count as f64;
        avg_y /= bucket_count as f64;

        // Find point with largest triangle area
        let mut max_area = 0.0;
        let mut max_idx = bucket_start;

        for i in bucket_start..bucket_end {
            // Triangle area with previous point, average, and candidate
            let area = triangle_area(
                times[prev_idx] as f64, values[prev_idx],
                avg_x, avg_y,
                times[i] as f64, values[i],
            );

            if area > max_area {
                max_area = area;
                max_idx = i;
            }
        }

        result.push((times[max_idx], values[max_idx]));
        prev_idx = max_idx;
    }

    result.push((times[n - 1], values[n - 1]));
    result
}

fn triangle_area(
    ax: f64, ay: f64,
    bx: f64, by: f64,
    cx: f64, cy: f64,
) -> f64 {
    ((ax * (by - cy) + bx * (cy - ay) + cx * (ay - by)) / 2.0).abs()
}
```

### SQL Usage

```sql
-- Downsample to 100 points
SELECT time, value
FROM unnest((
    SELECT lttb(time, value, 100)
    FROM high_frequency_data
));

-- With time bucketing for visualization
SELECT
  time_bucket('1 hour', time) as hour,
  (unnest(lttb(time, value, 50))).*
FROM sensor_data
WHERE time >= NOW() - INTERVAL '7 days'
GROUP BY 1;
```

---

## Gap Filling and Interpolation

### Gap Filling with LOCF

```rust
/// Gap filling with Last Observation Carried Forward
pub struct GapFillLocf {
    bucket_start: i64,
    bucket_end: i64,
    bucket_size: i64,
    last_value: Option<f64>,
    current_bucket: i64,
}

impl GapFillLocf {
    pub fn new(bucket_start: i64, bucket_end: i64, bucket_size: i64) -> Self {
        Self {
            bucket_start,
            bucket_end,
            bucket_size,
            last_value: None,
            current_bucket: bucket_start,
        }
    }

    pub fn feed(&mut self, bucket: i64, value: f64) -> Vec<(i64, f64)> {
        let mut result = Vec::new();

        // Fill gaps with last value
        while self.current_bucket < bucket {
            if let Some(last) = self.last_value {
                result.push((self.current_bucket, last));
            }
            self.current_bucket += self.bucket_size;
        }

        // Add actual value
        result.push((bucket, value));
        self.last_value = Some(value);
        self.current_bucket = bucket + self.bucket_size;

        result
    }

    pub fn finish(mut self) -> Vec<(i64, f64)> {
        let mut result = Vec::new();

        while self.current_bucket < self.bucket_end {
            if let Some(last) = self.last_value {
                result.push((self.current_bucket, last));
            }
            self.current_bucket += self.bucket_size;
        }

        result
    }
}
```

### Linear Interpolation

```rust
/// Gap filling with linear interpolation
pub struct GapFillInterpolate {
    bucket_start: i64,
    bucket_end: i64,
    bucket_size: i64,
    prev_point: Option<(i64, f64)>,
    pending_points: Vec<(i64, f64)>,
}

impl GapFillInterpolate {
    pub fn new(bucket_start: i64, bucket_end: i64, bucket_size: i64) -> Self {
        Self {
            bucket_start,
            bucket_end,
            bucket_size,
            prev_point: None,
            pending_points: Vec::new(),
        }
    }

    fn interpolate(start: (i64, f64), end: (i64, f64), x: i64) -> f64 {
        let t = (x - start.0) as f64 / (end.0 - start.0) as f64;
        start.1 + t * (end.1 - start.1)
    }

    pub fn feed(&mut self, bucket: i64, value: f64) -> Vec<(i64, f64)> {
        self.pending_points.push((bucket, value));

        if self.pending_points.len() < 2 {
            return Vec::new();
        }

        let mut result = Vec::new();
        let points = std::mem::take(&mut self.pending_points);

        if let (Some(prev), Some(curr)) = (points.first(), points.get(1)) {
            // Fill gaps between prev and curr
            let mut current_bucket = ((prev.0 / self.bucket_size) + 1) * self.bucket_size;

            while current_bucket < curr.0 {
                let interpolated = Self::interpolate(*prev, *curr, current_bucket);
                result.push((current_bucket, interpolated));
                current_bucket += self.bucket_size;
            }

            result.push(*curr);
            self.pending_points = vec![*curr];
        }

        result
    }
}
```

### SQL Usage

```sql
-- Gap fill with LOCF
SELECT
  time_bucket_gapfill('1 hour', time, start, end) as bucket,
  locf(min(value)) as value
FROM metrics
GROUP BY 1
ORDER BY 1;

-- Gap fill with interpolation
SELECT
  time_bucket_gapfill('1 hour', time, start, end) as bucket,
  interpolate(avg(value)) as value
FROM metrics
GROUP BY 1
ORDER BY 1;

-- Combined: LOCF for some columns, interpolate for others
SELECT
  time_bucket_gapfill('1 hour', time, start, end) as bucket,
  locf(min(temperature)) as temp,
  interpolate(avg(humidity)) as humidity
FROM sensor_data
GROUP BY 1
ORDER BY 1;
```

---

## Counter Aggregation

### Problem: Counter Wrapping and Resets

Counters (like network bytes) monotonically increase until reset:

```
Counter Value
    │
    │         ╱ (reset here)
    │        ╱
    │       ╱
    │      │
    │     ╱│
    │    ╱ │
    │   ╱  │
    │  ╱   │
    │ ╱    │
    │╱     │
    └──────┴──────────> Time

Rate calculation must handle:
1. Counter resets (value drops)
2. Counter wraps (uint64 overflow)
3. Missing data points
```

### Implementation

```rust
/// Counter aggregation state
pub struct CounterAgg {
    prev_time: Option<i64>,
    prev_value: Option<f64>,
    accumulated_rate: f64,
    has_reset: bool,
    max_value: f64,  // For detecting wraps
}

impl CounterAgg {
    pub fn new() -> Self {
        Self {
            prev_time: None,
            prev_value: None,
            accumulated_rate: 0.0,
            has_reset: false,
            max_value: 0.0,
        }
    }

    pub fn add(&mut self, time: i64, value: f64) {
        if let (Some(prev_time), Some(prev_value)) = (self.prev_time, self.prev_value) {
            let time_delta = time - prev_time;

            if time_delta > 0 {
                // Check for reset or wrap
                if value < prev_value {
                    // Counter reset or wrap
                    self.has_reset = true;
                    // Assume wrap if close to max, otherwise reset
                    let wrap_amount = if value > prev_value * 0.9 {
                        f64::MAX - prev_value + value
                    } else {
                        0.0  // Reset, no additional rate
                    };
                    self.accumulated_rate += wrap_amount / time_delta as f64;
                } else {
                    // Normal increment
                    self.accumulated_rate += (value - prev_value) / time_delta as f64;
                }
            }
        }

        self.max_value = self.max_value.max(value);
        self.prev_time = Some(time);
        self.prev_value = Some(value);
    }

    pub fn rate(&self) -> f64 {
        self.accumulated_rate
    }

    pub fn delta(&self) -> Option<f64> {
        if self.has_reset {
            None  // Can't determine true delta with resets
        } else {
            self.prev_value
        }
    }
}
```

### SQL Usage

```sql
-- Calculate counter rate
SELECT
  time_bucket('5 min', time) as bucket,
  delta(counter_agg(time, bytes_in)) as bytes_delta,
  rate(counter_agg(time, bytes_in)) as bytes_per_sec
FROM network_stats
GROUP BY 1;

-- Handle counter resets
SELECT
  time_bucket('1 hour', time) as bucket,
  coalesce(
    delta(counter_agg(time, requests)),
    0  -- Assume reset, start from 0
  ) as requests
FROM web_stats
GROUP BY 1;
```

---

## Rust Implementation Guide

### Crate Structure

```rust
// Cargo.toml
[package]
name = "timescaledb-toolkit-rs"
version = "0.1.0"

[dependencies]
pgrx = "0.12"
serde = { version = "1.0", features = ["derive"] }
rand = "0.8"

// lib.rs
pub mod time_weight;
pub mod uddsketch;
pub mod hyperloglog;
pub mod stats_agg;
pub mod lttb;
pub mod counter_agg;

// Use flat_serialize for efficient serialization
// (as in the original toolkit)
```

### Serialization for PostgreSQL

```rust
/// Trait for types that can be serialized to PostgreSQL
pub trait PgSerializable: Sized {
    fn to_datum(&self, fcinfo: pg_sys::FunctionCallInfo) -> pg_sys::Datum;
    fn from_datum(datum: pg_sys::Datum) -> Option<Self>;
}

/// Example: TimeWeightSummary serialization
impl PgSerializable for TimeWeightSummary {
    fn to_datum(&self, fcinfo: pg_sys::FunctionCallInfo) -> pg_sys::Datum {
        unsafe {
            let size = size_of::<TimeWeightSummary>();
            let ptr = pg_sys::palloc(size) as *mut TimeWeightSummary;
            *ptr = self.clone();
            pg_sys::Datum::from(ptr)
        }
    }

    fn from_datum(datum: pg_sys::Datum) -> Option<Self> {
        if datum.is_null() {
            return None;
        }
        unsafe {
            let ptr = datum.pointer_cast::<TimeWeightSummary>();
            Some((*ptr).clone())
        }
    }
}
```

---

## Related Documentation

- [TimescaleDB Architecture](./timescaledb-architecture.md)
- [Query Optimization](./query-optimization.md)
- [Rust Implementation](./rust-revision.md)
