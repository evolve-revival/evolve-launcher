.PHONY: build test run migrate cert cap test-game

build:
	go build -o bin/evolve-server ./cmd/server

test:
	go test ./...

test-integration:
	TEST_DATABASE_URL="$(TEST_DATABASE_URL)" go test ./...

test-game:
	@bash scripts/test-game.sh

run: cap
	PORT=443 CERT_FILE=certs/server.crt KEY_FILE=certs/server.key /tmp/evolve-server-bin

cap: build
	cp ./bin/evolve-server /tmp/evolve-server-bin
	sudo setcap cap_net_bind_service=+ep /tmp/evolve-server-bin

cert:
	mkdir -p certs
	printf 'subjectAltName=IP:127.0.0.1\n' > certs/san.ext
	openssl genrsa -out certs/server.key 2048
	openssl req -new -key certs/server.key \
		-subj "/CN=127.0.0.1/O=EvolveRevival" \
		-out certs/server.csr
	openssl x509 -req -in certs/server.csr \
		-CA certs/ca.crt -CAkey certs/ca.key -CAcreateserial \
		-out certs/server.crt -days 3650 \
		-extfile certs/san.ext
	rm -f certs/server.csr certs/san.ext

migrate:
	psql "$(DATABASE_URL)" -f internal/db/migrations/001_init.sql
