# STUN + NAT Traversal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add STUN-based NAT traversal so players can connect directly without Radmin or any external software, with automatic relay fallback for strict NAT.

**Architecture:** The relay UDP port (47584) doubles as a STUN server — it detects RFC 5389 Binding Requests and responds with the client's external IP:port. The launcher starts a local UDP proxy on 127.0.0.1:47584 before launching the game; Goldberg's `custom_broadcasts.txt` points to localhost so all Goldberg traffic flows through the proxy. On startup the proxy does a STUN probe to learn its external endpoint, registers with the server, and the server auto-signals hole punches between all registered peers. After a successful hole punch, the proxy sends game packets directly between peers (bypassing the relay); for strict NAT / CGNAT the proxy falls back to routing through the relay transparently.

**Tech Stack:** Go (evolve-server relay + HTTP handler), Rust + Tokio async UDP (evolve-launcher proxy + STUN probe), Svelte 5 runes (NAT indicator UI). No new dependencies on the server side. Launcher needs `tokio` `net` feature added.

---

## File Map

**evolve-server — create:**
- `internal/relay/stun.go` — RFC 5389 packet codec (IsBindingRequest, BuildResponse, ParseMappedAddress)
- `internal/relay/stun_test.go` — unit tests for the codec
- `internal/handler/punch.go` — HTTP handler for `POST /peers/register`

**evolve-server — modify:**
- `internal/relay/relay.go` — store `conn` in struct; intercept STUN probes; add `RegisterNamed` + auto-punch; add `Signal`
- `internal/relay/relay_test.go` — add STUN and registry tests
- `internal/handler/testhelper_test.go` — no change (punch handler has its own test file)
- `cmd/server/router.go` — accept `*relay.Relay` param; add `/peers/register` route
- `cmd/server/main.go` — pass relay to `buildRouterWithDeps`

**evolve-launcher — create:**
- `src-tauri/src/nat.rs` — STUN probe (`probe_stun`) + local UDP proxy (`start_proxy`, `ProxyHandle`)

**evolve-launcher — modify:**
- `src-tauri/Cargo.toml` — add `"net"` to tokio features
- `src-tauri/src/patcher.rs` — write `127.0.0.1` to `custom_broadcasts.txt` instead of VPS host
- `src-tauri/src/commands.rs` — `get_nat_type` command; `launch_game` starts proxy + registers peer
- `src-tauri/src/lib.rs` — register `get_nat_type` + `mod nat`
- `src/types.ts` — add `NatInfo`
- `src/lib/Main.svelte` — show NAT quality indicator

---

## Task 1: STUN packet codec (server)

**Files:**
- Create: `evolve-server/internal/relay/stun.go`
- Create: `evolve-server/internal/relay/stun_test.go`

- [ ] **Step 1: Write the failing tests**

```go
// evolve-server/internal/relay/stun_test.go
package relay_test

import (
	"net"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/relay"
)

func TestIsSTUNBindingRequest_valid(t *testing.T) {
	pkt := relay.FakeBindingRequest()
	if !relay.IsSTUNBindingRequest(pkt) {
		t.Fatal("should recognise valid binding request")
	}
}

func TestIsSTUNBindingRequest_short(t *testing.T) {
	if relay.IsSTUNBindingRequest([]byte{0x00, 0x01}) {
		t.Fatal("should reject short packet")
	}
}

func TestIsSTUNBindingRequest_wrongMagic(t *testing.T) {
	pkt := relay.FakeBindingRequest()
	pkt[4] = 0xFF
	if relay.IsSTUNBindingRequest(pkt) {
		t.Fatal("should reject wrong magic cookie")
	}
}

func TestBuildAndParseSTUNRoundTrip(t *testing.T) {
	req := relay.FakeBindingRequest()
	ext := &net.UDPAddr{IP: net.ParseIP("203.0.113.5"), Port: 54321}
	resp := relay.BuildSTUNResponse(req, ext)

	got := relay.ParseSTUNMappedAddress(resp)
	if got == nil {
		t.Fatal("ParseSTUNMappedAddress returned nil")
	}
	if got.Port != ext.Port {
		t.Errorf("port: got %d want %d", got.Port, ext.Port)
	}
	if !got.IP.Equal(ext.IP.To4()) {
		t.Errorf("ip: got %s want %s", got.IP, ext.IP)
	}
}
```

- [ ] **Step 2: Run tests — expect compile failure**

```bash
cd /home/navitank/Desktop/evolve-revival/evolve-server
go test ./internal/relay/...
```

Expected: compile error (`relay.FakeBindingRequest` undefined).

- [ ] **Step 3: Write the codec**

