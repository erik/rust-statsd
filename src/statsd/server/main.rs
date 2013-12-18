extern mod std;
extern mod extra;

extern mod statsd;

use statsd::server::backend::Backend;
use statsd::server::backends::console::Console;
use statsd::server::backends::graphite::Graphite;
use statsd::server::buckets::Buckets;

use std::from_str::FromStr;
use std::io::Timer;
use std::io::buffered;
use std::io::net::ip::{Ipv4Addr, SocketAddr};
use std::io::net::{addrinfo, tcp};
use std::io::net::udp::UdpSocket;
use std::io::{Listener, Acceptor};
use std::option::{Some, None};
use std::os;
use std::comm::{Port, Chan, SharedChan, stream};
use std::str;

use extra::arc::MutexArc;
use extra::getopts::{optopt, optflag, getopts};


static FLUSH_INTERVAL_MS: u64 = 10000;
static MAX_PACKET_SIZE: uint = 256;

static DEFAULT_UDP_PORT: u16 = 8125;
static DEFAULT_TCP_PORT: u16 = 8126;


/// Different kinds of events we accept in the main event loop.
enum Event {
    FlushTimer,
    UdpMessage(~[u8]),
    TcpMessage(~tcp::TcpStream)
}


/// Run in a new task for each management connection made to the server.
fn management_connection_loop(tcp_stream: ~tcp::TcpStream,
                              buckets_arc: MutexArc<Buckets>) {
    let mut stream = buffered::BufferedStream::new(*tcp_stream);

    loop {
        // XXX: this will fail if non-utf8 characters are used
        let end_conn = stream.read_line().map_default(false, |line| {
            buckets_arc.access(|buckets| {
                let (resp, end_conn) = buckets.do_management_line(line);

                stream.write(resp.as_bytes());
                stream.write(['\n' as u8]);
                stream.flush();

                end_conn
            })
        });

        if end_conn {
            break
        }
    }
}


fn flush_timer_loop(chan: SharedChan<~Event>, int_ms: u64) {
    let mut timer = Timer::new().unwrap();
    let periodic = timer.periodic(int_ms);

    loop {
        periodic.recv();
        chan.send(~FlushTimer);
    }
}


/// Accept incoming TCP connection to the statsd management port.
fn management_server_loop(chan: SharedChan<~Event>, port: u16) {
    let addr = SocketAddr { ip: Ipv4Addr(0, 0, 0, 0), port: port };
    let listener = tcp::TcpListener::bind(addr).unwrap();
    let mut acceptor = listener.listen();

    for stream in acceptor.incoming() {
        stream.map(|stream| {
            chan.send(~TcpMessage(~stream));
        });
    }
}


/// Accept incoming UDP data from statsd clients.
fn udp_server_loop(chan: SharedChan<~Event>, port: u16) {
    let addr = SocketAddr { ip: Ipv4Addr(0, 0, 0, 0), port: port };
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
            chan.send(~UdpMessage(msg));
        });
    }
}


fn print_usage() {
    println!("Usage: {} [options]", os::args()[0]);
    println("  -h --help               Show usage information");
    println("  --graphite host[:port]  Enable the graphite backend. \
Port will default to 2003 if not specified.");
    println("  --console               Enable console output.");
    println!("  --port port             Have the statsd server listen on this \
UDP port. Defaults to {}.", DEFAULT_UDP_PORT);
    println!("  --admin-port port       Have the admin server listen on this \
TCP port. Defaults to {}.", DEFAULT_TCP_PORT);
    println!("  --flush                 Flush interval, in seconds. Defaults \
to {}.", FLUSH_INTERVAL_MS / 1000);
}


fn main() {
    let args = os::args();

    let opts = ~[
        optflag("h"), optflag("help"),
        optopt("graphite"),
        optflag("console"),
        optopt("port"),
        optopt("admin-port"),
        optopt("flush")
    ];

    let matches = match getopts(args.tail(), opts) {
        Ok(m) => { m },
        Err(f) => {
            println(f.to_err_msg());
            return print_usage();
        }
    };

    if matches.opt_present("h") || matches.opt_present("help") {
        return print_usage();
    }

    let mut backends: ~[~Backend] = ~[];

    if matches.opt_present("graphite") {
        // We can safely unwrap here because getopt handles the error condition
        // for us. Probably.
        let arg_str = matches.opt_str("graphite").unwrap();
        let mut iter = arg_str.split(':');

        let host = iter.next().unwrap();
        let port = match iter.next() {
            Some(port) => match FromStr::from_str(port) {
                Some(port) => port,
                None => {
                    println!("Invalid port number: {}", port);
                    return print_usage();
                }
            },
            None => 2003
        };

        let addr = match addrinfo::get_host_addresses(host) {
            Some(ref addrs) if addrs.len() > 0 => addrs[0],
            _ => {
                println!("Bad host name {}", host);
                return;
            }
        };

        let graphite = ~Graphite::new(SocketAddr{ip: addr, port: port});
        backends.push(graphite as ~Backend);

        info!("Using graphite backend ({}:{}).", host, port);
    }

    if matches.opt_present("console") {
        backends.push(~Console::new() as ~Backend);
        info!("Using console backend.");
    }

    let udp_port = match matches.opt_str("port") {
        Some(port_str) => match FromStr::from_str(port_str) {
            Some(port) => port,
            None => {
                println!("Invalid port number: {}", port_str);
                return print_usage();
            }
        },
        None => DEFAULT_UDP_PORT
    };

    let tcp_port = match matches.opt_str("admin-port") {
        Some(port_str) => match FromStr::from_str(port_str) {
            Some(port) => port,
            None => {
                println!("Invalid port number: {}", port_str);
                return print_usage();
            }
        },
        None => DEFAULT_TCP_PORT
    };

    let flush_interval = match matches.opt_str("flush") {
        Some(str_secs) => match from_str::<u64>(str_secs) {
            Some(secs) => secs * 1000,
            None => {
                println!("Invalid integer: {}", str_secs);
                return print_usage();
            }
        },
        None => FLUSH_INTERVAL_MS
    };

    let (event_port, event_chan_): (Port<~Event>, Chan<~Event>) = stream();
    let event_chan = SharedChan::new(event_chan_);

    let flush_chan = event_chan.clone();
    let mgmt_chan = event_chan.clone();
    let udp_chan = event_chan.clone();

    spawn(proc() { flush_timer_loop(flush_chan, flush_interval) });
    spawn(proc() { management_server_loop(mgmt_chan, tcp_port) });
    spawn(proc() { udp_server_loop(udp_chan, udp_port) });

    let buckets = Buckets::new();
    let buckets_arc = MutexArc::new(buckets);

    // Main event loop.
    loop {
        match *event_port.recv() {
            // Flush timeout
            FlushTimer => buckets_arc.access(|buckets| {
                for ref mut backend in backends.mut_iter() {
                    backend.flush_buckets(buckets);
                }

                buckets.flush();
            }),

            // Management server
            TcpMessage(s) => {
                // Clone the arc so the new task gets its own copy.
                let buckets_arc = buckets_arc.clone();

                // Spin up a new thread to handle the TCP stream.
                spawn(proc() { management_connection_loop(s, buckets_arc) });
            },

            // UDP message received
            UdpMessage(buf) => buckets_arc.access(|buckets| {
                str::from_utf8_opt(buf)
                    .and_then(|string| FromStr::from_str(string))
                    .map(|metric| buckets.add_metric(metric))
                    .or_else(|| { buckets.bad_messages += 1; None });
            }),
        }
    }
}
