#!/bin/sh
set -e

echo "ZecBox Relay starting..."

# Start WireGuard
wg-quick up wg0
echo "WireGuard tunnel up"

# Start socat relay: forward TCP :8233 to home node through tunnel
TUNNEL_HOME_IP="${TUNNEL_HOME_IP:-10.13.37.2}"
socat TCP-LISTEN:8233,fork,reuseaddr TCP:${TUNNEL_HOME_IP}:8233 &
SOCAT_PID=$!
echo "Relay active: forwarding :8233 to ${TUNNEL_HOME_IP}:8233"

# Handle shutdown signals
cleanup() {
    echo "Shutting down..."
    kill $SOCAT_PID 2>/dev/null || true
    wg-quick down wg0 2>/dev/null || true
    exit 0
}
trap cleanup TERM INT

# Wait for socat to exit
wait $SOCAT_PID
