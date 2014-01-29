DOC_PATH=doc
RUSTC=rustc

all:
	rustc --lib src/statsd/lib.rs --out-dir .
	rustc --bin src/statsd/server/main.rs -o statsd -L .

check: all
	rustc --test src/statsd/test.rs -L . -o test
	./test
	rm test

doc:
	rustdoc src/statsd/lib.rs

clean:
	rm -f *.so statsd


.PHONY: all check doc clean
