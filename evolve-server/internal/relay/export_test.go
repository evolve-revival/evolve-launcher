package relay

// FakeBindingRequest returns a minimal valid STUN Binding Request for tests.
func FakeBindingRequest() []byte {
	msg := make([]byte, 20)
	msg[0] = 0x00
	msg[1] = 0x01
	copy(msg[4:8], stunMagicBytes[:])
	copy(msg[8:20], []byte("txid12345678"))
	return msg
}
