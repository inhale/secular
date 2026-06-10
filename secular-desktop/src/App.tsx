// Secular Desktop — Dark Theme v3 (matches Android)
import React, { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/* ─── Types ─── */
type ConnState = 'disconnected' | 'connecting' | 'connected';
type Screen = 'dashboard' | 'server-list' | 'add-server' | 'server-config' | 'query-log';

/** Tray state payload from Rust backend */
interface TrayStatePayload {
  connected: boolean;
  connecting: boolean;
  server: string;
  session_time?: string;
  download_pkts?: number;
  upload_pkts?: number;
}

/** Server config matching Android ServerProfile / TrustTunnel TOML spec */
interface ServerConfig {
  /** Display name from tt:// link (may be empty) */
  name: string;
  /** IP:port address (e.g. "185.103.24.4:443") */
  address: string;
  /** SNI hostname for TLS handshake */
  hostname: string;
  /** Username for TrustTunnel auth */
  username: string;
  /** Password for TrustTunnel auth */
  password: string;
  /** Protocol: "http2" | "http3" */
  upstream_protocol: string;
  /** DNS upstreams (one per line) */
  dns_upstreams: string[];
  /** Allow IPv6 traffic */
  has_ipv6: boolean;
  /** Certificate PEM or path */
  certificate: string;
  /** Skip TLS verification */
  skip_verification: boolean;
  /** Anti-DPI */
  anti_dpi: boolean;
  /** Change system DNS to route through tunnel */
  change_system_dns: boolean;
  /** Domains/IPs to bypass (not routed through VPN tunnel).
   *  Supports: domain.com, *.domain.com, 1.2.3.4, 1.2.3.4:443, *:80 */
  bypass_domains: string[];
}

interface ServerInfo {
  id: string;
  name: string;
  meta: string;
  config: ServerConfig;
  isDefault: boolean;
}

interface LogLine {
  time: string;
  level: 'ok' | 'info' | 'warn' | 'error';
  message: string;
}

/* ─── SVG Icons ─── */
const IconHome = () => (
  <svg viewBox="0 0 24 24">
    <path d="M3 9.5L12 3l9 6.5V20a1 1 0 01-1 1H4a1 1 0 01-1-1V9.5z" />
    <path d="M9 21V12h6v9" />
  </svg>
);

const IconLog = () => (
  <svg viewBox="0 0 24 24">
    <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8l-6-6z" />
    <path d="M14 2v6h6" />
    <path d="M16 13H8" />
    <path d="M16 17H8" />
    <path d="M10 9H8" />
  </svg>
);

const IconAdd = () => (
  <svg viewBox="0 0 24 24">
    <circle cx="12" cy="12" r="10" />
    <path d="M12 8v8" />
    <path d="M8 12h8" />
  </svg>
);

const IconGear = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
    <path d="M12 2v3m0 14v3M2 12h3m14 0h3M4.93 4.93l2.12 2.12m9.9 9.9l2.12 2.12M4.93 19.07l2.12-2.12m9.9-9.9l2.12-2.12" />
    <circle cx="12" cy="12" r="7" />
    <circle cx="12" cy="12" r="3" />
  </svg>
);

const IconScan = () => (
  <svg viewBox="0 0 24 24">
    <path d="M3 7V5a2 2 0 012-2h2" />
    <path d="M17 3h2a2 2 0 012 2v2" />
    <path d="M21 17v2a2 2 0 01-2 2h-2" />
    <path d="M7 21H5a2 2 0 01-2-2v-2" />
    <path d="M7 12h10" />
  </svg>
);

const IconUpload = () => (
  <svg viewBox="0 0 24 24">
    <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" />
    <polyline points="17 8 12 3 7 8" />
    <line x1="12" y1="3" x2="12" y2="15" />
  </svg>
);

const IconEdit = () => (
  <svg viewBox="0 0 24 24">
    <path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7" />
    <path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z" />
  </svg>
);

const IconCopy = () => (
  <svg viewBox="0 0 24 24">
    <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
    <path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1" />
  </svg>
);

const IconTrash = () => (
  <svg viewBox="0 0 24 24">
    <polyline points="3 6 5 6 21 6" />
    <path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2" />
  </svg>
);

const IconBack = () => (
  <svg viewBox="0 0 24 24">
    <path d="M19 12H5" />
    <polyline points="12 19 5 12 12 5" />
  </svg>
);

const IconEyeOn = () => (
  <svg viewBox="0 0 24 24">
    <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
    <circle cx="12" cy="12" r="3" />
  </svg>
);

const IconEyeOff = () => (
  <svg viewBox="0 0 24 24">
    <path d="M17.94 17.94A10.07 10.07 0 0112 20c-7 0-11-8-11-8a18.45 18.45 0 015.06-5.94M9.9 4.24A9.12 9.12 0 0112 4c7 0 11 8 11 8a18.5 18.5 0 01-2.16 3.19m-6.72-1.07a3 3 0 11-4.24-4.24" />
    <line x1="1" y1="1" x2="23" y2="23" />
  </svg>
);

const IconCaret = () => (
  <svg viewBox="0 0 24 24">
    <polyline points="6 9 12 15 18 9" />
  </svg>
);

/* ─── S Logo SVG ─── */
const SLogo = ({ color = 'currentColor', size = 64 }: { color?: string; size?: number }) => (
  <svg className="s-logo-svg" width={size} height={size} viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg">
    <g className="s-path" fill={color}>
      <path d="M320.4 66.0 c-15.8 5.9 -23.2 16.5 -23.2 33.4 0.0 13.7 6.5 23.6 19.5 30.2 12.1 6.3 20.8 6.3 32.8 -0.0 17.6 -9.3 23.9 -29.5 15.0 -46.9 -7.8 -15.4 -27.6 -22.8 -44.0 -16.7 z" />
      <path d="M214.6 74.4 c-41.0 13.9 -66.8 42.5 -68.8 75.9 -0.9 12.4 0.0 16.3 4.8 26.3 12.6 25.6 38.0 41.2 104.1 64.2 21.5 7.4 44.5 16.1 51.2 19.5 13.2 6.7 27.3 20.6 29.5 29.3 1.5 6.5 1.3 6.3 10.2 2.0 13.7 -7.2 23.0 -16.3 29.5 -29.5 5.9 -11.5 6.7 -15.6 6.7 -30.8 0.0 -21.3 -5.6 -35.4 -20.0 -49.7 -14.3 -14.1 -29.5 -21.7 -73.3 -36.0 -44.3 -14.5 -53.8 -20.2 -61.0 -35.8 -5.2 -11.5 -5.2 -20.0 0.2 -30.6 2.4 -4.8 4.3 -8.9 4.3 -9.1 0.0 -1.3 -3.7 -0.4 -17.6 4.3 z" />
      <path d="M177.9 229.5 c-23.9 15.2 -33.8 34.9 -32.1 63.1 1.3 20.0 6.7 33.4 18.9 45.6 12.1 12.1 32.1 21.7 73.3 34.7 50.1 15.8 63.3 25.8 65.3 48.8 0.7 9.5 0.0 13.7 -3.5 19.7 -2.6 4.1 -3.7 7.6 -2.8 7.6 6.5 -0.0 35.4 -11.9 45.1 -18.7 34.7 -23.9 48.4 -59.0 34.5 -88.7 -6.5 -14.1 -25.2 -31.2 -43.8 -40.8 -8.2 -4.1 -34.1 -14.1 -57.3 -22.1 -43.6 -15.0 -67.7 -25.8 -75.7 -34.5 -2.4 -2.6 -6.3 -8.5 -8.5 -12.8 l-3.9 -8.0 -9.5 6.1 z" />
      <path d="M175.7 388.8 c-22.6 11.5 -25.8 40.4 -6.7 56.6 14.8 12.4 34.3 12.1 49.0 -0.9 18.2 -16.1 14.1 -45.1 -8.0 -56.0 -12.4 -5.9 -22.3 -5.9 -34.3 0.2 z" />
    </g>
    <circle cx="320.4" cy="66.1" r="14.0" fill={color} />
    <circle cx="175.7" cy="388.8" r="14.0" fill={color} />
  </svg>
);

