#!/usr/bin/env bash
# Start containers that already exist.
#
#   ./start.sh all           start everything
#   ./start.sh -t app        start the rust container and tail its logs
#   ./start.sh -r nginx      restart nginx (picks up conf.d changes)
#   ./start.sh -rbt all      restart, rebuild and tail everything
#
# If images don't exist yet, use ./build.sh.
set -euo pipefail
cd "$(dirname "$0")"

source ./build/includes/colors.sh
source ./build/includes/docker_utils.sh

usage() {
    cat <<USAGE
Usage: $0 [-t] [-r] [-b] [-h] <all|service>

  -t, --tail     follow logs after starting
  -r, --restart  stop the service(s) first
  -b, --build    rebuild images before starting
  -h, --help     this message

Services: ${SERVICES[*]} (partial names work, e.g. "ng")
USAGE
    exit "${1:-0}"
}

TAIL_LOGS=false
RESTART=false
BUILD=false
ARGS=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help|-\?) usage 0 ;;
        -t|--tail)     TAIL_LOGS=true; shift ;;
        -r|--restart)  RESTART=true; shift ;;
        -b|--build)    BUILD=true; shift ;;
        -[a-z][a-z]*)
            flags="${1:1}"
            for (( i=0; i<${#flags}; i++ )); do
                case "${flags:$i:1}" in
                    t) TAIL_LOGS=true ;;
                    r) RESTART=true ;;
                    b) BUILD=true ;;
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

if [ "$RESTART" = true ]; then
    info "Stopping $DESC..."
    if [ -n "$SERVICE" ]; then
        dc stop "$SERVICE"
    else
        dc down --remove-orphans
    fi
fi

UP_ARGS=(-d)
[ "$BUILD" = true ] && { info "Rebuilding $DESC..."; UP_ARGS+=(--build); }

info "Starting $DESC..."
if [ -n "$SERVICE" ]; then
    dc up ${UP_ARGS[@]+"${UP_ARGS[@]}"} "$SERVICE"
else
    dc up ${UP_ARGS[@]+"${UP_ARGS[@]}"}
fi

if [ -n "$SERVICE" ]; then
    wait_healthy "$SERVICE" 30 || true
else
    health_check_all
    print_endpoints
fi

if [ "$TAIL_LOGS" = true ]; then
    info "Tailing logs for $DESC (ctrl-c to detach; containers keep running)..."
    if [ -n "$SERVICE" ]; then
        dc logs -f --tail 100 "$SERVICE"
    else
        dc logs -f --tail 100
    fi
fi
