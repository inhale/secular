// Secular Desktop — Dark Theme v3 (matches Android)
import React, { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, emit } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-shell';

/* ─── Types ─── */
type ConnState = 'disconnected' | 'connecting' | 'connected';
type Screen = 'dashboard' | 'server-list' | 'add-server' | 'server-config' | 'query-log';

interface ServerConfig {
  host: string;
  port: number;
  sni: string;
  auth_token: string;
  protocol: string;
  allow_ipv6: boolean;
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
  <svg viewBox="0 0 24 24">
    <circle cx="12" cy="12" r="3" />
    <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
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
}

const Dashboard: React.FC<DashboardProps> = ({ connState, onToggleConnect, onNav, servers, onSetDefault, onEditServer }) => {
  const isActive = connState === 'connecting' || connState === 'connected';

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
            <span className="metric-value">0</span>
            <span className="metric-label">SESSION</span>
          </div>
          <div className="metric-item">
            <span className="metric-value">0</span>
            <span className="metric-label">DOWNLOAD PKTS</span>
          </div>
          <div className="metric-item">
            <span className="metric-value">0</span>
            <span className="metric-label">UPLOAD PKTS</span>
          </div>
        </div>

        {/* Connect button */}
        <div className="connect-area">
          <div className={`connect-circle ${connState}`} onClick={onToggleConnect}>
            {isActive && <div className="spinner-ring" />}
            <div className="s-logo-container">
              <SLogo color={isActive ? '#00FF66' : '#FFFFFF'} size={64} />
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
                    {srv.config.host}:{srv.config.port} / {srv.config.protocol.toUpperCase()}
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
}

const AddServer: React.FC<AddServerProps> = ({ onNav, onAddServer, onEditNewServer }) => {
  const [link, setLink] = useState('');

  const handleAdd = () => {
    if (!link.trim()) return;
    // Try to parse secular:// link or plain host:port
    const config: ServerConfig = {
      host: link.replace('secular://', '').split(':')[0] || link,
      port: parseInt(link.split(':')[1], 10) || 443,
      sni: '',
      auth_token: '',
      protocol: 'h2',
      allow_ipv6: false,
    };
    onAddServer(config);
    setLink('');
    onNav('dashboard');
  };

  const handleTomlUpload = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'TOML', extensions: ['toml'] }],
      });
      if (!selected) return;
      // Read the file content via invoke
      const content: string = await invoke('read_file', { path: selected });
      // Parse simple TOML: [server] host=... port=... sni=... auth_token=...
      const config: ServerConfig = parseTomlConfig(content);
      onAddServer(config);
      onNav('dashboard');
    } catch (err) {
      console.error('TOML upload failed:', err);
    }
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
              className="link-input"
              value={link}
              onChange={e => setLink(e.target.value)}
              placeholder="secular://server?token=..."
              onKeyDown={e => e.key === 'Enter' && handleAdd()}
            />
            <button className="link-add-btn" onClick={handleAdd}>Add</button>
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

/** Minimal TOML parser for [server] section */
function parseTomlConfig(content: string): ServerConfig {
  const config: ServerConfig = {
    host: '',
    port: 443,
    sni: '',
    auth_token: '',
    protocol: 'h2',
    allow_ipv6: false,
  };
  const lines = content.split('\n');
  let inServer = false;
  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith('[')) {
      inServer = trimmed === '[server]' || trimmed === '[connection]';
      continue;
    }
    if (!inServer) continue;
    const [key, ...rest] = trimmed.split('=');
    const value = rest.join('=').trim().replace(/^"|"$/g, '');
    if (key.trim() === 'host') config.host = value;
    else if (key.trim() === 'port') config.port = parseInt(value, 10) || 443;
    else if (key.trim() === 'sni') config.sni = value;
    else if (key.trim() === 'auth_token' || key.trim() === 'username') config.auth_token = value;
    else if (key.trim() === 'protocol') config.protocol = value || 'h2';
    else if (key.trim() === 'allow_ipv6') config.allow_ipv6 = value === 'true';
  }
  return config;
}

/* ─── Screen: Server Config ─── */
interface ServerConfigScreenProps {
  server: ServerInfo;
  isNew: boolean;
  onSave: (config: ServerInfo) => void;
  onDelete: (id: string) => void;
  onNav: (s: Screen) => void;
}

