validate:
	wrkflw run  .github/workflows/build_and_prerelease_linux.yml
audit:
	cargo audit

upgrade:
	cargo upgrade

test:
	cargo test --all