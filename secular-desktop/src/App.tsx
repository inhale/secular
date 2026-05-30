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
            <path d="M320.4 66.0 c-15.8 5.9 -23.2 16.5 -23.2 33.4 0.0 13.7 6.5 23.6 19.5 30.2 12.1 6.3 20.8 6.3 32.8 -0.0 17.6 -9.3 23.9 -29.5 15.0 -46.9 -7.8 -15.4 -27.6 -22.8 -44.0 -16.7 z"/>
            <path d="M214.6 74.4 c-41.0 13.9 -66.8 42.5 -68.8 75.9 -0.9 12.4 0.0 16.3 4.8 26.3 12.6 25.6 38.0 41.2 104.1 64.2 21.5 7.4 44.5 16.1 51.2 19.5 13.2 6.7 27.3 20.6 29.5 29.3 1.5 6.5 1.3 6.3 10.2 2.0 13.7 -7.2 23.0 -16.3 29.5 -29.5 5.9 -11.5 6.7 -15.6 6.7 -30.8 0.0 -21.3 -5.6 -35.4 -20.0 -49.7 -14.3 -14.1 -29.5 -21.7 -73.3 -36.0 -44.3 -14.5 -53.8 -20.2 -61.0 -35.8 -5.2 -11.5 -5.2 -20.0 0.2 -30.6 2.4 -4.8 4.3 -8.9 4.3 -9.1 0.0 -1.3 -3.7 -0.4 -17.6 4.3 z"/>
            <path d="M177.9 229.5 c-23.9 15.2 -33.8 34.9 -32.1 63.1 1.3 20.0 6.7 33.4 18.9 45.6 12.1 12.1 32.1 21.7 73.3 34.7 50.1 15.8 63.3 25.8 65.3 48.8 0.7 9.5 0.0 13.7 -3.5 19.7 -2.6 4.1 -3.7 7.6 -2.8 7.6 6.5 -0.0 35.4 -11.9 45.1 -18.7 34.7 -23.9 48.4 -59.0 34.5 -88.7 -6.5 -14.1 -25.2 -31.2 -43.8 -40.8 -8.2 -4.1 -34.1 -14.1 -57.3 -22.1 -43.6 -15.0 -67.7 -25.8 -75.7 -34.5 -2.4 -2.6 -6.3 -8.5 -8.5 -12.8 l-3.9 -8.0 -9.5 6.1 z"/>
            <path d="M175.7 388.8 c-22.6 11.5 -25.8 40.4 -6.7 56.6 14.8 12.4 34.3 12.1 49.0 -0.9 18.2 -16.1 14.1 -45.1 -8.0 -56.0 -12.4 -5.9 -22.3 -5.9 -34.3 0.2 z"/>
          </g>
          <g fill="#d02b57">
            <circle cx="320.4" cy="66.1" r="14.0"/>
            <circle cx="175.7" cy="388.8" r="14.0"/>
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