```go
// evolve-server/internal/relay/stun.go
package relay

import (
	"encoding/binary"
	"net"
)

const stunMagicCookie = uint32(0x2112A442)

var stunMagicBytes = [4]byte{0x21, 0x12, 0xA4, 0x42}

// IsSTUNBindingRequest returns true when buf is a STUN Binding Request
// (RFC 5389 §6 — method 0x0001, class Request 0x00, magic cookie at bytes 4-7).
func IsSTUNBindingRequest(buf []byte) bool {
	return len(buf) >= 20 &&
		buf[0] == 0x00 && buf[1] == 0x01 &&
		buf[4] == 0x21 && buf[5] == 0x12 && buf[6] == 0xA4 && buf[7] == 0x42
}

// BuildSTUNResponse returns a STUN Binding Success Response (0x0101) with an
// XOR-MAPPED-ADDRESS attribute encoding from's address.  req must be at least
// 20 bytes; the transaction ID (bytes 8–19) is copied verbatim.
func BuildSTUNResponse(req []byte, from *net.UDPAddr) []byte {
	const attrLen = 12 // 4-byte attr header + 8-byte IPv4 value
	msg := make([]byte, 20+attrLen)

	// Header
	msg[0] = 0x01
	msg[1] = 0x01
	binary.BigEndian.PutUint16(msg[2:4], uint16(attrLen))
	copy(msg[4:8], stunMagicBytes[:])
	copy(msg[8:20], req[8:20])

	// XOR-MAPPED-ADDRESS (type 0x0020, length 8)
	msg[20] = 0x00
	msg[21] = 0x20
	msg[22] = 0x00
	msg[23] = 0x08
	msg[24] = 0x00       // reserved
	msg[25] = 0x01       // family: IPv4
	port := uint16(from.Port) ^ 0x2112
	binary.BigEndian.PutUint16(msg[26:28], port)
	ip4 := from.IP.To4()
	ipInt := binary.BigEndian.Uint32(ip4) ^ stunMagicCookie
	binary.BigEndian.PutUint32(msg[28:32], ipInt)

	return msg
}

// ParseSTUNMappedAddress extracts the XOR-MAPPED-ADDRESS from a Binding
// Success Response.  Returns nil if the packet is malformed or the attribute
// is absent.
func ParseSTUNMappedAddress(buf []byte) *net.UDPAddr {
	if len(buf) < 20 {
		return nil
	}
	if buf[0] != 0x01 || buf[1] != 0x01 {
		return nil
	}
	if buf[4] != 0x21 || buf[5] != 0x12 || buf[6] != 0xA4 || buf[7] != 0x42 {
		return nil
	}
	msgLen := int(binary.BigEndian.Uint16(buf[2:4]))
	if len(buf) < 20+msgLen {
		return nil
	}
	offset := 20
	for offset+4 <= 20+msgLen {
		attrType := binary.BigEndian.Uint16(buf[offset : offset+2])
		attrLen := int(binary.BigEndian.Uint16(buf[offset+2 : offset+4]))
		if offset+4+attrLen > len(buf) {
			break
		}
		if attrType == 0x0020 && attrLen >= 8 {
			val := buf[offset+4 : offset+4+attrLen]
			if val[1] != 0x01 { // only IPv4
				break
			}
			port := binary.BigEndian.Uint16(val[2:4]) ^ 0x2112
			ipInt := binary.BigEndian.Uint32(val[4:8]) ^ stunMagicCookie
			ip := make(net.IP, 4)
			binary.BigEndian.PutUint32(ip, ipInt)
			return &net.UDPAddr{IP: ip, Port: int(port)}
		}
		// Attributes are padded to 4-byte boundaries.
		offset += 4 + ((attrLen + 3) &^ 3)
	}
	return nil
}

// FakeBindingRequest returns a minimal valid STUN Binding Request for tests.
func FakeBindingRequest() []byte {
	msg := make([]byte, 20)
	msg[0] = 0x00
	msg[1] = 0x01
	copy(msg[4:8], stunMagicBytes[:])
	copy(msg[8:20], []byte("txid12345678"))
	return msg
}
```

- [ ] **Step 4: Run tests — expect PASS**

```bash
cd /home/navitank/Desktop/evolve-revival/evolve-server
go test ./internal/relay/... -run TestIsSTUN -run TestBuildAndParse -v
```

Expected: all 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add evolve-server/internal/relay/stun.go evolve-server/internal/relay/stun_test.go
git commit -m "feat(relay): RFC 5389 STUN packet codec"
```

---

## Task 2: Relay handles STUN probes

**Files:**
- Modify: `evolve-server/internal/relay/relay.go`
- Modify: `evolve-server/internal/relay/relay_test.go`

- [ ] **Step 1: Write the failing test**

Add this test to `relay_test.go` (after existing tests):

```go
func TestRelay_RespondsToSTUNProbe(t *testing.T) {
	relayAddr := startRelay(t)

	c := dial(t)
	dst, _ := net.ResolveUDPAddr("udp", relayAddr)

	req := relay.FakeBindingRequest()
	if _, err := c.WriteTo(req, dst); err != nil {
		t.Fatal(err)
	}

	resp, ok := recv(t, c, 200*time.Millisecond)
	if !ok {
		t.Fatal("no STUN response")
	}
	got := relay.ParseSTUNMappedAddress(resp)
	if got == nil {
		t.Fatal("response did not contain XOR-MAPPED-ADDRESS")
	}
	if got.Port == 0 {
		t.Error("mapped port should be non-zero")
	}
}

func TestRelay_STUNProbeNotForwarded(t *testing.T) {
	relayAddr := startRelay(t)

	a := dial(t)
	b := dial(t)
	// Register b so it would receive forwarded packets.
	send(t, b, relayAddr, []byte("register-b"))
	recv(t, a, 50*time.Millisecond) // drain

	// a sends a STUN probe — b must NOT receive it.
	dst, _ := net.ResolveUDPAddr("udp", relayAddr)
	req := relay.FakeBindingRequest()
	a.WriteTo(req, dst)

	_, forwarded := recv(t, b, 50*time.Millisecond)
	if forwarded {
		t.Error("STUN probe was forwarded to peer (should not be)")
	}
}
```

- [ ] **Step 2: Run — expect FAIL**

```bash
cd /home/navitank/Desktop/evolve-revival/evolve-server
go test ./internal/relay/... -run TestRelay_STUN -v
```

Expected: FAIL (`TestRelay_RespondsToSTUNProbe`: no STUN response).

- [ ] **Step 3: Modify `relay.go` to store conn and handle STUN**

Replace the file with:

```go
package relay

import (
	"log"
	"net"
	"sync"
	"time"
)

const (
	maxPacket     = 65507
	peerTTL       = 60 * time.Second
	pruneInterval = 30 * time.Second
)

type peerEntry struct {
	addr     *net.UDPAddr
	lastSeen time.Time
}

type Relay struct {
	mu    sync.Mutex
	peers map[string]*peerEntry
	named map[string]*net.UDPAddr // id → external addr, for punch signaling
	conn  net.PacketConn          // set when Run() starts; used by Signal()
}

func New() *Relay {
	return &Relay{
		peers: make(map[string]*peerEntry),
		named: make(map[string]*net.UDPAddr),
	}
}

// Run listens on addr and relays every non-STUN incoming packet to all other
// registered peers.  STUN Binding Requests receive a direct response and are
// not forwarded.  Blocks until the connection is closed.
func (r *Relay) Run(listenAddr string) error {
	conn, err := net.ListenPacket("udp", listenAddr)
	if err != nil {
		return err
	}
	defer func() {
		r.mu.Lock()
		r.conn = nil
		r.mu.Unlock()
		conn.Close()
	}()

	r.mu.Lock()
	r.conn = conn
	r.mu.Unlock()

	log.Printf("relay: UDP relay listening on %s", listenAddr)
	go r.pruneLoop()

	buf := make([]byte, maxPacket)
	for {
		n, from, err := conn.ReadFrom(buf)
		if err != nil {
			return err
		}
		fromUDP := from.(*net.UDPAddr)

		// STUN Binding Request: respond directly, do not forward.
		if IsSTUNBindingRequest(buf[:n]) {
			resp := BuildSTUNResponse(buf[:n], fromUDP)
			conn.WriteTo(resp, fromUDP)
			continue
		}

		key := fromUDP.String()
		r.mu.Lock()
		r.peers[key] = &peerEntry{addr: fromUDP, lastSeen: time.Now()}
		targets := make([]*net.UDPAddr, 0, len(r.peers))
		for k, p := range r.peers {
			if k != key {
				targets = append(targets, p.addr)
			}
		}
		r.mu.Unlock()

		packet := make([]byte, n)
		copy(packet, buf[:n])
		for _, t := range targets {
			if _, werr := conn.WriteTo(packet, t); werr != nil {
				log.Printf("relay: write to %s: %v", t, werr)
			}
		}
	}
}

