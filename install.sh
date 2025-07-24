#!/bin/bash

# Order Coffee Installation Script
# This script installs the order-coffee server and sets up systemd service

set -e

cd /home/gunn/order-coffee
rm -r ./target

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   print_error "This script must be run as root. Use: sudo ./install.sh"
   exit 1
fi

# Check if we're on a systemd system
if ! command -v systemctl &> /dev/null; then
    print_error "systemctl not found. This script requires systemd."
    exit 1
fi

# Check if systemd-inhibit is available
if ! command -v systemd-inhibit &> /dev/null; then
    print_error "systemd-inhibit not found. Please install systemd package."
    exit 1
fi

print_status "Starting order-coffee installation..."

print_status "Stopping order-coffee.service..."
systemctl stop order-coffee.service

# Build the project
print_status "Building order-coffee..."
if ! cargo build --release; then
    print_error "Failed to build order-coffee"
    exit 1
fi

# Copy binary to system location
print_status "Installing binary to /usr/local/bin..."
cp target/release/order-coffee /usr/local/bin/
chmod +x /usr/local/bin/order-coffee

# Copy systemd service file
print_status "Installing systemd service..."
cp order-coffee.service /etc/systemd/system/

# Reload systemd and enable service
print_status "Enabling systemd service..."
systemctl daemon-reload
systemctl enable order-coffee.service

# Start the service
print_status "Starting order-coffee service..."
systemctl start order-coffee.service

# Check service status
sleep 2
if systemctl is-active --quiet order-coffee.service; then
    print_status "✅ order-coffee service is running successfully!"
else
    print_error "❌ Failed to start order-coffee service"
    print_status "Check service status with: systemctl status order-coffee.service"
    print_status "Check logs with: journalctl -u order-coffee.service -f"
    exit 1
fi

# Show service status
print_status "Service status:"
systemctl status order-coffee.service --no-pager -l

print_status ""
print_status "🎉 Installation completed successfully!"
print_status ""
print_status "The order-coffee server is now running on port 20553"
print_status "You can test it with:"
print_status "  curl -X POST http://localhost:20553/coffee"
print_status "  curl -X POST http://localhost:20553/chill"
print_status "  curl http://localhost:20553/status"
print_status ""
print_status "Service management commands:"
print_status "  sudo systemctl start order-coffee.service    # Start service"
print_status "  sudo systemctl stop order-coffee.service     # Stop service"
print_status "  sudo systemctl restart order-coffee.service  # Restart service"
print_status "  sudo systemctl status order-coffee.service   # Check status"
print_status "  sudo journalctl -u order-coffee.service -f   # View logs"
print_status ""
print_status "To uninstall:"
print_status "  sudo systemctl stop order-coffee.service"
print_status "  sudo systemctl disable order-coffee.service"
print_status "  sudo rm /etc/systemd/system/order-coffee.service"
print_status "  sudo rm /usr/local/bin/order-coffee"
print_status "  sudo systemctl daemon-reload"
