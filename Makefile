.PHONY: setup smtp up stop status check test format contracts smoke acceptance
setup:
	python3 scripts/dev.py configure
smtp:
	python3 scripts/configure_smtp.py
up:
	python3 scripts/dev.py up
stop:
	python3 scripts/dev.py stop
status:
	python3 scripts/dev.py status
format:
	cargo fmt --all
	pnpm format
contracts:
	pnpm api:generate
check:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets --locked -- -D warnings
	pnpm format:check
	pnpm check
	pnpm api:check
test:
	cargo test --workspace --locked
	python3 -m unittest discover -s scripts -p 'test_*.py'
smoke:
	python3 scripts/smoke.py

acceptance:
	python3 scripts/acceptance.py
