#[link(name = "statsd",
       vers = "0.0.0",
       url = "http://github.com/boredomist/rust-statsd")];

#[comment = "statsd implementation"];
#[license = "MIT"];
#[crate_type = "lib"];
#[feature(globs)];

extern mod std;
extern mod extra;

pub use client::*;

pub mod metric;
pub mod buckets;
pub mod client;

// XXX: This feels like the wrong way of doing this.
pub mod server {
    pub mod main;
}
