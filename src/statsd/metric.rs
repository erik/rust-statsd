use std::fmt;
use std::from_str::FromStr;
use std::num;
use std::option::{Option, Some, None};


#[deriving(Eq)]
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


#[deriving(Eq)]
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
                // We don't want to allow blank key names.
                if pos == 0 {
                    return None
                }

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
            "c|@" => match FromStr::from_str(line.slice_from(end_idx)) {
                Some(sample) => Counter(sample),
                None => return None
            },

            // Unknown type
            _ => return None
        };

        Some(Metric {kind: kind, name: name, value: value})
    }
}


#[cfg(test)]
mod tests {
    use super::Metric;

    #[test]
    fn test_from_str_valid_input() {
        let in_out_map = ~[
            ("f.o.o:1|c",      Metric {kind: super::Counter(1.0), name: ~"f.o.o", value: 1.0}),
            ("foo:9.1|c|@0.5", Metric {kind: super::Counter(0.5), name: ~"foo", value: 9.1}),
            ("foo:2|c|@1",     Metric {kind: super::Counter(1.0), name: ~"foo", value: 2.0}),
            ("foo:2|c|@123",   Metric {kind: super::Counter(123.0), name: ~"foo", value: 2.0}),
            ("foo:12.3|ms",    Metric {kind: super::Timer, name: ~"foo", value: 12.3}),
            ("foo:1|ms",       Metric {kind: super::Timer, name: ~"foo", value: 1.0}),
            ("foo:1|h",        Metric {kind: super::Histogram, name: ~"foo", value: 1.0}),
            ("foo:1.23|h",     Metric {kind: super::Histogram, name: ~"foo", value: 1.23}),
            ("foo:1|g",        Metric {kind: super::Gauge, name: ~"foo", value: 1.0}),
            ("foo:1.23|g",     Metric {kind: super::Gauge, name: ~"foo", value: 1.23})
        ];

        for (input, expected) in in_out_map.move_iter() {
            let actual: Metric = FromStr::from_str(input).unwrap();

            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn test_from_str_invalid_input() {
        let inputs = ~[
            "f",
            "f:",
            "f:c",
            "f:1.0|",
            "f:1.0|c@",
            ":|@",
            ":1.0|c"
        ];

        for input in inputs.move_iter() {
            let metric: Option<Metric> = FromStr::from_str(input);
            assert!(metric.is_none());
        }
    }
}
