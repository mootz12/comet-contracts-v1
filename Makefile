default: build

test: build
	cargo test --all --tests

build:
	stellar contract build
	mkdir -p target/wasm32v1-none/optimized
	stellar contract optimize \
		--wasm target/wasm32v1-none/release/contracts.wasm \
		--wasm-out target/wasm32v1-none/optimized/comet.wasm
	stellar contract optimize \
		--wasm target/wasm32v1-none/release/factory.wasm \
		--wasm-out target/wasm32v1-none/optimized/comet_factory.wasm
	cd target/wasm32v1-none/optimized/ && \
		for i in *.wasm ; do \
			ls -l "$$i"; \
		done

clean:
	cargo clean

FUZZ_TIME ?= 0
FUZZ_ARGS = -max_total_time=$(FUZZ_TIME) -max_len=128 -len_control=0
# -s none: the address sanitizer cannot link soroban-env-host's static initializers on macOS.
fuzz: fuzz-deposit-withdraw

fuzz-deposit-withdraw:
	cd fuzz && cargo +nightly fuzz run -s none fuzz_deposit_withdraw -- $(FUZZ_ARGS)

fuzz-swap:
	cd fuzz && cargo +nightly fuzz run -s none fuzz_swap -- $(FUZZ_ARGS)
