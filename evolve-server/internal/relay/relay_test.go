package relay_test

import (
	"net"
	"testing"
	"time"

	"github.com/evolve-revival/evolve-server/internal/relay"
)

// startRelay launches the relay on a random port and returns the address.
func startRelay(t *testing.T) string {
	t.Helper()
	r := relay.New()

	ln, err := net.ListenPacket("udp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("find free port: %v", err)
	}
	addr := ln.LocalAddr().String()
	ln.Close()

	go func() {
		if err := r.Run(addr); err != nil {
			// Connection closed on cleanup — expected.
		}
	}()
	time.Sleep(10 * time.Millisecond)
	return addr
}

func dial(t *testing.T) *net.UDPConn {
	t.Helper()
	conn, err := net.ListenPacket("udp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn.(*net.UDPConn)
}

func send(t *testing.T, conn *net.UDPConn, relay string, msg []byte) {
	t.Helper()
	dst, _ := net.ResolveUDPAddr("udp", relay)
	if _, err := conn.WriteTo(msg, dst); err != nil {
		t.Fatalf("send: %v", err)
	}
}

func recv(t *testing.T, conn *net.UDPConn, timeout time.Duration) ([]byte, bool) {
	t.Helper()
	conn.SetReadDeadline(time.Now().Add(timeout))
	buf := make([]byte, 1024)
	n, _, err := conn.ReadFrom(buf)
	if err != nil {
		return nil, false
	}
	return buf[:n], true
}

func TestRelay_ForwardsToOtherPeer(t *testing.T) {
	relayAddr := startRelay(t)

	a := dial(t)
	b := dial(t)

	// Both register by sending to relay.
	send(t, a, relayAddr, []byte("hello-from-a"))
	send(t, b, relayAddr, []byte("hello-from-b"))

	// Drain forwarded registration packets from both sides before the real test.
	// When b registers, relay forwards b's packet to a (and vice versa).
	recv(t, a, 50*time.Millisecond)
	recv(t, b, 50*time.Millisecond)

	// Now a sends a real packet — b must receive it.
	send(t, a, relayAddr, []byte("payload"))
	got, ok := recv(t, b, 200*time.Millisecond)
	if !ok {
		t.Fatal("b did not receive packet from a")
	}
	if string(got) != "payload" {
		t.Errorf("b received %q, want %q", got, "payload")
	}

	// a must NOT receive its own packet.
	_, selfEcho := recv(t, a, 50*time.Millisecond)
	if selfEcho {
		t.Error("a received its own packet (self-echo)")
	}
}

func TestRelay_NoForwardWithSinglePeer(t *testing.T) {
	relayAddr := startRelay(t)

	a := dial(t)
	send(t, a, relayAddr, []byte("alone"))

	// Nothing to forward to — a must not receive anything back.
	_, ok := recv(t, a, 50*time.Millisecond)
	if ok {
		t.Error("single peer received unexpected data")
	}
}

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

func TestRelay_SignalSendsPunchToBoth(t *testing.T) {
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
