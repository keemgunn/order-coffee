# Order Coffee ☕

A simple HTTP server for preventing system sleep on Pop!_OS/Ubuntu during remote SSH sessions.

## Overview

Order Coffee is a lightweight Rust-based HTTP server that provides API endpoints to control system sleep behavior. It's designed to solve the common problem where Pop!_OS/Ubuntu systems go to sleep even during active SSH sessions, since SSH activity is not considered "non-idle" by default.

## Features

- 🚀 **Simple HTTP API** - Control sleep prevention with simple POST requests
- ⚡ **Lightweight** - Built in Rust for minimal resource usage
- 🔧 **Systemd Integration** - Automatic startup on boot
- 📊 **Status Monitoring** - Check server status and uptime
- 🛡️ **Graceful Shutdown** - Proper cleanup of sleep inhibitors
- 📝 **Comprehensive Logging** - Detailed logs for troubleshooting
- 🔒 **Security Focused** - Minimal privileges and secure defaults

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
# Start the server manually
order-coffee --port 20553

# Prevent system sleep
curl -X POST http://192.168.0.200:20553/coffee

# Allow system sleep
curl -X POST http://192.168.0.200:20553/chill

# Check status
curl http://192.168.0.200:20553/status

# Health check
curl http://192.168.0.200:20553/health
```

### Command Line Options

```bash
order-coffee --help
```

```
A simple HTTP server to prevent system sleep on Pop!_OS/Ubuntu

Usage: order-coffee [OPTIONS]

Options:
  -p, --port <PORT>    Port to bind the server to [default: 20553]
      --host <HOST>    Host address to bind to [default: 0.0.0.0]
  -v, --verbose        Enable verbose logging
  -h, --help           Print help
  -V, --version        Print version
```

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST   | `/coffee` | Enable sleep prevention |
| POST   | `/chill`  | Disable sleep prevention |
| GET    | `/status` | Get server status and state |
| GET    | `/health` | Health check endpoint |

### Response Examples

**POST /coffee:**
```json
{
  "status": "active",
  "message": "Sleep prevention enabled",
  "timestamp": "2025-01-08T12:53:45Z"
}
```

**GET /status:**
```json
{
  "status": "active",
  "uptime": "2h 15m 30s",
  "port": 20553,
  "host": "0.0.0.0",
  "inhibitor_active": true,
  "last_action": "coffee",
  "last_action_time": "2025-01-08T12:53:45Z"
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
