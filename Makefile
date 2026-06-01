.PHONY: dev build test clean deps

deps:
	@test -d web/node_modules || npm install --prefix web

dev: deps
	@echo "Starting dev servers..."
	@cargo run & npm run dev --prefix web

build: deps
	@echo "Building frontend..."
	@npm run build --prefix web
	@echo "Building backend..."
	@cargo build --release

test:
	@echo "Running backend tests..."
	@cargo test
	@echo "Running frontend tests..."
	@npm test --prefix web -- --run

clean:
	@echo "Cleaning..."
	@rm -rf target/
	@rm -rf web/node_modules/
	@rm -rf static/
	@echo "Done."