/* ─── Bottom Navigation ─── */
interface BottomNavProps {
  active: Screen;
  onNav: (s: Screen) => void;
}

const BottomNav: React.FC<BottomNavProps> = ({ active, onNav }) => {
  const navItems: { key: Screen; icon: React.ReactNode }[] = [
    { key: 'query-log', icon: <IconLog /> },
    { key: 'dashboard', icon: <IconHome /> },
    { key: 'add-server', icon: <IconAdd /> },
  ];

  return (
    <div className="bottom-nav">
      {navItems.map(({ key, icon }) => (
        <div
          key={key}
          className={`nav-icon ${active === key ? 'active' : ''}`}
          onClick={() => onNav(key)}
        >
          {icon}
        </div>
      ))}
    </div>
  );
};

/* ─── Screen: Dashboard ─── */
interface DashboardProps {
  connState: ConnState;
  onToggleConnect: () => void;
  onNav: (s: Screen) => void;
  servers: ServerInfo[];
  onSetDefault: (id: string) => void;
  onEditServer: (id: string) => void;
  sessionTime: string;
  downloadPkts: number;
  uploadPkts: number;
}

const Dashboard: React.FC<DashboardProps> = ({ connState, onToggleConnect, onNav, servers, onSetDefault, onEditServer, sessionTime, downloadPkts, uploadPkts }) => {
  const isActive = connState === 'connecting';
  const isConnected = connState === 'connected';

  return (
    <div className="screen">
      <div className="screen-content status-padding">
        {/* Status label */}
        <div className={`status-label ${connState}`}>
          {connState === 'connected' ? 'Connected' : connState === 'connecting' ? 'Connecting' : 'Disconnected'}
        </div>

        {/* Metrics row */}
        <div className="metrics-row">
          <div className="metric-item">
            <span className="metric-value">{sessionTime}</span>
            <span className="metric-label">SESSION</span>
          </div>
          <div className="metric-item">
            <span className="metric-value">{downloadPkts}</span>
            <span className="metric-label">DOWNLOAD PKTS</span>
          </div>
          <div className="metric-item">
            <span className="metric-value">{uploadPkts}</span>
            <span className="metric-label">UPLOAD PKTS</span>
          </div>
        </div>

        {/* Connect button */}
        <div className="connect-area">
          <div className={`connect-circle ${connState}`} onClick={onToggleConnect}>
            {isActive && <div className="spinner-ring" />}
            <div className="s-logo-container">
              <SLogo color={isActive || isConnected ? '#00FF66' : '#FFFFFF'} size={64} />
            </div>
          </div>
        </div>

        {/* Server decks — all servers inline on dashboard */}
        <div className="server-decks">
          {servers.length === 0 ? (
            <div className="server-deck" onClick={() => onNav('add-server')}>
              <div className="server-deck-left">
                <div className="server-deck-name">No server configured</div>
                <div className="server-deck-meta">Tap to add one</div>
              </div>
            </div>
          ) : (
            servers.map((srv) => (
              <div
                key={srv.id}
                className={`server-deck ${srv.isDefault ? 'active' : ''}`}
                onClick={() => {
                  if (!srv.isDefault) {
                    onSetDefault(srv.id);
                  }
                }}
              >
                <div className="server-deck-left">
                  <div className="server-deck-name">
                    {srv.name}
                    {srv.isDefault && <span className="server-deck-dot" />}
                  </div>
                  <div className="server-deck-meta">
                    {(srv.config.address || `${(srv.config as any).host}:${(srv.config as any).port || 443}`)} / {(srv.config.upstream_protocol || (srv.config as any).protocol || 'http2') === 'http3' ? 'QUIC' : 'HTTP/2'}
                  </div>
                </div>
                <div
                  className="server-deck-gear"
                  onClick={(e) => {
                    e.stopPropagation();
                    onEditServer(srv.id);
                  }}
                >
                  <IconGear />
                </div>
              </div>
            ))
          )}
        </div>
      </div>
      <BottomNav active="dashboard" onNav={onNav} />
    </div>
  );
};

/* ─── Screen: Server List ─── */
interface ServerListProps {
  servers: ServerInfo[];
  onNav: (s: Screen) => void;
  onSetDefault: (id: string) => void;
  onEditServer: (id: string) => void;
}

const ServerList: React.FC<ServerListProps> = ({ servers, onNav, onSetDefault, onEditServer }) => {
  return (
    <div className="screen">
      <div className="header-row">
        <h1 className="screen-title">My Servers</h1>
      </div>
      <div className="screen-content">
        <div className="server-list">
          {servers.length === 0 ? (
            <div className="empty-servers">
              No servers configured.<br />Tap the + button to add one.
            </div>
          ) : (
            servers.map((srv) => (
              <div
                key={srv.id}
                className={`server-item ${srv.isDefault ? 'default' : ''}`}
                onClick={() => {
                  if (!srv.isDefault) onSetDefault(srv.id);
                }}
              >
                <div className="server-item-row">
                  <span className="server-item-name">{srv.name}</span>
                  <div className="server-item-right">
                    <span className="server-item-default">{srv.isDefault ? '● DEFAULT' : ''}</span>
                    <span className="server-item-gear" onClick={(e) => { e.stopPropagation(); onEditServer(srv.id); }}>
                      <IconGear />
                    </span>
                  </div>
                </div>
                <div className="server-item-meta">{srv.meta}</div>
              </div>
            ))
          )}
        </div>
      </div>
      <BottomNav active="server-list" onNav={onNav} />
    </div>
  );
};

