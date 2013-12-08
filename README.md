rust-statsd
===========

It's a `statsd` server/client implementation. In Rust.

Run `make` to build the server/client. The server can be run with
`./bin/statsd`, the client library is available in
`./lib/<arch>/libstatsd-...so`.

**Note**: everything is in progress. It works, but things are bound to
  change. Feedback welcome.

Client
------

```rust
let statsd_host: SocketAddr = FromStr::from_str("hostname:8125").unwrap();
let client = statsd::Client::new(statsd_host);

// Increment the "foo" counter by 1 50% of the time.
client.incr("foo", 0.5);

// Decrement the "bar" counter by 1 10% of the time.
client.decr("bar", 0.1);

// Add 10 to the "foo" counter.
client.count("foo", 10);

// Subtract 10 from the "foo" counter 50% of the time.
client.count_sampled("foo", -10, 0.5);

// Set the "bar" gauge to 123.45
client.gauge("bar", 123.45);

// Add a run to the "quux" timer taking 300ms.
client.time("quux", 300);

// Add a run to "quux" with the time taken to execute the given proc.
client.time_block("quux", proc() { /* expensive computation here */ });
```

Server
------

Usage:

```
Usage: ./bin/statsd [options]
  -h --help               Show usage information
  --graphite host[:port]  Enable the graphite backend. Port will default to 2003 if not specified.
  --console               Enable console output.
```

### Backends

#### Console
Prints out a YAML representation of the buckets on each flush.
```yaml
2013-12-06T22:41:05-08:00:
  counters:
    a: 1
    b: 2
    c: 3
  gauges:
    d: 1
  timers:
    e:
      min: 0
      max: 8919
      count: 7137
      mean: 1087.236094
      stddev: 1824.204297
      upper_95: 5376.8
```

#### Graphite
Exports buckets in a Graphite-friendly format over a TCP stream.

License
-------
MIT License

Copyright (c) 2013 Erik Price

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the “Software”), to deal in
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
of the Software, and to permit persons to whom the Software is furnished to do
so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
