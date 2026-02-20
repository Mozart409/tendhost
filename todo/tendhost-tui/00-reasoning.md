# tendhost-tui: Reasoning & Design

## Purpose

The `tendhost-tui` crate provides a terminal user interface for monitoring and controlling the tendhost daemon in real-time. It offers:
- Live dashboard showing all hosts and their states
- Real-time updates via WebSocket events
- Interactive host management (trigger updates, reboots, etc.)
- Visual feedback for operations in progress

## Design Goals

1. **Real-time updates**: Live dashboard via WebSocket events from daemon
2. **Intuitive navigation**: Vim-style keybindings (j/k/g/G) + arrow keys
3. **Visual clarity**: Color-coded states, progress indicators, status bars
4. **Responsive**: Async architecture that never blocks UI rendering
5. **Error resilient**: Handle connection loss gracefully, auto-reconnect
6. **Keyboard-driven**: Full functionality without mouse

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                          tendhost-tui                               │
│                                                                     │
│  ┌─────────────────┐    ┌─────────────────┐    ┌────────────────┐  │
│  │   UI Thread     │◄──►│   App State     │◄──►│  Event Loop    │  │
│  │  (rendering)    │    │   (model)       │    │  (crossterm)   │  │
│  └────────┬────────┘    └────────┬────────┘    └────────────────┘  │
│           │                      │                                  │
│           │                      │                                  │
│           ▼                      ▼                                  │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                      WebSocket Task                          │   │
│  │  • Receives WsEvent from daemon                              │   │
│  │  • Sends updates to App State via mpsc channel               │   │
│  │  • Auto-reconnects on disconnect                             │   │
│  └─────────────────────────────────────────────────────────────┘   │
└───────────────────────────────────┬─────────────────────────────────┘
                                    │
                                    ▼
                    ┌───────────────────────────────┐
                    │       tendhost daemon         │
                    │  REST API + WebSocket /ws     │
                    └───────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility |
|-----------|---------------|
| `App` | Application state, model, action handling |
| `UI` | Widget rendering, layout management |
| `EventLoop` | Keyboard/terminal events, tick timer |
| `WsTask` | WebSocket connection, event forwarding |

## UI Layout

```
┌──────────────────────────────── tendhost ────────────────────────────────┐
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────────┐ │
│  │  Host           │ State      │ OS             │ Packages │ Updated  │ │
│  ├─────────────────┼────────────┼────────────────┼──────────┼──────────┤ │
│  │▸ debian-vm      │ ● Idle     │ Debian 12      │ 42       │ 2h ago   │ │
│  │  centos-docker  │ ◐ Updating │ CentOS 9       │ 18       │ 1d ago   │ │
│  │  fedora-ct      │ ● Idle     │ Fedora 39      │ 5        │ 30m ago  │ │
│  │  proxmox-1      │ ○ Offline  │ Proxmox 8      │ --       │ --       │ │
│  │  ubuntu-web     │ ● Idle     │ Ubuntu 24.04   │ 0        │ 5m ago   │ │
│  │                 │            │                │          │          │ │
│  │                 │            │                │          │          │ │
│  └─────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ┌──────────── Host Details: debian-vm ────────────────────────────────┐ │
│  │                                                                     │ │
│  │  State: Idle          OS: Debian GNU/Linux 12 (bookworm)            │ │
│  │  CPU: 4 cores         Memory: 8GB (2.1GB used)                      │ │
│  │  Uptime: 15d 4h       Last Update: 2026-02-20 18:30:00              │ │
│  │                                                                     │ │
│  │  Upgradable Packages: 42                                            │ │
│  │  ├── linux-image-amd64 (6.1.0-18 → 6.1.0-19)                        │ │
│  │  ├── openssh-server (9.2p1-2 → 9.2p1-3)                             │ │
│  │  └── ... (40 more)                                                  │ │
│  │                                                                     │ │
│  └─────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ┌───────────────────────── Event Log ─────────────────────────────────┐ │
│  │ 18:35:02 centos-docker: Updating → Idle (completed)                 │ │
│  │ 18:34:45 centos-docker: Installing package 18/18: vim               │ │
│  │ 18:34:30 centos-docker: Installing package 17/18: curl              │ │
│  │ 18:30:00 debian-vm: Querying inventory                              │ │
│  └─────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
├──────────────────────────────────────────────────────────────────────────┤
│  [j/k] Navigate  [Enter] Details  [u] Update  [r] Reboot  [q] Quit      │
└──────────────────────────────────────────────────────────────────────────┘
```

## State Colors

| State | Symbol | Color |
|-------|--------|-------|
| `Idle` | ● | Green |
| `Querying` | ◐ | Yellow |
| `PendingUpdates` | ● | Yellow |
| `Updating` | ◐ | Blue (animated) |
| `WaitingReboot` | ◎ | Cyan |
| `Rebooting` | ◐ | Magenta |
| `Verifying` | ◐ | Cyan |
| `Failed` | ✗ | Red |
| `Offline` | ○ | Gray |

