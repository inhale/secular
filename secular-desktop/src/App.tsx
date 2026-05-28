// Secular Desktop — Main App Component (from design system)
import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

interface ConnectionConfig {
  host: string;
  port: number;
  sni: string;
  auth_token: string;
  protocol: string;
  allow_ipv6: boolean;
}

type ConnState = 'disconnected' | 'connecting' | 'connected';

const ConnStates: Record<ConnState, ConnState> = {
  disconnected: 'disconnected',
  connecting: 'connecting',
  connected: 'connected',
};

const App: React.FC = () => {
  const [connState, setConnState] = useState<ConnState>(ConnStates.disconnected);
  const [server, setServer] = useState('');
  const [port, setPort] = useState('443');
  const [sni, setSni] = useState('');
  const [token, setToken] = useState('');
  const [protocol, setProtocol] = useState('h2');
  const [allowIpv6, setAllowIpv6] = useState(false);

  const handleConnect = async () => {
    try {
      if (connState === ConnStates.connected) {
        await invoke('disconnect');
        setConnState(ConnStates.disconnected);
      } else if (connState === ConnStates.disconnected) {
        setConnState(ConnStates.connecting);
        await invoke('connect', {
          config: {
            host: server,
            port: parseInt(port, 10) || 443,
            sni: sni || server,
            auth_token: token,
            protocol,
            allow_ipv6: allowIpv6,
          },
        });
        setConnState(ConnStates.connected);
      }
    } catch (err) {
      console.error('Connection error:', err);
      setConnState(ConnStates.disconnected);
    }
  };

  const btnLabel = connState === ConnStates.connected ? 'Disconnect'
    : connState === ConnStates.connecting ? 'Connecting...'
    : 'Connect';

  const btnClass = connState === ConnStates.connected ? 'connect-btn disconnected'
    : connState === ConnStates.connecting ? 'connect-btn connecting'
    : 'connect-btn';

  return (
    <div>
      {/* Logo */}
      <div className="logo-container">
        <svg className="logo" viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg">
          <rect width="512" height="512" rx="96" fill="#F5F7FA"/>
          <path d="M 80 80 C 160 80, 280 140, 320 220 C 360 300, 340 380, 260 432"
                stroke="#242424" stroke-width="32" fill="none" strokeLinecap="round"/>
          <path d="M 432 80 C 352 80, 232 140, 192 220 C 152 300, 172 380, 252 432"
                stroke="#242424" stroke-width="32" fill="none" strokeLinecap="round"/>
          <circle cx="80" cy="80" r="16" fill="#d02b57"/>
          <circle cx="432" cy="432" r="16" fill="#d02b57"/>
        </svg>
      </div>

      {/* Status */}
      <div className="status-section">
        <div className="status-label">Status</div>
        <div className={`status-value ${connState}`}>
          {connState === ConnStates.connected ? 'Connected'
            : connState === ConnStates.connecting ? 'Connecting'
            : 'Disconnected'}
        </div>
      </div>

      {/* Connect Button — pill shape */}
      <div className="connect-section">
        <button className={btnClass} onClick={handleConnect}>
          {btnLabel}
        </button>
      </div>

      {/* Server Info */}
      {connState === ConnStates.connected && (
        <div className="server-section">
          <div className="server-card">
            <div className="server-row">
              <span className="server-row-label">Server</span>
              <span className="server-row-value">{server}:{port}</span>
            </div>
            <div className="server-row">
              <span className="server-row-label">Protocol</span>
              <span className="server-row-value">{protocol.toUpperCase()}</span>
            </div>
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
            className="settings-select"
            value={protocol}
            onChange={e => setProtocol(e.target.value)}
          >
            <option value="h2">HTTP/2</option>
            <option value="quic">QUIC</option>
          </select>
        </div>
        <div className="settings-row">
          <span className="settings-label">Allow IPv6</span>
          <button
            className={`toggle ${allowIpv6 ? 'active' : ''}`}
            onClick={() => setAllowIpv6(!allowIpv6)}
          />
        </div>
      </div>

      {/* Footer Tabs */}
      <div className="footer">
        <div className="footer-tab active">Home</div>
        <div className="footer-tab">History</div>
        <div className="footer-tab">Settings</div>
      </div>

      <div className="version-footer">Secular v0.1.0</div>
    </div>
  );
};

export default App;
