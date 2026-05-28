// Secular Desktop — Main App Component
import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

interface ConnectionState {
  connected: boolean;
  server: string;
  protocol: string;
  bytes_sent: number;
  bytes_received: number;
}

const App: React.FC = () => {
  const [state, setState] = useState<ConnectionState>({
    connected: false,
    server: '',
    protocol: 'h2',
    bytes_sent: 0,
    bytes_received: 0,
  });

  const [server, setServer] = useState('');
  const [port, setPort] = useState('443');
  const [sni, setSni] = useState('');
  const [token, setToken] = useState('');
  const [protocol, setProtocol] = useState('h2');
  const [allowIpv6, setAllowIpv6] = useState(false);

  const handleConnect = async () => {
    try {
      if (state.connected) {
        await invoke('disconnect');
        setState(prev => ({ ...prev, connected: false, server: '' }));
      } else {
        const result = await invoke<ConnectionState>('connect', {
          config: {
            host: server,
            port: parseInt(port, 10) || 443,
            sni: sni || server,
            auth_token: token,
            protocol,
            allow_ipv6: allowIpv6,
          },
        });
        setState(result);
      }
    } catch (err) {
      console.error('Connection error:', err);
    }
  };

  const formatBytes = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1048576).toFixed(1)} MB`;
  };

  return (
    <div>
      {/* Logo */}
      <div className="logo-container">
        <svg className="logo" viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg">
          <rect width="512" height="512" rx="96" fill="#0D0F12"/>
          <path d="M 120 100 C 200 100, 260 160, 260 200 C 260 240, 200 280, 160 280 C 120 280, 80 300, 80 340 C 80 380, 140 420, 220 420"
                stroke="#00F5D4" strokeWidth="32" fill="none" strokeLinecap="round"/>
          <path d="M 392 100 C 312 100, 252 160, 252 200 C 252 240, 312 280, 352 280 C 392 280, 432 300, 432 340 C 432 380, 372 420, 292 420"
                stroke="#00F5D4" strokeWidth="32" fill="none" strokeLinecap="round"/>
          <circle cx="120" cy="100" r="24" fill="#00F5D4"/>
          <circle cx="292" cy="420" r="24" fill="#00F5D4"/>
        </svg>
      </div>

      {/* Status */}
      <div className="status-section">
        <div className="status-label">Status</div>
        <div className={`status-value ${state.connected ? 'connected' : 'disconnected'}`}>
          {state.connected ? 'Connected' : 'Disconnected'}
        </div>
      </div>

      {/* Connect Button */}
      <div className="connect-section">
        <button
          className={`connect-btn ${state.connected ? 'connected' : ''}`}
          onClick={handleConnect}
        >
          {state.connected ? 'Disconnect' : 'Connect'}
        </button>
      </div>

      {/* Server Info */}
      {state.connected && (
        <div className="server-section">
          <div className="server-card">
            <div className="server-row">
              <span className="server-row-label">Server</span>
              <span className="server-row-value">{state.server}</span>
            </div>
            <div className="server-row">
              <span className="server-row-label">Protocol</span>
              <span className="server-row-value">{state.protocol.toUpperCase()}</span>
            </div>
          </div>
        </div>
      )}

      {/* Stats */}
      {state.connected && (
        <div className="stats-section">
          <div className="stat-card">
            <div className="stat-value">{formatBytes(state.bytes_sent)}</div>
            <div className="stat-label">Sent</div>
          </div>
          <div className="stat-card">
            <div className="stat-value">{formatBytes(state.bytes_received)}</div>
            <div className="stat-label">Received</div>
          </div>
        </div>
      )}

      {/* Settings */}
      <div className="settings-section">
        <div className="settings-row">
          <span className="settings-label">Server</span>
          <input
            className="settings-input"
            value={server}
            onChange={e => setServer(e.target.value)}
            placeholder="vpn.example.com"
          />
        </div>
        <div className="settings-row">
          <span className="settings-label">Port</span>
          <input
            className="settings-input"
            value={port}
            onChange={e => setPort(e.target.value)}
            placeholder="443"
          />
        </div>
        <div className="settings-row">
          <span className="settings-label">Protocol</span>
          <select
            className="settings-input"
            value={protocol}
            onChange={e => setProtocol(e.target.value)}
          >
            <option value="h2">HTTP/2</option>
            <option value="quic">QUIC</option>
          </select>
        </div>
        <div className="settings-row">
          <span className="settings-label">Allow IPv6</span>
          <div
            className={`toggle ${allowIpv6 ? 'active' : ''}`}
            onClick={() => setAllowIpv6(!allowIpv6)}
          />
        </div>
      </div>

      {/* Footer */}
      <div className="footer">Secular v0.1.0</div>
    </div>
  );
};

export default App;
