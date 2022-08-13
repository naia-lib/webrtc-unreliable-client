use super::header::*;

use std::collections::HashMap;

// A Builder allows incrementally packing a DNS message.
//
// Example usage:
//	b := NewBuilder(Header{...})
//	b.enable_compression()
//	// Optionally start a section and add things to that section.
//	// Repeat adding sections as necessary.
//	buf, err := b.Finish()
//	// If err is nil, buf[2:] will contain the built bytes.
#[derive(Default)]
pub struct Builder {
    // msg is the storage for the message being built.
    pub msg: Option<Vec<u8>>,

    // section keeps track of the current section being built.
    pub section: Section,

    // header keeps track of what should go in the header when Finish is
    // called.
    pub header: HeaderInternal,

    // start is the starting index of the bytes allocated in msg for header.
    pub start: usize,

    // compression is a mapping from name suffixes to their starting index
    // in msg.
    pub compression: Option<HashMap<String, usize>>,
}