/* ─── Screen: Add Server ─── */
interface AddServerProps {
  onNav: (s: Screen) => void;
  onAddServer: (config: ServerConfig) => void;
  onEditNewServer: () => void;
  onImportConfig: (config: ServerConfig) => void;
}

const AddServer: React.FC<AddServerProps> = ({ onNav, onAddServer, onEditNewServer, onImportConfig }) => {
  const [link, setLink] = useState('');

  const doAdd = useCallback((raw: string) => {
    if (!raw.trim()) return;
    const trimmed = raw.trim();
    console.log('[IMPORT] doAdd called, length:', trimmed.length);
    if (trimmed.startsWith('tt://') || trimmed.startsWith('secular://')) {
      const b64 = trimmed.replace(/^(tt|secular):\/\/\??/, '');
      console.log('[IMPORT] b64 length:', b64.length);
      try {
        const toml = atob(b64);
        console.log('[IMPORT] toml length:', toml.length);
        const config = parseTomlConfig(toml);
        console.log('[IMPORT] config:', JSON.stringify(config));
        onImportConfig(config);
        setLink('');
        return;
      } catch (e) {
        console.error('[IMPORT] decode failed:', e);
      }
    } else {
      console.log('[IMPORT] not tt://, plain host');
    }
    const host = trimmed.replace('secular://', '').split(':')[0] || trimmed;
    const port = parseInt(trimmed.split(':')[1], 10) || 443;
    onAddServer({ name: '', address: `${host}:${port}`, hostname: host, username: '', password: '', upstream_protocol: 'http2', dns_upstreams: ['9.9.9.9', '149.112.112.112'], has_ipv6: false, certificate: '', skip_verification: false, anti_dpi: false, change_system_dns: true, bypass_domains: [] });
    setLink('');
    onNav('dashboard');
  }, [onAddServer, onImportConfig, onNav]);

  const linkInputRef = useRef<HTMLInputElement>(null);

  const handleAdd = async () => {
    let raw = linkInputRef.current?.value ?? link;
    // If input is empty, try reading from clipboard (user gesture satisfies security)
    if (!raw.trim()) {
      try {
        raw = await navigator.clipboard.readText();
        if (raw) setLink(raw);
      } catch {}
    }
    doAdd(raw);
  };

  const handleTomlUpload = async () => {
    // Use hidden file input to pick a .toml file
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.toml';
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return;
      const text = await file.text();
      const config = parseTomlConfig(text);
      onAddServer(config);
      onNav('dashboard');
    };
    input.click();
  };

  return (
    <div className="screen">
      <div className="add-header">
        <h1>Add Connection</h1>
      </div>
      <div className="screen-content">
        {/* Link input */}
        <div className="link-section">
          <div className="field-label">Insert Link</div>
          <div className="link-input-row">
            <input
              ref={linkInputRef}
              className="link-input"
              value={link}
              onChange={e => setLink(e.target.value)}
              placeholder="Paste tt:// link here"
              onKeyDown={e => e.key === 'Enter' && handleAdd()}
            />
            <button className="link-add-btn" onClick={() => handleAdd()}>Add</button>
          </div>
        </div>

        {/* OR divider */}
        <div className="or-divider">
          <div className="or-line" />
          <span className="or-text">OR</span>
          <div className="or-line" />
        </div>

        {/* Action buttons */}
        <button className="action-btn" onClick={() => {}}>
          <span className="action-btn-icon"><IconScan /></span>
          Scan QR Code
        </button>
        <button className="action-btn" onClick={handleTomlUpload}>
          <span className="action-btn-icon"><IconUpload /></span>
          Upload .toml file
        </button>
        <button className="action-btn" onClick={onEditNewServer}>
          <span className="action-btn-icon"><IconEdit /></span>
          Add Server Manually
        </button>
      </div>
      <BottomNav active="add-server" onNav={onNav} />
    </div>
  );
};

