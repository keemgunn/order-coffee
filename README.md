# Order Coffee ☕

A state-managed HTTP server for intelligent system suspension control on Pop!_OS/Ubuntu.

## Overview

Order Coffee is a lightweight Rust-based HTTP server that provides API endpoints to control system suspension behavior with advanced state management. Version 2.0 introduces multi-state tracking, automatic suspension timers, and ollama service integration to ensure proper service lifecycle management before system suspension.

## Features

- 🚀 **Multi-State Management** - Track multiple states (coffee, ollama) that can prevent suspension
- ⏰ **Automatic Suspension Timer** - Configurable countdown when all states are inactive
- 🔧 **Ollama Service Integration** - Automatic start/stop of ollama.service with state changes
- 🛠️ **Service Recovery** - Escalating recovery attempts for failed ollama service operations
- 📊 **Enhanced Status Monitoring** - Real-time state tracking with timer information
- 🚨 **Error Tracking** - Visible error states for client monitoring
- ⚡ **Lightweight** - Built in Rust for minimal resource usage
- 🔧 **Systemd Integration** - Automatic startup on boot with proper service management
- 🛡️ **Graceful Shutdown** - Proper cleanup and state management
- 📝 **Comprehensive Logging** - Detailed logs with extensive comments for learning
- 🔒 **Security Focused** - Root privileges for system control with minimal attack surface

## Quick Start

### Prerequisites

- Pop!_OS, Ubuntu, or any systemd-based Linux distribution
- Rust toolchain (for building from source)
- `systemd-inhibit` command available

### Installation

1. **Clone the repository:**
   ```bash
   git clone <repository-url>
   cd order-coffee
   ```

2. **Build and install:**
   ```bash
   chmod +x install.sh
   sudo ./install.sh
   ```

3. **Verify installation:**
   ```bash
   curl http://localhost:20553/health
   ```

### Manual Installation

If you prefer to install manually:

1. **Build the project:**
   ```bash
   cargo build --release
   ```

2. **Copy binary:**
   ```bash
   sudo cp target/release/order-coffee /usr/local/bin/
   sudo chmod +x /usr/local/bin/order-coffee
   ```

3. **Install systemd service:**
   ```bash
   sudo cp order-coffee.service /etc/systemd/system/
   sudo systemctl daemon-reload
   sudo systemctl enable order-coffee.service
   sudo systemctl start order-coffee.service
   ```

## Usage

### Basic Commands

```bash
# Start the server manually with custom timer (5 minutes)
order-coffee --port 20553 --timer 5

# Enable coffee state (prevents suspension)
curl -X POST http://192.168.0.200:20553/coffee

# Disable coffee state
curl -X POST http://192.168.0.200:20553/chill

# Enable ollama state and start ollama.service
curl -X POST http://192.168.0.200:20553/ollama-on

# Disable ollama state and stop ollama.service
curl -X POST http://192.168.0.200:20553/ollama-off

# Check current states and timer status
curl http://192.168.0.200:20553/status

# Health check
curl http://192.168.0.200:20553/health
```

### New v2.0 Workflow

The upgraded server uses **state-based suspension control**:

1. **Multiple States**: The system tracks multiple independent states (`coffee`, `ollama`)
2. **Suspension Logic**: System stays awake if ANY state is `true`
3. **Automatic Timer**: When ALL states become `false`, a configurable timer starts
4. **Smart Suspension**: After timer expires, system suspends automatically
5. **Service Management**: Ollama service is properly stopped before suspension

**Example Workflow:**
```bash
# 1. Start ollama for AI work
curl -X POST http://localhost:20553/ollama-on
# → ollama.service starts, system stays awake

# 2. Also enable coffee state for other work
curl -X POST http://localhost:20553/coffee
# → Both states active, system stays awake

# 3. Finish AI work, disable ollama
curl -X POST http://localhost:20553/ollama-off
# → ollama.service stops, but coffee state keeps system awake

# 4. Finish other work, disable coffee
curl -X POST http://localhost:20553/chill
# → All states inactive, 10-minute timer starts

# 5. System automatically suspends after timer expires
# → ollama.service was already stopped safely
```

### Command Line Options

```bash
order-coffee --help
```

```
A state-managed HTTP server to control system suspension

Usage: order-coffee [OPTIONS]

Options:
  -p, --port <PORT>    Port to bind the server to [default: 20553]
      --host <HOST>    Host address to bind to [default: 0.0.0.0]
  -t, --timer <TIMER>  Suspension timer duration in minutes [default: 10]
  -v, --verbose        Enable verbose logging
  -h, --help           Print help
  -V, --version        Print version
```

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST   | `/coffee` | Enable coffee state (prevents suspension) |
| POST   | `/chill`  | Disable coffee state |
| POST   | `/ollama-on` | Enable ollama state and start ollama.service |
| POST   | `/ollama-off` | Disable ollama state and stop ollama.service |
| GET    | `/status` | Get current system states and timer status |
| GET    | `/health` | Health check endpoint |

### Response Examples

