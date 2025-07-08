# Order Coffee - Usage Manual

A comprehensive guide to using the order-coffee server for preventing system sleep on Pop!_OS/Ubuntu.

## Table of Contents

- [Quick Start](#quick-start)
- [API Reference](#api-reference)
- [Advanced Usage](#advanced-usage)
- [Shell Integration](#shell-integration)
- [Automation Examples](#automation-examples)
- [Troubleshooting](#troubleshooting)

## Quick Start

### Starting the Server

```bash
# Start with default settings (port 20553, bind to all interfaces)
./order-coffee

# Start with custom port
./order-coffee --port 8080

# Start with custom host and port
./order-coffee --host 127.0.0.1 --port 8080

# Start with verbose logging
./order-coffee --verbose
```

### Basic Usage Examples

```bash
# Prevent system sleep
curl -X POST http://192.168.0.200:20553/coffee

# Allow system sleep
curl -X POST http://192.168.0.200:20553/chill

# Check current status
curl http://192.168.0.200:20553/status

# Health check
curl http://192.168.0.200:20553/health
```

## API Reference

### POST /coffee - Prevent System Sleep

Activates sleep prevention by starting a systemd-inhibit process.

**Request:**
```bash
curl -X POST http://192.168.0.200:20553/coffee
```

**Response:**
```json
{
  "status": "active",
  "message": "Sleep prevention enabled",
  "timestamp": "2025-01-08T12:53:45Z"
}
```

**Advanced Examples:**
```bash
# With verbose output
curl -v -X POST http://192.168.0.200:20553/coffee

# With custom headers
curl -X POST \
  -H "Content-Type: application/json" \
  -H "User-Agent: MyApp/1.0" \
  http://192.168.0.200:20553/coffee

# With timeout
curl -X POST --max-time 10 http://192.168.0.200:20553/coffee

# Silent mode (no output)
curl -s -X POST http://192.168.0.200:20553/coffee > /dev/null
```

### POST /chill - Allow System Sleep

Deactivates sleep prevention by stopping the systemd-inhibit process.

**Request:**
```bash
curl -X POST http://192.168.0.200:20553/chill
```

**Response:**
```json
{
  "status": "inactive",
  "message": "Sleep prevention disabled",
  "timestamp": "2025-01-08T12:53:50Z"
}
```

**Advanced Examples:**
```bash
# With error handling
curl -f -X POST http://192.168.0.200:20553/chill || echo "Failed to disable sleep prevention"

# Store response in variable
RESPONSE=$(curl -s -X POST http://192.168.0.200:20553/chill)
echo "Server response: $RESPONSE"
```

### GET /status - Check Current Status

Returns detailed information about the server and current sleep prevention state.

**Request:**
```bash
curl http://192.168.0.200:20553/status
```

**Response:**
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

**Advanced Examples:**
```bash
# Pretty print JSON response
curl -s http://192.168.0.200:20553/status | jq

# Extract specific fields
curl -s http://192.168.0.200:20553/status | jq -r '.status'
curl -s http://192.168.0.200:20553/status | jq -r '.uptime'

# Check if inhibitor is active
STATUS=$(curl -s http://192.168.0.200:20553/status | jq -r '.inhibitor_active')
if [ "$STATUS" = "true" ]; then
    echo "Sleep prevention is active"
else
    echo "Sleep prevention is inactive"
fi
```

### GET /health - Health Check

Simple health check endpoint for monitoring.

**Request:**
```bash
curl http://192.168.0.200:20553/health
```

**Response:**
```json
{
  "status": "ok",
  "timestamp": "2025-01-08T12:53:55Z",
  "version": "1.0.0"
}
```

**Advanced Examples:**
```bash
# Health check with exit code
curl -f http://192.168.0.200:20553/health && echo "Server is healthy"

# Monitor health every 30 seconds
watch -n 30 'curl -s http://192.168.0.200:20553/health | jq'

# Health check for monitoring systems
curl -s http://192.168.0.200:20553/health | jq -e '.status == "ok"' > /dev/null
```

## Advanced Usage

### Error Handling

```bash
# Handle connection errors
if ! curl -f -s http://192.168.0.200:20553/health > /dev/null; then
    echo "Server is not reachable"
    exit 1
fi

# Retry logic
for i in {1..3}; do
    if curl -f -X POST http://192.168.0.200:20553/coffee; then
        echo "Successfully enabled sleep prevention"
        break
    else
        echo "Attempt $i failed, retrying..."
        sleep 2
    fi
done

# Check response status
RESPONSE=$(curl -s -w "%{http_code}" -X POST http://192.168.0.200:20553/coffee)
HTTP_CODE="${RESPONSE: -3}"
BODY="${RESPONSE%???}"

if [ "$HTTP_CODE" -eq 200 ]; then
    echo "Success: $BODY"
else
    echo "Error (HTTP $HTTP_CODE): $BODY"
fi
```

### Configuration Testing

```bash
# Test different ports
for port in 20553 8080 3000; do
    echo "Testing port $port..."
    if curl -f -s http://192.168.0.200:$port/health > /dev/null; then
        echo "Server found on port $port"
        break
    fi
done

# Test connectivity
nc -zv 192.168.0.200 20553 && echo "Port is open" || echo "Port is closed"

# Test with different hosts
for host in localhost 127.0.0.1 192.168.0.200; do
    if curl -f -s http://$host:20553/health > /dev/null; then
        echo "Server accessible via $host"
    fi
done
```

## Shell Integration

### Bash/Zsh Aliases

Add these to your `~/.bashrc` or `~/.zshrc`:

```bash
# Basic aliases
alias work-start='curl -X POST http://192.168.0.200:20553/coffee'
alias work-end='curl -X POST http://192.168.0.200:20553/chill'
alias work-status='curl -s http://192.168.0.200:20553/status | jq'

# Advanced aliases with error handling
alias coffee='curl -f -X POST http://192.168.0.200:20553/coffee && echo "☕ Sleep prevention enabled"'
alias chill='curl -f -X POST http://192.168.0.200:20553/chill && echo "😴 Sleep prevention disabled"'
```

### Shell Functions

```bash
# Work session management
work_session() {
    case "$1" in
        start)
            echo "🚀 Starting work session..."
            if curl -f -X POST http://192.168.0.200:20553/coffee; then
                echo "✅ Sleep prevention enabled"
            else
                echo "❌ Failed to enable sleep prevention"
                return 1
            fi
            ;;
        end)
            echo "🏁 Ending work session..."
            if curl -f -X POST http://192.168.0.200:20553/chill; then
                echo "✅ Sleep prevention disabled"
            else
                echo "❌ Failed to disable sleep prevention"
                return 1
            fi
            ;;
        status)
            echo "📊 Current status:"
            curl -s http://192.168.0.200:20553/status | jq
            ;;
        *)
            echo "Usage: work_session {start|end|status}"
            return 1
            ;;
    esac
}

# SSH with automatic sleep prevention
ssh_work() {
    if [ -z "$1" ]; then
        echo "Usage: ssh_work <hostname>"
        return 1
    fi
    
    echo "🔌 Enabling sleep prevention..."
    curl -f -X POST http://192.168.0.200:20553/coffee
    
    echo "🔗 Connecting to $1..."
    ssh "$1"
    
    echo "💤 Disabling sleep prevention..."
    curl -f -X POST http://192.168.0.200:20553/chill
}

# Smart work session (auto-disable after timeout)
timed_work() {
    local duration=${1:-3600}  # Default 1 hour
    
    echo "⏰ Starting timed work session for ${duration}s..."
    curl -f -X POST http://192.168.0.200:20553/coffee
    
    # Schedule automatic disable
    (sleep "$duration" && curl -f -X POST http://192.168.0.200:20553/chill && echo "⏰ Work session timed out") &
    
    echo "✅ Sleep prevention enabled for ${duration} seconds"
    echo "💡 Use 'work_session end' to disable early"
}
```

### Environment Variables

```bash
# Set default server URL
export ORDER_COFFEE_URL="http://192.168.0.200:20553"

# Use in scripts
curl -X POST "$ORDER_COFFEE_URL/coffee"
curl -X POST "$ORDER_COFFEE_URL/chill"
curl "$ORDER_COFFEE_URL/status"
```

## Automation Examples

### Cron Jobs

```bash
# Enable during work hours (9 AM - 6 PM, Monday-Friday)
0 9 * * 1-5 curl -X POST http://192.168.0.200:20553/coffee
0 18 * * 1-5 curl -X POST http://192.168.0.200:20553/chill

# Health check every 5 minutes
*/5 * * * * curl -f http://192.168.0.200:20553/health || echo "Order Coffee server is down" | mail -s "Server Alert" admin@example.com
```

### Systemd Timer (Alternative to Cron)

Create `/etc/systemd/system/work-hours.service`:
```ini
[Unit]
Description=Enable sleep prevention during work hours

[Service]
Type=oneshot
ExecStart=/usr/bin/curl -X POST http://192.168.0.200:20553/coffee
```

Create `/etc/systemd/system/work-hours.timer`:
```ini
[Unit]
Description=Work hours timer

[Timer]
OnCalendar=Mon..Fri 09:00
Persistent=true

[Install]
WantedBy=timers.target
```

Enable: `sudo systemctl enable --now work-hours.timer`

### Monitoring Scripts

```bash
#!/bin/bash
# monitor-coffee.sh - Monitor order-coffee server

SERVER_URL="http://192.168.0.200:20553"
LOG_FILE="/var/log/order-coffee-monitor.log"

log_message() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') - $1" >> "$LOG_FILE"
}

# Check server health
if ! curl -f -s "$SERVER_URL/health" > /dev/null; then
    log_message "ERROR: Server health check failed"
    # Send notification (replace with your notification method)
    notify-send "Order Coffee" "Server is down!"
    exit 1
fi

# Check status and log
STATUS=$(curl -s "$SERVER_URL/status" | jq -r '.status')
UPTIME=$(curl -s "$SERVER_URL/status" | jq -r '.uptime')

log_message "INFO: Server status: $STATUS, uptime: $UPTIME"

# Alert if inhibitor has been active for too long (> 8 hours)
INHIBITOR_ACTIVE=$(curl -s "$SERVER_URL/status" | jq -r '.inhibitor_active')
if [ "$INHIBITOR_ACTIVE" = "true" ]; then
    LAST_ACTION_TIME=$(curl -s "$SERVER_URL/status" | jq -r '.last_action_time')
    # Add logic to check if it's been too long...
    log_message "WARNING: Sleep inhibitor has been active since $LAST_ACTION_TIME"
fi
```

### Integration with Other Tools

```bash
# Integration with tmux
tmux_work() {
    curl -X POST http://192.168.0.200:20553/coffee
    tmux new-session -d -s work
    tmux send-keys -t work "echo 'Work session started - sleep prevention enabled'" Enter
    tmux attach -t work
    curl -X POST http://192.168.0.200:20553/chill
}

# Integration with Docker
docker_work() {
    curl -X POST http://192.168.0.200:20553/coffee
    docker run -it --rm ubuntu:latest bash
    curl -X POST http://192.168.0.200:20553/chill
}

# Integration with VS Code
code_work() {
    curl -X POST http://192.168.0.200:20553/coffee
    code "$@"
    curl -X POST http://192.168.0.200:20553/chill
}
```

## Troubleshooting

### Connection Issues

```bash
# Test basic connectivity
ping 192.168.0.200

# Test port accessibility
telnet 192.168.0.200 20553
# or
nc -zv 192.168.0.200 20553

# Check if server is listening
ss -tlnp | grep 20553
# or
netstat -tlnp | grep 20553
```

### Server Issues

```bash
# Check if systemd service is running
sudo systemctl status order-coffee.service

# View recent logs
sudo journalctl -u order-coffee.service -n 50

# Follow logs in real-time
sudo journalctl -u order-coffee.service -f

# Restart service
sudo systemctl restart order-coffee.service
```

### API Testing

```bash
# Test with verbose output
curl -v http://192.168.0.200:20553/health

# Test with different HTTP methods
curl -X GET http://192.168.0.200:20553/status
curl -X POST http://192.168.0.200:20553/coffee
curl -X POST http://192.168.0.200:20553/chill

# Test response times
curl -w "@curl-format.txt" -o /dev/null -s http://192.168.0.200:20553/status

# curl-format.txt content:
#     time_namelookup:  %{time_namelookup}\n
#        time_connect:  %{time_connect}\n
#     time_appconnect:  %{time_appconnect}\n
#    time_pretransfer:  %{time_pretransfer}\n
#       time_redirect:  %{time_redirect}\n
#  time_starttransfer:  %{time_starttransfer}\n
#                     ----------\n
#          time_total:  %{time_total}\n
```

### System Verification

```bash
# Check if systemd-inhibit is working
systemd-inhibit --list

# Test systemd-inhibit manually
systemd-inhibit --what=sleep:idle --who=test --why="Testing" sleep 10

# Check system sleep settings
systemctl status sleep.target suspend.target

# View power management logs
journalctl -u systemd-logind -f
```

### Performance Monitoring

```bash
# Monitor server resources
top -p $(pgrep order-coffee)

# Monitor network connections
ss -tuln | grep 20553

# Monitor API response times
while true; do
    time curl -s http://192.168.0.200:20553/health > /dev/null
    sleep 5
done
```

## Tips and Best Practices

1. **Always test endpoints** after server restart
2. **Monitor logs** regularly for any issues
3. **Use health checks** in monitoring systems
4. **Set up alerts** for server downtime
5. **Document your specific use cases** and create custom scripts
6. **Test failover scenarios** (what happens if server goes down)
7. **Consider security** - restrict access if needed
8. **Backup your configuration** and scripts

For more information, see the main README.md file.
