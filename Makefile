DOC_PATH=doc
RUSTC=rustc

build_cmd= rustc -Llib --out-dir $(BUILD_PATH)

all:
	rustpkg build statsd

clean:
	rustpkg clean statsd
