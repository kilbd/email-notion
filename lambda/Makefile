.PHONY: build-rust

build-rust:
		cargo zigbuild --release --target x86_64-unknown-linux-gnu.2.28
		mkdir -p zips/assign-task zips/triage-task
		cp target/x86_64-unknown-linux-gnu/release/assign-task zips/assign-task/bootstrap
