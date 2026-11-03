#!/usr/bin/env bash
# ==============================================================================
# Antigravity Multi-User Server Installation Script
# Target Platforms: Ubuntu 22.04/24.04 LTS, Debian 12, RHEL/Rocky 9
# ==============================================================================
set -euo pipefail

# 1. Root & Environment Checks
if [[ $EUID -ne 0 ]]; then
    echo "[-] Error: install_server.sh must be executed as root (or via sudo)." >&2
    exit 1
fi

if ! command -v systemctl >/dev/null 2>&1 || [[ ! -d /run/systemd/system ]]; then
    echo "[-] Error: Systemd must be active as PID 1." >&2
    exit 1
fi

echo "[*] Step 1: Detecting OS distribution..."
OS_FAMILY="unknown"
if [[ -f /etc/os-release ]]; then
    . /etc/os-release
    case "$ID" in
        ubuntu|debian)
            OS_FAMILY="debian"
            ;;
        rhel|rocky|almalinux|fedora|centos)
            OS_FAMILY="rhel"
            ;;
        *)
            echo "[-] Warning: Unsupported distribution '$ID'. Assuming Debian-compatible."
            OS_FAMILY="debian"
            ;;
    esac
fi

# 2. Package Installation
echo "[*] Step 2: Installing core dependencies and Caddy..."
if [[ "$OS_FAMILY" == "debian" ]]; then
    export DEBIAN_FRONTEND=noninteractive
    apt-get update -qq
    apt-get install -y -qq \
        curl jq tar gzip libpam0g-dev build-essential \
        pkg-config libssl-dev dbus debian-keyring debian-archive-keyring apt-transport-https \
        clang libclang-dev

    # Install official Caddy repo if not present
    if ! command -v caddy >/dev/null 2>&1; then
        curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg --yes
        curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | tee /etc/apt/sources.list.d/caddy-stable.list
        apt-get update -qq
        apt-get install -y -qq caddy
    fi
elif [[ "$OS_FAMILY" == "rhel" ]]; then
    dnf install -y -q \
        curl jq tar gzip pam-devel gcc gcc-c++ \
        pkgconfig openssl-devel dbus clang clang-devel
    if ! command -v caddy >/dev/null 2>&1; then
        dnf install -y -q 'dnf-command(copr)'
        dnf copr enable -y @caddy/caddy
        dnf install -y -q caddy
    fi
fi

# 3. User & Group Management
echo "[*] Step 3: Setting up system groups and service accounts..."
if ! getent group agy-users >/dev/null 2>&1; then
    groupadd -r agy-users
    echo "[+] Created system group 'agy-users'."
fi

# Unprivileged service user for agy-proxy loopback daemon
if ! id -u agy-proxy >/dev/null 2>&1; then
    useradd -r -s /usr/sbin/nologin -d /var/lib/agy-proxy -g agy-users agy-proxy
    echo "[+] Created service account 'agy-proxy'."
fi

# 4. Directory Hierarchy & Permissions
echo "[*] Step 4: Provisioning directory structure and permissions..."
mkdir -p /opt/antigravity/bin
mkdir -p /opt/gemini/skills-library
mkdir -p /etc/antigravity
mkdir -p /var/lib/agy-proxy
mkdir -p /var/log/antigravity
mkdir -p /etc/systemd/user

# Permissions:
# - /opt/gemini/skills-library: 0755 root:agy-users (Read-only projection source)
chmod 0755 /opt/gemini/skills-library
chown root:agy-users /opt/gemini/skills-library

# - /opt/antigravity/bin: 0755 root:root
chmod 0755 /opt/antigravity/bin
chown root:root /opt/antigravity/bin

# - /etc/antigravity: 0750 root:agy-users
chmod 0750 /etc/antigravity
chown root:agy-users /etc/antigravity

# - /var/lib/agy-proxy: 0700 agy-proxy:agy-users
chmod 0700 /var/lib/agy-proxy
chown agy-proxy:agy-users /var/lib/agy-proxy

# - /var/log/antigravity: 0775 root:agy-users
chmod 0775 /var/log/antigravity
chown root:agy-users /var/log/antigravity

