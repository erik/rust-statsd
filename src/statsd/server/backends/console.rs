//! Print out a YAML representation of the server's buckets to the attached
//! terminal on every flush event.

use server::backend::Backend;
use server::buckets::Buckets;

use std::fmt;

use collections::hashmap::HashMap;
use time;

use test::stats::Stats;


pub struct Console {
    last_flush_time: i64,
    last_flush_length: i64
}


impl Console {
    pub fn new() -> Console {
        Console {
            last_flush_time: 0,
            last_flush_length: 0,
        }
    }

    fn fmt_line<T: fmt::Show>(&mut self, key: &str, value: T) {
        println!("    {}: {}", key, value)
    }
}


/// Code common to both Histograms and Timers (because they're the same)
fn print_stats(hist: &HashMap<~str, ~[f64]>) {
    for (key, values) in hist.iter() {
        let samples: &[f64] = *values;

        println!("    {key}:
      min: {min}
      max: {max}
      count: {count}
      mean: {mean}
      stddev: {std}
      upper_95: {max_threshold}",
                 key=*key,
                 min=samples.min(),
                 max=samples.max(),
                 count=samples.len(),
                 mean=samples.mean(),
                 std=samples.std_dev(),
                 max_threshold=samples.percentile(95.0));
    }
}


impl Backend for Console {
    fn flush_buckets(&mut self, buckets: &Buckets) -> () {
        println!("{}:", time::now().rfc3339());

        println!("  counters:");
        for (key, value) in buckets.counters.iter() {
            self.fmt_line(*key, *value);
        }

        println!("  gauges:");
        for (key, value) in buckets.gauges.iter() {
            self.fmt_line(*key, *value);
        }

        println!("  timers:");
        print_stats(&buckets.timers);

        println!("  histograms:");
        print_stats(&buckets.histograms);
    }
}