**POST /coffee:**
```json
{
  "status": "active",
  "message": "Coffee state enabled",
  "timestamp": "2025-07-24T12:42:00Z",
  "states": {
    "coffee": true,
    "ollama": false,
    "errors": []
  }
}
```

**POST /ollama-on:**
```json
{
  "status": "active",
  "message": "Ollama state enabled and service started",
  "timestamp": "2025-07-24T12:42:00Z",
  "states": {
    "coffee": false,
    "ollama": true,
    "errors": []
  }
}
```

**GET /status:**
```json
{
  "states": {
    "coffee": true,
    "ollama": false,
    "errors": []
  },
  "timer_active": false,
  "timer_remaining_seconds": null,
  "uptime": "2h 15m 30s",
  "port": 20553,
  "host": "0.0.0.0",
  "last_action": "coffee",
  "last_action_time": "2025-07-24T12:42:00Z"
}
```

**GET /status (with timer active):**
```json
{
  "states": {
    "coffee": false,
    "ollama": false,
    "errors": []
  },
  "timer_active": true,
  "timer_remaining_seconds": 480,
  "uptime": "2h 15m 30s",
  "port": 20553,
  "host": "0.0.0.0",
  "last_action": "chill",
  "last_action_time": "2025-07-24T12:42:00Z"
}
```

## Systemd Service Management

```bash
# Check service status
sudo systemctl status order-coffee.service

# Start/stop/restart service
sudo systemctl start order-coffee.service
sudo systemctl stop order-coffee.service
sudo systemctl restart order-coffee.service

# Enable/disable auto-start
sudo systemctl enable order-coffee.service
sudo systemctl disable order-coffee.service

# View logs
sudo journalctl -u order-coffee.service -f
```

## Configuration

### Environment Variables

The service can be configured using environment variables in the systemd service file:

```ini
[Service]
Environment=RUST_LOG=info
Environment=ORDER_COFFEE_PORT=20553
Environment=ORDER_COFFEE_HOST=0.0.0.0
```

### Firewall Configuration

If you have a firewall enabled, you may need to allow access to the port:

```bash
# UFW (Ubuntu Firewall)
sudo ufw allow 20553

# iptables
sudo iptables -A INPUT -p tcp --dport 20553 -j ACCEPT
```

## SSH Integration Examples

### Basic SSH Session

```bash
# Before connecting via SSH
curl -X POST http://192.168.0.200:20553/coffee

# Connect via SSH
ssh user@192.168.0.200

# After SSH session
curl -X POST http://192.168.0.200:20553/chill
```

### Automated SSH Wrapper

Add to your `~/.bashrc` or `~/.zshrc`:

```bash
ssh_work() {
    curl -X POST http://192.168.0.200:20553/coffee
    ssh "$@"
    curl -X POST http://192.168.0.200:20553/chill
}
```

Usage: `ssh_work user@hostname`

## Troubleshooting

### Common Issues

1. **Server not starting:**
   - Check if `systemd-inhibit` is available: `which systemd-inhibit`
   - Check service logs: `sudo journalctl -u order-coffee.service`

2. **Permission denied:**
   - Ensure the binary has execute permissions
   - Check if the service user has necessary permissions

3. **Port already in use:**
   - Check what's using the port: `sudo ss -tlnp | grep 20553`
   - Change the port in the service file or command line

4. **Sleep prevention not working:**
   - Verify inhibitor is active: `systemd-inhibit --list`
   - Check system power settings
   - Review systemd-logind configuration

### Debug Mode

Run the server with verbose logging:

```bash
order-coffee --verbose
```

Or set the log level in the systemd service:

```ini
Environment=RUST_LOG=debug
```

### Testing systemd-inhibit

Test the underlying mechanism manually:

```bash
# This should prevent sleep for 60 seconds
systemd-inhibit --what=sleep:idle --who=test --why="Testing" sleep 60
```

## Development

### Building from Source

```bash
# Clone repository
git clone <repository-url>
cd order-coffee

# Build debug version
cargo build

# Build release version
cargo build --release

# Run tests
cargo test

# Run with cargo
cargo run -- --port 8080 --verbose
```

### Project Structure

```
order-coffee/
├── Cargo.toml              # Rust project configuration
├── src/
│   └── main.rs            # Main server implementation
├── order-coffee.service   # Systemd service file
├── install.sh            # Installation script
├── README.md             # This file
└── USAGE.md              # Detailed usage examples
```

## Security Considerations

- The server runs as root (required for `systemd-inhibit` access)
- Limited file system access (only `/tmp` is writable)
- Network access is limited to the configured port
- All input is validated and sanitized
- Consider firewall rules to restrict network access if needed

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Inspired by the `caffeine` utility
- Built with the excellent Rust ecosystem
- Thanks to the systemd project for power management tools

## Support

For detailed usage examples and advanced configurations, see [USAGE.md](USAGE.md).

For issues and questions:
1. Check the troubleshooting section above
2. Review the logs: `sudo journalctl -u order-coffee.service`
3. Open an issue on the project repository

---

**Happy coding! ☕**
