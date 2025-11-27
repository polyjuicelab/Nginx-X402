#!/bin/bash
# Start mock backend and nginx for integration testing

# Start mock backend in background
/usr/local/bin/mock-backend.py &
BACKEND_PID=$!

# Wait for backend to be ready
sleep 1

# Start nginx in foreground
exec nginx -g "daemon off;"

