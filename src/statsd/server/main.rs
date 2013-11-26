use buckets::{Buckets};


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
static MAX_PACKET_SIZE: uint = 1024;


#[main]
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
                        // XXX: this will fail if non-utf8 characters are used
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
                    .map(|metric| buckets.add_metric(metric))
                    .or_else(|| { buckets.bad_messages += 1; None });
            },
        }
    }
}
