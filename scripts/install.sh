#!/usr/bin/env bash
set -euo pipefail

readonly NANOSCALE_ROOT="/opt/nanoscale"
readonly CONFIG_FILE_PATH="${NANOSCALE_ROOT}/config.json"
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SUDOERS_TARGET="/etc/sudoers.d/nanoscale"
readonly SERVICE_FILE_PATH="/etc/systemd/system/nanoscale.service"
readonly SYSTEM_UPDATER_PATH="/usr/local/bin/nanoscale-system-updater"
readonly DEFAULT_REPO_SLUG="dmuraco/NanoScale"

ROLE=""
JOIN_TOKEN=""
UPDATE_MODE="false"
APT_UPDATED="false"

usage() {
  echo "Usage:"
  echo "  install.sh --role orchestrator"
  echo "  install.sh --join <token>"
  echo "  install.sh --update"
  exit 1
}

require_root() {
  if [[ "${EUID}" -ne 0 ]]; then
    echo "Error: install.sh must run as root."
    exit 1
  fi
}

require_command() {
  local command_name="$1"
  local install_hint="$2"

  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "Error: required command not found: ${command_name}"
    echo "Hint: ${install_hint}"
    exit 1
  fi
}

resolve_repo_root() {
  if command -v git >/dev/null 2>&1 && git rev-parse --show-toplevel >/dev/null 2>&1; then
    git rev-parse --show-toplevel
    return
  fi

  (cd "${SCRIPT_DIR}/.." && pwd)
}

require_repo_root() {
  local repo_root="$1"

  if [[ ! -f "${repo_root}/Cargo.toml" || ! -f "${repo_root}/package.json" ]]; then
    echo "Error: expected to run from the NanoScale repo root (missing Cargo.toml/package.json)."
    echo "Current repo root guess: ${repo_root}"
    exit 1
  fi
}

parse_args() {
  if [[ "$#" -eq 0 ]]; then
    usage
  fi

  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --role)
        if [[ "${2:-}" != "orchestrator" ]]; then
          echo "Error: --role only supports 'orchestrator'."
          usage
        fi
        ROLE="orchestrator"
        shift 2
        ;;
      --join)
        if [[ -z "${2:-}" ]]; then
          echo "Error: --join requires a token."
          usage
        fi
        JOIN_TOKEN="$2"
        shift 2
        ;;
      --update)
        UPDATE_MODE="true"
        shift
        ;;
      *)
        echo "Error: unknown argument '$1'."
        usage
        ;;
    esac
  done

  local selected_modes=0

  if [[ -n "${ROLE}" ]]; then
    selected_modes=$((selected_modes + 1))
  fi
  if [[ -n "${JOIN_TOKEN}" ]]; then
    selected_modes=$((selected_modes + 1))
  fi
  if [[ "${UPDATE_MODE}" == "true" ]]; then
    selected_modes=$((selected_modes + 1))
  fi

  if [[ "${selected_modes}" -gt 1 ]]; then
    echo "Error: use only one mode: --role orchestrator, --join <token>, or --update."
    usage
  fi

  if [[ "${selected_modes}" -eq 0 ]]; then
    usage
  fi
}

install_with_apt() {
  local package="$1"

  if [[ "${APT_UPDATED}" == "false" ]]; then
    apt-get update -y
    APT_UPDATED="true"
  fi

  DEBIAN_FRONTEND=noninteractive apt-get install -y "${package}"
}

install_with_dnf() {
  local package="$1"
  dnf install -y "${package}"
}

install_with_yum() {
  local package="$1"
  yum install -y "${package}"
}

install_package() {
  local package="$1"

  if command -v apt-get >/dev/null 2>&1; then
    install_with_apt "${package}"
    return
  fi

  if command -v dnf >/dev/null 2>&1; then
    install_with_dnf "${package}"
    return
  fi

  if command -v yum >/dev/null 2>&1; then
    install_with_yum "${package}"
    return
  fi

  echo "Error: no supported package manager found to install '${package}'."
  exit 1
}

