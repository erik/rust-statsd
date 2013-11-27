use server::backend::Backend;
use server::buckets::Buckets;

use std::io::net::ip::SocketAddr;
use std::fmt::Default;

use extra::time;
use extra::stats::Summary;

pub struct Graphite {
    host: SocketAddr,
    last_flush_time: i64,
    last_flush_length: i64,
    prefix: ~str
}


impl Graphite {
    fn new(prefix: &str, host: SocketAddr) -> Graphite {

        Graphite {
            host: host,
            last_flush_time: 0,
            last_flush_length: 0,
            prefix: format!("{}.", prefix)
        }
    }

    fn fmt_line<T: Default>(&mut self, key: &str, value: T, time: i64) -> ~str {
        format!("{}{} {} {}\n", self.prefix, key, value, time)
    }
}


impl Backend for Graphite {
    fn flush_buckets(&mut self, buckets: &Buckets) -> () {
        let start = time::get_time().sec;
        let mut str_buf = ~"";

        for (key, value) in buckets.counters.iter() {
            let key = format!("counters.{}", *key);
            str_buf.push_str(self.fmt_line(key, *value, start));
        }

        for (key, value) in buckets.gauges.iter() {
            let key = format!("gauges.{}", *key);
            str_buf.push_str(self.fmt_line(key, *value, start));
        }

        for (key, value) in buckets.timers.iter() {
            let key = format!("timers.{}", *key);
            let _ = Summary::new(*value);
            str_buf.push_str(self.fmt_line(key, "TODO: stats", start));
        }

        str_buf.push_str(self.fmt_line(
            "graphiteStats.last_flush", self.last_flush_time, start));

        str_buf.push_str(self.fmt_line(
            "graphiteStats.flush_time", self.last_flush_time, start));

        let end_time = time::get_time().sec;
        let flush_length = end_time - start;
        self.last_flush_length = flush_length;
        self.last_flush_time = end_time;
    }
}