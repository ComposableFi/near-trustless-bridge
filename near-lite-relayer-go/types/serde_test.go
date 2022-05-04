package types

import (
	"log"
	"testing"

	"github.com/near/borsh-go"
	"github.com/stretchr/testify/assert"
)

// This module check that the structs are constructed in the correct way
// so that the serialization and deserialization matches to what borsh
// outputs in the near-primitives counterparty (which is implemented in Rust)

func TestDirection(t *testing.T) {
	// check that borsh enums work as I expect :)
	l := Left
	r := Right
	data, err := borsh.Serialize(l)
	if err != nil {
		log.Fatal(err)
	}
	assert.Equal(t, []byte{0}, data)
	data, err = borsh.Serialize(r)
	if err != nil {
		log.Fatal(err)
	}
	assert.Equal(t, []byte{1}, data)
}