/** TOML parser matching Android TomlFileParser — handles [endpoint] sections, triple-quoted strings, arrays, top-level exclusions */
function parseTomlConfig(content: string): ServerConfig {
  const config: ServerConfig = {
    name: '',
    address: '',
    hostname: '',
    username: '',
    password: '',
    upstream_protocol: 'http2',
    dns_upstreams: [],
    has_ipv6: true,
    certificate: '',
    skip_verification: false,
    anti_dpi: false,
    change_system_dns: true,
    bypass_domains: [],
  };
  const fields: Record<string, string> = {};
  const lines = content.split('\n');
  let currentSection = '';
  let tripleQuoteKey: string | null = null;
  let tripleQuoteBuf: string[] = [];

  for (const line of lines) {
    // Inside triple-quoted string?
    if (tripleQuoteKey !== null) {
      if (line.includes('"""')) {
        // End of triple quote
        tripleQuoteBuf.push(line.substring(0, line.indexOf('"""')));
        fields[tripleQuoteKey] = tripleQuoteBuf.join('\n');
        tripleQuoteKey = null;
        tripleQuoteBuf = [];
      } else {
        tripleQuoteBuf.push(line);
      }
      continue;
    }

    const trimmed = line.trim();
    if (trimmed.startsWith('#') || trimmed === '') continue;
    if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
      currentSection = trimmed.substring(1, trimmed.length - 1);
      continue;
    }
    const eqIdx = trimmed.indexOf('=');
    if (eqIdx > 0) {
      const key = trimmed.substring(0, eqIdx).trim();
      let rawValue = trimmed.substring(eqIdx + 1).trim();
      const fullKey = currentSection ? `${currentSection}.${key}` : key;

      // Handle triple-quoted string """..."""
      if (rawValue.startsWith('"""')) {
        const afterOpen = rawValue.substring(3);
        if (afterOpen.includes('"""')) {
          // Single-line triple quote: """value"""
          fields[fullKey] = afterOpen.substring(0, afterOpen.indexOf('"""'));
          fields[key] = fields[fullKey];
        } else {
          // Multi-line triple quote — start collecting
          tripleQuoteKey = key; // use bare key for both
          tripleQuoteBuf = [afterOpen];
        }
        continue;
      }

      // Strip outer quotes
      rawValue = rawValue.replace(/^"|"$/g, '');
      fields[fullKey] = rawValue;
      fields[key] = rawValue; // also store bare key
    }
  }
  // Read fields matching Android's fallback chain
  config.hostname = fields['hostname'] || fields['endpoint.hostname'] || fields['endpoints.hostname'] || '';
  const addrRaw = fields['addresses'] || fields['endpoint.addresses'] || '';
  if (addrRaw) {
    const cleaned = addrRaw.replace(/^\[|\]$/g, '').replace(/"/g, '').replace(/'/g, '');
    config.address = cleaned.split(',').map((s: string) => s.trim()).filter((s: string) => s)[0] || '';
  }
  config.username = fields['username'] || fields['endpoint.username'] || fields['endpoints.username'] || '';
  config.password = fields['password'] || fields['endpoint.password'] || fields['endpoints.password'] || '';
  config.upstream_protocol = fields['upstream_protocol'] || fields['endpoint.upstream_protocol'] || 'http2';
  const dnsRaw = fields['dns_upstreams'] || fields['endpoint.dns_upstreams'] || '';
  if (dnsRaw) {
    const cleaned = dnsRaw.replace(/^\[|\]$/g, '').replace(/"/g, '').replace(/'/g, '');
    config.dns_upstreams = cleaned.split(',').map((s: string) => s.trim()).filter((s: string) => s);
  }
  config.has_ipv6 = (fields['has_ipv6'] || fields['endpoint.has_ipv6'] || 'true') === 'true';
  config.skip_verification = (fields['skip_verification'] || fields['endpoint.skip_verification'] || 'false') === 'true';
  config.anti_dpi = (fields['anti_dpi'] || fields['endpoint.anti_dpi'] || 'false') === 'true';
  config.change_system_dns = (fields['change_system_dns'] || 'true') === 'true';
  config.certificate = fields['certificate'] || fields['endpoint.certificate'] || '';
  config.name = fields['name'] || fields['endpoint.name'] || '';

  // Parse top-level exclusions (bypass domains) — TrustTunnel format: exclusions = ["domain.com", "*.example.com"]
  const exclRaw = fields['exclusions'] || '';
  if (exclRaw) {
    const cleaned = exclRaw.replace(/^\[|\]$/g, '').replace(/"/g, '').replace(/'/g, '');
    config.bypass_domains = cleaned.split(',').map((s: string) => s.trim()).filter((s: string) => s);
  }

  return config;
}

/* ─── Screen: Server Config (matches Android screen3) ─── */
interface ServerConfigScreenProps {
  server: ServerInfo;
  isNew: boolean;
  onSave: (config: ServerInfo) => void;
  onDelete: (id: string) => void;
  onNav: (s: Screen) => void;
}

const ServerConfigScreen: React.FC<ServerConfigScreenProps> = ({ server, isNew, onSave, onDelete, onNav }) => {
  // Safe defaults in case config fields are missing (e.g. from old localStorage)
  const safeConfig = {
    address: server.config.address || '',
    hostname: server.config.hostname || '',
    username: server.config.username || '',
    password: server.config.password || '',
    upstream_protocol: server.config.upstream_protocol || 'http2',
    dns_upstreams: server.config.dns_upstreams || ['9.9.9.9', '149.112.112.112'],
    has_ipv6: server.config.has_ipv6 ?? false,
    certificate: server.config.certificate || '',
    skip_verification: server.config.skip_verification ?? false,
    anti_dpi: server.config.anti_dpi ?? false,
    change_system_dns: server.config.change_system_dns ?? true,
    bypass_domains: server.config.bypass_domains || [],
  };
  const [name, setName] = useState(server.name);
  const [address, setAddress] = useState(safeConfig.address);
  const [hostname, setHostname] = useState(safeConfig.hostname);
  const [username, setUsername] = useState(safeConfig.username);
  const [password, setPassword] = useState(safeConfig.password);
  const [passwordVisible, setPasswordVisible] = useState(false);
  const [protocol, setProtocol] = useState(safeConfig.upstream_protocol === 'http3' ? 1 : 0);
  const [dns, setDns] = useState(safeConfig.dns_upstreams.join('\n'));
  const [hasIpv6, setHasIpv6] = useState(safeConfig.has_ipv6);
  const [changeDns, setChangeDns] = useState(safeConfig.change_system_dns);
  const [bypassDomains, setBypassDomains] = useState<string[]>(safeConfig.bypass_domains);
  const [newBypass, setNewBypass] = useState('');
  const [protocolOpen, setProtocolOpen] = useState(false);

  const handleSave = () => {
    const addr = address.trim();
    const dnsLines = dns.split('\n').map(l => l.trim()).filter(l => l);
    if (!name.trim()) {
      setName(hostname || addr || 'New Server');
    }
    onSave({
      ...server,
      name: name.trim() || hostname || addr || 'New Server',
      config: {
        name: server.config.name || '',
        address: addr,
        hostname: hostname.trim() || addr.split(':')[0],
        username: username.trim(),
        password,
        upstream_protocol: protocol === 1 ? 'http3' : 'http2',
        dns_upstreams: dnsLines.length ? dnsLines : ['9.9.9.9', '149.112.112.112'],
        has_ipv6: hasIpv6,
        certificate: server.config.certificate,
        skip_verification: server.config.skip_verification,
        anti_dpi: server.config.anti_dpi,
        change_system_dns: changeDns,
        bypass_domains: bypassDomains,
      },
    });
    onNav('dashboard');
  };

  const handleDelete = () => {
    onDelete(server.id);
    onNav('dashboard');
  };

  const addBypass = () => {
    const d = newBypass.trim();
    if (d && !bypassDomains.includes(d)) {
      setBypassDomains([...bypassDomains, d]);
    }
    setNewBypass('');
  };

  const removeBypass = (idx: number) => {
    setBypassDomains(bypassDomains.filter((_, i) => i !== idx));
  };

  const protocols = ['HTTP/2', 'QUIC'];

  return (
    <div className="screen">
      <div className="config-header-bar">
        <div className="config-back" onClick={() => onNav(isNew ? 'add-server' : 'dashboard')}>
          <IconBack />
        </div>
        <h1>{isNew ? 'New Server' : name || 'Server Config'}</h1>
        {!isNew && (
          <button className="config-trash-btn" onClick={handleDelete} title="Delete server">
            <IconTrash />
          </button>
        )}
      </div>
      <div className="screen-content config-content">
        {/* Server Name */}
        <div className="config-field">
          <div className="config-field-label">SERVER NAME</div>
          <input className="config-field-input" value={name} onChange={e => setName(e.target.value)} placeholder="My Server" />
        </div>

        {/* IP Address */}
        <div className="config-field">
          <div className="config-field-label">IP ADDRESS</div>
          <input className="config-field-input" value={address} onChange={e => setAddress(e.target.value)} placeholder="185.103.24.4:443" />
        </div>

        {/* Hostname (SNI) */}
        <div className="config-field">
          <div className="config-field-label">HOSTNAME FOR TLS HANDSHAKE</div>
          <input className="config-field-input" value={hostname} onChange={e => setHostname(e.target.value)} placeholder="e.g. server.example.com" />
        </div>

        {/* Username */}
        <div className="config-field">
          <div className="config-field-label">USERNAME</div>
          <input className="config-field-input" value={username} onChange={e => setUsername(e.target.value)} placeholder="Username" />
        </div>

        {/* Password with reveal toggle */}
        <div className="config-field">
          <div className="config-field-label">PASSWORD</div>
          <div className="password-input-row">
            <input
              className="config-field-input password-input"
              type={passwordVisible ? 'text' : 'password'}
              value={password}
              onChange={e => setPassword(e.target.value)}
              placeholder="Password"
            />
            <button className="password-toggle" onClick={() => setPasswordVisible(!passwordVisible)}>
              {passwordVisible ? <IconEyeOn /> : <IconEyeOff />}
            </button>
          </div>
        </div>

        {/* Protocol dropdown */}
        <div className="config-field">
          <div className="config-field-label">PROTOCOL</div>
          <div className="protocol-dropdown" onClick={() => setProtocolOpen(!protocolOpen)}>
            <span className="protocol-value">{protocols[protocol]}</span>
            <span className="protocol-caret"><IconCaret /></span>
          </div>
          {protocolOpen && (
            <div className="protocol-options">
              {protocols.map((p, i) => (
                <div key={p} className={`protocol-option ${i === protocol ? 'active' : ''}`} onClick={() => { setProtocol(i); setProtocolOpen(false); }}>
                  {p}
                </div>
              ))}
            </div>
          )}
        </div>

        {/* DNS Servers */}
        <div className="config-field">
          <div className="config-field-label">DNS SERVERS</div>
          <textarea
            className="config-field-textarea"
            value={dns}
            onChange={e => setDns(e.target.value)}
            placeholder="One DNS server per line"
            rows={3}
          />
        </div>

        {/* Certificate upload */}
        <div className="config-field">
          <div className="config-field-label">CERTIFICATE</div>
          <button className="cert-upload-btn" onClick={() => {
            const input = document.createElement('input');
            input.type = 'file';
            input.accept = '.pem,.crt,.cer';
            input.onchange = async () => {
              const file = input.files?.[0];
              if (!file) return;
              const cert = await file.text();
              // Store cert in server config
              onSave({ ...server, config: { ...server.config, certificate: cert } });
            };
            input.click();
          }}>
            <span className="action-btn-icon"><IconUpload /></span>
            {safeConfig.certificate ? 'Certificate loaded ✓' : 'Upload .pem file'}
          </button>
        </div>

        {/* IPv6 toggle */}
        <div className="config-field config-toggle-field">
          <label className="toggle-label">
            <span>Allow IPv6 traffic</span>
            <div className="toggle-switch">
              <input type="checkbox" checked={hasIpv6} onChange={e => setHasIpv6(e.target.checked)} />
              <span className="toggle-slider" />
            </div>
          </label>
        </div>

        {/* Change System DNS toggle */}
        <div className="config-field config-toggle-field">
          <label className="toggle-label">
            <span>Change system DNS</span>
            <div className="toggle-switch">
              <input type="checkbox" checked={changeDns} onChange={e => setChangeDns(e.target.checked)} />
              <span className="toggle-slider" />
            </div>
          </label>
        </div>

        {/* Bypass List — domains/IPs excluded from VPN */}
        <div className="config-field">
          <div className="config-field-label">BYPASS LIST</div>
          <div className="config-field-hint">Domains and IPs that skip the VPN tunnel</div>
          <div className="bypass-list">
            {bypassDomains.map((domain, idx) => (
              <div key={idx} className="bypass-item">
                <span className="bypass-domain">{domain}</span>
                <button className="bypass-remove" onClick={() => removeBypass(idx)}>×</button>
              </div>
            ))}
          </div>
          <div className="bypass-add-row">
            <input
              className="config-field-input bypass-input"
              value={newBypass}
              onChange={e => setNewBypass(e.target.value)}
              placeholder="example.com, *.example.com, 1.2.3.4"
              onKeyDown={e => e.key === 'Enter' && addBypass()}
            />
            <button className="bypass-add-btn" onClick={addBypass}>Add</button>
          </div>
        </div>

        <div className="config-save-row">
          <button className="config-save-btn-bottom" onClick={handleSave}>
            SAVE
          </button>
        </div>
      </div>
      <BottomNav active="dashboard" onNav={onNav} />
    </div>
  );
};

/* ─── Screen: Query Log ─── */
interface QueryLogProps {
  logs: LogLine[];
  onNav: (s: Screen) => void;
  onClear: () => void;
}

const IconFilter = () => (
  <svg viewBox="0 0 24 24">
    <path d="M22 3H2l8 9.46V19l4 2v-8.54L22 3z" />
  </svg>
);

const QueryLog: React.FC<QueryLogProps> = ({ logs, onNav, onClear }) => {
  const [filterOpen, setFilterOpen] = useState(false);
  const [filterLevels, setFilterLevels] = useState<Set<string>>(new Set(['ok', 'info', 'warn', 'error']));

  const toggleLevel = (level: string) => {
    setFilterLevels(prev => {
      const next = new Set(prev);
      if (next.has(level)) next.delete(level); else next.add(level);
      return next;
    });
  };

  const filteredLogs = logs.filter(l => filterLevels.has(l.level));

  const handleCopy = () => {
    const text = filteredLogs.map(l => `[${l.time}] ${l.level.toUpperCase()} ${l.message}`).join('\n');
    navigator.clipboard?.writeText(text).catch(() => {});
  };

  const handleSelectAll = () => {
    setFilterLevels(new Set(['ok', 'info', 'warn', 'error']));
  };

  const handleApply = () => {
    setFilterOpen(false);
  };

  return (
    <div className="screen">
      <div className="log-header">
        <div className="log-header-left">
          <h1>Query Log</h1>
        </div>
        <div className="log-header-right">
          <div className="log-action-icon" title="Copy" onClick={handleCopy}>
            <IconCopy />
          </div>
          <div className="log-action-icon" title="Clear" onClick={onClear}>
            <IconTrash />
          </div>
          <div className="log-action-icon" title="Filter" onClick={() => setFilterOpen(!filterOpen)}>
            <IconFilter />
          </div>
        </div>
      </div>
      {filterOpen && (
        <div className="filter-popup open">
          {(['ok', 'info', 'warn', 'error'] as const).map(level => (
            <div
              key={level}
              className={`filter-opt ${filterLevels.has(level) ? 'active' : ''}`}
              onClick={() => toggleLevel(level)}
            >
              <span className={`filter-dot ${level === 'error' ? 'err' : level}`} />
              <span>{level === 'ok' ? 'OK' : level === 'info' ? 'Info' : level === 'warn' ? 'Warn' : 'Error'}</span>
              {filterLevels.has(level) && <span className="filter-check">✓</span>}
            </div>
          ))}
          <div className="filter-buttons-row">
            <div className="filter-all-row" onClick={handleSelectAll}>
              <span>Select All</span>
            </div>
            <div className="filter-apply-row" onClick={handleApply}>
              <span>Apply</span>
            </div>
          </div>
        </div>
      )}
      <div className="screen-content" style={{ display: 'flex', flexDirection: 'column' }}>
        <div className="log-card" style={{ margin: '0 0 16px' }}>
          <div className="log-lines">
            {filteredLogs.length === 0 ? (
              <div className="log-empty-msg" style={{ textAlign: 'center', padding: '32px 0', color: '#8A8A8A', opacity: 0.4 }}>
                No logs yet.<br />Events will appear here when connecting.
              </div>
            ) : (
              filteredLogs.map((line, i) => (
                <div key={i} className="log-line">
                  <span className="log-time">{line.time}</span>
                  <span className={`log-msg-level-${line.level}`}>{line.message}</span>
                </div>
              ))
            )}
          </div>
        </div>
      </div>
      <BottomNav active="query-log" onNav={onNav} />
    </div>
  );
};

/* ─── Persistence helpers ─── */
const STORAGE_KEY = 'secular-servers';

function loadServers(): ServerInfo[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const servers: ServerInfo[] = JSON.parse(raw);
      // Migrate old format (host/port/auth_token) to new format (address/hostname/username/password)
      return servers.map(s => {
        const c = s.config as any;
        if (c.host !== undefined && c.address === undefined) {
          // Old format — migrate
          return {
            ...s,
            config: {
              name: '',
              address: c.host ? `${c.host}:${c.port || 443}` : '',
              hostname: c.sni || c.host || '',
              username: c.auth_token || '',
              password: '',
              upstream_protocol: (c.protocol === 'quic' || c.protocol === 'http3') ? 'http3' : 'http2',
              dns_upstreams: ['9.9.9.9', '149.112.112.112'],
              has_ipv6: c.allow_ipv6 || false,
              certificate: '',
              skip_verification: false,
              anti_dpi: false,
              change_system_dns: true,
              bypass_domains: [],
            },
          };
        }
        // Ensure bypass_domains exists on existing configs
        if (c.bypass_domains === undefined) {
          return { ...s, config: { ...s.config, bypass_domains: [] } };
        }
        return s;
      });
    }
  } catch {}
  return [];
}