## Keybindings

### Navigation
| Key | Action |
|-----|--------|
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `g` | Jump to first host |
| `G` | Jump to last host |
| `Tab` | Switch panel focus |
| `/` | Search hosts |
| `Esc` | Clear search / close popup |

### Actions
| Key | Action |
|-----|--------|
| `Enter` | Show host details |
| `u` | Trigger update on selected host |
| `U` | Trigger fleet update |
| `r` | Reboot selected host |
| `R` | Retry failed host |
| `a` | Acknowledge failure |
| `i` | Refresh inventory |

### Global
| Key | Action |
|-----|--------|
| `q` | Quit application |
| `?` | Show help popup |
| `c` | Toggle connection status |

## Data Flow

### Startup Sequence

```
1. Parse CLI args (--server, --config)
2. Initialize terminal (crossterm raw mode)
3. Load initial host list via HTTP GET /hosts
4. Connect WebSocket to /ws/events
5. Start event loop (tick + keyboard)
6. Render initial frame
```

### Event Processing

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Keyboard   │     │   WebSocket  │     │  Tick Timer  │
│   Events     │     │   Events     │     │  (250ms)     │
└──────┬───────┘     └──────┬───────┘     └──────┬───────┘
       │                    │                    │
       ▼                    ▼                    ▼
  ┌────────────────────────────────────────────────────┐
  │                    Event Loop                       │
  │                                                    │
  │  tokio::select! {                                  │
  │      Some(key) = keyboard_rx.recv() => handle_key, │
  │      Some(ws) = ws_rx.recv() => handle_ws_event,   │
  │      _ = tick_interval.tick() => handle_tick,      │
  │  }                                                 │
  └────────────────────────────────────────────────────┘
                           │
                           ▼
                    ┌──────────────┐
                    │  App::update │
                    │  (mutate)    │
                    └──────┬───────┘
                           │
                           ▼
                    ┌──────────────┐
                    │  UI::render  │
                    │  (draw)      │
                    └──────────────┘
```

## Error Handling

### Connection Loss
- Show "Disconnected" indicator in status bar
- Auto-reconnect with exponential backoff (1s, 2s, 4s, ... 60s max)
- Queue user actions and replay after reconnect
- Show reconnection countdown

### HTTP Errors
- Display error toast notification
- Log to event panel
- Allow retry via keybinding

### Render Errors
- Graceful degradation (show "Error" in cell)
- Log full error to debug file

## Dependencies

### Required (already in Cargo.toml)
- `ratatui` - TUI framework
- `tokio` - Async runtime
- `tokio-tungstenite` - WebSocket (via tendhost-client)
- `serde_json` - JSON parsing
- `tracing` - Logging
- `color-eyre` - Error handling
- `tendhost-api` - Shared types
- `tendhost-client` - HTTP + WebSocket client

### Additional Needed
- `crossterm` - Terminal backend, keyboard events
- `clap` - CLI argument parsing
- `chrono` - Timestamp formatting
- `unicode-width` - Text truncation

## File Structure

```
tendhost-tui/
├── Cargo.toml
└── src/
    ├── main.rs          # Entry point, CLI args, terminal init
    ├── app.rs           # App state and update logic
    ├── ui/
    │   ├── mod.rs       # UI module exports
    │   ├── layout.rs    # Layout calculations
    │   ├── hosts.rs     # Host list widget
    │   ├── details.rs   # Host details panel
    │   ├── events.rs    # Event log panel
    │   ├── help.rs      # Help popup
    │   └── statusbar.rs # Bottom status bar
    ├── event.rs         # Event handling (keyboard, WS, tick)
    ├── action.rs        # User action types
    └── config.rs        # TUI configuration
```

## Implementation Phases

1. **Foundation** (2 hours)
   - Terminal init/restore with crossterm
   - Basic App struct with quit handling
   - Main event loop skeleton

2. **Core UI** (3 hours)
   - Layout system (header, main, details, events, status)
   - Host list table widget
   - Basic styling and colors

3. **WebSocket Integration** (2 hours)
   - Connect via tendhost-client
   - Event forwarding to App
   - State updates from events

4. **Navigation & Selection** (1.5 hours)
   - Keyboard navigation (j/k/g/G)
   - Host selection state
   - Focus management

5. **Host Details Panel** (1.5 hours)
   - Detail view for selected host
   - Inventory display
   - Package list

6. **Actions** (1.5 hours)
   - Update trigger (u)
   - Reboot trigger (r)
   - Retry/acknowledge (R/a)
   - HTTP calls via client

7. **Polish** (2 hours)
   - Help popup (?)
   - Search functionality (/)
   - Animation/spinners
   - Error toasts

**Total estimated time**: ~12-14 hours

## Future Enhancements

- [ ] Mouse support
- [ ] Themes/color schemes
- [ ] Configuration file support
- [ ] Host grouping view
- [ ] Fleet update wizard
- [ ] SSH console integration
- [ ] Export to JSON/CSV
