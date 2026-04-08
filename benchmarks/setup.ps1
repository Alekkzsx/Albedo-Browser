# Setup script for Browser HTML parser benchmarks (PowerShell)
# This script installs Node.js dependencies for Chrome and Firefox comparison benchmarks

Write-Host "🚀 Setting up Browser HTML Parser Benchmarks" -ForegroundColor Cyan
Write-Host ""

# Check if Node.js is installed
try {
    $nodeVersion = node --version
    Write-Host "✓ Node.js is installed: $nodeVersion" -ForegroundColor Green
} catch {
    Write-Host "❌ Node.js is not installed" -ForegroundColor Red
    Write-Host ""
    Write-Host "Please install Node.js first:"
    Write-Host "  Download from https://nodejs.org/"
    Write-Host ""
    exit 1
}

Write-Host ""

# Check if npm is installed
try {
    $npmVersion = npm --version
    Write-Host "✓ npm is installed: $npmVersion" -ForegroundColor Green
} catch {
    Write-Host "❌ npm is not installed" -ForegroundColor Red
    Write-Host "Please install npm (usually comes with Node.js)"
    exit 1
}

Write-Host ""

# Ask which browsers to install
Write-Host "Which browser automation tools would you like to install?" -ForegroundColor Yellow
Write-Host "  1) Puppeteer (Chrome) only"
Write-Host "  2) Playwright (Firefox) only"
Write-Host "  3) Both Puppeteer and Playwright"
Write-Host "  4) Skip installation"
Write-Host ""

$choice = Read-Host "Enter choice (1-4)"
Write-Host ""

switch ($choice) {
    "1" {
        Write-Host "📦 Installing Puppeteer for Chrome benchmarks..." -ForegroundColor Yellow
        Write-Host "   (This will download Chrome, ~170MB)"
        Write-Host ""
        npm install puppeteer
        Write-Host ""
        Write-Host "✓ Puppeteer installed successfully" -ForegroundColor Green
    }
    "2" {
        Write-Host "📦 Installing Playwright for Firefox benchmarks..." -ForegroundColor Yellow
        Write-Host "   (This will download Firefox, ~80MB)"
        Write-Host ""
        npm install playwright
        npx playwright install firefox
        Write-Host ""
        Write-Host "✓ Playwright installed successfully" -ForegroundColor Green
    }
    "3" {
        Write-Host "📦 Installing Puppeteer and Playwright..." -ForegroundColor Yellow
        Write-Host "   (This will download Chrome and Firefox, ~250MB total)"
        Write-Host ""
        npm install puppeteer playwright
        npx playwright install firefox
        Write-Host ""
        Write-Host "✓ Both tools installed successfully" -ForegroundColor Green
    }
    "4" {
        Write-Host "⚠️  Skipping installation" -ForegroundColor Yellow
        Write-Host "   Basic benchmarks will still work, but won't use real browsers"
    }
    default {
        Write-Host "❌ Invalid choice" -ForegroundColor Red
        exit 1
    }
}

Write-Host ""
Write-Host "✅ Setup complete!" -ForegroundColor Green
Write-Host ""
Write-Host "You can now run browser comparison benchmarks:"
Write-Host "  cargo test --release chrome_comparison -- --nocapture"
Write-Host "  cargo test --release firefox_comparison -- --nocapture"
Write-Host ""