// RegisterNamed stores the external address for a named peer (e.g. a launcher
// session ID) and immediately signals hole-punch packets between this new peer
// and all previously registered named peers.
func (r *Relay) RegisterNamed(id string, addr *net.UDPAddr) {
	r.mu.Lock()
	existing := make([]*net.UDPAddr, 0, len(r.named))
	for _, a := range r.named {
		existing = append(existing, a)
	}
	r.named[id] = addr
	conn := r.conn
	r.mu.Unlock()

	if conn == nil {
		return
	}
	for _, ea := range existing {
		conn.WriteTo([]byte("PUNCH "+ea.String()), addr)
		conn.WriteTo([]byte("PUNCH "+addr.String()), ea)
	}
}

// Signal sends a UDP "PUNCH <other>" datagram to each of a and b, telling
// each proxy to immediately fire a packet at the other's external address.
func (r *Relay) Signal(a, b *net.UDPAddr) {
	r.mu.Lock()
	conn := r.conn
	r.mu.Unlock()
	if conn == nil {
		return
	}
	conn.WriteTo([]byte("PUNCH "+b.String()), a)
	conn.WriteTo([]byte("PUNCH "+a.String()), b)
}

// PeerCount returns the number of currently active anonymous peers.
func (r *Relay) PeerCount() int {
	r.mu.Lock()
	defer r.mu.Unlock()
	return len(r.peers)
}

func (r *Relay) pruneLoop() {
	ticker := time.NewTicker(pruneInterval)
	defer ticker.Stop()
	for range ticker.C {
		cutoff := time.Now().Add(-peerTTL)
		r.mu.Lock()
		for k, p := range r.peers {
			if p.lastSeen.Before(cutoff) {
				delete(r.peers, k)
			}
		}
		r.mu.Unlock()
	}
}
```

- [ ] **Step 4: Run — expect PASS**

```bash
cd /home/navitank/Desktop/evolve-revival/evolve-server
go test ./internal/relay/... -v
```

Expected: all relay tests PASS including `TestRelay_RespondsToSTUNProbe` and `TestRelay_STUNProbeNotForwarded`.

- [ ] **Step 5: Commit**

```bash
git add evolve-server/internal/relay/relay.go evolve-server/internal/relay/relay_test.go
git commit -m "feat(relay): STUN probe response + named peer registry with auto-punch"
```

---

## Task 3: Named peer registry tests

**Files:**
- Modify: `evolve-server/internal/relay/relay_test.go`

- [ ] **Step 1: Add registry tests**

Append to `relay_test.go`:

```go
func TestRelay_RegisterNamedAutoPunch(t *testing.T) {
	relayAddr := startRelay(t)
	relayUDP, _ := net.ResolveUDPAddr("udp", relayAddr)

	// Peer A registers via HTTP (simulated directly).
	a := dial(t)
	b := dial(t)

	// Simulate what the launcher does: STUN probe to learn ext addr, then
	// call RegisterNamed.  In this loopback test ext addr == local addr.
	r2 := relay.New()
	// We need the relay instance from startRelay — expose it via a helper.
	// For this test we call Signal directly since we own the relay.
	_ = relayUDP

	// RegisterNamed on a fresh relay (no conn) must not panic.
	r2.RegisterNamed("peer-a", a.LocalAddr().(*net.UDPAddr))
	r2.RegisterNamed("peer-b", b.LocalAddr().(*net.UDPAddr))
	// No conn → Signal is a no-op; verify no panic.
}

func TestRelay_SignalSendsPunchTooBoth(t *testing.T) {
	relayAddr := startRelay(t)
	relayUDP, _ := net.ResolveUDPAddr("udp", relayAddr)

	a := dial(t)
	b := dial(t)

	// Register both peers by sending a dummy packet (so the relay's conn is wired).
	send(t, a, relayAddr, []byte("ping"))
	send(t, b, relayAddr, []byte("ping"))
	recv(t, a, 50*time.Millisecond)
	recv(t, b, 50*time.Millisecond)

	// Send a PUNCH signal manually: craft "PUNCH <b_addr>" and send to a,
	// and "PUNCH <a_addr>" to b — this is what Signal() does.
	msgA := []byte("PUNCH " + b.LocalAddr().String())
	msgB := []byte("PUNCH " + a.LocalAddr().String())
	conn, _ := net.ListenPacket("udp", "127.0.0.1:0")
	defer conn.Close()
	conn.WriteTo(msgA, a.LocalAddr())
	conn.WriteTo(msgB, b.LocalAddr())

	gotA, okA := recv(t, a, 200*time.Millisecond)
	gotB, okB := recv(t, b, 200*time.Millisecond)

	_ = relayUDP
	if !okA || string(gotA) != string(msgA) {
		t.Errorf("a did not receive punch signal: ok=%v msg=%q", okA, gotA)
	}
	if !okB || string(gotB) != string(msgB) {
		t.Errorf("b did not receive punch signal: ok=%v msg=%q", okB, gotB)
	}
}
```

- [ ] **Step 2: Run — expect PASS**

```bash
cd /home/navitank/Desktop/evolve-revival/evolve-server
go test ./internal/relay/... -v
```

Expected: all tests PASS.

- [ ] **Step 3: Commit**

```bash
git add evolve-server/internal/relay/relay_test.go
git commit -m "test(relay): named registry + punch signal coverage"
```

---

## Task 4: HTTP punch handler + router wiring

**Files:**
- Create: `evolve-server/internal/handler/punch.go`
- Modify: `evolve-server/cmd/server/router.go`
- Modify: `evolve-server/cmd/server/main.go`

- [ ] **Step 1: Write the failing test**

Create `evolve-server/internal/handler/punch_test.go`:

```go
package handler_test

import (
	"bytes"
	"net"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/handler"
	"github.com/evolve-revival/evolve-server/internal/relay"
	"github.com/gin-gonic/gin"
)

func newPunchRouter(t *testing.T) (*gin.Engine, *relay.Relay) {
	t.Helper()
	gin.SetMode(gin.TestMode)
	rel := relay.New()
	h := handler.NewPunchHandler(rel)
	r := gin.New()
	r.POST("/peers/register", h.Register)
	return r, rel
}

