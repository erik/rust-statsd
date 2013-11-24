extern mod std;

use std::select;
use std::rt::comm::{Port, Chan, stream};
use std::io::Timer;
use std::hashmap::HashMap;
use std::fmt;
use std::from_str::FromStr;
use std::io::net::ip::SocketAddr;
use std::io::net::udp::UdpSocket;
use std::num;
use std::option::{Option, Some, None};
use std::str;


static FLUSH_INTERVAL_MS: u64 = 10000;
static MAX_PACKET_SIZE: uint = 1024;


enum MetricKind {
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


struct Metric {
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


struct Buckets {
    counters:   HashMap<~str, f64>,
    gauges:     HashMap<~str, f64>,
    histograms: HashMap<~str, f64>,
    meters:     HashMap<~str, f64>,
    timers:     HashMap<~str, ~[f64]>
}


#[deriving(ToStr)]
impl Buckets {
    fn new() -> Buckets {
        Buckets {
            counters: HashMap::new(),
            gauges: HashMap::new(),
            histograms: HashMap::new(),
            meters: HashMap::new(),
            timers: HashMap::new()
        }
    }

    fn flush(&self) -> () {
    }

    fn add_metric(&mut self, metric: Metric) {
        match metric.kind {
            Gauge      => self.add_gauge(metric),
            Timer      => self.add_timer(metric),
            Counter(_) => self.add_counter(metric),
            Histogram  => self.add_histogram(metric),
            Meter      => self.add_meter(metric)
        }
    }

    fn add_counter(&mut self, m: Metric) {
        let sample_rate = match m.kind {
            Counter(s) => s,
            _ => fail!("fasd")
        };

        self.counters.insert_or_update_with(
            m.name.clone(), 0.0, |_, v| *v += m.value * (1.0 / sample_rate));
    }
    fn add_gauge(&mut self, m: Metric) {
        self.gauges.insert(m.name.clone(), m.value);
    }

    fn add_timer(&mut self, m: Metric) {
        self.timers.insert_or_update_with(
            m.name.clone(), ~[], |_, v| v.push(m.value)
        );
    }

    fn add_histogram(&mut self, m: Metric) {
        warn!("Histogram not implemented: {}", m)
    }

    fn add_meter(&mut self, m: Metric) {
        warn!("Meter not implemented: {}", m)
    }

}


/// Handle a buffer containing the contents of a single UDP packet received by
/// the server.
fn handle_message(buf: &[u8]) -> Option<Metric> {
    match str::from_utf8_opt(buf).and_then(|s| FromStr::from_str(s)) {
        Some(m) => {
            println!("==> {}", m);
            Some(m)
        },
        None => {
            println!("==> Bad input");
            None
        }
    }
}


fn main() {
    let mut buckets = Buckets::new();

    let (udp_port, udp_chan): (Port<~[u8]>, Chan<~[u8]>) = stream();

    // UDP server loop
    do spawn {
        let socket: SocketAddr = FromStr::from_str("0.0.0.0:9991").unwrap();
        let mut server = UdpSocket::bind(socket).unwrap();
        let mut buf = [0u8, ..MAX_PACKET_SIZE];

        loop {
            server.recvfrom(buf).map(|(nread, _)| {
                // Messages this large probably are bad in some way.
                if nread == MAX_PACKET_SIZE {
                    warn!("Max packet size exceeded.");
                }

                // Use the slice to strip out trailing \0 characters
                let msg = buf.slice_to(nread).to_owned();
                udp_chan.send(msg);
            });
        }
    }

    // XXX: The ~[u8] is only to appease the type system, and is almost certainly
    // wrong. Only empty vectors are sent.
    let (flush_port, flush_chan): (Port<~[u8]>, Chan<~[u8]>) = stream();

    // Flush timer loop
    do spawn {
        let mut timer = Timer::new().unwrap();
        let periodic = timer.periodic(FLUSH_INTERVAL_MS);

        loop {
            periodic.recv();
            flush_chan.send(~[]);
        }
    }

    // TODO: management server loop

    let mut ports = ~[udp_port, flush_port];

    loop {
        match select::select(ports) {
            // UDP message received
            0 => {
                let msg = ports[0].recv();

                handle_message(msg).map(|metric| {
                    buckets.add_metric(metric);
                });
            },

            // Flush timeout
            1 => {
                ports[1].recv();
                println!("flush");
                buckets.flush();
            },

            _ => ()
        }
    }

}
