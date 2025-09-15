#!/bin/bash

# Fix USB permissions for nRF52840-DK debugging
# This script sets up udev rules and user permissions for probe-rs and VS Code debugging

set -e

echo "🔧 Setting up USB permissions for nRF52840-DK debugging..."

# Check if running as root
if [[ $EUID -eq 0 ]]; then
   echo "❌ Please don't run this script as root (don't use sudo)"
   echo "   The script will ask for sudo when needed"
   exit 1
fi

# Create udev rules for nRF52840-DK
echo "📝 Creating udev rules for nRF52840-DK..."
sudo tee /etc/udev/rules.d/99-probe-rs.rules > /dev/null << 'EOF'
# nRF52840-DK (SEGGER J-Link OB)
SUBSYSTEM=="usb", ATTR{idVendor}=="1366", ATTR{idProduct}=="1051", MODE="0664", GROUP="plugdev"

# General SEGGER J-Link rules  
SUBSYSTEM=="usb", ATTR{idVendor}=="1366", MODE="0664", GROUP="plugdev"

# Additional probe-rs compatible devices
SUBSYSTEM=="usb", ATTR{idVendor}=="0483", MODE="0664", GROUP="plugdev"  # ST-Link
SUBSYSTEM=="usb", ATTR{idVendor}=="15ba", MODE="0664", GROUP="plugdev"  # Olimex
EOF

echo "✅ udev rules created"

# Reload udev rules
echo "🔄 Reloading udev rules..."
sudo udevadm control --reload-rules
sudo udevadm trigger

# Add user to plugdev group
echo "👤 Adding user '$USER' to plugdev group..."
sudo usermod -a -G plugdev $USER

echo ""
echo "✅ USB permissions setup complete!"
echo ""
echo "📋 Next steps:"
echo "   1. Unplug and reconnect your nRF52840-DK"
echo "   2. Log out and log back in (or reboot)"  
echo "   3. Try debugging again in VS Code"
echo ""
echo "🔍 To verify setup, run: groups | grep plugdev"
echo ""