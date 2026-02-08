//! WebSocket compression configuration
//!
//! this module provides configuration for WebSocket compression (per-message deflate).

/// Configuration for WebSocket compression
#[derive(Debug, Clone, Copy)]
pub struct WsCompressionConfig {
    /// Minimum size of message to compress (in bytes)
    pub min_size: usize,
    /// Server window bits (9-15)
    pub window_bits: u8,
    /// Client window bits (9-15)
    pub client_window_bits: u8,
}

impl Default for WsCompressionConfig {
    fn default() -> Self {
        Self {
            min_size: 256,
            window_bits: 15,
            client_window_bits: 15,
        }
    }
}

impl WsCompressionConfig {
    /// Create a new compression config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set minimum message size to compress
    pub fn min_size(mut self, size: usize) -> Self {
        self.min_size = size;
        self
    }

    /// Set server window bits (9-15)
    pub fn window_bits(mut self, bits: u8) -> Self {
        self.window_bits = bits.clamp(9, 15);
        self
    }

    /// Set client window bits (9-15)
    pub fn client_window_bits(mut self, bits: u8) -> Self {
        self.client_window_bits = bits.clamp(9, 15);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WebSocketUpgrade;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_compression_negotiation(
            min_size in 0usize..10000,
            window_bits in 9u8..15,
            client_window_bits in 9u8..15,
            client_supports_compression in proptest::bool::ANY,
        ) {
            let config = WsCompressionConfig::new()
                .min_size(min_size)
                .window_bits(window_bits)
                .client_window_bits(client_window_bits);

            let client_extensions = if client_supports_compression {
                Some(
                    "permessage-deflate; server_max_window_bits=15; client_max_window_bits"
                        .to_string(),
                )
            } else {
                None
            };

            let upgrade = WebSocketUpgrade::new(
                "dGhlIHNhbXBsZSBub25jZQ==".to_string(),
                client_extensions,
                None, // No OnUpgrade future in tests
            )
            .compress(config);

            let response = upgrade.into_response_inner();
            let ext_header = response.headers().get("Sec-WebSocket-Extensions");

            if client_supports_compression {
                assert!(ext_header.is_some(), "Header missing when client supports compression");
                let header_str = ext_header.unwrap().to_str().unwrap();
                assert!(header_str.contains("permessage-deflate"));

                if window_bits < 15 {
                    assert!(header_str.contains(&format!("server_max_window_bits={}", window_bits)));
                }
                if client_window_bits < 15 {
                    assert!(header_str.contains(&format!("client_max_window_bits={}", client_window_bits)));
                }
            } else {
                assert!(ext_header.is_none());
            }
        }
    }
}
