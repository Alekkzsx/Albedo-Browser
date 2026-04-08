#!/bin/bash

# Setup script for Browser HTML parser benchmarks
# This script installs Node.js dependencies for Chrome and Firefox comparison benchmarks

set -e

echo "🚀 Setting up Browser HTML Parser Benchmarks"
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

# Ask which browsers to install
echo "Which browser automation tools would you like to install?"
echo "  1) Puppeteer (Chrome) only"
echo "  2) Playwright (Firefox) only"
echo "  3) Both Puppeteer and Playwright"
echo "  4) Skip installation"
echo ""

read -p "Enter choice (1-4): " choice
echo ""

case $choice in
    1)
        echo "📦 Installing Puppeteer for Chrome benchmarks..."
        echo "   (This will download Chrome, ~170MB)"
        echo ""
        npm install puppeteer
        echo ""
        echo "✓ Puppeteer installed successfully"
        ;;
    2)
        echo "📦 Installing Playwright for Firefox benchmarks..."
        echo "   (This will download Firefox, ~80MB)"
        echo ""
        npm install playwright
        npx playwright install firefox
        echo ""
        echo "✓ Playwright installed successfully"
        ;;
    3)
        echo "📦 Installing Puppeteer and Playwright..."
        echo "   (This will download Chrome and Firefox, ~250MB total)"
        echo ""
        npm install puppeteer playwright
        npx playwright install firefox
        echo ""
        echo "✓ Both tools installed successfully"
        ;;
    4)
        echo "⚠️  Skipping installation"
        echo "   Basic benchmarks will still work, but won't use real browsers"
        ;;
    *)
        echo "❌ Invalid choice"
        exit 1
        ;;
esac

echo ""
echo "✅ Setup complete!"
echo ""
echo "You can now run browser comparison benchmarks:"
echo "  cargo test --release chrome_comparison -- --nocapture"
echo "  cargo test --release firefox_comparison -- --nocapture"
echo ""
