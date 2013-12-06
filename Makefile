DOC_PATH=doc
RUSTC=rustc

build_cmd= rustc -Llib --out-dir $(BUILD_PATH)

all:
	rustpkg install statsd

clean:
	rustpkg clean statsd