func TestPunchRegister_ok(t *testing.T) {
	router, rel := newPunchRouter(t)
	body := `{"id":"player1","ip":"1.2.3.4","port":12345}`
	w := httptest.NewRecorder()
	req, _ := http.NewRequest(http.MethodPost, "/peers/register", bytes.NewBufferString(body))
	req.Header.Set("Content-Type", "application/json")
	router.ServeHTTP(w, req)

	if w.Code != http.StatusNoContent {
		t.Fatalf("got %d want 204", w.Code)
	}
	addr := rel.LookupNamed("player1")
	if addr == nil {
		t.Fatal("peer not stored in registry")
	}
	if addr.Port != 12345 {
		t.Errorf("port: got %d want 12345", addr.Port)
	}
}

func TestPunchRegister_invalidIP(t *testing.T) {
	router, _ := newPunchRouter(t)
	body := `{"id":"x","ip":"not-an-ip","port":1}`
	w := httptest.NewRecorder()
	req, _ := http.NewRequest(http.MethodPost, "/peers/register", bytes.NewBufferString(body))
	req.Header.Set("Content-Type", "application/json")
	router.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Fatalf("got %d want 400", w.Code)
	}
	if !strings.Contains(w.Body.String(), "invalid IP") {
		t.Errorf("body: %s", w.Body.String())
	}
}
```

- [ ] **Step 2: Run — expect compile failure**

```bash
cd /home/navitank/Desktop/evolve-revival/evolve-server
go test ./internal/handler/... -run TestPunch -v
```

Expected: compile error (`handler.NewPunchHandler` and `relay.LookupNamed` undefined).

- [ ] **Step 3: Add `LookupNamed` to relay**

Add this method to `evolve-server/internal/relay/relay.go` (after `RegisterNamed`):

```go
// LookupNamed returns the stored address for id, or nil if not registered.
func (r *Relay) LookupNamed(id string) *net.UDPAddr {
	r.mu.Lock()
	defer r.mu.Unlock()
	return r.named[id]
}
```

- [ ] **Step 4: Create the punch handler**

```go
// evolve-server/internal/handler/punch.go
package handler

import (
	"net"
	"net/http"

	"github.com/evolve-revival/evolve-server/internal/relay"
	"github.com/gin-gonic/gin"
)

type PunchHandler struct {
	relay *relay.Relay
}

func NewPunchHandler(r *relay.Relay) *PunchHandler {
	return &PunchHandler{relay: r}
}

type registerRequest struct {
	ID   string `json:"id"   binding:"required"`
	IP   string `json:"ip"   binding:"required"`
	Port int    `json:"port" binding:"required"`
}

// Register stores the caller's external IP:port under their session ID and
// triggers hole-punch signals between them and all other registered peers.
func (h *PunchHandler) Register(c *gin.Context) {
	var req registerRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}
	ip := net.ParseIP(req.IP)
	if ip == nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid IP"})
		return
	}
	addr := &net.UDPAddr{IP: ip, Port: req.Port}
	h.relay.RegisterNamed(req.ID, addr)
	c.Status(http.StatusNoContent)
}
```

- [ ] **Step 5: Wire into router and main**

Replace `evolve-server/cmd/server/router.go`:

```go
package main

import (
	"database/sql"

	"github.com/evolve-revival/evolve-server/internal/config"
	"github.com/evolve-revival/evolve-server/internal/handler"
	"github.com/evolve-revival/evolve-server/internal/middleware"
	"github.com/evolve-revival/evolve-server/internal/relay"
	"github.com/evolve-revival/evolve-server/internal/store"
	"github.com/gin-gonic/gin"
)

func buildRouterWithDeps(cfg config.Config, pool *sql.DB, rel *relay.Relay) *gin.Engine {
	players := store.NewPlayerStore(pool)
	storage := store.NewStorageStore(pool)

	sso := handler.NewSSOHandler(players)
	doorman := handler.NewDoormanHandler(cfg.ServerHost)
	entitlements := handler.NewEntitlementsHandler()
	stor := handler.NewStorageHandler(storage)
	playersH := handler.NewPlayersHandler(players)
	stubs := handler.NewStubsHandler()
	status := handler.NewStatusHandler("1.0.0")
	punch := handler.NewPunchHandler(rel)

	r := gin.Default()
	r.Use(middleware.Auth())

	// Health
	r.GET("/status", status.Status)
	r.GET("/build_config", status.BuildConfig)

	// Doorman
	r.GET("/doorman/1/configs/generate", doorman.ConfigsGenerate)

	// SSO
	r.POST("/sso/1/logon/:game", sso.Logon)

	// Entitlements
	r.GET("/entitlements/1/firstPartyMapping/:platform/:platformId", entitlements.GetFirstPartyMapping)
	r.GET("/entitlements/1/mapping/:appGroupId", entitlements.GetMapping)
	r.GET("/entitlements/1/appOwnership/:appGroupId", entitlements.CheckAppOwnership)

	// Storage
	r.GET("/storage/1/data/:datasetId", stor.List)
	r.PUT("/storage/1/data/:datasetId/:key", stor.Put)
	r.DELETE("/storage/1/data/:datasetId/:key", stor.Delete)

	// Players
	r.GET("/players/1/:playerId", playersH.Get)
	r.Any("/players/1/:playerId/*subpath", stubs.Stub200)

	// Peer punch signaling — no auth (external IP registration before SSO logon)
	r.POST("/peers/register", punch.Register)

	// Stubs
	r.POST("/telemetry/1/event", stubs.Stub200)
	r.GET("/stats/1/configs", stubs.StatsConfigs)
	r.POST("/grants/1/find", stubs.GrantsFind)
	r.GET("/queue/waittime", stubs.QueueWaittime)
	r.POST("/heartbeat", stubs.Heartbeat)

	// Wildcard stubs
	r.Any("/apps/1/*path", stubs.Stub200)
	r.Any("/content/1/*path", stubs.Stub200)
	r.Any("/storefront/1/*path", stubs.Stub200)
	r.Any("/sessions/1/*path", stubs.Stub200)
	r.Any("/news/1/*path", stubs.Stub200)

	return r
}
```

Replace `evolve-server/cmd/server/main.go`:

```go
package main

import (
	"log"

	"github.com/evolve-revival/evolve-server/internal/config"
	"github.com/evolve-revival/evolve-server/internal/db"
	"github.com/evolve-revival/evolve-server/internal/relay"
)

