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
}

func New() *Relay {
	return &Relay{peers: make(map[string]*peerEntry)}
}

// Run listens on addr (e.g. ":47584") and relays every incoming packet to all
// other currently registered peers. Blocks until the connection is closed.
func (r *Relay) Run(listenAddr string) error {
	conn, err := net.ListenPacket("udp", listenAddr)
	if err != nil {
		return err
	}
	defer conn.Close()
	log.Printf("relay: UDP relay listening on %s", listenAddr)

	go r.pruneLoop()

	buf := make([]byte, maxPacket)
	for {
		n, from, err := conn.ReadFrom(buf)
		if err != nil {
			return err
		}

		fromUDP := from.(*net.UDPAddr)
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

		// Copy packet before goroutine hand-off to avoid buf race.
		packet := make([]byte, n)
		copy(packet, buf[:n])

		for _, t := range targets {
			if _, werr := conn.WriteTo(packet, t); werr != nil {
				log.Printf("relay: write to %s: %v", t, werr)
			}
		}
	}
}

// PeerCount returns the number of currently active peers.
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
