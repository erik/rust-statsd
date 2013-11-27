use metric;
use metric::Metric;

use std::hashmap::HashMap;

use extra::time;


pub struct Buckets {
    counters:   HashMap<~str, f64>,
    gauges:     HashMap<~str, f64>,
    histograms: HashMap<~str, f64>,
    meters:     HashMap<~str, f64>,
    timers:     HashMap<~str, ~[f64]>,

    server_start_time: time::Timespec,
    last_message: time::Timespec,
    bad_messages: uint
}


impl Buckets {
    pub fn new() -> Buckets {
        Buckets {
            counters: HashMap::new(),
            gauges: HashMap::new(),
            histograms: HashMap::new(),
            meters: HashMap::new(),
            timers: HashMap::new(),

            server_start_time: time::get_time(),
            last_message: time::get_time(),
            bad_messages: 0
        }
    }

    pub fn flush(&mut self) {
        // TODO: write me.
    }

    pub fn handle_management_cmd(&mut self, line: &str) -> ~str {
        let mut words = line.words();

        match words.next().unwrap_or("") {
            "delcounters" => ~"",
            "deltimers" => ~"",
            "gauges" => ~"",
            "health" => ~"",
            "stats" => ~"",
            "timers" => ~"",
            x => format!("ERROR: Unknown command: {}", x)
        }
    }

    pub fn add_metric(&mut self, metric: Metric) {
        let key = metric.name.clone();
        let val = metric.value;

        match metric.kind {
            metric::Counter(sample_rate) => {
                self.counters
                    .insert_or_update_with(key, 0.0, |_, v| {
                        *v += val * (1.0 / sample_rate)
                    }
                );
            },
            metric::Gauge => {
                self.gauges
                    .insert(key, val);
            },
            metric::Timer => {
                self.timers
                    .insert_or_update_with(key, ~[], |_, v| v.push(val));
            },
            metric::Histogram => warn!("Histogram not implemented: {}", metric),
            metric::Meter => warn!("Meter not implemented: {}", metric)
        }

        self.last_message = time::get_time();
    }
}
