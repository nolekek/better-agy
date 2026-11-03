#!/usr/bin/env bash
# ==============================================================================
# better-agy Universal Server Installer
# One-liner: curl -fsSL https://raw.githubusercontent.com/nolekek/better-agy/main/install.sh | sudo bash
# ==============================================================================
set -euo pipefail

# 1. Require root privileges
if [[ $EUID -ne 0 ]]; then
    echo "[-] Error: install.sh must be executed as root (or via sudo)." >&2
    exit 1
fi

echo "=========================================================="
echo "      better-agy: Antigravity Multi-User Server"
echo "=========================================================="

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" 2>/dev/null && pwd || echo "/tmp")"

TEMP_DIR=""
cleanup() {
    if [[ -n "$TEMP_DIR" && -d "$TEMP_DIR" ]]; then
        rm -rf "$TEMP_DIR"
    fi
}
trap cleanup EXIT

# If executed directly without local repo clone (e.g. piped via curl | bash)
if [[ ! -d "$SCRIPT_DIR/crates" || ! -d "$SCRIPT_DIR/deploy" ]]; then
    echo "[*] Fetching better-agy repository into build workspace..."
    TEMP_DIR=$(mktemp -d /tmp/better-agy-install.XXXXXX)
    if ! command -v git >/dev/null 2>&1; then
        echo "[*] Installing git..."
        if command -v apt-get >/dev/null 2>&1; then
            apt-get update -qq && apt-get install -y -qq git
        elif command -v dnf >/dev/null 2>&1; then
            dnf install -y -q git
        fi
    fi
    git clone --depth 1 https://github.com/nolekek/better-agy.git "$TEMP_DIR"
    SRC_DIR="$TEMP_DIR"
else
    SRC_DIR="$SCRIPT_DIR"
fi

cd "$SRC_DIR"

# 2. System provision: packages, Caddy, groups, permissions, systemd units
echo "[*] Provisioning OS dependencies, Caddy, PAM, and Systemd services..."
chmod +x "$SRC_DIR/deploy/scripts/install_server.sh"
"$SRC_DIR/deploy/scripts/install_server.sh"

# 3. Ensure Rust toolchain is installed for compiling binaries
if ! command -v cargo >/dev/null 2>&1; then
    echo "[*] Cargo not found. Installing Rust toolchain via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
    if [[ -f "$HOME/.cargo/env" ]]; then
        # shellcheck disable=SC1091
        . "$HOME/.cargo/env"
    fi
    export PATH="$HOME/.cargo/bin:$PATH"
fi

# 4. Build release binaries
echo "[*] Compiling better-agy binaries (agy-hub, agy-proxy, agy-cli)..."
cargo build --release -p agy-cli -p agy-hub -p agy-proxy

echo "[*] Installing binaries to /opt/antigravity/bin/..."
mkdir -p /opt/antigravity/bin
cp target/release/agy /opt/antigravity/bin/
cp target/release/agy-hub /opt/antigravity/bin/
cp target/release/agy-proxy /opt/antigravity/bin/
chmod 0755 /opt/antigravity/bin/agy /opt/antigravity/bin/agy-hub /opt/antigravity/bin/agy-proxy
ln -sf /opt/antigravity/bin/agy /usr/local/bin/agy 2>/dev/null || true

# 5. Install add_developer script
if [[ -f "$SRC_DIR/deploy/scripts/add_developer.sh" ]]; then
    cp "$SRC_DIR/deploy/scripts/add_developer.sh" /opt/antigravity/bin/
    chmod 0755 /opt/antigravity/bin/add_developer.sh
    ln -sf /opt/antigravity/bin/add_developer.sh /usr/local/bin/add_developer 2>/dev/null || true
fi

# 6. Deploy central skills library
echo "[*] Deploying central skills library to /opt/gemini/skills-library/..."
mkdir -p /opt/gemini/skills-library
cp -r "$SRC_DIR/data/skills-library/"* /opt/gemini/skills-library/
chmod -R 0755 /opt/gemini/skills-library
chown -R root:agy-users /opt/gemini/skills-library

# 7. Deploy Caddy configuration if missing
if [[ -f "$SRC_DIR/deploy/caddy/Caddyfile" ]]; then
    if [[ ! -f /etc/caddy/Caddyfile ]]; then
        mkdir -p /etc/caddy
        cp "$SRC_DIR/deploy/caddy/Caddyfile" /etc/caddy/Caddyfile
        echo "[+] Installed default Caddyfile to /etc/caddy/Caddyfile"
    fi
fi

# 8. Reload systemd and verify
systemctl daemon-reload
echo "[*] Validating installation health..."
if command -v /opt/antigravity/bin/agy >/dev/null 2>&1; then
    /opt/antigravity/bin/agy doctor || true
fi

echo ""
echo "=========================================================="
echo "  better-agy installation completed successfully!"
echo "=========================================================="
echo "Next steps:"
echo " 1. Configure domain in /etc/caddy/Caddyfile (or set AGY_DOMAIN)"
echo " 2. Enable & start services:"
echo "    sudo systemctl enable --now agy-proxy agy-hub caddy"
echo " 3. Provision developer accounts:"
echo "    sudo add_developer <username> [preset] [password]"
echo " 4. Check system diagnostics:"
echo "    agy doctor"
echo "=========================================================="
