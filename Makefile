.PHONY: all build clean fmt fmt-check init lint pre-commit test full-test

all: init build test

build:
	@echo ⚙️ Building a release...
	@cargo +nightly b -r --workspace
	@ls -l target/wasm32-unknown-unknown/release/*.wasm

fmt:
	@echo ⚙️ Checking a format...
	@cargo fmt --all --check

init:
	@echo ⚙️ Installing a toolchain \& a target...
	@rustup toolchain add nightly
	@rustup target add wasm32-unknown-unknown --toolchain nightly

lint:
	@echo ⚙️ Running the linter...
	@cargo +nightly clippy --workspace -- -D warnings

pre-commit: fmt lint full-test

test: 
	@echo ⚙️ Running unit tests...
	@if [ ! -f "./target/ft_main.wasm" ]; then\
	    curl -L\
	        "https://github.com/gear-dapps/sharded-fungible-token/releases/download/2.0.0/ft_main-2.0.0.opt.wasm"\
	        -o "./target/ft_main.wasm";\
	fi
	@if [ ! -f "./target/ft_logic.opt.wasm" ]; then\
	    curl -L\
	        "https://github.com/gear-dapps/sharded-fungible-token/releases/download/2.0.0/ft_logic-2.0.0.opt.wasm"\
	        -o "./target/ft_logic.opt.wasm";\
	fi
	@if [ ! -f "./target/ft_storage.opt.wasm" ]; then\
	    curl -L\
	        "https://github.com/gear-dapps/sharded-fungible-token/releases/download/2.0.0/ft_storage-2.0.0.opt.wasm"\
	        -o "./target/ft_storage.opt.wasm";\
	fi
	@echo ──────────── Run tests ────────────────────────
	@cargo +nightly test --release
	
full-test: deps
	@echo ⚙️ Running all tests...
	@cargo +nightly t -- --include-ignored