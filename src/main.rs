extern mod std;

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


struct Metric {
    kind: MetricKind,
    name: ~str,
    value: f64
}


/// Attempt to parse an input string into a Metric struct. Bad inputs will
/// simply return a None.
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
        }

        // Unknown type
        _ => return None
    };

    Some(Metric {kind: kind, name: name, value: value})
}


/// Handle a buffer containing the contents of a single UDP packet received by
/// the server.
fn handle_message(buf: &[u8]) {

    let metric: Metric = match str::from_utf8_opt(buf) {
        Some(string) => {

            match parse_metric(string) {
                Some(m) => m,
                None => return
            }
        },
        // Just ignore badly formatted metrics
        None => {
            println!("bad format");
            return
        }
    };

    // FIXME: This feels wrong. Very wrong.
    let name: &str = metric.name;
    let value: f64 = metric.value;

    println!("→ {} => {}", name, value);

    match metric {
        Metric { kind: Gauge, _ } => {
            println!("gauge")
        },
        Metric { kind: Timer, _ } => {
            println!("timer")
        },
        Metric { kind: Histogram, _ } => {
            println!("histogram")
        },
        Metric { kind: Meter, _ } => {
            println!("meter")
        },
        Metric { kind: Counter(s), _ } => {
            println!("counter {}", s)
        }
    }
}


fn main() {
    let socket: SocketAddr = FromStr::from_str("0.0.0.0:9991").unwrap();
    let mut server = UdpSocket::bind(socket).unwrap();

    loop {
        let mut buf = ~[0u8, ..MAX_PACKET_SIZE];

        match server.recvfrom(buf) {

            Some((nread, _)) => {
                // Messages this large probably are bad in some way.
                if nread == MAX_PACKET_SIZE {
                    warn!("Max packet size exceeded.");
                }

                // Use the slice to strip out trailing \0 characters
                handle_message(buf.slice_to(nread));
            },
            None => ()
        }
    }
}
