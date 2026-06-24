// Package relay implements a UDP relay for Goldberg Steam emulator peer discovery.
// Goldberg sends lobby announcements to addresses listed in custom_broadcasts.txt.
// This relay receives those packets and forwards them to all other registered peers,
// enabling internet multiplayer without LAN adjacency.
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
			if resp != nil {
				conn.WriteTo(resp, fromUDP)
			}
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

// LookupNamed returns the stored address for id, or nil if not registered.
func (r *Relay) LookupNamed(id string) *net.UDPAddr {
	r.mu.Lock()
	defer r.mu.Unlock()
	return r.named[id]
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
