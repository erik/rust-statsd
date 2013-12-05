extern mod std;
extern mod extra;

extern mod statsd;

use statsd::server::buckets::Buckets;
use statsd::server::backend::Backend;
use statsd::server::backends::graphite::Graphite;
use statsd::server::backends::console::Console;

use std::from_str::FromStr;
use std::io::Timer;
use std::io::buffered;
use std::io::net::ip::SocketAddr;
use std::io::net::tcp;
use std::io::net::udp::UdpSocket;
use std::io::{Listener, Acceptor};
use std::option::{Some, None};
use std::rt::comm::{Port, Chan, SharedChan, stream};
use std::str;

use extra::arc::MutexArc;

static FLUSH_INTERVAL_MS: u64 = 10000;
static MAX_PACKET_SIZE: uint = 256;


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


fn flush_timer_loop(chan: SharedChan<~Event>) {
    let mut timer = Timer::new().unwrap();
    let periodic = timer.periodic(FLUSH_INTERVAL_MS);

    loop {
        periodic.recv();
        chan.send(~FlushTimer);
    }
}


fn management_server_loop(chan: SharedChan<~Event>) {
    let addr: SocketAddr = FromStr::from_str("0.0.0.0:8126").unwrap();
    let listener = tcp::TcpListener::bind(addr).unwrap();
    let mut acceptor = listener.listen();

    for stream in acceptor.incoming() {
        stream.map(|stream| {
            chan.send(~TcpMessage(~stream));
        });
    }
}

fn udp_server_loop(chan: SharedChan<~Event>) {
    let addr: SocketAddr = FromStr::from_str("0.0.0.0:8125").unwrap();
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


#[main]
fn main() {
    // TODO: make this configurable
    let mut backends: ~[~Backend] = ~[];
    {
        let graphite_host = FromStr::from_str("0.0.0.0:2003").unwrap();
        backends.push(~Graphite::new(graphite_host) as ~Backend);

        backends.push(~Console::new() as ~Backend);
    }

    let (event_port, event_chan_): (Port<~Event>, Chan<~Event>) = stream();
    let event_chan = SharedChan::new(event_chan_);

    let flush_chan = event_chan.clone();
    let mgmt_chan = event_chan.clone();
    let udp_chan = event_chan.clone();

    spawn(proc() { flush_timer_loop(flush_chan) });
    spawn(proc() { management_server_loop(mgmt_chan) });
    spawn(proc() { udp_server_loop(udp_chan) });

    let buckets = Buckets::new();
    let buckets_arc = MutexArc::new(buckets);

    // XXX: Handle broken pipe task failure.
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