ensure_dependency() {
  local command_name="$1"
  local package_name="$2"

  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "Installing missing dependency: ${package_name}"
    install_package "${package_name}"
  fi
}

ensure_dependencies() {
  ensure_dependency "curl" "curl"
  ensure_dependency "git" "git"
  ensure_dependency "nginx" "nginx"
  ensure_dependency "sqlite3" "sqlite3"
  ensure_dependency "ufw" "ufw"
  ensure_dependency "certbot" "certbot"
}

build_and_install_agent() {
  local repo_root="$1"

  require_command "cargo" "Install Rust (rustup) so 'cargo' is available."
  require_command "install" "Install coreutils so the 'install' command is available."

  echo "Building Rust agent (release)…"
  (
    cd "${repo_root}"
    cargo build --release -p agent
  )

  echo "Installing agent binary to ${NANOSCALE_ROOT}/backend-bin…"
  install -m 0755 "${repo_root}/target/release/agent" "${NANOSCALE_ROOT}/backend-bin"
}

build_dashboard() {
  local repo_root="$1"

  require_command "bun" "Install Bun so 'bun' is available (https://bun.sh)."

  echo "Installing JS dependencies (bun)…"
  (
    cd "${repo_root}"
    bun install
  )

  echo "Building dashboard (bun run build)…"
  (
    cd "${repo_root}"
    bun run build
  )
}

ensure_group_and_user() {
  if ! getent group nanoscale >/dev/null; then
    groupadd --system nanoscale
  fi

  if ! id -u nanoscale >/dev/null 2>&1; then
    useradd --system --gid nanoscale --home-dir "${NANOSCALE_ROOT}" --shell /bin/false nanoscale
  fi
}

create_directories() {
  mkdir -p "${NANOSCALE_ROOT}/"{bin,data,sites,config,logs,tmp}
  chown -R nanoscale:nanoscale "${NANOSCALE_ROOT}"
  chmod 0711 "${NANOSCALE_ROOT}/sites"
}

create_default_backend_config() {
  if [[ -f "${CONFIG_FILE_PATH}" ]]; then
    echo "Keeping existing backend config: ${CONFIG_FILE_PATH}"
    return
  fi

  cat > "${CONFIG_FILE_PATH}" <<'JSON'
{
  "database_path": "/opt/nanoscale/data/nanoscale.db",
  "tls_email": "",
  "orchestrator": {
    "bind_address": "0.0.0.0:4000",
    "server_id": "orchestrator-local",
    "server_name": "orchestrator",
    "worker_ip": "127.0.0.1",
    "base_domain": ""
  },
  "worker": {
    "orchestrator_url": "http://127.0.0.1:4000",
    "ip": "127.0.0.1",
    "name": "worker-node",
    "bind": "0.0.0.0:4000"
  }
}
JSON

  chown nanoscale:nanoscale "${CONFIG_FILE_PATH}"
  chmod 0644 "${CONFIG_FILE_PATH}"
  echo "Created backend config: ${CONFIG_FILE_PATH}"
}

configure_sudoers() {
  cat > "${SUDOERS_TARGET}" <<'EOF'
nanoscale ALL=(root) NOPASSWD: /bin/systemctl restart nanoscale.service
nanoscale ALL=(root) NOPASSWD: /usr/local/bin/nanoscale-system-updater
EOF

  chown root:root "${SUDOERS_TARGET}"
  chmod 0440 "${SUDOERS_TARGET}"
  visudo -c
}

