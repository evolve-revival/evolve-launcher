.PHONY: build test run migrate

build:
	go build -o bin/evolve-server ./cmd/server

test:
	go test ./...

test-integration:
	TEST_DATABASE_URL="$(TEST_DATABASE_URL)" go test ./...

run: build
	./bin/evolve-server

migrate:
	psql "$(DATABASE_URL)" -f internal/db/migrations/001_init.sql
