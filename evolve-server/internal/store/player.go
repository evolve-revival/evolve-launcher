package store

import (
	"database/sql"
	"fmt"
	"time"

	"github.com/google/uuid"
)

type Player struct {
	Id          string
	SteamId     string
	DisplayName string
	CreatedAt   time.Time
	IsNew       bool
}

type PlayerStore struct{ db *sql.DB }

func NewPlayerStore(db *sql.DB) *PlayerStore { return &PlayerStore{db: db} }

// UpsertBySteamId finds or creates a player row for the given Steam ID.
func (s *PlayerStore) UpsertBySteamId(steamId, displayName string) (*Player, error) {
	var p Player
	err := s.db.QueryRow(
		`SELECT id, steam_id, display_name, created_at FROM players WHERE steam_id = $1`,
		steamId,
	).Scan(&p.Id, &p.SteamId, &p.DisplayName, &p.CreatedAt)

	if err == sql.ErrNoRows {
		p.Id = uuid.New().String()
		p.SteamId = steamId
		p.DisplayName = displayName
		p.IsNew = true
		_, err = s.db.Exec(
			`INSERT INTO players (id, steam_id, display_name) VALUES ($1, $2, $3)`,
			p.Id, steamId, displayName,
		)
		if err != nil {
			return nil, fmt.Errorf("insert player: %w", err)
		}
		return &p, nil
	}
	if err != nil {
		return nil, fmt.Errorf("query player: %w", err)
	}
	return &p, nil
}
