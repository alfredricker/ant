#!/usr/bin/env bash
# Shared helpers for build.sh / start.sh / stop.sh.
# Assumes colors.sh has already been sourced.

set -o pipefail

# Repo root, regardless of where the script was invoked from.
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

ENVIRONMENT="${ENVIRONMENT:-dev}"
DOCKER_DIR="$REPO_ROOT/build/$ENVIRONMENT"
COMPOSE_FILE="$DOCKER_DIR/docker-compose.yml"
ENV_DIR="$DOCKER_DIR/env"
ENV_FILE="$ENV_DIR/.env"

SERVICES=(db app nginx)

die() {
    echo -e "${RED}$*${ENDCOLOR}" >&2
    exit 1
}

info()  { echo -e "${BLUE}$*${ENDCOLOR}"; }
warn()  { echo -e "${YELLOW}$*${ENDCOLOR}"; }
ok()    { echo -e "${GREEN}$*${ENDCOLOR}"; }

# ---------------------------------------------------------------------------
# .env
# ---------------------------------------------------------------------------
# In dev, .env is a symlink to the committed .env.example so everyone shares the
# same defaults; point it elsewhere locally and nothing here will clobber it.
ensure_env() {
    [ -d "$ENV_DIR" ] || die "Missing env dir: $ENV_DIR"

    if [ "$ENVIRONMENT" = "dev" ]; then
        if [ ! -e "$ENV_FILE" ]; then
            warn "Creating .env symlink -> .env.example"
            ln -s .env.example "$ENV_FILE"
        fi
    fi

    [ -e "$ENV_FILE" ] || die "[FATAL] No env file at $ENV_FILE"
}

# ---------------------------------------------------------------------------
# compose wrapper — always the same file, env file and working dir
# ---------------------------------------------------------------------------
dc() {
    docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" "$@"
}

compose_services() {
    dc config --services 2>/dev/null
}

# Resolve a partial name ("ng", "post") to a real compose service.
resolve_service() {
    local query="$1"
    local matches=()

    [ -z "$query" ] && return 1
    if [[ "$query" == "all" || "$query" == "ALL" ]]; then
        echo ""
        return 0
    fi

    local svc
    while IFS= read -r svc; do
        [ -z "$svc" ] && continue
        if [ "$svc" = "$query" ]; then
            echo "$svc"
            return 0
        fi
        [[ "$svc" == *"$query"* ]] && matches+=("$svc")
    done < <(compose_services)

    case ${#matches[@]} in
        1) echo "${matches[0]}"; return 0 ;;
        0) echo -e "${RED}No service matching '$query'. Known: ${SERVICES[*]}${ENDCOLOR}" >&2; return 1 ;;
        *) echo -e "${RED}'$query' is ambiguous: ${matches[*]}${ENDCOLOR}" >&2; return 1 ;;
    esac
}

# ---------------------------------------------------------------------------
# health
# ---------------------------------------------------------------------------
# Waits on the healthcheck declared in docker-compose.yml.
wait_healthy() {
    local service="$1" timeout="${2:-120}" cid state
    cid="$(dc ps -q "$service" 2>/dev/null)"
    if [ -z "$cid" ]; then
        warn "[$service] not running, skipping health check"
        return 1
    fi

    # No healthcheck defined -> nothing to wait on.
    if ! docker inspect -f '{{if .State.Health}}yes{{end}}' "$cid" | grep -q yes; then
        ok "[$service] up"
        return 0
    fi

    info "[$service] waiting for healthy (up to ${timeout}s)..."
    local waited=0
    while [ "$waited" -lt "$timeout" ]; do
        state="$(docker inspect -f '{{.State.Health.Status}}' "$cid" 2>/dev/null)"
        case "$state" in
            healthy)   ok "[$service] healthy"; return 0 ;;
            unhealthy) echo -e "${RED}[$service] unhealthy${ENDCOLOR}"; dc logs --tail 30 "$service"; return 1 ;;
        esac
        sleep 2
        waited=$((waited + 2))
    done

    warn "[$service] still $state after ${timeout}s"
    return 1
}

health_check_all() {
    wait_healthy db 60      || true
    wait_healthy app 30     || true
    wait_healthy nginx 60   || true
}

print_endpoints() {
    local port
    port="$(grep -E '^NGINX_PORT=' "$ENV_FILE" 2>/dev/null | tail -1 | cut -d= -f2)"
    port="${port:-8035}"
    echo ""
    ok "  app (via nginx):  http://localhost:${port}"
    ok "  nginx health:     http://localhost:${port}/healthz"
    ok "  postgres:         127.0.0.1:5432"
    echo ""
}
