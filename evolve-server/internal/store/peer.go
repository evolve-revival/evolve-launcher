package store

import (
	"database/sql"
)

type Peer struct {
	LobbyId  string
	PlayerId string
	IP       string
	Port     int
}

type PeerStore struct{ db *sql.DB }

func NewPeerStore(db *sql.DB) *PeerStore { return &PeerStore{db: db} }

func (s *PeerStore) Register(lobbyId, playerId, ip string, port int) error {
	_, err := s.db.Exec(
		`INSERT INTO peers (lobby_id, player_id, ip, port)
		 VALUES ($1, $2, $3, $4)
		 ON CONFLICT (lobby_id, player_id)
		 DO UPDATE SET ip = $3, port = $4, registered_at = NOW()`,
		lobbyId, playerId, ip, port,
	)
	return err
}

func (s *PeerStore) GetByLobby(lobbyId string) ([]Peer, error) {
	rows, err := s.db.Query(
		`SELECT lobby_id, player_id, ip, port FROM peers
		 WHERE lobby_id = $1
		   AND registered_at > NOW() - INTERVAL '5 minutes'`,
		lobbyId,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var peers []Peer
	for rows.Next() {
		var p Peer
		if err := rows.Scan(&p.LobbyId, &p.PlayerId, &p.IP, &p.Port); err != nil {
			return nil, err
		}
		peers = append(peers, p)
	}
	return peers, rows.Err()
}

func (s *PeerStore) PurgeExpired() error {
	_, err := s.db.Exec(`DELETE FROM peers WHERE registered_at < NOW() - INTERVAL '5 minutes'`)
	return err
}
