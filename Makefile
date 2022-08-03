
MSRV = 1.57

.PHONY: msrv
msrv:
	cargo +$(MSRV) check --tests

.PHONY: check
check: msrv
	cargo fmt -- --check
	cargo clippy
	cargo audit

.PHONY: release
release: target/release/bard

target/release/bard:
	cargo build --release

.PHONY: test
test:
	cargo test
	cargo test -- --ignored project_all_features
#	cargo test -- --ignored
# TODO: ^ run the ignored tests too once https://github.com/sunng87/handlebars-rust/issues/509 is solved

.PHONY: examples
examples:
	cd default && cargo run -- make
	cd example && cargo run -- make
	cd tests/test-projects/all-features && cargo run -- make
