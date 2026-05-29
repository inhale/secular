// secular-ios/Secular/ContentView.swift
// Secular iOS — Main UI (light theme)

import SwiftUI
import NetworkExtension

struct ContentView: View {
    @StateObject private var vpnManager = VPNManager.shared

    var body: some View {
        VStack(spacing: 24) {
            Spacer()

            // Logo: §
            Text("§")
                .font(.system(size: 72, weight: .bold, design: .default))
                .foregroundColor(Color(hex: "242424"))

            Spacer()

            // Status
            VStack(spacing: 4) {
                Text("Status")
                    .font(.caption)
                    .foregroundColor(Color(hex: "7A869A"))

                Text(vpnManager.statusText)
                    .font(.headline)
                    .foregroundColor(vpnManager.isConnected ? Color(hex: "00F5D4") : Color(hex: "7A869A"))
            }

            // Connect Button (pill shape)
            Button(action: {
                vpnManager.toggleConnection()
            }) {
                Text(vpnManager.buttonText)
                    .font(.system(size: 16, weight: .semibold))
                    .foregroundColor(.white)
                    .frame(width: 200, height: 48)
                    .background(vpnManager.buttonColor)
                    .cornerRadius(24)
            }
            .disabled(vpnManager.isConnecting)

            // Server Info
            if vpnManager.isConnected {
                VStack(spacing: 8) {
                    HStack {
                        Text("Server")
                            .foregroundColor(Color(hex: "7A869A"))
                        Spacer()
                        Text("vpn.example.com:443")
                            .foregroundColor(Color(hex: "242424"))
                    }
                    HStack {
                        Text("Protocol")
                            .foregroundColor(Color(hex: "7A869A"))
                        Spacer()
                        Text("HTTP/2")
                            .foregroundColor(Color(hex: "242424"))
                    }
                }
                .padding(.horizontal, 40)
                .font(.subheadline)
            }

            Spacer()

            // Footer tabs
            HStack(spacing: 40) {
                Text("Home")
                    .foregroundColor(Color(hex: "d02b57"))
                    .fontWeight(.semibold)
                Text("History")
                    .foregroundColor(Color(hex: "7A869A"))
                Text("Settings")
                    .foregroundColor(Color(hex: "7A869A"))
            }
            .font(.subheadline)

            Text("Secular v0.1.0")
                .font(.caption2)
                .foregroundColor(Color(hex: "7A869A"))
                .padding(.bottom, 16)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color(hex: "F5F7FA"))
    }
}

// MARK: - Color Helper

extension Color {
    init(hex: String) {
        let hex = hex.trimmingCharacters(in: CharacterSet.alphanumerics.inverted)
        var int: UInt64 = 0
        Scanner(string: hex).scanHexInt64(&int)
        let a, r, g, b: UInt64
        switch hex.count {
        case 3: // RGB (12-bit)
            (a, r, g, b) = (255, (int >> 8) * 17, (int >> 4 & 0xF) * 17, (int & 0xF) * 17)
        case 6: // RGB (24-bit)
            (a, r, g, b) = (255, int >> 16, int >> 8 & 0xFF, int & 0xFF)
        case 8: // ARGB (32-bit)
            (a, r, g, b) = (int >> 24, int >> 16 & 0xFF, int >> 8 & 0xFF, int & 0xFF)
        default:
            (a, r, g, b) = (1, 1, 1, 0)
        }
        self.init(
            .sRGB,
            red: Double(r) / 255,
            green: Double(g) / 255,
            blue: Double(b) / 255,
            opacity: Double(a) / 255
        )
    }
}

// MARK: - VPN Manager

class VPNManager: ObservableObject {
    static let shared = VPNManager()

    @Published var isConnected = false
    @Published var isConnecting = false

    var statusText: String {
        if isConnecting { return "Connecting" }
        return isConnected ? "Connected" : "Disconnected"
    }

    var buttonText: String {
        if isConnecting { return "Connecting..." }
        return isConnected ? "Disconnect" : "Connect"
    }

    var buttonColor: Color {
        if isConnected { return Color(hex: "FF3B30") }
        return Color(hex: "d02b57")
    }

    private let manager = NETunnelProviderManager()

    private init() {
        loadVPNConfiguration()
    }

    private func loadVPNConfiguration() {
        NETunnelProviderManager.loadAllFromPreferences { [weak self] managers, error in
            if let error = error {
                print("Failed to load VPN configs: \(error)")
                return
            }
            if let managers = managers, !managers.isEmpty {
                self?.manager = managers.first!
            }
        }
    }

    func toggleConnection() {
        if isConnected {
            disconnect()
        } else {
            connect()
        }
    }

    private func connect() {
        isConnecting = true

        // Configure the tunnel
        let proto = NETunnelProviderProtocol()
        proto.providerBundleIdentifier = "com.secular.vpn.extension"
        proto.serverAddress = "vpn.example.com"

        // Pass configuration to the extension
        proto.providerConfiguration = [
            "host": "vpn.example.com",
            "port": 443,
            "sni": "vpn.example.com",
            "auth_token": "YOUR_AUTH_TOKEN",
            "protocol": "h2"
        ]

        manager.protocolConfiguration = proto
        manager.localizedDescription = "Secular VPN"
        manager.isEnabled = true

        manager.saveToPreferences { [weak self] error in
            DispatchQueue.main.async {
                if let error = error {
                    print("Failed to save VPN config: \(error)")
                    self?.isConnecting = false
                    return
                }

                // Start the tunnel
                do {
                    try (self?.manager.connection as? NETunnelProviderSession)?.startTunnel()
                    self?.isConnected = true
                    print("VPN tunnel started successfully")
                } catch {
                    print("Failed to start tunnel: \(error)")
                }
                self?.isConnecting = false
            }
        }
    }

    private func disconnect() {
        (manager.connection as? NETunnelProviderSession)?.stopTunnel()
        isConnected = false
        print("VPN tunnel stopped")
    }
}
