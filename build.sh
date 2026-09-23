#!/usr/bin/env bash
# Build (or rebuild) and start the ant stack.
#
#   ./build.sh all            build + start everything, detached
#   ./build.sh app            build + start just the rust container
#   ./build.sh -f all         foreground (ctrl-c stops)
#   ./build.sh -c all         clean first: drop containers, images and volumes
#   ./build.sh --no-cache all rebuild images from scratch
#
# Day to day you want ./start.sh; reach for this when a Dockerfile,
# Cargo.toml or the compose file changed.
set -euo pipefail
cd "$(dirname "$0")"

source ./build/includes/colors.sh
source ./build/includes/docker_utils.sh

usage() {
    cat <<USAGE
Usage: $0 [-f] [-v] [-c] [--no-cache] [-h] <all|service>

  -f, --foreground  run in the foreground and stream logs
  -v, --verbose     verbose docker compose output
  -c, --clean       remove containers, images and volumes first
                    ${RED}WARNING: -c destroys the database volume${ENDCOLOR}
      --no-cache    build images without the layer cache
  -h, --help        this message

Services: ${SERVICES[*]} (partial names work, e.g. "ng")
USAGE
    exit "${1:-0}"
}

DETACHED=true
VERBOSE=false
CLEAN=false
NO_CACHE=false
ARGS=()
START_TIME=$(date +%s)

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help)       usage 0 ;;
        -f|--foreground) DETACHED=false; shift ;;
        -v|--verbose)    VERBOSE=true; shift ;;
        -c|--clean)      CLEAN=true; shift ;;
        --no-cache)      NO_CACHE=true; shift ;;
        -[a-z][a-z]*)
            # combined short flags, e.g. -vc
            flags="${1:1}"
            for (( i=0; i<${#flags}; i++ )); do
                case "${flags:$i:1}" in
                    f) DETACHED=false ;;
                    v) VERBOSE=true ;;
                    c) CLEAN=true ;;
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

# ---------------------------------------------------------------------------
# stop what's running
# ---------------------------------------------------------------------------
if [ -n "$SERVICE" ]; then
    info "Stopping $SERVICE..."
    dc stop "$SERVICE" >/dev/null 2>&1 || true
else
    info "Stopping the stack..."
    dc down --remove-orphans >/dev/null 2>&1 || true
fi

# ---------------------------------------------------------------------------
# clean
# ---------------------------------------------------------------------------
if [ "$CLEAN" = true ]; then
    if [ -n "$SERVICE" ]; then
        warn "Removing container and image for $SERVICE..."
        dc rm -fsv "$SERVICE" >/dev/null 2>&1 || true
    else
        warn "Removing containers, images and volumes (database included)..."
        dc down --rmi local --volumes --remove-orphans || true
        docker builder prune -f >/dev/null 2>&1 || true
    fi
    ok "Cleaned."
fi

# ---------------------------------------------------------------------------
# build
# ---------------------------------------------------------------------------
BUILD_ARGS=()
[ "$NO_CACHE" = true ] && BUILD_ARGS+=(--no-cache)
[ "$VERBOSE" = true ] && BUILD_ARGS+=(--progress plain)

info "Building $DESC..."
if [ -n "$SERVICE" ]; then
    dc build ${BUILD_ARGS[@]+"${BUILD_ARGS[@]}"} "$SERVICE"
else
    dc build ${BUILD_ARGS[@]+"${BUILD_ARGS[@]}"}
fi

# ---------------------------------------------------------------------------
# up
# ---------------------------------------------------------------------------
info "Starting $DESC..."
UP_ARGS=()
[ "$DETACHED" = true ] && UP_ARGS+=(-d)

if [ -n "$SERVICE" ]; then
    dc up ${UP_ARGS[@]+"${UP_ARGS[@]}"} "$SERVICE"
else
    dc up ${UP_ARGS[@]+"${UP_ARGS[@]}"}
fi

[ "$DETACHED" = true ] || exit 0

# ---------------------------------------------------------------------------
# health + summary
# ---------------------------------------------------------------------------
if [ -n "$SERVICE" ]; then
    wait_healthy "$SERVICE" 300 || true
else
    health_check_all
fi

echo -e "${GREEN}===================================${ENDCOLOR}"
echo -e "${GREEN} ant ($ENVIRONMENT) is up${ENDCOLOR}"
echo -e "${GREEN}===================================${ENDCOLOR}"
echo "Built and started in $(( $(date +%s) - START_TIME ))s"
print_endpoints
echo "Logs: ./start.sh -t all   (or docker compose -f $COMPOSE_FILE logs -f)"
