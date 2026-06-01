#!/usr/bin/env bash
set -e

# Nexum dev server launcher
# Starts both backend (cargo run) and frontend (vite dev) concurrently.
# Ctrl-C cleanly kills both processes.

trap 'kill $CARGO_PID $VITE_PID 2>/dev/null; wait 2>/dev/null; exit' INT TERM

echo "Starting Nexum dev servers..."

cargo run &
CARGO_PID=$!

npm run dev --prefix web &
VITE_PID=$!

echo "Backend (PID $CARGO_PID) and frontend (PID $VITE_PID) running."
echo "Press Ctrl-C to stop both."

wait
