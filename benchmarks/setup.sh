#!/bin/bash

# Setup script for Chrome HTML parser benchmarks
# This script installs Node.js dependencies for Chrome comparison benchmarks

set -e

echo "🚀 Setting up Chrome HTML Parser Benchmarks"
echo ""

# Check if Node.js is installed
if ! command -v node &> /dev/null; then
    echo "❌ Node.js is not installed"
    echo ""
    echo "Please install Node.js first:"
    echo "  - Ubuntu/Debian: sudo apt install nodejs npm"
    echo "  - macOS: brew install node"
    echo "  - Windows: Download from https://nodejs.org/"
    echo ""
    exit 1
fi

echo "✓ Node.js is installed: $(node --version)"
echo ""

# Check if npm is installed
if ! command -v npm &> /dev/null; then
    echo "❌ npm is not installed"
    echo "Please install npm (usually comes with Node.js)"
    exit 1
fi

echo "✓ npm is installed: $(npm --version)"
echo ""

# Install Puppeteer (optional)
echo "📦 Installing Puppeteer for accurate Chrome benchmarks..."
echo "   (This will download Chrome, ~170MB)"
echo ""

read -p "Install Puppeteer? (y/n) " -n 1 -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]; then
    npm install puppeteer
    echo ""
    echo "✓ Puppeteer installed successfully"
else
    echo "⚠️  Skipping Puppeteer installation"
    echo "   Basic benchmarks will still work, but won't use real Chrome"
fi

echo ""
echo "✅ Setup complete!"
echo ""
echo "You can now run Chrome comparison benchmarks:"
echo "  cargo test --release chrome_comparison -- --nocapture"
echo ""
