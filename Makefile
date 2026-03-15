.PHONY: validate dedupe build-pages stale doctor book serve ci-content

validate:
	cargo run -p factctl -- validate

dedupe:
	cargo run -p factctl -- dedupe

build-pages:
	cargo run -p factctl -- build-pages

stale:
	cargo run -p factctl -- stale

doctor:
	cargo run -p factctl -- doctor

book: build-pages
	mdbook build

serve: build-pages
	mdbook serve

ci-content:
	cargo run -p factctl -- validate
	cargo run -p factctl -- dedupe --fail-on-high-confidence-duplicate
	cargo run -p factctl -- build-pages
	git diff --exit-code -- src generated
	mdbook build
