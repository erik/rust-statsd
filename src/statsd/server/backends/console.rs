use server::backend::Backend;
use server::buckets::Buckets;

use std::fmt::Default;

use extra::time;
use extra::stats::Stats;


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

    fn fmt_line<T: Default>(&mut self, key: &str, value: T) {
        let now = time::now_utc();
        println!("{} {} {}", now.rfc3339(), key, value)
    }
}


impl Backend for Console {
    fn flush_buckets(&mut self, buckets: &Buckets) -> () {
        let start = time::get_time().sec;

        for (key, value) in buckets.counters.iter() {
            let key = format!("counters.{}", *key);
            self.fmt_line(key, *value);
        }

        for (key, value) in buckets.gauges.iter() {
            let key = format!("gauges.{}", *key);
            self.fmt_line(key, *value);
        }

        for (key, values) in buckets.timers.iter() {
            let samples: &[f64] = *values;
            let key = format!("timers.{}", *key);

            println!("{key}.min {min}
{key}.max {max}
{key}.count {count}
{key}.mean {mean}
{key}.stddev {std}
{key}.upper_95 {max_threshold}",
                     key=key,
                     min=samples.min(),
                     max=samples.max(),
                     count=samples.len(),
                     mean=samples.mean(),
                     std=samples.std_dev(),
                     max_threshold=samples.percentile(95.0));
        }

        self.fmt_line("last_flush", self.last_flush_time);

        self.fmt_line("flush_time", self.last_flush_time);

        let end_time = time::get_time().sec;
        let flush_length = end_time - start;
        self.last_flush_length = flush_length;
        self.last_flush_time = end_time;
    }
}