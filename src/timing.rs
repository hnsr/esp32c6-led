use esp_hal::time::Instant;
use esp_println::println;

#[derive(Clone, Copy)]
struct Sample {
    interval_us: u64,
    work_us: u64,
}

pub struct LoopTimings<const N: usize> {
    label: &'static str,
    samples: [Sample; N],
    recorded: usize,
    previous_start_us: Option<u64>,
    previous_work_us: u64,
    reported: bool,
}

impl<const N: usize> LoopTimings<N> {
    pub const fn new(label: &'static str) -> Self {
        assert!(N > 0, "timing capture needs at least one sample");

        Self {
            label,
            samples: [Sample {
                interval_us: 0,
                work_us: 0,
            }; N],
            recorded: 0,
            previous_start_us: None,
            previous_work_us: 0,
            reported: false,
        }
    }

    /// Call at the beginning of each iteration.
    pub fn begin(&mut self) {
        // Once full, measurement becomes a cheap no-op.
        if self.recorded == N {
            return;
        }

        let now = now_us();

        // The previous iteration's interval becomes known now.
        // Pair it with that previous iteration's work duration.
        if let Some(previous_start) = self.previous_start_us {
            self.samples[self.recorded] = Sample {
                interval_us: now - previous_start,
                work_us: self.previous_work_us,
            };
            self.recorded += 1;
        }

        self.previous_start_us = Some(now);
    }

    /// Call after active work, before waiting for the next iteration.
    pub fn end_work(&mut self) {
        if self.recorded == N {
            return;
        }

        let start = self.previous_start_us
            .expect("call begin before end_work");

        self.previous_work_us = now_us() - start;
    }

    /// Print completed measurements once.
    ///
    /// Returns true only when printing happened, allowing the caller
    /// to reset its ticker after the deliberate reporting pause.
    pub fn report_if_ready(&mut self) -> bool {
        if self.recorded != N || self.reported {
            return false;
        }
        self.reported = true;

        // Report the greatest work duration with its associated interval.
        let slowest = self.samples
            .iter()
            .max_by_key(|sample| sample.work_us)
            .unwrap();

        println!(
            "[{}] slowest work: interval={} us, work={} us",
            self.label, slowest.interval_us, slowest.work_us,
        );

        // Sorting happens only after capture. It needs no heap, but
        // deliberately replaces chronological order with interval order.
        self.samples.sort_unstable_by_key(|sample| sample.interval_us);

        println!("[{}] longest intervals: interval_us, work_us", self.label);
        for sample in self.samples.iter().rev().take(8) {
            println!("{}, {}", sample.interval_us, sample.work_us);
        }

        true
    }
}

fn now_us() -> u64 {
    Instant::now().duration_since_epoch().as_micros()
}