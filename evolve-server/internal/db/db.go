package db

import (
	"database/sql"
	_ "github.com/lib/pq"
)

func Open(dsn string) (*sql.DB, error) {
	pool, err := sql.Open("postgres", dsn)
	if err != nil {
		return nil, err
	}
	if err := pool.Ping(); err != nil {
		return nil, err
	}
	return pool, nil
}
