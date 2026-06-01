.PHONY: dev build test clean

dev:
	@echo "Starting dev servers..."
	@npm install --prefix web 2>/dev/null
	@cargo run & npm run dev --prefix web

build:
	@echo "Building frontend..."
	@npm install --prefix web 2>/dev/null
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
