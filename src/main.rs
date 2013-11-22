extern mod std;

use std::hashmap::HashMap;
use std::from_str::FromStr;
use std::io::net::ip::SocketAddr;
use std::io::net::udp::UdpSocket;
use std::num;
use std::option::{Option, Some, None};
use std::str;


static MAX_PACKET_SIZE: uint = 1024;


enum MetricKind {
    Counter(f64), // sample rate
    Gauge,
    Timer,
    Histogram,
    Meter
}


impl ToStr for MetricKind {
    fn to_str(&self) -> ~str {
        match *self {
            Gauge      => ~"Gauge",
            Timer      => ~"Timer",
            Histogram  => ~"Histogram",
            Meter      => ~"Meter",
            Counter(s) => format!("Counter(s={})", s)
        }
    }
}



#[deriving(ToStr)]
struct Metric {
    kind: MetricKind,
    name: ~str,
    value: f64
}


impl Metric {
    fn to_str(&self) -> ~str {
        format!("{}({}) => {}", self.name, self.kind.to_str(), self.value)
    }
}


type Bucket = HashMap<~str, f64>;
struct State {
    counters:   Bucket,
    gauges:     Bucket,
    histograms: Bucket,
    meters:     Bucket,
    timers:     HashMap<~str, ~[f64]>
}


#[deriving(ToStr)]
impl State {
    fn new() -> State {
        State {
            counters: HashMap::new(),
            gauges: HashMap::new(),
            histograms: HashMap::new(),
            meters: HashMap::new(),
            timers: HashMap::new()
        }
    }

    fn add_metric(&mut self, metric: Metric) {
        match metric.kind {
            Gauge      => self.add_gauge(metric),
            Timer      => self.add_timer(metric),
            Counter(_) => self.add_counter(metric),
            Histogram  => warn!("Histogram not implemented"),
            Meter      => warn!("Meter not implemented")
        }
    }

    fn add_timer(&mut self, m: Metric) {
        self.timers.insert_or_update_with(
            m.name.clone(), ~[], |_, v| v.push(m.value)
        );
    }

    fn add_gauge(&mut self, m: Metric) {
        self.gauges.insert(m.name.clone(), m.value);
    }

    fn add_counter(&mut self, m: Metric) {
        let sample_rate = match m.kind {
            Counter(s) => s,
            _ => fail!("fasd")
        };

        self.counters.insert_or_update_with(
            m.name.clone(), 0.0, |_, v| *v += m.value * (1.0 / sample_rate));
    }
}


/// Attempt to parse an input string into a Metric struct. Bad inputs will
/// simply return a None.
///
/// Valid message formats are:
///    <str:metric_name>:<f64:value>|<str:type>
///    <str:metric_name>:<f64:value>|c|@<f64:sample_rate>
fn parse_metric(line: &str) -> Option<Metric> {
    // Pointer to position in line
    let mut idx = 0u;

    let name = match line.find(':') {
        // Badly formatted, bail.
        None => return None,

        Some(pos) => {
            idx += pos + 1;

            line.slice_to(pos).to_owned()
        }
    };

    let value: f64 = match line.slice_from(idx).find('|') {
        // Bail
        None => return None,

        Some(pos) => {
            let val: f64 = match FromStr::from_str(line.slice(idx, idx + pos)) {
                Some(x) => x,
                None => return None
            };
            idx += pos + 1;

            val
        }
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


/// Handle a buffer containing the contents of a single UDP packet received by
/// the server.
fn handle_message(buf: &[u8]) -> Option<Metric> {
    str::from_utf8_opt(buf).and_then(|string| {
        parse_metric(string)
    })
}


fn main() {
    let socket: SocketAddr = FromStr::from_str("0.0.0.0:9991").unwrap();
    let mut server = UdpSocket::bind(socket).unwrap();
    let mut state = ~State::new();

    loop {
        let mut buf = ~[0u8, ..MAX_PACKET_SIZE];

        match server.recvfrom(buf) {

            Some((nread, _)) => {
                // Messages this large probably are bad in some way.
                if nread == MAX_PACKET_SIZE {
                    warn!("Max packet size exceeded.");
                }

                // Use the slice to strip out trailing \0 characters
                let metric = handle_message(buf.slice_to(nread));
                if metric.is_some() {
                    state.add_metric(metric.unwrap());
                }
            },

            None => ()
        }
    }
}
