use std::fmt;
use std::from_str::FromStr;
use std::cmp;
use std::option::{Option, Some, None};


/// All known metric types that we can send or receive.
#[deriving(Eq)]
pub enum MetricKind {
    Counter(f64), // sample rate
    Gauge,
    Timer,
    Histogram
}


impl fmt::Show for MetricKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Gauge      => write!(f.buf, "Gauge"),
            Timer      => write!(f.buf, "Timer"),
            Histogram  => write!(f.buf, "Histogram"),
            Counter(s) => write!(f.buf, "Counter(s={})", s)
        }
    }
}


/// Internal represenation of a line sent by a statsd client.
#[deriving(Eq)]
pub struct Metric {
    pub kind: MetricKind,
    pub name: ~str,
    pub value: f64
}


impl fmt::Show for Metric {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f.buf, "{}({}) => {}", self.name, self.kind, self.value)
    }
}

impl FromStr for Metric {

    /// Valid message formats are:
    ///
    /// - `<str:metric_name>:<f64:value>|<str:type>`
    /// - `<str:metric_name>:<f64:value>|c|@<f64:sample_rate>`
    fn from_str(line: &str) -> Option<Metric> {
        // Pointer to position in line
        let mut idx = 0u;

        let name = match line.find(':') {
            // We don't want to allow blank key names.
            Some(pos) if pos != 0 => {
                idx += pos + 1;
                line.slice_to(pos).to_owned()
            },

            _ => return None
        };

        // Try to parse `<f64>|`, return None if no match is found.
        let value_opt = line.slice_from(idx).find('|').and_then(|loc| {
            FromStr::from_str(line.slice(idx, idx + loc)).map(|val| {
                idx += loc + 1;
                val
            })
        });

        let value = match value_opt {
            Some(v) => v,
            None => return None
        };

        let end_idx = cmp::min(idx + 3, line.len());

        let kind = match line.slice(idx, end_idx) {
            "c" => Counter(1.0),
            "ms" => Timer,
            "h" => Histogram,
            "g" => Gauge,
            // Sampled counter
            "c|@" => match FromStr::from_str(line.slice_from(end_idx)) {
                Some(sample) => Counter(sample),
                None => return None
            },

            // Unknown type
            _ => return None
        };

        Some(Metric { kind: kind, name: name, value: value })
    }
}
