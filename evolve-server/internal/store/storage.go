package store

import (
	"database/sql"
	"encoding/json"
	"fmt"

	"github.com/google/uuid"
)

type StorageItem struct {
	Id        string
	DatasetId string
	PlayerId  string
	ItemKey   string
	Data      json.RawMessage
}

type StorageStore struct{ db *sql.DB }

func NewStorageStore(db *sql.DB) *StorageStore { return &StorageStore{db: db} }

func (s *StorageStore) List(datasetId, playerId string) ([]StorageItem, error) {
	rows, err := s.db.Query(
		`SELECT id, dataset_id, player_id, item_key, data FROM storage_items
		 WHERE dataset_id = $1 AND player_id = $2`,
		datasetId, playerId,
	)
	if err != nil {
		return nil, fmt.Errorf("storage list: %w", err)
	}
	defer rows.Close()

	var items []StorageItem
	for rows.Next() {
		var item StorageItem
		if err := rows.Scan(&item.Id, &item.DatasetId, &item.PlayerId, &item.ItemKey, &item.Data); err != nil {
			return nil, err
		}
		items = append(items, item)
	}
	return items, rows.Err()
}

func (s *StorageStore) Put(datasetId, playerId, itemKey string, data json.RawMessage) error {
	id := uuid.New().String()
	_, err := s.db.Exec(
		`INSERT INTO storage_items (id, dataset_id, player_id, item_key, data)
		 VALUES ($1, $2, $3, $4, $5)
		 ON CONFLICT (dataset_id, player_id, item_key)
		 DO UPDATE SET data = $5, updated_at = NOW()`,
		id, datasetId, playerId, itemKey, data,
	)
	return err
}

func (s *StorageStore) Delete(datasetId, playerId, itemKey string) error {
	_, err := s.db.Exec(
		`DELETE FROM storage_items WHERE dataset_id=$1 AND player_id=$2 AND item_key=$3`,
		datasetId, playerId, itemKey,
	)
	return err
}
