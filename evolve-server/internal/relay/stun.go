package relay

import (
	"encoding/binary"
	"net"
)

const (
	stunMagicCookie   = uint32(0x2112A442)
	stunMagicPortXOR  = uint16(0x2112)
)

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
	msg[24] = 0x00 // reserved
	msg[25] = 0x01 // family: IPv4
	port := uint16(from.Port) ^ stunMagicPortXOR
	binary.BigEndian.PutUint16(msg[26:28], port)
	ip4 := from.IP.To4()
	if ip4 == nil {
		return nil
	}
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
			port := binary.BigEndian.Uint16(val[2:4]) ^ stunMagicPortXOR
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
