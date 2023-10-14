# makefile frontend

clean:
	rm -rf dist
	rm -rf ../target

serve:
	cargo make serve --disable-check-for-updates

serve-release:
	cargo make serve-release --disable-check-for-updates

check:
	cargo make checks
	cargo make clippy-nightly

release:
	LA_ENV="prod" cargo make build-and-check
