use std::from_str::FromStr;
use std::io::net::ip::SocketAddr;
use std::io::net::udp::UdpSocket;
use std::rand::{random, Open01};

use extra::time;


pub struct Client {
    dest: SocketAddr,
    sock: UdpSocket
}


impl Client {

    // TODO: allow prefixing keys
    // TODO: does it make sense to allow sampling on thing other than ctrs?

    pub fn new(dest: SocketAddr) -> Client {
        // XXX: Is this the right way to do this?
        let client_addr: SocketAddr = FromStr::from_str("0.0.0.0:0").unwrap();
        let sock = UdpSocket::bind(client_addr).unwrap();

        Client { dest: dest, sock: sock }
    }

    pub fn incr(&mut self, name: &str, sample_rate: f64) {
        self.count_sampled(name, 1.0, sample_rate);
    }

    pub fn decr(&mut self, name: &str, sample_rate: f64) {
        self.count_sampled(name, -1.0, sample_rate);
    }

    pub fn count(&mut self, name: &str, value: f64) {
        self.count_sampled(name, value, 1.0);
    }

    pub fn count_sampled(&mut self, name: &str, value: f64, sample_rate: f64) {
        let data = format!("{}:{}|c@{}", name, value, sample_rate);
        self.send(data);
    }

    pub fn gauge(&mut self, name: &str, value: f64) {
        let data = format!("{}:{}|g", name, value);
        self.send(data);
    }

    pub fn time(&mut self, name: &str, ms: uint) {
        let data = format!("{}:{}|ms", name, ms);
        self.send(data);
    }

    // Measure the time taken to execute the given function
    pub fn time_block(&mut self, name: &str, block: &fn() -> ()) {
        let start_time = time::precise_time_ns();
        block();
        let run_time_ms = (time::precise_time_ns() - start_time) / 1000;

        self.time(name, (run_time_ms as uint));
    }

    fn send(&mut self, data: &str) {
        self.sock.sendto(data.as_bytes(), self.dest);
    }

    fn send_sampled(&mut self, data: &str, sample_rate: f64) {
        // XXX: Make sure this is seeded properly.
        let rand: Open01<f64> = random();

        if *rand > sample_rate || sample_rate >= 1.0 {
            self.send(data);
        }
    }

}