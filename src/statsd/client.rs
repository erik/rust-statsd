use std::from_str::FromStr;
use std::io::net::ip::SocketAddr;
use std::io::net::udp::UdpSocket;
use std::rand::random;

use time;


/** Simple interface to a statsd host.

Does only minimal computation (basically just whether or not to send sampled
data and timing a function call). Most work is handled by the server.

**TODO**: allow prefixing keys.
*/
pub struct Client {
    priv dest: SocketAddr,
    priv sock: UdpSocket
}


impl Client {
    /// Construct a new statsd client given a hostname and port.
    pub fn new(dest: SocketAddr) -> Client {
        // XXX: Is this the right way to do this?
        let client_addr: SocketAddr = FromStr::from_str("0.0.0.0:0").unwrap();
        let sock = UdpSocket::bind(client_addr).unwrap();

        Client { dest: dest, sock: sock }
    }

    /// Increment the given `name` by one with a probability of `sample_rate`.
    pub fn incr(&mut self, name: &str, sample_rate: f64) {
        self.count_sampled(name, 1.0, sample_rate);
    }

    /// Decrement the given `name` by one with a probability of `sample_rate`.
    pub fn decr(&mut self, name: &str, sample_rate: f64) {
        self.count_sampled(name, -1.0, sample_rate);
    }

    /// Add `value` to the given `name`.
    pub fn count(&mut self, name: &str, value: f64) {
        self.count_sampled(name, value, 1.0);
    }

    /// Add `value` to the given `name` with a probability of `sample_rate`.
    pub fn count_sampled(&mut self, name: &str, value: f64, sample_rate: f64) {
        let data = format!("{}:{}|c@{}", name, value, sample_rate);
        self.send_sampled(data, sample_rate);
    }

    /// Simply set the given `name` to `value`.
    pub fn gauge(&mut self, name: &str, value: f64) {
        let data = format!("{}:{}|g", name, value);
        self.send(data);
    }

    /** Specify that this instance of `name` took `ms` milliseconds.

    It doesn't matter what you use here. Statsd bizarrely treats timed values
    specially. A better name for this kind of value would be `histogram`,
    because that's what's really being calculated from the server side. Some
    server implementations (such as the one included here) support histogram
    keys.
    */
    pub fn time(&mut self, name: &str, ms: uint) {
        let data = format!("{}:{}|ms", name, ms);
        self.send(data);
    }

    /// Similar to `time`, but the `ms` value sent is the amount of time taken
    /// to execute the proc.
    pub fn time_block(&mut self, name: &str, block: proc()) {
        let start_time = time::precise_time_ns();
        block();
        let run_time_ms = (time::precise_time_ns() - start_time) / 1000;

        self.time(name, (run_time_ms as uint));
    }

    /// Append `val` to the vector `name`. Server will generate summary
    /// statistics for the vector on each flush.
    pub fn hist(&mut self, name: &str, val: f64) {
        let data = format!("{}:{}|h", name, val);
        self.send(data);
    }

    /// Data goes in, data comes out.
    fn send(&mut self, data: &str) {
        // TODO: Currently just ignoring send errors, should other behavior be
        //       used?
        let _ = self.sock.sendto(data.as_bytes(), self.dest);
    }

    /// Data goes in, data comes out. With a defined probability.
    fn send_sampled(&mut self, data: &str, sample_rate: f64) {
        if random::<f64>() > sample_rate || sample_rate >= 1.0 {
            self.send(data);
        }
    }

}