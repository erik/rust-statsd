extern mod std;
extern mod extra;

use std::fmt;
use std::from_str::FromStr;
use std::hashmap::HashMap;
use std::io::Timer;
use std::io::buffered;
use std::io::net::ip::SocketAddr;
use std::io::net::tcp;
use std::io::net::udp::UdpSocket;
use std::io::{Listener, Acceptor};
use std::num;
use std::option::{Option, Some, None};
use std::rt::comm::{Port, Chan, SharedChan, stream};
use std::str;

use extra::time;
use extra::arc::MutexArc;
//use extra::stats::Stats;


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
    timers:     HashMap<~str, ~[f64]>,

    server_start_time: time::Timespec,
    last_message: time::Timespec,
    bad_messages: uint
}


impl Buckets {
    fn new() -> Buckets {
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

    fn flush(&mut self) {
    }

    fn handle_management_cmd(&mut self, line: &str) -> ~str {
        let mut words = line.word_iter();

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

    fn add_metric(&mut self, metric: Metric) {
        let key = metric.name.clone();
        let val = metric.value;

        match metric.kind {
            Gauge => {
                self.gauges
                    .insert(key, val);
            },
            Timer => {
                self.timers
                    .insert_or_update_with(key, ~[], |_, v| v.push(val));
            },
            Counter(sample_rate) => {
                self.counters
                    .insert_or_update_with(key, 0.0, |_, v| {
                        *v += val * (1.0 / sample_rate)
                    }
                );
            },

            Histogram => warn!("Histogram not implemented: {}", metric),
            Meter => warn!("Meter not implemented: {}", metric)
        }

        self.last_message = time::get_time();
    }
}


/// FIXME: this function's name doesn't correspond to what it actually does.
/// Handle a buffer containing the contents of a single packet received by
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

    enum Event {
        FlushTimer,
        UdpMessage(~[u8]),
        TcpMessage(~tcp::TcpStream)
    }

    let (event_port, event_chan_): (Port<~Event>, Chan<~Event>) = stream();
    let event_chan = SharedChan::new(event_chan_);

    // UDP server loop
    let udp_chan = event_chan.clone();
    do spawn {
        let addr: SocketAddr = FromStr::from_str("0.0.0.0:9991").unwrap();
        let mut socket = UdpSocket::bind(addr).unwrap();
        let mut buf = [0u8, ..MAX_PACKET_SIZE];

        loop {
            socket.recvfrom(buf).map(|(nread, _)| {
                // Messages this large probably are bad in some way.
                if nread == MAX_PACKET_SIZE {
                    warn!("Max packet size exceeded.");
                }

                // Use the slice to strip out trailing \0 characters
                let msg = buf.slice_to(nread).to_owned();
                udp_chan.send(~UdpMessage(msg));
            });
        }
    }

    // Flush timer loop
    let flush_chan = event_chan.clone();
    do spawn {
        let mut timer = Timer::new().unwrap();
        let periodic = timer.periodic(FLUSH_INTERVAL_MS);

        loop {
            periodic.recv();
            flush_chan.send(~FlushTimer);
        }
    }

    // Management server loop
    let mgmt_chan = event_chan.clone();
    do spawn {
        let addr: SocketAddr = FromStr::from_str("0.0.0.0:8126").unwrap();
        let listener = tcp::TcpListener::bind(addr).unwrap();
        let mut acceptor = listener.listen();

        for stream in acceptor.incoming() {
            stream.map(|stream| {
                mgmt_chan.send(~TcpMessage(~stream));
            });
        }
    }

    let buckets = Buckets::new();
    let buckets_arc = MutexArc::new(buckets);

    // XXX: Handle broken pipe task failure.
    loop {
        match *event_port.recv() {
            // Flush timeout
            FlushTimer => do buckets_arc.access |buckets| {
                buckets.flush();
            },

            // Management server
            TcpMessage(s) => {
                // Clone the arc so the new task gets its own copy.
                let buckets_arc = buckets_arc.clone();

                do spawn {
                    let mut stream = buffered::BufferedStream::new(*s);

                    loop {
                        match stream.read_line() {
                            Some(line) => do buckets_arc.access |buckets| {
                                let resp = buckets.handle_management_cmd(line);

                                stream.write(resp.as_bytes());
                                stream.flush();
                            },
                            None => { break; }
                        }
                    }
                }
            },

            // UDP message received
            UdpMessage(buf) => do buckets_arc.access |buckets| {
                str::from_utf8_opt(buf)
                    .and_then(|string| FromStr::from_str(string))
                    .map(|metric| buckets.add_metric(metric));
            },
        }
    }
}
