extern mod std;

use std::option::{Option, Some, None};
use std::io::net::udp::UdpSocket;
use std::io::net::ip::SocketAddr;
use std::from_str::FromStr;
use std::str;


static MAX_PACKET_SIZE: uint = 1024;

#[deriving(ToStr)]
enum MetricKind {
    Counter(f64), // sample rate
    Gauge,
    Timer,
    Histogram,
    Meter
}


#[deriving(ToStr)]
struct Metric {
    kind: MetricKind,
    name: ~str,
    value: f64
}



fn parse_metric(line: &str) -> Option<Metric> {
    let mut idx = 0u;

    let name = match line.find(':') {
        // Badly formatted, bail.
        None => return None,

        Some(pos) => {
            idx += pos + 1;
            line.slice_to(pos)
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

    // FIXME: This is horrible! Doesn't actually work as intended.
    let kind = match line.slice(idx, idx + 2) {
        "c\0" => Counter(1.0),
        "ms" => Timer,
        "h\0" => Histogram,
        "m\0" => Meter,
        "g\0" => Gauge,

        // Unknown type
        _ => return None
    };

    Some(Metric {kind: kind, name: name.to_owned(), value: value})
}


fn handle_message(buf: ~[u8]) {

    let metric: Metric = match str::from_utf8_opt(buf) {
        Some(string) => {

            println!("---> {}", string);

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
    let (name, value) = (metric.name.clone(), metric.value);

    println!("{} => {}", name, value);

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
                if nread == MAX_PACKET_SIZE {
                    warn!("Max packet size exceeded.");
                }

                handle_message(buf);
            },
            None => ()
        }
    }
}