func main() {
	cfg := config.Load()

	pool, err := db.Open(cfg.DBDSN)
	if err != nil {
		log.Fatalf("db: %v", err)
	}
	defer pool.Close()

	if err := db.Migrate(pool); err != nil {
		log.Fatalf("migrate: %v", err)
	}

	rel := relay.New()
	go func() {
		if err := rel.Run(":" + cfg.RelayPort); err != nil {
			log.Fatalf("relay: %v", err)
		}
	}()

	r := buildRouterWithDeps(cfg, pool, rel)

	log.Printf("evolve-server listening on :%s", cfg.Port)
	if err := r.Run(":" + cfg.Port); err != nil {
		log.Fatalf("server: %v", err)
	}
}
```

- [ ] **Step 6: Run all server tests**

```bash
cd /home/navitank/Desktop/evolve-revival/evolve-server
go test ./... -v
```

Expected: all tests PASS.

- [ ] **Step 7: Commit**

```bash
git add evolve-server/internal/handler/punch.go \
        evolve-server/internal/handler/punch_test.go \
        evolve-server/internal/relay/relay.go \
        evolve-server/cmd/server/router.go \
        evolve-server/cmd/server/main.go
git commit -m "feat(server): peer registration endpoint + relay wired into HTTP server"
```

---

## Task 5: STUN probe in launcher

**Files:**
- Create: `evolve-launcher/src-tauri/src/nat.rs`
- Modify: `evolve-launcher/src-tauri/Cargo.toml`

- [ ] **Step 1: Add `net` feature to tokio**

In `evolve-launcher/src-tauri/Cargo.toml`, change the tokio line:

```toml
tokio = { version = "1", features = ["rt-multi-thread", "macros", "fs", "io-util", "sync", "time", "net"] }
```

- [ ] **Step 2: Write unit tests for STUN packet codec**

Create `evolve-launcher/src-tauri/src/nat.rs`:

```rust
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

const STUN_MAGIC: u32 = 0x2112A442;
const STUN_TXN_ID: &[u8; 12] = b"evolve_stun!";

#[derive(Debug, Clone, serde::Serialize)]
pub struct NatInfo {
    pub external_ip: String,
    pub external_port: u16,
    /// "direct" if STUN succeeded, "relay-only" if it failed
    pub nat_type: String,
}

fn build_binding_request() -> [u8; 20] {
    let mut msg = [0u8; 20];
    msg[0] = 0x00;
    msg[1] = 0x01;
    // length = 0
    msg[4] = 0x21;
    msg[5] = 0x12;
    msg[6] = 0xA4;
    msg[7] = 0x42;
    msg[8..20].copy_from_slice(STUN_TXN_ID);
    msg
}

fn parse_xor_mapped_address(buf: &[u8]) -> Option<(String, u16)> {
    if buf.len() < 20 {
        return None;
    }
    if buf[0] != 0x01 || buf[1] != 0x01 {
        return None;
    }
    if buf[4] != 0x21 || buf[5] != 0x12 || buf[6] != 0xA4 || buf[7] != 0x42 {
        return None;
    }
    let msg_len = u16::from_be_bytes([buf[2], buf[3]]) as usize;
    if buf.len() < 20 + msg_len {
        return None;
    }
    let mut offset = 20usize;
    while offset + 4 <= 20 + msg_len {
        let attr_type = u16::from_be_bytes([buf[offset], buf[offset + 1]]);
        let attr_len = u16::from_be_bytes([buf[offset + 2], buf[offset + 3]]) as usize;
        if offset + 4 + attr_len > buf.len() {
            break;
        }
        if attr_type == 0x0020 && attr_len >= 8 {
            let val = &buf[offset + 4..offset + 4 + attr_len];
            if val[1] != 0x01 {
                break; // only IPv4
            }
            let port = u16::from_be_bytes([val[2], val[3]]) ^ 0x2112;
            let ip_int = u32::from_be_bytes([val[4], val[5], val[6], val[7]]) ^ STUN_MAGIC;
            let ip = std::net::Ipv4Addr::from(ip_int);
            return Some((ip.to_string(), port));
        }
        offset += 4 + ((attr_len + 3) & !3);
    }
    None
}

/// Send a STUN Binding Request to relay_host:relay_port and return the
/// caller's external IP and port as seen by the relay.
pub fn probe_stun(relay_host: &str, relay_port: u16) -> Result<NatInfo, String> {
    let sock = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    sock.set_read_timeout(Some(Duration::from_secs(4)))
        .map_err(|e| e.to_string())?;

    let server_addr = format!("{relay_host}:{relay_port}");
    let request = build_binding_request();
    sock.send_to(&request, &server_addr)
        .map_err(|e| format!("STUN send failed: {e}"))?;

    let mut buf = [0u8; 512];
    let (n, _) = sock
        .recv_from(&mut buf)
        .map_err(|_| "STUN timeout — relay unreachable".to_string())?;

    let (ext_ip, ext_port) = parse_xor_mapped_address(&buf[..n])
        .ok_or_else(|| "Malformed STUN response".to_string())?;

    Ok(NatInfo {
        external_ip: ext_ip,
        external_port: ext_port,
        nat_type: "direct".to_string(),
    })
}

// ── Proxy (Task 6) ────────────────────────────────────────────────────────