function saveServers(servers: ServerInfo[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(servers));
}

/* ─── Main App ─── */
const App: React.FC = () => {
  const [screen, setScreen] = useState<Screen>('dashboard');
  const [connState, setConnState] = useState<ConnState>('disconnected');
  const [servers, setServers] = useState<ServerInfo[]>(loadServers);
  const [editingServer, setEditingServer] = useState<ServerInfo | null>(null);
  const [isNewServer, setIsNewServer] = useState(false);
  const [logs, setLogs] = useState<LogLine[]>([
    { time: '12:00:01', level: 'info', message: 'Application started' },
    { time: '12:00:02', level: 'ok', message: 'Configuration loaded' },
  ]);
  const [sessionStart, setSessionStart] = useState<number | null>(null);
  const [sessionTime, setSessionTime] = useState('00:00:00');
  const [downloadPkts, setDownloadPkts] = useState(0);
  const [uploadPkts, setUploadPkts] = useState(0);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const activeServer = servers.find(s => s.isDefault) || servers[0] || null;

  // Persist servers on change
  useEffect(() => {
    saveServers(servers);
  }, [servers]);

  // Throttled tray update — updates immediately on state change, then at most once per 5s for stats
  const trayUpdateTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    const serverName = activeServer?.name || 'No Server';
    console.log('[TRAY-JS] effect fired:', { connState, serverName, sessionTime, downloadPkts, uploadPkts });

    // Debounce: clear pending timer, schedule new one
    if (trayUpdateTimer.current) clearTimeout(trayUpdateTimer.current);
    trayUpdateTimer.current = setTimeout(() => {
      const isInvokeDefined = typeof invoke === 'function';
      console.log('[TRAY-JS] invoke called:', { connected: connState === 'connected', connecting: connState === 'connecting', server: serverName, sessionTime, downloadPkts, uploadPkts, isInvokeDefined });
      if (!isInvokeDefined) {
        console.error('[TRAY-JS] invoke is NOT a function! Tauri bridge unavailable.');
        return;
      }
      invoke('update_tray', {
        connected: connState === 'connected',
        connecting: connState === 'connecting',
        server: serverName,
        sessionTime: sessionTime || '00:00:00',
        downloadPkts: downloadPkts || 0,
        uploadPkts: uploadPkts || 0,
      }).then(() => {
        console.log('[TRAY-JS] invoke resolved OK');
      }).catch((err: any) => {
        console.error('[TRAY-JS] invoke rejected:', err);
      });
    }, 200);
  }, [connState, activeServer?.name, sessionTime, downloadPkts, uploadPkts]);

  // (tray-connect listener moved after handleToggleConnect definition)

  // Listen for tray-nav event (user clicked a navigation item in tray menu)
  useEffect(() => {
    const unlisten = listen<{ screen: string }>('tray-nav', (event) => {
      const screen = event.payload?.screen as Screen | undefined;
      if (screen) {
        setScreen(screen);
        // Also show the window when navigating from tray
        invoke('show_window').catch(() => {});
      }
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);

  const addLog = (level: LogLine['level'], message: string) => {
    const now = new Date();
    const time = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}:${String(now.getSeconds()).padStart(2, '0')}`;
    setLogs(prev => [...prev, { time, level, message }]);
  };

  // Tunnel log polling: read trusttunnel_client output and inject into query log
  const [lastLogOffset, setLastLogOffset] = useState(0);
  useEffect(() => {
    if (connState !== 'connected') return;
    const interval = setInterval(async () => {
      try {
        const fullLog: string = await invoke('read_tunnel_log');
        if (!fullLog || fullLog === 'No tunnel log yet') return;
        // Only process new lines since last read
        const lines = fullLog.split('\n').filter(Boolean);
        if (lines.length <= lastLogOffset) return;
        const newLines = lines.slice(lastLogOffset);
        setLastLogOffset(lines.length);
        // Parse TrustTunnel log format: "[LEVEL] [MODULE] message" or "HH:MM:SS [LEVEL] ..."
        const parsed = newLines.map(line => {
          let level: LogLine['level'] = 'info';
          let msg = line.trim();
          // Detect level from common patterns
          if (/\b(ERROR|ERR)\b/i.test(line.substring(0, 30))) level = 'error';
          else if (/\b(WARN|WARNING)\b/i.test(line.substring(0, 30))) level = 'warn';
          else if (/\b(TRACE)\b/i.test(line.substring(0, 30))) level = 'info'; // trace → info (verbose)
          else if (/\b(DEBUG|DBG)\b/i.test(line.substring(0, 30))) level = 'info';
          else if (/\b(INFO)\b/i.test(line.substring(0, 30))) level = 'ok';
          // Strip timestamp prefix like "2025-06-02 01:06:42.123 " if present
          msg = msg.replace(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d+\s+/, '');
          // Extract just the time portion for display
          const now = new Date();
          const time = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}:${String(now.getSeconds()).padStart(2, '0')}`;
          return { time, level, message: msg };
        });
        setLogs(prev => [...prev, ...parsed]);
      } catch { /* ignore read errors */ }
    }, 2000); // Poll every 2 seconds
    return () => clearInterval(interval);
  }, [connState, lastLogOffset]);

  // Session timer
  useEffect(() => {
    if (connState === 'connected' && sessionStart) {
      timerRef.current = setInterval(() => {
        const elapsed = Math.floor((Date.now() - sessionStart) / 1000);
        const h = String(Math.floor(elapsed / 3600)).padStart(2, '0');
        const m = String(Math.floor((elapsed % 3600) / 60)).padStart(2, '0');
        const s = String(elapsed % 60).padStart(2, '0');
        setSessionTime(`${h}:${m}:${s}`);
      }, 1000);
      return () => { if (timerRef.current) clearInterval(timerRef.current); };
    }
  }, [connState, sessionStart]);

  // Packet counters (simulated while connected)
  useEffect(() => {
    if (connState !== 'connected') return;
    const interval = setInterval(() => {
      setDownloadPkts(prev => prev + Math.floor(Math.random() * 50));
      setUploadPkts(prev => prev + Math.floor(Math.random() * 30));
    }, 2000);
    return () => clearInterval(interval);
  }, [connState]);

  const handleToggleConnect = async () => {
    console.log('[HANDLE-TOGGLE] called, connState:', connState, 'activeServer:', activeServer?.name, 'cfg.address:', activeServer?.config?.address, 'cfg.username:', activeServer?.config?.username);
    try {
      if (connState === 'connected') {
        console.log('[HANDLE-TOGGLE] disconnecting...');
        await invoke('disconnect');
        setConnState('disconnected');
        setLastLogOffset(0);
        if (timerRef.current) { clearInterval(timerRef.current); timerRef.current = null; }
        setSessionStart(null);
        setSessionTime('00:00:00');
        setDownloadPkts(0);
        setUploadPkts(0);
        addLog('warn', 'Disconnected from server');
      } else if (connState === 'disconnected') {
        if (!activeServer) {
          console.log('[HANDLE-TOGGLE] no activeServer!');
          addLog('error', 'No server configured');
          return;
        }
        const cfg = activeServer.config;
        console.log('[HANDLE-TOGGLE] connecting to', cfg.address, 'user:', cfg.username);
        if (!cfg.address) {
          addLog('error', 'Server has no address configured');
          return;
        }
        if (!cfg.username || !cfg.password) {
          addLog('error', `Server "${activeServer.name}" needs username and password`);
          return;
        }
        setConnState('connecting');
        addLog('info', `Connecting to ${cfg.address}...`);
        try {
          await invoke('connect', { config: cfg });
          setConnState('connected');
          setSessionStart(Date.now());
          setDownloadPkts(0);
          setUploadPkts(0);
          addLog('ok', `Connected to ${cfg.address} via TrustTunnel`);
        } catch (err) {
          console.log('[HANDLE-TOGGLE] connect failed:', err);
          setConnState('disconnected');
          addLog('error', `Connection failed: ${err}`);
        }
      } else {
        console.log('[HANDLE-TOGGLE] neither connected nor disconnected, state:', connState);
      }
    } catch (err) {
      console.error('Connection error:', err);
      setConnState('disconnected');
      addLog('error', `Error: ${err}`);
    }
  };

  // Keep ref to latest handleToggleConnect so the tray-connect listener
  // always sees current closure (activeServer, connState, etc.)
  const handleToggleConnectRef = useRef(handleToggleConnect);
  handleToggleConnectRef.current = handleToggleConnect;

  // Register tray connect handler on window for Rust direct JS eval
  useEffect(() => {
    (window as any).__secular_tray_connect = () => {
      console.log('[TRAY-CONNECT] called via window.__secular_tray_connect');
      handleToggleConnectRef.current();
    };
    return () => { delete (window as any).__secular_tray_connect; };
  }, []);

  useEffect(() => {
    const unlisten = listen('tray-connect', () => {
      console.log('[TRAY-CONNECT] event received');
      handleToggleConnectRef.current();
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);

  // Handle tt:// deep link URLs from macOS
  useEffect(() => {
    const unlisten = listen<string>('deeplink', (event) => {
      const url = event.payload;
      console.log('[DEEPLINK] received:', url);
      if (url && (url.startsWith('tt://') || url.startsWith('secular://'))) {
        const b64 = url.replace(/^(tt|secular):\/\//, '');
        try {
          const toml = atob(b64);
          const config = parseTomlConfig(toml);
          console.log('[DEEPLINK] decoded config:', config);
          handleImportConfig(config);
          // Show the window if it was hidden
          invoke('show_window').catch(() => {});
        } catch (e) {
          console.error('[DEEPLINK] failed to decode:', e);
        }
      }
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);

  const handleAddServer = (config: ServerConfig) => {
    const newServer: ServerInfo = {
      id: Date.now().toString(),
      name: config.hostname || config.address || 'New Server',
      meta: `${config.address} / ${config.upstream_protocol === 'http3' ? 'QUIC' : 'HTTP/2'}`,
      isDefault: servers.length === 0,
      config,
    };
    setServers(prev => [...prev, newServer]);
    addLog('info', `Server "${newServer.name}" added`);
  };

  const handleSaveServer = (updated: ServerInfo) => {
    const withMeta = {
      ...updated,
      meta: `${updated.config.address} / ${updated.config.upstream_protocol === 'http3' ? 'QUIC' : 'HTTP/2'}`,
    };
    if (isNewServer) {
      // Adding new server from config screen
      setServers(prev => [...prev, withMeta]);
      addLog('info', `Server "${withMeta.name}" added`);
    } else {
      setServers(prev => prev.map(s => s.id === updated.id ? withMeta : s));
      addLog('info', `Server "${updated.name}" updated`);
    }
    setEditingServer(null);
    setIsNewServer(false);
  };

  const handleDeleteServer = (id: string) => {
    setServers(prev => prev.filter(s => s.id !== id));
    addLog('warn', 'Server deleted');
    setEditingServer(null);
    setIsNewServer(false);
  };

  const handleSetDefault = (id: string) => {
    setServers(prev => prev.map(s => ({ ...s, isDefault: s.id === id })));
    addLog('info', 'Default server changed');
  };

  const handleEditServer = (id: string) => {
    const srv = servers.find(s => s.id === id);
    if (srv) {
      setEditingServer(srv);
      setIsNewServer(false);
      setScreen('server-config');
    }
  };

  const handleEditNewServer = () => {
    setEditingServer({
      id: Date.now().toString(),
      name: '',
      meta: '',
      isDefault: servers.length === 0,
      config: {
        name: '',
        address: '',
        hostname: '',
        username: '',
        password: '',
        upstream_protocol: 'http2',
        dns_upstreams: ['9.9.9.9', '149.112.112.112'],
        has_ipv6: false,
        certificate: '',
        skip_verification: false,
        anti_dpi: false,
        change_system_dns: true,
      },
    });
    setIsNewServer(true);
    setScreen('server-config');
  };

  const handleImportConfig = (config: ServerConfig) => {
    setEditingServer({
      id: Date.now().toString(),
      name: config.name || config.hostname || config.address || 'Imported Server',
      meta: `${config.address} / ${config.upstream_protocol === 'http3' ? 'QUIC' : 'HTTP/2'}`,
      isDefault: servers.length === 0,
      config,
    });
    setIsNewServer(true);
    setScreen('server-config');
  };

  const handleClearLogs = () => {
    setLogs([]);
  };

  const handleNav = (s: Screen) => {
    setScreen(s);
  };

  // ─── Tray menu popup window ───
  // The popup is a separate webview window loading index.html?tray-menu.
  // We detect this via the URL query param and render only the tray menu.
  const isTrayMenu =
    typeof window !== "undefined" && window.location.search.includes("tray-menu");

  const [trayStats, setTrayStats] = useState({
    connected: false,
    connecting: false,
    server: "",
    sessionTime: "00:00:00",
    downloadPkts: 0,
    uploadPkts: 0,
  });

  // Listen for live stats from Rust (works in both main and popup windows)
  useEffect(() => {
    const unlisten = listen<TrayStatePayload>("tray-state-update", (event) => {
      const p = event.payload;
      setTrayStats({
        connected: p.connected,
        connecting: p.connecting,
        server: p.server,
        sessionTime: p.session_time || "00:00:00",
        downloadPkts: p.download_pkts || 0,
        uploadPkts: p.upload_pkts || 0,
      });
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Tray menu actions — sent to Rust backend
  const trayInvoke = (action: string, screen?: string) => {
    invoke("tray-action", { action, screen }).catch(() => {});
  };

  // ─── Tray Menu Render (popup window) ───
  if (isTrayMenu) {
    return (
      <div
        className="tray-menu-overlay"
        onClick={() => trayInvoke("close")}
      >
        <div
          className="tray-menu-popup"
          onClick={(e) => e.stopPropagation()}
        >
          <div
            className={`tray-menu-status ${trayStats.connected ? "connected" : trayStats.connecting ? "connecting" : "disconnected"}`}
          >
            <div className="tray-menu-status-dot" />
            <div className="tray-menu-status-text">
              {trayStats.connected
                ? `Connected to ${trayStats.server}`
                : trayStats.connecting
                  ? `Connecting to ${trayStats.server}...`
                  : "Disconnected"}
            </div>
          </div>

          {trayStats.connected && (
            <div className="tray-menu-stats">
              <div className="tray-menu-stat">
                <span className="tray-menu-stat-value">{trayStats.sessionTime}</span>
                <span className="tray-menu-stat-label">SESSION</span>
              </div>
              <div className="tray-menu-stat">
                <span className="tray-menu-stat-value">{trayStats.downloadPkts}</span>
                <span className="tray-menu-stat-label">↓ PKTS</span>
              </div>
              <div className="tray-menu-stat">
                <span className="tray-menu-stat-value">{trayStats.uploadPkts}</span>
                <span className="tray-menu-stat-label">↑ PKTS</span>
              </div>
            </div>
          )}

          <div className="tray-menu-sep" />
          <div className="tray-menu-item" onClick={() => trayInvoke("connect")}>
            {trayStats.connected ? "Disconnect" : trayStats.connecting ? "Cancel" : "Connect"}
          </div>
          <div className="tray-menu-sep" />
          <div className="tray-menu-item" onClick={() => trayInvoke("nav", "dashboard")}>
            Dashboard
          </div>
          <div className="tray-menu-item" onClick={() => trayInvoke("nav", "server-list")}>
            My Servers
          </div>
          <div className="tray-menu-item" onClick={() => trayInvoke("nav", "add-server")}>
            Add Server
          </div>
          <div className="tray-menu-item" onClick={() => trayInvoke("nav", "query-log")}>
            Query Log
          </div>
          <div className="tray-menu-sep" />
          <div className="tray-menu-item" onClick={() => trayInvoke("show")}>
            Show Secular
          </div>
          <div className="tray-menu-item" onClick={() => trayInvoke("hide")}>
            Hide Secular
          </div>
          <div className="tray-menu-sep" />
          <div className="tray-menu-item tray-menu-quit" onClick={() => trayInvoke("quit")}>
            Quit Secular
          </div>
        </div>
      </div>
    );
  }

  // ─── Main App Render ───
  return (
    <>
      {screen === "dashboard" && (
        <Dashboard
          connState={connState}
          onToggleConnect={handleToggleConnect}
          onNav={handleNav}
          servers={servers}
          onSetDefault={handleSetDefault}
          onEditServer={handleEditServer}
          sessionTime={sessionTime}
          downloadPkts={downloadPkts}
          uploadPkts={uploadPkts}
        />
      )}
      {screen === 'server-list' && (
        <ServerList
          servers={servers}
          onNav={handleNav}
          onSetDefault={handleSetDefault}
          onEditServer={handleEditServer}
        />
      )}
      {screen === 'add-server' && (
        <AddServer
          onNav={handleNav}
          onAddServer={handleAddServer}
          onEditNewServer={handleEditNewServer}
          onImportConfig={handleImportConfig}
        />
      )}
      {screen === 'server-config' && editingServer && (
        <ServerConfigScreen
          server={editingServer}
          isNew={isNewServer}
          onSave={handleSaveServer}
          onDelete={handleDeleteServer}
          onNav={handleNav}
        />
      )}
      {screen === 'query-log' && (
        <QueryLog
          logs={logs}
          onNav={handleNav}
          onClear={handleClearLogs}
        />
      )}
    </>
  );
};

export default App;
