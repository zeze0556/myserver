TARGET?=mynas

default: $(TARGET)

mynas:
	RUST_BACKTRACE=1 cargo build -r --target=x86_64-unknown-linux-musl
#	cargo build --release

run: mynas
	RUST_BACKTRACE=1 target/x86_64-unknown-linux-musl/release/mynas

test:
	RRUST_BACKTRACE=full target/x86_64-unknown-linux-musl/release/mynas

clean:
	rm -rf target
