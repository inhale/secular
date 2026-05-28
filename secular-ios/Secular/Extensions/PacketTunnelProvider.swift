// iOS NEPacketTunnelProvider implementation
import NetworkExtension
import os.log

class PacketTunnelProvider: NEPacketTunnelProvider {
    
    private let logger = Logger(subsystem: "com.secular.vpn", category: "tunnel")
    private var engine: OpaquePointer?  // secular-core FFI handle
    
    // MARK: - Tunnel Lifecycle
    
    func startTunnel(options: [String : NSObject]?, completionHandler: @escaping (Error?) -> Void) {
        logger.info("Starting Secular tunnel...")
        
        guard let config = options as? [String: Any],
              let host = config["host"] as? String,
              let port = config["port"] as? Int,
              let sni = config["sni"] as? String,
              let authToken = config["auth_token"] as? String else {
            completionHandler(NSError(domain: "Secular", code: 1, userInfo: [
                NSLocalizedDescriptionKey: "Missing configuration"
            ]))
            return
        }
        
        // Build tunnel settings
        let settings = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: host)
        
        // IPv4: route all traffic
        let ipv4 = NEIPv4Settings(addresses: ["10.0.0.2"], subnetMasks: ["255.255.255.255"])
        ipv4.includedRoutes = [NEIPv4Route.default()]
        settings.ipv4Settings = ipv4
        
        // DNS through tunnel
        let dns = NEDNSSettings(servers: ["9.9.9.9"])
        settings.dnsSettings = dns
        
        // MTU
        settings.tunnelOverheadBytes = 40
        settings.mtu = NSNumber(value: 1380)
        
        setTunnelNetworkSettings(settings) { [weak self] error in
            if let error = error {
                self?.logger.error("Failed to set tunnel settings: \(error.localizedDescription)")
                completionHandler(error)
                return
            }
            
            // TODO: Initialize secular-core via FFI
            // let configJson = buildConfigJson(host: host, port: port, sni: sni, token: authToken)
            // self?.engine = configJson.withUnsafeBytes { ptr in
            //     secular_create(ptr.bindMemory(to: UInt8.self).baseAddress, configJson.count)
            // }
            
            self?.logger.info("Tunnel settings applied, starting packet reading...")
            self?.startReadingPackets()
            completionHandler(nil)
        }
    }
    
    func stopTunnel(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        logger.info("Stopping tunnel: \(reason.rawValue)")
        
        // TODO: Cleanup secular-core
        // if let engine = engine {
        //     secular_destroy(engine)
        //     self.engine = nil
        // }
        
        completionHandler()
    }
    
    // MARK: - Packet Handling
    
    private func startReadingPackets() {
        // Read packets from the tunnel (memory-conscious for 15MB iOS extension limit)
        packet Flow.readPackets { [weak self] packets, protocols in
            guard let self = self else { return }
            
            for (index, packet) in packets.enumerated() {
                let protocolFamily = protocols[index]
                
                // TODO: Pass to secular-core for processing
                // if let engine = self.engine {
                //     let processed = self.processPacket(packet, engine: engine)
                //     self.packetFlow.writePackets([processed], withProtocols: [protocolFamily])
                // }
                
                // For now, echo back (placeholder)
                self.packetFlow.writePackets([packet], withProtocols: [protocolFamily])
            }
            
            // Continue reading (prevents memory buildup by not retaining packets)
            self.startReadingPackets()
        }
    }
    
    // Placeholder for FFI packet processing
    private func processPacket(_ packet: Data, engine: OpaquePointer) -> Data {
        // TODO: Call secular-core FFI to process packet
        return packet
    }
}