const ServerConfigScreen: React.FC<ServerConfigScreenProps> = ({ server, isNew, onSave, onDelete, onNav }) => {
  const [name, setName] = useState(server.name);
  const [host, setHost] = useState(server.config.host);
  const [port, setPort] = useState(String(server.config.port));
  const [username, setUsername] = useState(server.config.auth_token);
  const [password, setPassword] = useState('');
  const [sni, setSni] = useState(server.config.sni);
  const [protocol, setProtocol] = useState(server.config.protocol);

  const handleSave = () => {
    onSave({
      ...server,
      name: name || host || 'New Server',
      config: {
        ...server.config,
        host,
        port: parseInt(port, 10) || 443,
        auth_token: username,
        sni,
        protocol,
      },
    });
    onNav('dashboard');
  };

  const handleDelete = () => {
    onDelete(server.id);
    onNav('dashboard');
  };

  return (
    <div className="screen">
      <div className="config-header-bar">
        <div className="config-back" onClick={() => onNav(isNew ? 'add-server' : 'dashboard')}>
          <IconBack />
        </div>
        <h1>{isNew ? 'New Server' : 'Server Config'}</h1>
        <button className="config-save-btn" onClick={handleSave}>
          Save
        </button>
      </div>
      <div className="screen-content config-content">
        <div className="config-field">
          <div className="config-field-label">Name</div>
          <input className="config-field-input" value={name} onChange={e => setName(e.target.value)} placeholder="My Server" />
        </div>
        <div className="config-field">
          <div className="config-field-label">Address</div>
          <input className="config-field-input" value={host} onChange={e => setHost(e.target.value)} placeholder="vpn.example.com" />
        </div>
        <div className="config-field">
          <div className="config-field-label">Port</div>
          <input className="config-field-input" type="number" value={port} onChange={e => setPort(e.target.value)} placeholder="443" />
        </div>
        <div className="config-field">
          <div className="config-field-label">Protocol</div>
          <div className="protocol-toggle">
            <button className={`proto-btn ${protocol === 'h2' ? 'active' : ''}`} onClick={() => setProtocol('h2')}>H2</button>
            <button className={`proto-btn ${protocol === 'quic' ? 'active' : ''}`} onClick={() => setProtocol('quic')}>QUIC</button>
          </div>
        </div>
        <div className="config-field">
          <div className="config-field-label">Username / Token</div>
          <input className="config-field-input" value={username} onChange={e => setUsername(e.target.value)} placeholder="auth token" />
        </div>
        <div className="config-field">
          <div className="config-field-label">Password</div>
          <input className="config-field-input" type="password" value={password} onChange={e => setPassword(e.target.value)} placeholder="password" />
        </div>
        <div className="config-field">
          <div className="config-field-label">SNI</div>
          <input className="config-field-input" value={sni} onChange={e => setSni(e.target.value)} placeholder="SNI hostname" />
        </div>

        {!isNew && (
          <div className="config-delete-row">
            <button className="delete-btn" onClick={handleDelete}>
              Delete Server
            </button>
          </div>
        )}
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

  const filterLabel = filterLevels.size === 4 ? 'All' : filterLevels.size === 0 ? 'None' : `${filterLevels.size}`;

  return (
    <div className="screen">
      <div className="log-header">
        <div className="log-header-left">
          <h1>Query Log</h1>
          <span className="log-filter-badge" onClick={() => setFilterOpen(!filterOpen)}>{filterLabel}</span>
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
    if (raw) return JSON.parse(raw);
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

  const activeServer = servers.find(s => s.isDefault) || servers[0] || null;

  // Persist servers on change
  useEffect(() => {
    saveServers(servers);
  }, [servers]);

  // Notify Rust backend about connection state changes (updates tray menu)
  useEffect(() => {
    emit('tray-state-changed', connState === 'connected' ? 'connected' : 'disconnected');
  }, [connState]);

  // Listen for tray-connect event (user clicked Connect/Disconnect in tray menu)
  useEffect(() => {
    const unlisten = listen('tray-connect', () => {
      handleToggleConnect();
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);

  const addLog = (level: LogLine['level'], message: string) => {
    const now = new Date();
    const time = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}:${String(now.getSeconds()).padStart(2, '0')}`;
    setLogs(prev => [...prev, { time, level, message }]);
  };

  const handleToggleConnect = async () => {
    try {
      if (connState === 'connected') {
        await invoke('disconnect');
        setConnState('disconnected');
        addLog('warn', 'Disconnected from server');
      } else if (connState === 'disconnected') {
        if (!activeServer) {
          addLog('error', 'No server configured');
          return;
        }
        setConnState('connecting');
        addLog('info', `Connecting to ${activeServer.config.host}:${activeServer.config.port}...`);
        try {
          await invoke('connect', { config: activeServer.config });
          setConnState('connected');
          addLog('ok', `Connected to ${activeServer.name}`);
        } catch (err) {
          setConnState('disconnected');
          addLog('error', `Connection failed: ${err}`);
        }
      }
    } catch (err) {
      console.error('Connection error:', err);
      setConnState('disconnected');
      addLog('error', `Error: ${err}`);
    }
  };

  const handleAddServer = (config: ServerConfig) => {
    const newServer: ServerInfo = {
      id: Date.now().toString(),
      name: config.host || 'New Server',
      meta: `${config.host || '?'}:${config.port} / ${config.protocol.toUpperCase()}`,
      isDefault: servers.length === 0,
      config,
    };
    setServers(prev => [...prev, newServer]);
    addLog('info', `Server "${newServer.name}" added`);
  };

  const handleSaveServer = (updated: ServerInfo) => {
    const withMeta = {
      ...updated,
      meta: `${updated.config.host}:${updated.config.port} / ${updated.config.protocol.toUpperCase()}`,
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
        host: '',
        port: 443,
        sni: '',
        auth_token: '',
        protocol: 'h2',
        allow_ipv6: false,
      },
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

  return (
    <>
      {screen === 'dashboard' && (
        <Dashboard
          connState={connState}
          onToggleConnect={handleToggleConnect}
          onNav={handleNav}
          servers={servers}
          onSetDefault={handleSetDefault}
          onEditServer={handleEditServer}
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
