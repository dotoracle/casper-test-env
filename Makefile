PINNED_TOOLCHAIN := $(shell cat rust-toolchain)
prepare:
	rustup target add wasm32-unknown-unknown
	rustup component add clippy --toolchain ${PINNED_TOOLCHAIN}
	rustup component add rustfmt --toolchain ${PINNED_TOOLCHAIN}

build-test-session:
	cargo build --release -p test-session --target wasm32-unknown-unknown
	wasm-strip target/wasm32-unknown-unknown/release/test-session.wasm
	mkdir -p test-env/wasm
	cp target/wasm32-unknown-unknown/release/test-session.wasm ./test-env/wasm/get-session.wasm

clippy:
	cd test-env && cargo clippy --all-targets -- -D warnings
	cd test-session && cargo clippy --all-targets -- -D warnings

check-lint: clippy
	cd test-env && cargo fmt -- --check
	cd test-session && cargo fmt -- --check

lint: clippy
	cd test-env && cargo fmt
	cd test-session && cargo fmt

