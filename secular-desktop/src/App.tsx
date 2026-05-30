// Secular Desktop — Main App Component (from design system)
import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

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
          <g fill="#242424">
            <path d="M320 446 c-16 -6 -23 -16 -23 -33 0 -14 7 -24 20 -30 12 -6 21 -6 33 0 18 9 24 30 15 47 -8 15 -28 23 -44 17z"/>
            <path d="M215 438 c-41 -14 -67 -43 -69 -76 -1 -12 0 -16 5 -26 13 -26 38 -41 104 -64 21 -7 44 -16 51 -20 13 -7 27 -21 30 -29 2 -7 1 -6 10 -2 14 7 23 16 30 30 6 11 7 16 7 31 0 21 -6 35 -20 50 -14 14 -30 22 -73 36 -44 15 -54 20 -61 36 -5 11 -5 20 0 31 2 5 4 9 4 9 0 1 -4 0 -18 -4z"/>
            <path d="M178 282 c-24 -15 -34 -35 -32 -63 1 -20 7 -33 19 -46 12 -12 32 -22 73 -35 50 -16 63 -26 65 -49 1 -10 0 -14 -3 -20 -3 -4 -4 -8 -3 -8 7 0 35 12 45 19 35 24 48 59 34 89 -7 14 -25 31 -44 41 -8 4 -34 14 -57 22 -44 15 -68 26 -76 34 -2 3 -6 8 -8 13 l-4 8 -10 -6z"/>
            <path d="M176 123 c-23 -11 -26 -40 -7 -57 15 -12 34 -12 49 1 18 16 14 45 -8 56 -12 6 -22 6 -34 0z"/>
          </g>
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
          <span className="settings-label">SNI</span>
          <input
            className="settings-input"
            value={sni}
            onChange={e => setSni(e.target.value)}
            placeholder="optional sni hostname"
          />
        </div>
        <div className="settings-row">
          <span className="settings-label">Auth Token</span>
          <input
            className="settings-input"
            type="password"
            value={token}
            onChange={e => setToken(e.target.value)}
            placeholder="auth token"
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
