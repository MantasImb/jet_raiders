#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_DIR="${ROOT_DIR}/.tmp/ci-smoke-logs"
mkdir -p "${LOG_DIR}"

POSTGRES_CONTAINER_NAME="jet-raiders-ci-smoke-postgres"
POSTGRES_STARTED_BY_SCRIPT=0

AUTH_PID=""
MATCHMAKING_PID=""
GAME_PID=""
HEAD_PID=""

cleanup() {
  local exit_code=$?

  for pid in "${HEAD_PID}" "${GAME_PID}" "${MATCHMAKING_PID}" "${AUTH_PID}"; do
    if [[ -n "${pid}" ]] && kill -0 "${pid}" 2>/dev/null; then
      kill "${pid}" 2>/dev/null || true
    fi
  done

  for pid in "${HEAD_PID}" "${GAME_PID}" "${MATCHMAKING_PID}" "${AUTH_PID}"; do
    if [[ -n "${pid}" ]] && kill -0 "${pid}" 2>/dev/null; then
      wait "${pid}" 2>/dev/null || true
    fi
  done

  if [[ "${POSTGRES_STARTED_BY_SCRIPT}" -eq 1 ]]; then
    docker rm -f "${POSTGRES_CONTAINER_NAME}" >/dev/null 2>&1 || true
  fi

  if [[ "${exit_code}" -ne 0 ]]; then
    echo "ci_smoke failed. service logs are in ${LOG_DIR}" >&2
  fi

  exit "${exit_code}"
}

trap cleanup EXIT INT TERM

wait_for_http_ok() {
  local url="$1"
  local name="$2"
  local max_attempts="${3:-60}"

  local attempt
  for attempt in $(seq 1 "${max_attempts}"); do
    if curl --fail --silent "${url}" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done

  echo "Timed out waiting for ${name} at ${url}" >&2
  return 1
}

extract_json_field() {
  local json="$1"
  local key="$2"

  printf '%s' "${json}" \
    | tr -d '\n' \
    | sed -n "s/.*\"${key}\"[[:space:]]*:[[:space:]]*\"\([^\"]*\)\".*/\1/p"
}

start_ephemeral_postgres_if_needed() {
  if [[ -n "${DATABASE_URL:-}" ]]; then
    return 0
  fi

  if ! command -v docker >/dev/null 2>&1; then
    echo "DATABASE_URL is unset and docker is not available for ephemeral Postgres" >&2
    exit 1
  fi

  docker rm -f "${POSTGRES_CONTAINER_NAME}" >/dev/null 2>&1 || true

  docker run --detach --rm \
    --name "${POSTGRES_CONTAINER_NAME}" \
    --env POSTGRES_USER=jet \
    --env POSTGRES_PASSWORD=jet \
    --env POSTGRES_DB=jet_raiders \
    --publish 55432:5432 \
    postgres:16-alpine >/dev/null

  POSTGRES_STARTED_BY_SCRIPT=1
  export DATABASE_URL="postgres://jet:jet@127.0.0.1:55432/jet_raiders"

  local attempt
  for attempt in $(seq 1 60); do
    if docker exec "${POSTGRES_CONTAINER_NAME}" pg_isready -U jet >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done

  echo "Ephemeral Postgres did not become ready in time" >&2
  exit 1
}

start_services() {
  (
    cd "${ROOT_DIR}/auth_server"
    AUTH_SERVER_BIND_HOST=127.0.0.1 \
    BACKEND_PORTS_CONFIG_PATH=../config/backend_ports.toml \
    DATABASE_URL="${DATABASE_URL}" \
    cargo run
  ) >"${LOG_DIR}/auth_server.log" 2>&1 &
  AUTH_PID=$!

  (
    cd "${ROOT_DIR}/matchmaking_server"
    MATCHMAKING_SERVER_BIND_HOST=127.0.0.1 \
    BACKEND_PORTS_CONFIG_PATH=../config/backend_ports.toml \
    REGION_CONFIG_PATH=../config/regions.toml \
    cargo run
  ) >"${LOG_DIR}/matchmaking_server.log" 2>&1 &
  MATCHMAKING_PID=$!

  (
    cd "${ROOT_DIR}/game_server"
    GAME_SERVER_BIND_HOST=127.0.0.1 \
    GAME_SERVER_PORT=3001 \
    AUTH_SERVICE_URL=http://127.0.0.1:3002 \
    cargo run
  ) >"${LOG_DIR}/game_server.log" 2>&1 &
  GAME_PID=$!

  (
    cd "${ROOT_DIR}/head_server"
    HEAD_SERVER_BIND_HOST=127.0.0.1 \
    BACKEND_PORTS_CONFIG_PATH=../config/backend_ports.toml \
    AUTH_SERVICE_URL=http://127.0.0.1:3002 \
    MATCHMAKING_SERVICE_URL=http://127.0.0.1:3003 \
    REGION_CONFIG_PATH=../config/regions.toml \
    cargo run
  ) >"${LOG_DIR}/head_server.log" 2>&1 &
  HEAD_PID=$!
}

run_smoke_flow() {
  wait_for_http_ok "http://127.0.0.1:3002/health" "auth_server"
  wait_for_http_ok "http://127.0.0.1:3003/health" "matchmaking_server"
  wait_for_http_ok "http://127.0.0.1:3001/health" "game_server"
  wait_for_http_ok "http://127.0.0.1:3000/health" "head_server"

  local guest_one
  guest_one="$(curl --fail --silent \
    --header 'Content-Type: application/json' \
    --data '{"display_name":"SmokePlayerOne"}' \
    http://127.0.0.1:3000/guest/init)"

  local guest_two
  guest_two="$(curl --fail --silent \
    --header 'Content-Type: application/json' \
    --data '{"display_name":"SmokePlayerTwo"}' \
    http://127.0.0.1:3000/guest/init)"

  local session_token_one
  session_token_one="$(extract_json_field "${guest_one}" "session_token")"
  local session_token_two
  session_token_two="$(extract_json_field "${guest_two}" "session_token")"

  if [[ -z "${session_token_one}" || -z "${session_token_two}" ]]; then
    echo "Failed to parse session tokens from /guest/init responses" >&2
    echo "guest_one=${guest_one}" >&2
    echo "guest_two=${guest_two}" >&2
    return 1
  fi

  local queue_one
  queue_one="$(curl --fail --silent \
    --header 'Content-Type: application/json' \
    --data '{"session_token":"'"${session_token_one}"'","player_skill":1200,"region":"eu-west"}' \
    http://127.0.0.1:3000/matchmaking/queue)"

  local queue_two
  queue_two="$(curl --fail --silent \
    --header 'Content-Type: application/json' \
    --data '{"session_token":"'"${session_token_two}"'","player_skill":1201,"region":"eu-west"}' \
    http://127.0.0.1:3000/matchmaking/queue)"

  if ! printf '%s' "${queue_one}" | grep -q '"status"'; then
    echo "first matchmaking queue response did not include status: ${queue_one}" >&2
    return 1
  fi

  if ! printf '%s' "${queue_two}" | grep -q '"status":"matched"'; then
    echo "second matchmaking queue response did not produce matched status: ${queue_two}" >&2
    return 1
  fi

  if ! printf '%s' "${queue_two}" | grep -q '"ws_url"'; then
    echo "matched response did not include ws_url: ${queue_two}" >&2
    return 1
  fi
}

main() {
  start_ephemeral_postgres_if_needed
  start_services
  run_smoke_flow
  echo "ci_smoke passed"
}

main "$@"
