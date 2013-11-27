BUILD_PATH=build
DOC_PATH=doc
RUSTC=rustc

build_cmd= rustc -Llib --out-dir $(BUILD_PATH)

all: mkdirs statsd-lib statsd-server

mkdirs:
	mkdir -p $(BUILD_PATH)

statsd-lib:
	$(RUSTC) src/statsd/statsd.rc --out-dir $(BUILD_PATH)

statsd-server:
	$(RUSTC) src/statsd/statsd.rc --bin --out-dir $(BUILD_PATH)

clean:
	rm -rf $(BUILD_PATH)
