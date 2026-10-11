.PHONY: setup smtp up stop status dev dev-migrate dev-stop dev-status dev-logs dev-check check test format contracts smoke acceptance
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
dev:
	python3 scripts/dev.py --dev up
dev-migrate:
	python3 scripts/dev.py --dev migrate
dev-stop:
	python3 scripts/dev.py --dev stop
dev-status:
	python3 scripts/dev.py --dev status
dev-logs:
	python3 scripts/dev.py --dev logs
dev-check:
	python3 scripts/dev_smoke.py
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