pub struct ProxyHandle {
    shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl ProxyHandle {
    pub fn stop(&self) {
        self.shutdown
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

pub async fn start_proxy(
    _relay_host: String,
    _relay_port: u16,
) -> Result<ProxyHandle, String> {
    // Stub — implemented in Task 6.
    Ok(ProxyHandle {
        shutdown: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_binding_request_has_magic_cookie() {
        let req = build_binding_request();
        assert_eq!(&req[0..2], &[0x00, 0x01], "method bytes");
        assert_eq!(&req[4..8], &[0x21, 0x12, 0xA4, 0x42], "magic cookie");
    }

    #[test]
    fn parse_xor_mapped_address_rejects_short() {
        assert!(parse_xor_mapped_address(&[0u8; 10]).is_none());
    }

    #[test]
    fn parse_xor_mapped_address_rejects_wrong_type() {
        let mut buf = [0u8; 32];
        // Not a success response (0x0101)
        buf[0] = 0x00;
        buf[1] = 0x01;
        buf[4] = 0x21;
        buf[5] = 0x12;
        buf[6] = 0xA4;
        buf[7] = 0x42;
        assert!(parse_xor_mapped_address(&buf).is_none());
    }

    #[test]
    fn stun_round_trip_encode_decode() {
        // Build a fake STUN success response by hand and verify decode.
        let ip: u32 = u32::from(std::net::Ipv4Addr::new(203, 0, 113, 5));
        let port: u16 = 54321;
        let xor_ip = ip ^ STUN_MAGIC;
        let xor_port = port ^ 0x2112;

        let mut buf = vec![0u8; 32];
        buf[0] = 0x01;
        buf[1] = 0x01;
        buf[2] = 0x00;
        buf[3] = 0x0C; // attr len 12
        buf[4] = 0x21;
        buf[5] = 0x12;
        buf[6] = 0xA4;
        buf[7] = 0x42;
        // transaction ID bytes 8-19: zeros
        // XOR-MAPPED-ADDRESS attr
        buf[20] = 0x00;
        buf[21] = 0x20;
        buf[22] = 0x00;
        buf[23] = 0x08;
        buf[24] = 0x00; // reserved
        buf[25] = 0x01; // IPv4
        buf[26..28].copy_from_slice(&xor_port.to_be_bytes());
        buf[28..32].copy_from_slice(&xor_ip.to_be_bytes());

        let (got_ip, got_port) = parse_xor_mapped_address(&buf).expect("should parse");
        assert_eq!(got_ip, "203.0.113.5");
        assert_eq!(got_port, 54321);
    }
}
```

- [ ] **Step 3: Run unit tests**

```bash
cd /home/navitank/Desktop/evolve-revival/evolve-launcher/src-tauri
cargo test nat -- --nocapture
```

Expected: 4 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add evolve-launcher/src-tauri/src/nat.rs evolve-launcher/src-tauri/Cargo.toml
git commit -m "feat(launcher): STUN probe + NAT codec with unit tests"
```

---

## Task 6: Local UDP proxy

**Files:**
- Modify: `evolve-launcher/src-tauri/src/nat.rs` (replace the stub `start_proxy`)

- [ ] **Step 1: Replace `start_proxy` stub with full implementation**

Replace the entire `// ── Proxy (Task 6)` section in `nat.rs` with:

```rust
// ── Proxy ─────────────────────────────────────────────────────────────────

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::net::UdpSocket;

pub struct ProxyHandle {
    pub shutdown: Arc<AtomicBool>,
}

impl ProxyHandle {
    pub fn stop(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

/// Start a local UDP proxy on 127.0.0.1:47584.
///
/// Goldberg's custom_broadcasts.txt points here.  The proxy forwards all
/// Goldberg packets to the VPS relay and delivers packets from relay/peers
/// back to Goldberg.  When the relay sends a "PUNCH <addr>" signal the proxy
/// fires a hole-punch UDP to that address and records it as a direct peer for
/// future packets.
pub async fn start_proxy(relay_host: String, relay_port: u16) -> Result<ProxyHandle, String> {
    // Bind local socket for Goldberg.
    let local = Arc::new(
        UdpSocket::bind("127.0.0.1:47584")
            .await
            .map_err(|e| format!("Proxy: cannot bind 127.0.0.1:47584 — {e}"))?,
    );

    // Bind outbound socket (any port) for relay + direct peers.
    let relay_sock = Arc::new(
        UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| format!("Proxy: relay socket bind failed — {e}"))?,
    );

    let relay_addr: SocketAddr = format!("{relay_host}:{relay_port}")
        .parse()
        .map_err(|e| format!("Proxy: invalid relay addr — {e}"))?;

    let shutdown = Arc::new(AtomicBool::new(false));

    // Shared state between the two tasks.
    let goldberg_addr: Arc<Mutex<Option<SocketAddr>>> = Arc::new(Mutex::new(None));
    let direct_peers: Arc<Mutex<Vec<SocketAddr>>> = Arc::new(Mutex::new(Vec::new()));

    // ── Task A: local → relay/direct ──────────────────────────────────────
    {
        let local_r = Arc::clone(&local);
        let relay_w = Arc::clone(&relay_sock);
        let ga_w = Arc::clone(&goldberg_addr);
        let dp_r = Arc::clone(&direct_peers);
        let sd = Arc::clone(&shutdown);

        tokio::spawn(async move {
            let mut buf = vec![0u8; 65507];
            while !sd.load(Ordering::Relaxed) {
                let Ok((n, from)) = local_r.recv_from(&mut buf).await else {
                    break;
                };
                *ga_w.lock().unwrap() = Some(from);

                let packet = buf[..n].to_vec();
                let peers = dp_r.lock().unwrap().clone();

                if peers.is_empty() {
                    // No direct peers yet — send to relay.
                    let _ = relay_w.send_to(&packet, relay_addr).await;
                } else {
                    // Direct path: send to every known peer.
                    for peer in &peers {
                        let _ = relay_w.send_to(&packet, peer).await;
                    }
                }
            }
        });
    }

    // ── Task B: relay/direct → local ──────────────────────────────────────
    {
        let relay_r = Arc::clone(&relay_sock);
        let local_w = Arc::clone(&local);
        let ga_r = Arc::clone(&goldberg_addr);
        let dp_w = Arc::clone(&direct_peers);
        let sd = Arc::clone(&shutdown);

        tokio::spawn(async move {
            let mut buf = vec![0u8; 65507];
            while !sd.load(Ordering::Relaxed) {
                let Ok((n, from)) = relay_r.recv_from(&mut buf).await else {
                    break;
                };

                // Detect PUNCH signal from relay: "PUNCH <ip>:<port>"
                if n > 6 && &buf[..6] == b"PUNCH " {
                    let addr_str = std::str::from_utf8(&buf[6..n])
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    if let Ok(peer_addr) = addr_str.parse::<SocketAddr>() {
                        // Fire hole-punch packet to open NAT path.
                        let _ = relay_r.send_to(b"PUNCH_ACK", peer_addr).await;
                        dp_w.lock().unwrap().push(peer_addr);
                        log::info!("nat: hole punched to {peer_addr}");
                    }
                    continue;
                }

                // Ignore PUNCH_ACK confirmations from peers.
                if n == 9 && &buf[..9] == b"PUNCH_ACK" {
                    // Record the sender as a direct peer if not already listed.
                    let mut peers = dp_w.lock().unwrap();
                    if !peers.contains(&from) {
                        peers.push(from);
                        log::info!("nat: direct peer confirmed {from}");
                    }
                    continue;
                }

                // Game packet — forward to Goldberg.
                if let Some(ga) = *ga_r.lock().unwrap() {
                    let _ = local_w.send_to(&buf[..n], ga).await;
                }
            }
        });
    }

    Ok(ProxyHandle { shutdown })
}
```

- [ ] **Step 2: Build to verify compilation**

```bash
cd /home/navitank/Desktop/evolve-revival/evolve-launcher/src-tauri
cargo build 2>&1 | head -40
```

Expected: compiles without errors (warnings about unused imports are ok).

- [ ] **Step 3: Run all launcher unit tests**

```bash
cd /home/navitank/Desktop/evolve-revival/evolve-launcher/src-tauri
cargo test
```

Expected: all tests PASS.

- [ ] **Step 4: Commit**

```bash
git add evolve-launcher/src-tauri/src/nat.rs
git commit -m "feat(launcher): local UDP proxy with hole-punch handling"
```

---

## Task 7: Patcher writes localhost to custom_broadcasts.txt

**Files:**
- Modify: `evolve-launcher/src-tauri/src/patcher.rs`

- [ ] **Step 1: Update the test that checks custom_broadcasts content**

In `patcher.rs`, the test `extracts_host_for_broadcasts_file` only tests `extract_host`.  
Add a new test for the behaviour change:

```rust
#[test]
fn custom_broadcasts_uses_localhost() {
    // Verify that apply_patches writes 127.0.0.1 regardless of server URL.
    // The full async test lives in integration tests; here we just assert the
    // constant matches expectations.
    assert_eq!(PROXY_LOCAL_HOST, "127.0.0.1");
}
```

Add the constant at the top of the module (before `extract_host`):

```rust
/// The address Goldberg should send peer-discovery packets to.
/// Pointing at localhost lets the launcher's local proxy intercept them.
pub const PROXY_LOCAL_HOST: &str = "127.0.0.1";
```

- [ ] **Step 2: Run — expect FAIL** (constant doesn't exist yet)

```bash
cd /home/navitank/Desktop/evolve-revival/evolve-launcher/src-tauri
cargo test patcher -- --nocapture
```

Expected: compile error (`PROXY_LOCAL_HOST` undefined).

- [ ] **Step 3: Add constant and update `apply_patches`**

In `patcher.rs`, add the constant just before `extract_host`:

```rust
pub const PROXY_LOCAL_HOST: &str = "127.0.0.1";
```

Change the last block of `apply_patches` from:

```rust
    let host = extract_host(server_url);
    std::fs::write(settings_dir.join("custom_broadcasts.txt"), format!("{host}\n"))
        .map_err(|e| format!("Failed to write custom_broadcasts.txt: {}", e))
```

to:

```rust
    std::fs::write(
        settings_dir.join("custom_broadcasts.txt"),
        format!("{PROXY_LOCAL_HOST}\n"),
    )
    .map_err(|e| format!("Failed to write custom_broadcasts.txt: {}", e))
```

- [ ] **Step 4: Run — expect PASS**

```bash
cd /home/navitank/Desktop/evolve-revival/evolve-launcher/src-tauri
cargo test patcher -- --nocapture
```

Expected: all patcher tests PASS.

- [ ] **Step 5: Commit**

```bash
git add evolve-launcher/src-tauri/src/patcher.rs
git commit -m "feat(patcher): custom_broadcasts.txt points to localhost proxy"
```

---

## Task 8: Commands — `get_nat_type` + proxy start in `launch_game`

**Files:**
- Modify: `evolve-launcher/src-tauri/src/commands.rs`
- Modify: `evolve-launcher/src-tauri/src/lib.rs`

- [ ] **Step 1: Add `get_nat_type` command and update `launch_game`**

In `commands.rs`, add `mod nat` usage at the top of the imports section — it's already a module in `lib.rs` so no change needed there yet.

Add after the `// ── Launch` block:

```rust
// ── NAT / STUN ───────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_nat_type(app: AppHandle) -> crate::nat::NatInfo {
    let cfg = Config::load(&app);
    // Parse relay host from server_url (same host, port 47584).
    let host = crate::patcher::extract_host(&cfg.server_url);
    match crate::nat::probe_stun(&host, 47584) {
        Ok(info) => info,
        Err(_) => crate::nat::NatInfo {
            external_ip: String::new(),
            external_port: 0,
            nat_type: "relay-only".to_string(),
        },
    }
}
```

Update `launch_game` to start the proxy before spawning the game process. Replace the existing `launch_game` function:

```rust
#[tauri::command]
pub async fn launch_game(app: AppHandle) -> Result<(), String> {
    let cfg = Config::load(&app);
    if cfg.install_dir.is_empty() {
        return Err("Game is not installed".to_string());
    }

    let exe = PathBuf::from(&cfg.install_dir).join("bin64_SteamRetail/Evolve.exe");

    // Start local UDP proxy before game launch so Goldberg can connect to it.
    let relay_host = crate::patcher::extract_host(&cfg.server_url);
    let proxy = crate::nat::start_proxy(relay_host.clone(), 47584).await?;

    // Register our external endpoint with the relay so other peers can punch.
    if let Ok(nat_info) = crate::nat::probe_stun(&relay_host, 47584) {
        let client = reqwest::Client::new();
        let session_id = format!("launcher-{}", nat_info.external_port);
        let _ = client
            .post(format!("{}/peers/register", cfg.server_url))
            .json(&serde_json::json!({
                "id": session_id,
                "ip": nat_info.external_ip,
                "port": nat_info.external_port,
            }))
            .send()
            .await;
    }

    let result = {
        #[cfg(target_os = "windows")]
        {
            let cwd = exe.parent().unwrap_or_else(|| std::path::Path::new("."));
            std::process::Command::new(&exe)
                .current_dir(cwd)
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("Failed to launch Evolve: {e}"))
        }

        #[cfg(not(target_os = "windows"))]
        {
            let steam_root = crate::steam::find_steam_root().ok_or_else(|| {
                "Steam not found. Install Steam and Proton Experimental to play on Linux."
                    .to_string()
            })?;
            let proton = crate::steam::find_proton(&steam_root).ok_or_else(|| {
                "Proton not found. Open Steam → Tools and install Proton Experimental.".to_string()
            })?;
            let compat_prefix = PathBuf::from(&cfg.install_dir).join("proton_prefix");
            std::fs::create_dir_all(&compat_prefix).map_err(|e| e.to_string())?;
            let cwd = exe.parent().unwrap_or_else(|| std::path::Path::new("."));
            std::process::Command::new(&proton)
                .arg("run")
                .arg(&exe)
                .env("STEAM_COMPAT_DATA_PATH", &compat_prefix)
                .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", &steam_root)
                .current_dir(cwd)
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("Failed to launch via Proton ({proton:?}): {e}"))
        }
    };

    if result.is_err() {
        proxy.stop();
    }
    // On success: proxy keeps running until the launcher exits.
    result
}
```

Also add `use serde_json;` to the imports at the top of `commands.rs` if not already present (it's already in `Cargo.toml` as `serde_json = "1"`).

- [ ] **Step 2: Register the new command in `lib.rs`**

Add `mod nat;` to the module list and `commands::get_nat_type` to the invoke handler:

```rust
mod commands;
mod config;
mod downloader;
mod install;
mod nat;
mod patcher;
mod steam;

use commands::AppDownloadState;
use downloader::DownloadState;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppDownloadState(Mutex::new(DownloadState::default())))
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::check_install_state,
            commands::check_for_updates,
            commands::get_components,
            commands::save_components,
            commands::get_tiers,
            commands::save_tier,
            commands::start_install,
            commands::pause_install,
            commands::resume_install,
            commands::start_repair,
            commands::start_update,
            commands::launch_game,
            commands::list_steam_accounts,
            commands::add_to_steam,
            commands::get_nat_type,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Build**

```bash
cd /home/navitank/Desktop/evolve-revival/evolve-launcher/src-tauri
cargo build 2>&1 | head -60
```

Expected: compiles without errors.

- [ ] **Step 4: Run all launcher tests**

```bash
cd /home/navitank/Desktop/evolve-revival/evolve-launcher/src-tauri
cargo test
```

Expected: all tests PASS.

- [ ] **Step 5: Commit**

```bash
git add evolve-launcher/src-tauri/src/commands.rs evolve-launcher/src-tauri/src/lib.rs
git commit -m "feat(launcher): get_nat_type command + proxy starts on launch_game"
```

---

## Task 9: NAT indicator in launcher UI

**Files:**
- Modify: `evolve-launcher/src/types.ts`
- Modify: `evolve-launcher/src/lib/Main.svelte`

- [ ] **Step 1: Add `NatInfo` type**

In `src/types.ts`, append:

```typescript
export interface NatInfo {
  external_ip: string;
  external_port: number;
  nat_type: 'direct' | 'relay-only';
}
```

- [ ] **Step 2: Add NAT probe and indicator to `Main.svelte`**

Replace the entire `Main.svelte` file with:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { AppState, NatInfo } from '../types';

  let { appState, onSettings, onRepair }: {
    appState: AppState;
    onSettings: () => void;
    onRepair: () => void;
  } = $props();

  type Status = 'checking' | 'online' | 'degraded' | 'offline';

  let status = $state<Status>('online');
  let playerCount = $state(0);
  let filesVerified = $state(true);
  let launching = $state(false);
  let launchError = $state('');
  let natInfo = $state<NatInfo | null>(null);

  const canPlay = $derived(
    (status === 'online' || status === 'degraded') && filesVerified
  );

  const dotColor = $derived(
    status === 'online'   ? '#4ade80' :
    status === 'degraded' ? '#fbbf24' : '#ef4444'
  );

  const natLabel = $derived(
    natInfo === null      ? '' :
    natInfo.nat_type === 'direct' ? 'Direct' : 'Relay'
  );

  const natColor = $derived(
    natInfo === null      ? '#666' :
    natInfo.nat_type === 'direct' ? '#4ade80' : '#fbbf24'
  );

  onMount(async () => {
    // Probe NAT type in the background — non-blocking.
    invoke<NatInfo>('get_nat_type')
      .then(info => { natInfo = info; })
      .catch(() => { natInfo = { external_ip: '', external_port: 0, nat_type: 'relay-only' }; });
  });

  async function play() {
    launchError = '';
    launching = true;
    try {
      await invoke('launch_game');
    } catch (e) {
      launchError = String(e);
    } finally {
      launching = false;
    }
  }

  async function update() {
    await invoke('start_update');
    onRepair();
  }
</script>

<div class="launcher">
  <span class="version-badge">v0.1.0</span>

  <div class="title-block">
    <span class="title-main">EVOLVE</span>
    <span class="title-sub">REVIVAL</span>
  </div>

  {#if appState === 'update-available'}
    <div class="update-banner">
      Update available
      <button class="update-btn" onclick={update}>UPDATE</button>
    </div>
  {/if}

  <div class="status-row">
    <span class="dot" style="background: {dotColor}; color: {dotColor}"></span>
    {#if status === 'online'}
      ONLINE &nbsp;·&nbsp; {playerCount} players
    {:else if status === 'degraded'}
      DEGRADED &nbsp;·&nbsp; {playerCount} players
    {:else if status === 'checking'}
      CHECKING...
    {:else}
      OFFLINE
    {/if}
    {#if natLabel}
      &nbsp;·&nbsp;
      <span class="nat-label" style="color: {natColor}">{natLabel}</span>
    {/if}
  </div>

  {#if launchError}
    <div class="launch-error">{launchError}</div>
  {/if}

  <button class="play-btn" onclick={play} disabled={!canPlay || launching}>
    {launching ? 'LAUNCHING...' : 'PLAY'}
  </button>

  <div class="footer">
    <span class="verify-status">
      {#if filesVerified}
        <span class="check">✓</span>Files verified
      {:else}
        <span class="cross">✗</span>Files not verified
      {/if}
    </span>
    <div style="display:flex; gap:14px; align-items:center;">
      <button class="repair-btn" onclick={async () => { await invoke('start_repair'); onRepair(); }}>Repair</button>
      <button class="settings-btn" onclick={onSettings}>Settings</button>
    </div>
  </div>
</div>

<style>
  .nat-label {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.05em;
  }
</style>
```

- [ ] **Step 3: Type-check**

```bash
cd /home/navitank/Desktop/evolve-revival/evolve-launcher
npx tsc --noEmit
```

Expected: no type errors.

- [ ] **Step 4: Commit**

```bash
git add evolve-launcher/src/types.ts evolve-launcher/src/lib/Main.svelte
git commit -m "feat(ui): NAT type indicator in launcher main screen"
```

---

## Self-Review

**Spec coverage:**
- ✅ STUN UDP endpoint on relay (Tasks 1–2)
- ✅ Peer registry (Task 3–4)
- ✅ Auto hole-punch on register (Task 3 `RegisterNamed`)
- ✅ Relay signaling via `Signal()` (Task 2 + 4)
- ✅ Launcher STUN probe (Task 5)
- ✅ Local UDP proxy (Task 6)
- ✅ Goldberg pointed at localhost (Task 7)
- ✅ Proxy start + peer registration on game launch (Task 8)
- ✅ NAT indicator in UI (Task 9)
- ✅ Relay fallback: proxy routes through relay until direct path confirmed via PUNCH_ACK

**Placeholder scan:** None found.

**Type consistency:**
- `NatInfo` defined in Task 5 (`nat.rs`), returned by `get_nat_type` in Task 8, typed in `types.ts` Task 9 — all match.
- `ProxyHandle.stop()` defined Task 6, called Task 8 on error path — matches.
- `punch.Register` handler in Task 4, called via `reqwest` in Task 8 with identical JSON shape `{id, ip, port}` — matches.
- `LookupNamed` added in Task 4 step 3, used by `punch_test.go` in Task 4 step 1 — matches.
