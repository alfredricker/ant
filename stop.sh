#!/usr/bin/env bash
# Stop the ant stack.
#
#   ./stop.sh all        stop every container in the stack
#   ./stop.sh app        stop just the rust container
#   ./stop.sh -d all     stop and remove containers + network
#   ./stop.sh -dv all    ...and delete volumes (drops the database)
set -euo pipefail
cd "$(dirname "$0")"

source ./build/includes/colors.sh
source ./build/includes/docker_utils.sh

usage() {
    cat <<USAGE
Usage: $0 [-d] [-v] [-h] <all|service>

  -d, --down     remove containers and the network, not just stop them
  -v, --volumes  also remove volumes (implies -d)
                 ${RED}WARNING: this deletes the database${ENDCOLOR}
  -h, --help     this message

Services: ${SERVICES[*]} (partial names work, e.g. "ng")
USAGE
    exit "${1:-0}"
}

DOWN=false
VOLUMES=false
ARGS=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help|-\?) usage 0 ;;
        -d|--down)     DOWN=true; shift ;;
        -v|--volumes)  VOLUMES=true; DOWN=true; shift ;;
        -[a-z][a-z]*)
            flags="${1:1}"
            for (( i=0; i<${#flags}; i++ )); do
                case "${flags:$i:1}" in
                    d) DOWN=true ;;
                    v) VOLUMES=true; DOWN=true ;;
                    h) usage 0 ;;
                    *) echo -e "${RED}Invalid option: -${flags:$i:1}${ENDCOLOR}"; usage 1 ;;
                esac
            done
            shift ;;
        -*) echo -e "${RED}Invalid option: $1${ENDCOLOR}"; usage 1 ;;
        *)  ARGS+=("$1"); shift ;;
    esac
done

set -- ${ARGS[@]+"${ARGS[@]}"}
TARGET="${1:-}"
[ -z "$TARGET" ] && { echo -e "${RED}No service specified.${ENDCOLOR}"; usage 1; }

ensure_env
SERVICE="$(resolve_service "$TARGET")" || exit 1
DESC="${SERVICE:-all services}"

if [ -n "$SERVICE" ]; then
    if [ "$DOWN" = true ]; then
        info "Removing $SERVICE..."
        if [ "$VOLUMES" = true ]; then
            dc rm -fsv "$SERVICE"
        else
            dc rm -fs "$SERVICE"
        fi
    else
        info "Stopping $SERVICE..."
        dc stop "$SERVICE"
    fi
else
    if [ "$VOLUMES" = true ]; then
        warn "This removes the database volume. Ctrl-C within 5s to abort."
        sleep 5
        info "Tearing down the stack and its volumes..."
        dc down --volumes --remove-orphans
    elif [ "$DOWN" = true ]; then
        info "Tearing down the stack..."
        dc down --remove-orphans
    else
        info "Stopping the stack..."
        dc stop
    fi
fi

ok "Stopped $DESC."
