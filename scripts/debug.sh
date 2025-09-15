#!/bin/bash

# Debug script for nRF52840-DK with Embassy template
# This script starts a debug session with RTT logging

set -e

echo "🐛 Starting debug session for nRF52840-DK..."

# Check if probe-rs is installed
if ! command -v probe-rs &> /dev/null; then
    echo "❌ probe-rs not found. Please install it first:"
    echo "   cargo install probe-rs --features cli"
    exit 1
fi

# Build debug version
echo "🔧 Building debug version..."
cargo build

# Start RTT logging session
echo "📡 Starting RTT logging session..."
echo "Press Ctrl+C to stop logging"
probe-rs attach --chip nRF52840_xxAA