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
