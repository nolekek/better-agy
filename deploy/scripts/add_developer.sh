#!/usr/bin/env bash
# ==============================================================================
# Antigravity Developer Provisioning Script
# Usage: sudo ./add_developer.sh <username> [initial_preset] [password]
# ==============================================================================
set -euo pipefail

if [[ $EUID -ne 0 ]]; then
    echo "[-] Error: add_developer.sh must be executed as root (or via sudo)." >&2
    exit 1
fi

USERNAME="${1:-}"
INITIAL_PRESET="${2:-web}"
USER_PASS="${3:-}"

if [[ -z "$USERNAME" ]]; then
    echo "Usage: $0 <username> [initial_preset] [password]" >&2
    exit 1
fi

# POSIX username validation
if ! [[ "$USERNAME" =~ ^[a-z_][a-z0-9_-]{1,31}$ ]]; then
    echo "[-] Error: Invalid username '$USERNAME'. Must match POSIX lowercase regex ^[a-z_][a-z0-9_-]{1,31}$" >&2
    exit 1
fi

echo "[*] Step 1: Provisioning Linux user account '$USERNAME'..."
if id -u "$USERNAME" >/dev/null 2>&1; then
    echo "[!] User '$USERNAME' already exists. Ensuring group membership..."
    usermod -a -G agy-users "$USERNAME"
else
    useradd -m -s /bin/bash -G agy-users "$USERNAME"
    echo "[+] User '$USERNAME' created."
fi

# Set password if provided
if [[ -n "$USER_PASS" ]]; then
    echo "$USERNAME:$USER_PASS" | chpasswd
    echo "[+] Password configured for '$USERNAME'."
fi

# 2. Enforce Home Directory Isolation
HOMEDIR=$(eval echo "~$USERNAME")
chmod 0750 "$HOMEDIR"
chown "$USERNAME:$USERNAME" "$HOMEDIR"

# 3. Enable Systemd Linger
echo "[*] Step 2: Enabling systemd linger for background persistence..."
loginctl enable-linger "$USERNAME"
LINGER_STATUS=$(loginctl show-user "$USERNAME" -p Linger --value 2>/dev/null || true)
if [[ "$LINGER_STATUS" != "yes" ]]; then
    echo "[-] Warning: Linger status verification returned '$LINGER_STATUS'."
else
    echo "[+] Linger enabled successfully for '$USERNAME'."
fi

# 4. Provision Workspace Structure
echo "[*] Step 3: Setting up workspace directory structure..."
WORKSPACE_DIR="$HOMEDIR/workspace"
SKILLS_DIR="$WORKSPACE_DIR/.gemini/skills"

mkdir -p "$SKILLS_DIR"
chown -R "$USERNAME:$USERNAME" "$WORKSPACE_DIR"

# 5. Project Skills via Symlink Projection Engine
echo "[*] Step 4: Projecting initial preset '$INITIAL_PRESET'..."
if command -v agy >/dev/null 2>&1; then
    su - "$USERNAME" -c "agy preset switch $INITIAL_PRESET --workspace $WORKSPACE_DIR" 2>/dev/null || true
fi

# Fallback shell-based projection if agy is not installed yet
LIBRARY_DIR="/opt/gemini/skills-library"
ALWAYS_ACTIVE=("agent-preset-conductor" "grill-me" "systematic-debugging" "verification-before-completion" "concise-planning")

if [[ -d "$LIBRARY_DIR" ]]; then
    for skill in "${ALWAYS_ACTIVE[@]}"; do
        if [[ -d "$LIBRARY_DIR/$skill" ]]; then
            ln -sfn "$LIBRARY_DIR/$skill" "$SKILLS_DIR/$skill" 2>/dev/null || true
        fi
    done
    chown -h -R "$USERNAME:$USERNAME" "$SKILLS_DIR" 2>/dev/null || true
    echo "[+] Injected 5 ALWAYS_ACTIVE skills."
fi

# 6. Enable & Start Systemd User Service
echo "[*] Step 5: Initializing user-level systemd service..."
USER_UID=$(id -u "$USERNAME")

if command -v systemctl >/dev/null 2>&1; then
    su - "$USERNAME" -c "systemctl --user daemon-reload" 2>/dev/null || true
    su - "$USERNAME" -c "systemctl --user enable antigravity-web.service" 2>/dev/null || true
    su - "$USERNAME" -c "systemctl --user restart antigravity-web.service" 2>/dev/null || true
fi

# 7. Verification
echo "[*] Step 6: Verifying user socket readiness..."
SOCKET_PATH="/run/user/$USER_UID/antigravity.sock"
READY=false
for i in {1..10}; do
    if [[ -S "$SOCKET_PATH" ]]; then
        READY=true
        break
    fi
    sleep 0.5 2>/dev/null || true
done

if [[ "$READY" == "true" ]]; then
    echo "[+] SUCCESS: Developer '$USERNAME' onboarded. Socket active at $SOCKET_PATH"
else
    echo "[!] Note: Socket $SOCKET_PATH not yet active. It will spin up on demand via agy-hub D-Bus."
fi

echo "======================================================================"
echo " Developer Onboarding Complete: $USERNAME"
echo " Initial Preset: $INITIAL_PRESET"
echo " Web Access URL: https://<your-domain>/login"
echo "======================================================================"
