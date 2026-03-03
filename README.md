Status Upstream
====

A client/server monitoring system that actively probes services and reports status to a central server. The server stores check history in SQLite, serves a status API, and pushes notifications to external platforms.

## Architecture

```
Server                              Client (remote or local subprocess)
┌─────────────────────┐             ┌───────────────────────┐
│ Axum REST API       │◄── HTTP ────│ Check Runner          │
│  POST /api/v1/report│             │  - Command checks     │
│  GET  /api/v1/...   │             │  - TCP port checks    │
│                     │             │  - HTTP health checks  │
│ SQLite (pool)       │             │  - SSH banner checks   │
│ Notifier Registry   │             │  - TeamSpeak UDP probe │
│  - Statuspage.io    │             │  - ICMP ping (opt)     │
│  - Telegram         │             │  - Subnet scan         │
│ Scheduler           │             └───────────────────────┘
│                     │──spawns──► local client subprocess
└─────────────────────┘
```

Single binary with two modes:

- **`status-upstream server`** — Central server: receives reports, stores history, serves status API, triggers notifications, optionally spawns a local client for built-in probes.
- **`status-upstream client`** — Agent: runs configured checks and reports results to the server.

## Usage

```bash
# Start the server
status-upstream server --config config/server.toml

# Start a remote client agent
status-upstream client --config config/client.toml

# Run checks once and exit (used by server for local probes)
status-upstream client --config config/client.toml --once
```

### Environment

Control log level via `RUST_LOG`:

```bash
RUST_LOG=debug status-upstream server --config config/server.toml
```

## Configuration

Copy the default templates and edit them:

```bash
cp config/default.toml.default config/server.toml
cp config/client.toml.default  config/client.toml
```

### Server (`config/server.toml`)

```toml
[server]
bind = "127.0.0.1"
port = 41132
database = "status-upstream.db"
auth_token = "your-secret-token"
check_interval = 60           # seconds between local check runs
public_status_page = false    # allow unauthenticated GET on status endpoints

# Define monitored components
[[components]]
id = "web-prod"
name = "Production Web Server"

[[components]]
id = "db-primary"
name = "Primary Database"

# Local checks (server spawns client subprocess)
[[local_checks]]
component_id = "web-prod"
type = "http"
url = "https://example.com/health"
expected_status = 200

[[local_checks]]
component_id = "db-primary"
type = "tcp"
host = "10.0.0.5"
port = 5432

# Notifiers
[notifiers.statuspage]
enabled = true
api_key = "OAuth your-api-key"
[notifiers.statuspage.components]
"web-prod" = { page_id = "abc123", component_id = "def456" }

[notifiers.telegram]
enabled = true
bot_token = "123456:ABC-DEF"
chat_id = "-1001234567890"
```

### Client (`config/client.toml`)

```toml
[client]
server_url = "http://10.0.0.1:41132"
auth_token = "your-secret-token"
client_id = "datacenter-east"
check_interval = 30

[[checks]]
component_id = "web-prod"
type = "http"
url = "http://localhost:8080/health"
expected_status = 200

[[checks]]
component_id = "app-worker"
type = "command"
command = "systemctl is-active myapp"
timeout = 10

[[checks]]
component_id = "mail-server"
type = "tcp"
host = "mail.internal"
port = 25

[[checks]]
component_id = "git-server"
type = "ssh"
host = "git.internal"
port = 22

[[checks]]
component_id = "voice-server"
type = "teamspeak"
host = "ts.example.com"
port = 9987

[[checks]]
component_id = "office-subnet"
type = "subnet"
network = "192.168.1.0/24"
port = 22
```

## Check Types

| Type | Description | Platform |
|------|-------------|----------|
| `command` | Run a shell command; exit code 0 = operational | All (sh/cmd) |
| `tcp` | TCP connect to host:port | All |
| `http` | HTTP GET, verify status code | All |
| `ssh` | TCP connect + verify SSH protocol banner | All |
| `teamspeak` | UDP TS3INIT probe | All |
| `subnet` | TCP port check across all hosts in a CIDR | All |
| `ping` | ICMP echo (requires `ping` feature + root) | Linux/macOS |

### Aggregate Status

When a component has multiple checks, aggregate status is derived from the ratio of passing checks:

- **All pass** → `operational`
- **None pass** → `major_outage`
- **≥ 2/3 pass** → `degraded_performance`
- **< 2/3 pass** → `partial_outage`

## API Endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/` | No | Version info |
| `GET` | `/api/v1/health` | No | Server health check |
| `POST` | `/api/v1/report` | Bearer | Submit batch check report |
| `GET` | `/api/v1/components` | Optional | List all components |
| `GET` | `/api/v1/components/{id}` | Optional | Get component status |
| `GET` | `/api/v1/components/{id}/history` | Optional | Check history (`?limit=&since=`) |

Authentication uses `Authorization: Bearer <token>` header. GET endpoints require auth unless `public_status_page = true`.

## Building

```bash
cargo build --release

# With ICMP ping support (Linux/macOS only)
cargo build --release --features ping

# Optimized with LTO
cargo build --profile release-lto
```

## Notifiers

### Statuspage.io

Pushes component status changes to [Atlassian Statuspage](https://www.statuspage.io/) via their REST API. Requires an API key and per-component mapping of internal IDs to Statuspage page/component IDs.

### Telegram

Sends status change notifications to a Telegram chat via the Bot API. Messages include the component name, old status, and new status.

## License

[![](https://www.gnu.org/graphics/agplv3-155x51.png)](https://www.gnu.org/licenses/agpl-3.0.txt)

Copyright (C) 2022-2026 KunoiSayami

This program is free software: you can redistribute it and/or modify it under the terms of the
GNU Affero General Public License as published by the Free Software Foundation,
either version 3 of the License, or any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
See the GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License along with this program.
If not, see <https://www.gnu.org/licenses/>.