create_or_update_systemd_service() {
  require_command "systemctl" "Install systemd so 'systemctl' is available."

  cat > "${SERVICE_FILE_PATH}" <<'EOF'
[Unit]
Description=NanoScale Service
After=network.target

[Service]
Type=simple
User=nanoscale
Group=nanoscale
WorkingDirectory=/opt/nanoscale
Environment=NANOSCALE_CONFIG_PATH=/opt/nanoscale/config.json
ExecStart=/opt/nanoscale/backend-bin --role orchestrator
Restart=always
RestartSec=2

[Install]
WantedBy=multi-user.target
EOF

  chown root:root "${SERVICE_FILE_PATH}"
  chmod 0644 "${SERVICE_FILE_PATH}"
  systemctl daemon-reload
  systemctl enable nanoscale.service
}

extract_repo_slug_from_remote_url() {
  local remote_url="$1"

  local slug
  slug="$(printf '%s' "${remote_url}" | sed -E 's#^git@github.com:##; s#^https://github.com/##; s#\.git$##')"

  if [[ "${slug}" =~ ^[A-Za-z0-9._-]+/[A-Za-z0-9._-]+$ ]]; then
    printf '%s\n' "${slug}"
    return
  fi

  printf '%s\n' "${DEFAULT_REPO_SLUG}"
}

resolve_repo_slug_for_updates() {
  local repo_root="$1"

  if [[ -n "${NANOSCALE_REPO_SLUG:-}" ]]; then
    printf '%s\n' "${NANOSCALE_REPO_SLUG}"
    return
  fi

  if [[ -n "${repo_root}" ]] && command -v git >/dev/null 2>&1; then
    local remote_url
    remote_url="$(git -C "${repo_root}" config --get remote.origin.url || true)"
    if [[ -n "${remote_url}" ]]; then
      extract_repo_slug_from_remote_url "${remote_url}"
      return
    fi
  fi

  printf '%s\n' "${DEFAULT_REPO_SLUG}"
}

install_system_updater_wrapper() {
  local repo_slug="$1"

  cat > "${SYSTEM_UPDATER_PATH}" <<EOF
#!/usr/bin/env bash
set -euo pipefail

readonly REPO_SLUG="${repo_slug}"
readonly SETUP_URL="https://raw.githubusercontent.com/\${REPO_SLUG}/main/scripts/install.sh"
readonly SETUP_PATH="/tmp/nanoscale-setup.sh"

curl -fsSL "\${SETUP_URL}" -o "\${SETUP_PATH}"
chmod 0755 "\${SETUP_PATH}"
"\${SETUP_PATH}" --update
EOF

  chown root:root "${SYSTEM_UPDATER_PATH}"
  chmod 0700 "${SYSTEM_UPDATER_PATH}"
}

configure_firewall() {
  ufw --force enable
  ufw allow 22/tcp
  ufw allow 80/tcp
  ufw allow 443/tcp
  ufw allow 4000/tcp
}

print_mode_summary() {
  if [[ "${UPDATE_MODE}" == "true" ]]; then
    echo "Configured NanoScale system update prerequisites."
    return
  fi

  if [[ "${ROLE}" == "orchestrator" ]]; then
    echo "Configured orchestrator prerequisites."
    return
  fi

  echo "Configured worker prerequisites for join token: ${JOIN_TOKEN}"
}

main() {
  require_root
  parse_args "$@"

  ensure_dependencies
  ensure_group_and_user
  create_directories
  create_default_backend_config

  local repo_root=""
  if [[ "${UPDATE_MODE}" != "true" ]]; then
    repo_root="$(resolve_repo_root)"
    require_repo_root "${repo_root}"

    build_and_install_agent "${repo_root}"
    build_dashboard "${repo_root}"
  fi

  local repo_slug
  repo_slug="$(resolve_repo_slug_for_updates "${repo_root}")"

  create_or_update_systemd_service
  install_system_updater_wrapper "${repo_slug}"

  configure_sudoers
  configure_firewall
  print_mode_summary

  if [[ "${UPDATE_MODE}" == "true" ]]; then
    echo "NanoScale system update complete."
    return
  fi

  echo "NanoScale installation baseline complete."
}

main "$@"
