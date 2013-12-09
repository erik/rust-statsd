DOC_PATH=doc
RUSTC=rustc

build_cmd= rustc -Llib --out-dir $(BUILD_PATH)

all:
	rustpkg install statsd

check:
	rustpkg test statsd

doc:
	rustdoc src/statsd/lib.rs

clean:
	rustpkg clean statsd


.PHONY: all check doc clean
