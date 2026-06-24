package handler_test

import (
	"bytes"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/handler"
	"github.com/evolve-revival/evolve-server/internal/middleware"
	"github.com/evolve-revival/evolve-server/internal/store"
	"github.com/gin-gonic/gin"
)

const testDatasetId = "f47f9df6-b0b3-4c5e-9ab9-f9c1b0e3d1a0"

func setupStorage(t *testing.T) *gin.Engine {
	t.Helper()
	db := openTestDB(t)
	h := handler.NewStorageHandler(store.NewStorageStore(db))

	middleware.StoreToken("stor-tok", "player-stor-id")

	gin.SetMode(gin.TestMode)
	r := gin.New()
	r.Use(middleware.Auth())
	r.GET("/storage/1/data/:datasetId", h.List)
	r.PUT("/storage/1/data/:datasetId/:key", h.Put)
	r.DELETE("/storage/1/data/:datasetId/:key", h.Delete)
	return r
}

func TestStorage_PutAndList(t *testing.T) {
	r := setupStorage(t)

	// PUT
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodPut, "/storage/1/data/"+testDatasetId+"/mykey",
		bytes.NewBufferString(`{"score":42}`))
	req.Header.Set("Authorization", "Bearer stor-tok")
	r.ServeHTTP(w, req)
	if w.Code != http.StatusNoContent {
		t.Fatalf("PUT status = %d; body = %s", w.Code, w.Body)
	}

	// LIST
	w2 := httptest.NewRecorder()
	req2 := httptest.NewRequest(http.MethodGet, "/storage/1/data/"+testDatasetId, nil)
	req2.Header.Set("Authorization", "Bearer stor-tok")
	r.ServeHTTP(w2, req2)
	if w2.Code != http.StatusOK {
		t.Fatalf("LIST status = %d; body = %s", w2.Code, w2.Body)
	}
	if got := w2.Body.String(); len(got) < 10 {
		t.Errorf("LIST body too short: %s", got)
	}
}

func TestStorage_Delete(t *testing.T) {
	r := setupStorage(t)

	// PUT first
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodPut, "/storage/1/data/"+testDatasetId+"/delkey",
		bytes.NewBufferString(`{}`))
	req.Header.Set("Authorization", "Bearer stor-tok")
	r.ServeHTTP(w, req)

	// DELETE
	w2 := httptest.NewRecorder()
	req2 := httptest.NewRequest(http.MethodDelete, "/storage/1/data/"+testDatasetId+"/delkey", nil)
	req2.Header.Set("Authorization", "Bearer stor-tok")
	r.ServeHTTP(w2, req2)
	if w2.Code != http.StatusNoContent {
		t.Fatalf("DELETE status = %d", w2.Code)
	}
}
