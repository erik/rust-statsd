//! Export data to a specified graphite instance over TCP.

use server::backend::Backend;
use server::buckets::Buckets;

use std::io::net::ip::SocketAddr;
use std::io::net::tcp::TcpStream;
use std::fmt;
use std::hashmap::HashMap;

use extra::time;
use extra::stats::Stats;


pub struct Graphite {
    host: SocketAddr,
    last_flush_time: i64,
    last_flush_length: i64,
    prefix: ~str
}


impl Graphite {
    pub fn new(host: SocketAddr) -> Graphite {
        Graphite {
            host: host,
            last_flush_time: 0,
            last_flush_length: 0,
            prefix: ~""
        }
    }


    /// Create with a prefix that will be automatically prepended to all keys.
    pub fn new_with_prefix(prefix: &str, host: SocketAddr) -> Graphite {
        Graphite {
            host: host,
            last_flush_time: 0,
            last_flush_length: 0,
            prefix: format!("{}.", prefix)
        }
    }

    fn fmt_line<T: fmt::Show>(&mut self, key: &str, value: T, time: i64) -> ~str {
        format!("{}{} {} {}\n", self.prefix, key, value, time)
    }
}


/// Abstract out formatting code for both histograms and timers.
fn fmt_stats(start: i64, hist_kind: &str, hist: &HashMap<~str, ~[f64]>) -> ~str {
    let mut str_buf = ~"";

    for (key, values) in hist.iter() {
        // XXX: This isn't optimal, needs to read full list for each
        //      statistical value.

        let samples: &[f64] = *values;
        let key = format!("{}.{}", hist_kind, *key);

        let line = format!("{key}.min {min} {ts}
{key}.max {max} {ts}
{key}.count {count} {ts}
{key}.mean {mean} {ts}
{key}.stddev {std} {ts}
{key}.upper_95 {max_threshold} {ts}\n",
                           key=key, ts=start,
                           min=samples.min(),
                           max=samples.max(),
                           count=samples.len(),
                           mean=samples.mean(),
                           std=samples.std_dev(),
                           max_threshold=samples.percentile(95.0));

        str_buf.push_str(line);
    }

    str_buf
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

        str_buf.push_str(fmt_stats(start, "timers", &buckets.timers));
        str_buf.push_str(fmt_stats(start, "histograms", &buckets.histograms));

        str_buf.push_str(self.fmt_line(
            "graphiteStats.last_flush", self.last_flush_time, start));

        str_buf.push_str(self.fmt_line(
            "graphiteStats.flush_time", self.last_flush_time, start));

        let end_time = time::get_time().sec;
        let flush_length = end_time - start;
        self.last_flush_length = flush_length;
        self.last_flush_time = end_time;

        // Try to send the data to our Graphite instance, ignoring failures.
        let _ = TcpStream::connect(self.host).map(|ref mut stream| {
            let _ = stream.write(str_buf.as_bytes());
            let _ = stream.flush();
        });
    }
}