.PHONY: validate dedupe build-pages build stale doctor book serve ci-content

validate:
	cargo run -p factctl -- validate

dedupe:
	cargo run -p factctl -- dedupe

build-pages:
	cargo run -p factctl -- build-pages

build: book

stale:
	cargo run -p factctl -- stale

doctor:
	cargo run -p factctl -- doctor

# Work around mdBook "Unable to remove stale HTML output" on repeated local rebuilds.
book: build-pages
	mdbook clean
	mdbook build

serve: build-pages
	# Use a separate output directory so a long-running preview server does not race with `make build`.
	mdbook serve -d book-serve

ci-content:
	cargo run -p factctl -- validate
	cargo run -p factctl -- dedupe --fail-on-high-confidence-duplicate
	cargo run -p factctl -- build-pages
	git diff --exit-code -- src generated
	mdbook clean
	mdbook build