# Initialize empty vault if absent (Mode 0600)
if [[ ! -f /etc/antigravity/vault.enc ]]; then
    touch /etc/antigravity/vault.enc
    chmod 0600 /etc/antigravity/vault.enc
    chown root:root /etc/antigravity/vault.enc
fi

# PAM configuration for antigravity-hub
cat << 'EOF' > /etc/pam.d/antigravity-hub
# /etc/pam.d/antigravity-hub
# PAM configuration for Antigravity Central Hub
auth      requisite     pam_nologin.so
auth      required      pam_unix.so nullok
account   required      pam_unix.so
session   required      pam_unix.so
EOF
chmod 0644 /etc/pam.d/antigravity-hub

# 5. Deploy Systemd Units
echo "[*] Step 5: Deploying systemd services..."

# A. Loopback Proxy Unit (/etc/systemd/system/agy-proxy.service)
cat << 'EOF' > /etc/systemd/system/agy-proxy.service
[Unit]
Description=Antigravity Loopback Failover Proxy (agy-proxy)
After=network.target
Wants=network.target

[Service]
Type=simple
User=root
Group=agy-users
ExecStart=/opt/antigravity/bin/agy-proxy --listen 127.0.0.1:51128 --vault /etc/antigravity/vault.enc
Restart=always
RestartSec=3s
LimitNOFILE=65536
StandardOutput=append:/var/log/antigravity/proxy.log
StandardError=append:/var/log/antigravity/proxy.log

# Sandboxing
ProtectSystem=strict
ReadWritePaths=/var/lib/agy-proxy /var/log/antigravity /etc/antigravity
PrivateTmp=true

[Install]
WantedBy=multi-user.target
EOF

# B. Central Auth & Session Router Unit (/etc/systemd/system/agy-hub.service)
cat << 'EOF' > /etc/systemd/system/agy-hub.service
[Unit]
Description=Antigravity Central PAM Portal & Dynamic Router (agy-hub)
After=network.target agy-proxy.service
Requires=agy-proxy.service

[Service]
Type=simple
User=root
Group=root
ExecStart=/opt/antigravity/bin/agy-hub --bind 127.0.0.1:8080 --config /etc/antigravity/hub.json
Restart=always
RestartSec=3s
LimitNOFILE=65536
StandardOutput=append:/var/log/antigravity/hub.log
StandardError=append:/var/log/antigravity/hub.log

# Needs access to PAM and D-Bus system bus
PrivateTmp=true

[Install]
WantedBy=multi-user.target
EOF

# C. Universal User-Level Web IDE Service (/etc/systemd/user/antigravity-web.service)
cat << 'EOF' > /etc/systemd/user/antigravity-web.service
[Unit]
Description=Antigravity Multi-User Web IDE Session
After=network.target

[Service]
Type=simple
Environment="HTTPS_PROXY=http://127.0.0.1:51128"
Environment="NO_PROXY=localhost,127.0.0.1"
Environment="AGY_WORKSPACE=%h/workspace"
ExecStart=/opt/antigravity/bin/antigravity-ide serve-web \
    --without-connection-token \
    --socket-path=%t/antigravity.sock \
    --server-data-dir=%h/.antigravity-server \
    --user-data-dir=%h/.antigravity-data \
    --extensions-dir=%h/.antigravity-extensions
Restart=always
RestartSec=3s
KillMode=process

[Install]
WantedBy=default.target
EOF

# Symlink global CLI binary
ln -sf /opt/antigravity/bin/agy /usr/local/bin/agy 2>/dev/null || true

# Copy and symlink add_developer utility if adjacent
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd || echo "")"
if [[ -n "$SCRIPT_DIR" && -f "$SCRIPT_DIR/add_developer.sh" ]]; then
    cp "$SCRIPT_DIR/add_developer.sh" /opt/antigravity/bin/add_developer.sh
    chmod 0755 /opt/antigravity/bin/add_developer.sh
    ln -sf /opt/antigravity/bin/add_developer.sh /usr/local/bin/add_developer 2>/dev/null || true
fi

# Reload systemd
systemctl daemon-reload

echo "[+] install_server.sh completed successfully."
echo "[+] Next steps: Run 'add_developer <username> [preset] [password]' to provision developers."
