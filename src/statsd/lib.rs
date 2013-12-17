
#[link(name = "statsd",
       vers = "0.0.0",
       url = "http://github.com/boredomist/rust-statsd")];

/*! A pure Rust implementation of the statsd server.

The statsd protocol consistents of plain-text single-packet messages sent
over UDP, containing not much more than a key and (possibly sampled) value.

Due to the inherent design of the system, there is no guarantee that metrics
will be received by the server, and there is (by design) no indication of
this.
*/

#[comment = "statsd implementation"];
#[license = "MIT"];
#[crate_type = "lib"];
#[pkgid = "statsd"];


extern mod std;
extern mod extra;

pub mod metric;

pub mod client;

pub mod server {
    pub mod backend;
    pub mod buckets;

    pub mod backends {
        pub mod graphite;
        pub mod console;
    }
}