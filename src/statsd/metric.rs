use std::fmt;
use std::from_str::FromStr;
use std::num;
use std::option::{Option, Some, None};


pub enum MetricKind {
    Counter(f64), // sample rate
    Gauge,
    Timer,
    Histogram,
    Meter
}


impl fmt::Default for MetricKind {
    fn fmt(k: &MetricKind, f: &mut fmt::Formatter) {
        match *k {
            Gauge      => write!(f.buf, "Gauge"),
            Timer      => write!(f.buf, "Timer"),
            Histogram  => write!(f.buf, "Histogram"),
            Meter      => write!(f.buf, "Meter"),
            Counter(s) => write!(f.buf, "Counter(s={})", s)
        }
    }
}


pub struct Metric {
    kind: MetricKind,
    name: ~str,
    value: f64
}


impl fmt::Default for Metric {
    fn fmt(m: &Metric, f: &mut fmt::Formatter) {
        write!(f.buf, "{}({}) => {}", m.name, m.kind, m.value)
    }
}

impl FromStr for Metric {

    /// Valid message formats are:
    ///    <str:metric_name>:<f64:value>|<str:type>
    ///    <str:metric_name>:<f64:value>|c|@<f64:sample_rate>
    fn from_str(line: &str) -> Option<Metric> {
        // Pointer to position in line
        let mut idx = 0u;

        let name = match line.find(':') {
            Some(pos) => {
                idx += pos + 1;

                line.slice_to(pos).to_owned()
            },

            None => return None
        };

        let value: f64 = match line.slice_from(idx).find('|') {
            Some(pos) => {
                let val: Option<f64> = FromStr::from_str(line.slice(idx, idx + pos));
                idx += pos + 1;

                match val {
                    Some(x) => x,
                    None    => return None
                }
            },

            None => return None
        };

        let end_idx = num::min(idx + 3, line.len());

        let kind = match line.slice(idx, end_idx) {
            "c" => Counter(1.0),
            "ms" => Timer,
            "h" => Histogram,
            "m" => Meter,
            "g" => Gauge,
            // Sampled counter
            "c|@" => {
                let sample: f64 = match FromStr::from_str(line.slice_from(end_idx + 1)) {
                    Some(x) => x,
                    None => return None
                };

                Counter(sample)
            },

            // Unknown type
            _ => return None
        };

        Some(Metric {kind: kind, name: name, value: value})
    }
}
