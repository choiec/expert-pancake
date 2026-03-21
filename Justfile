set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
	@just --list

fmt:
	cargo fmt --all --check

fmt-fix:
	cargo fmt --all

lint:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

test-fast:
	cargo nextest run --workspace --lib --bins --tests

test-full:
	cargo nextest run --workspace --all-targets --all-features

verify-story feature="001-memory-ingest":
	case "{{feature}}" in \
	  001-memory-ingest) \
	    mkdir -p target/llvm-cov && \
	    cargo nextest run --workspace --all-targets --all-features && \
	    cargo llvm-cov nextest --workspace --all-features --lcov --output-path target/llvm-cov/{{feature}}.info \
	  ;; \
	  *) \
	    echo "Unknown feature: {{feature}}" >&2; exit 1 \
	  ;; \
	esac

mutants:
	cargo mutants --workspace --test-tool nextest

coverage:
	mkdir -p target/llvm-cov
	cargo llvm-cov nextest --workspace --all-features --lcov --output-path target/llvm-cov/lcov.info

bench:
	cargo bench --workspace --all-features