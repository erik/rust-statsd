extern crate statsd;


#[cfg(test)]
mod metric {
    use statsd::metric;
    use statsd::metric::Metric;

    use std::from_str::FromStr;

    #[test]
    fn test_from_str_valid_input() {
        let in_out_map = ~[
            ("f.o.o:1|c",      Metric {kind: metric::Counter(1.0), name: ~"f.o.o", value: 1.0}),
            ("foo:9.1|c|@0.5", Metric {kind: metric::Counter(0.5), name: ~"foo", value: 9.1}),
            ("foo:2|c|@1",     Metric {kind: metric::Counter(1.0), name: ~"foo", value: 2.0}),
            ("foo:2|c|@123",   Metric {kind: metric::Counter(123.0), name: ~"foo", value: 2.0}),
            ("foo:12.3|ms",    Metric {kind: metric::Timer, name: ~"foo", value: 12.3}),
            ("foo:1|ms",       Metric {kind: metric::Timer, name: ~"foo", value: 1.0}),
            ("foo:1|h",        Metric {kind: metric::Histogram, name: ~"foo", value: 1.0}),
            ("foo:1.23|h",     Metric {kind: metric::Histogram, name: ~"foo", value: 1.23}),
            ("foo:1|g",        Metric {kind: metric::Gauge, name: ~"foo", value: 1.0}),
            ("foo:1.23|g",     Metric {kind: metric::Gauge, name: ~"foo", value: 1.23})
        ];

        for (input, expected) in in_out_map.move_iter() {
            let actual: Metric = FromStr::from_str(input).unwrap();

            assert_eq!(expected, actual);
        }
    }


    #[test]
    fn test_from_str_invalid_input() {
        let inputs = ~[
            "f",
            "f:",
            "f:c",
            "f:1.0|",
            "f:1.0|c@",
            ":|@",
            ":1.0|c"
        ];

        for input in inputs.move_iter() {
            let metric: Option<Metric> = FromStr::from_str(input);
            assert!(metric.is_none());
        }
    }
}