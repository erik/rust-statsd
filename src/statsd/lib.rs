#[link(name = "statsd",
       vers = "0.0.0",
       url = "http://github.com/boredomist/rust-statsd")];

#[comment = "statsd implementation"];
#[license = "MIT"];
#[crate_type = "lib"];

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