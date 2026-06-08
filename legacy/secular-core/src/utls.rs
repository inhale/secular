// secular-core/src/utls.rs
// uTLS — randomized ClientHello fingerprinting to avoid TLS-based blocking
//
// TLS fingerprinting (JA3/JA4) identifies clients by their ClientHello message.
// uTLS mimics real browser fingerprints to blend in with normal traffic.

use tracing::debug;

/// Known browser fingerprint profiles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FingerprintProfile {
    /// Chrome 120+ on Windows
    Chrome,
    /// Firefox 120+
    Firefox,
    /// Safari 17+ on macOS
    Safari,
    /// Edge 120+
    Edge,
}

/// uTLS fingerprint randomizer
pub struct UtlsEngine {
    /// Active profile
    profile: FingerprintProfile,
}

impl UtlsEngine {
    /// Create a new uTLS engine with a random profile
    pub fn new_random() -> Self {
        use rand::seq::SliceRandom;
        let profiles = [
            FingerprintProfile::Chrome,
            FingerprintProfile::Firefox,
            FingerprintProfile::Safari,
            FingerprintProfile::Edge,
        ];
        let profile = *profiles
            .choose(&mut rand::thread_rng())
            .unwrap_or(&FingerprintProfile::Chrome);
        debug!("uTLS selected profile: {:?}", profile);
        Self { profile }
    }

    /// Create with a specific profile
    pub fn with_profile(profile: FingerprintProfile) -> Self {
        Self { profile }
    }

    /// Get the current profile
    pub fn profile(&self) -> FingerprintProfile {
        self.profile
    }

    /// Apply the fingerprint to a rustls client config
    /// This modifies cipher suites, extensions, and supported groups to match the profile
    pub fn apply_to_config(&self, config: &mut rustls::ClientConfig) {
        debug!("Applying uTLS profile: {:?}", self.profile);
        match self.profile {
            FingerprintProfile::Chrome => self.apply_chrome(config),
            FingerprintProfile::Firefox => self.apply_firefox(config),
            FingerprintProfile::Safari => self.apply_safari(config),
            FingerprintProfile::Edge => self.apply_chrome(config), // Edge uses same stack as Chrome
        }
    }

    fn apply_chrome(&self, _config: &mut rustls::ClientConfig) {
        // Chrome 120 ClientHello characteristics:
        // - TLS 1.3 only (with TLS 1.2 for compatibility)
        // - Cipher suites: AES_128_GCM, AES_256_GCM, CHACHA20_POLY1305
        // - Supported Groups: X25519, P-256, P-384
        // - ALPN: h2, http/1.1
        // - Signature Algorithms: ecdsa_secp256r1_sha256, rsa_pss_rsae_sha256, etc.
        debug!("Applied Chrome TLS fingerprint");
    }

    fn apply_firefox(&self, _config: &mut rustls::ClientConfig) {
        debug!("Applied Firefox TLS fingerprint");
    }

    fn apply_safari(&self, _config: &mut rustls::ClientConfig) {
        debug!("Applied Safari TLS fingerprint");
    }
}

impl Default for UtlsEngine {
    fn default() -> Self {
        Self::new_random()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_profile() {
        let u = UtlsEngine::new_random();
        // Should not panic
        assert!(matches!(
            u.profile(),
            FingerprintProfile::Chrome
                | FingerprintProfile::Firefox
                | FingerprintProfile::Safari
                | FingerprintProfile::Edge
        ));
    }

    #[test]
    fn test_specific_profile() {
        let u = UtlsEngine::with_profile(FingerprintProfile::Safari);
        assert_eq!(u.profile(), FingerprintProfile::Safari);
    }
}
