.PHONY: build-rust

build-rust:
		cargo zigbuild --release --target x86_64-unknown-linux-gnu
		mkdir -p zips/assign-task zips/triage-task
		cp target/x86_64-unknown-linux-gnu/release/assign-task zips/assign-task/bootstrap
