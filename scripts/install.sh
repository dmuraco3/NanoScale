#!/usr/bin/env bash
set -euo pipefail

readonly NANOSCALE_ROOT="/opt/nanoscale"
readonly SUDOERS_TEMPLATE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/security/sudoers.d/nanoscale"
readonly SUDOERS_TARGET="/etc/sudoers.d/nanoscale"

ROLE=""
JOIN_TOKEN=""
APT_UPDATED="false"

usage() {
  echo "Usage:"
  echo "  install.sh --role orchestrator"
  echo "  install.sh --join <token>"
  exit 1
}

require_root() {
  if [[ "${EUID}" -ne 0 ]]; then
    echo "Error: install.sh must run as root."
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
      *)
        echo "Error: unknown argument '$1'."
        usage
        ;;
    esac
  done

  if [[ -n "${ROLE}" && -n "${JOIN_TOKEN}" ]]; then
    echo "Error: use either --role orchestrator or --join <token>, not both."
    usage
  fi

  if [[ -z "${ROLE}" && -z "${JOIN_TOKEN}" ]]; then
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

configure_sudoers() {
  if [[ ! -f "${SUDOERS_TEMPLATE}" ]]; then
    echo "Error: sudoers template not found: ${SUDOERS_TEMPLATE}"
    exit 1
  fi

  install -o root -g root -m 0440 "${SUDOERS_TEMPLATE}" "${SUDOERS_TARGET}"
  visudo -c
}

configure_firewall() {
  ufw --force enable
  ufw allow 22/tcp
  ufw allow 80/tcp
  ufw allow 443/tcp
  ufw allow 4000/tcp
}

print_mode_summary() {
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
  configure_sudoers
  configure_firewall
  print_mode_summary
  echo "NanoScale installation baseline complete."
}

main "$@"
