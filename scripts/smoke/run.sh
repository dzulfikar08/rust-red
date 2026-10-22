#!/usr/bin/env bash
# End-to-end smoke test for flows exported from Node-RED.
#
#   1. node-red-contrib-modbus -> function -> node-red-contrib-postgresql, against a pymodbus
#      simulator and postgres:16-alpine. The export in crates/core/tests/nodered_compat/fixtures is
#      run unmodified, apart from the addresses of the simulator and the database.
#   2. stock mqtt in / mqtt out with a stock mqtt-broker config node, against eclipse-mosquitto,
#      anonymous and with a user name and password.
#
# Needs docker, python3 and a built rust-red binary:
#
#   scripts/smoke/run.sh target/release/rust-red
#
# Everything it starts (containers, the simulator, rust-red) is stopped again on exit.

set -euo pipefail

BIN=$(realpath "${1:?usage: run.sh <path to the rust-red binary>}")
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
FIXTURES="$ROOT/crates/core/tests/nodered_compat/fixtures"
WORK=$(mktemp -d)
PIDS=()
CONTAINERS=(rr-smoke-pg rr-smoke-mosq)

step() { printf '\n== %s\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }

cleanup() {
    for pid in "${PIDS[@]:-}"; do [ -n "$pid" ] && kill "$pid" 2>/dev/null || true; done
    docker rm -f "${CONTAINERS[@]}" >/dev/null 2>&1 || true
    rm -rf "$WORK"
}
trap cleanup EXIT

wait_for_port() { # port seconds
    for _ in $(seq 1 "$2"); do (exec 3<>"/dev/tcp/127.0.0.1/$1") 2>/dev/null && return 0; sleep 1; done
    fail "nothing listens on port $1"
}

start_rust_red() { # flows.json log
    (cd "$WORK" && exec "$BIN" run --headless --home "$WORK/home" "$1" >"$2" 2>&1) &
    PIDS+=($!)
}

stop_last() {
    local pid=${PIDS[-1]}
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    unset 'PIDS[-1]'
}

# ---------------------------------------------------------------- Modbus -> PostgreSQL
step "Modbus TCP simulator (pymodbus 3.6.9)"
python3 -m venv "$WORK/venv" >/dev/null
"$WORK/venv/bin/pip" install -q "pymodbus==3.6.9"
setsid "$WORK/venv/bin/python" "$ROOT/scripts/smoke/modbus_sim.py" 5020 >"$WORK/sim.log" 2>&1 &
PIDS+=($!)
wait_for_port 5020 60

step "PostgreSQL (postgres:16-alpine)"
docker rm -f "${CONTAINERS[@]}" >/dev/null 2>&1 || true
docker run -d --name rr-smoke-pg -e POSTGRES_PASSWORD=rustred -e POSTGRES_DB=plant \
    -p 127.0.0.1:5433:5432 postgres:16-alpine >/dev/null
for _ in $(seq 1 60); do docker exec rr-smoke-pg pg_isready -U postgres -d plant >/dev/null 2>&1 && break; sleep 1; done
sleep 2
psql_plant() { docker exec rr-smoke-pg psql -U postgres -d plant -tA -c "$1"; }
psql_plant "CREATE TABLE readings (id serial PRIMARY KEY, v1 int, v2 int, v3 int, ts timestamptz DEFAULT now())" >/dev/null

step "node-red-contrib-modbus -> function -> node-red-contrib-postgresql"
python3 - "$FIXTURES/node_red_modbus_postgres.json" "$WORK/modbus_postgres.json" <<'EOF'
import json, sys
flow = json.load(open(sys.argv[1]))
for node in flow:
    if node["id"] == "p1":
        node.update(port="5433", database="plant")
json.dump(flow, open(sys.argv[2], "w"))
EOF
PGUSER=postgres PGPASSWORD=rustred start_rust_red modbus_postgres.json "$WORK/modbus_postgres.log"
sleep 9
stop_last
ROWS=$(psql_plant "SELECT count(*) FROM readings")
FIRST=$(psql_plant "SELECT v1 || ',' || v2 || ',' || v3 FROM readings ORDER BY id LIMIT 1")
echo "rows inserted: $ROWS, first row: $FIRST"
[ "$ROWS" -ge 5 ] || { cat "$WORK/modbus_postgres.log"; fail "expected at least 5 rows, got $ROWS"; }
[ "$FIRST" = "100,200,300" ] || fail "expected the registers 100,200,300, got $FIRST"

# ---------------------------------------------------------------- MQTT
step "mosquitto (eclipse-mosquitto:2)"
cat >"$WORK/mosquitto.conf" <<'EOF'
persistence false
per_listener_settings true
listener 1883 127.0.0.1
allow_anonymous true
listener 1884 127.0.0.1
allow_anonymous false
password_file /mosquitto/config/passwd
EOF
docker run --rm -v "$WORK:/mosq" eclipse-mosquitto:2 sh -c \
    "mosquitto_passwd -c -b /mosq/passwd alice s3cret >/dev/null 2>&1; chmod 0700 /mosq/passwd; chown 1883:1883 /mosq/passwd"
docker run -d --name rr-smoke-mosq --network host \
    -v "$WORK/mosquitto.conf:/mosquitto/config/mosquitto.conf:ro" \
    -v "$WORK/passwd:/mosquitto/config/passwd:ro" eclipse-mosquitto:2 >/dev/null
wait_for_port 1883 30
wait_for_port 1884 30

# One round trip: publish to plant/line1/temp, expect the stock flow to republish on plant/echo.
mqtt_round_trip() { # flows.json log extra_sub_args...
    local flows=$1 log=$2
    shift 2
    start_rust_red "$flows" "$log"
    sleep 4
    docker exec rr-smoke-mosq mosquitto_sub -p 1883 -t plant/echo -C 1 -W 12 >"$WORK/echo.txt" &
    local sub=$!
    sleep 1
    docker exec rr-smoke-mosq mosquitto_pub -p 1883 -t plant/line1/temp -m '{"t":21.5}'
    wait "$sub" || true
    stop_last
}

step "stock mqtt-broker, anonymous"
cp "$FIXTURES/node_red_mqtt_stock.json" "$WORK/mqtt_anon.json"
: >"$WORK/echo.txt"
mqtt_round_trip mqtt_anon.json "$WORK/mqtt_anon.log"
echo "received on plant/echo: $(cat "$WORK/echo.txt")"
grep -q '"t":21.5' "$WORK/echo.txt" || { cat "$WORK/mqtt_anon.log"; fail "no echo through the anonymous broker"; }

step "stock mqtt-broker, user name and password (port 1884)"
python3 - "$FIXTURES/node_red_mqtt_stock.json" "$WORK" <<'EOF'
import json, sys
flow = json.load(open(sys.argv[1]))
for password, name in (("s3cret", "mqtt_auth.json"), ("wrong", "mqtt_badauth.json")):
    for node in flow:
        if node["type"] == "mqtt-broker":
            node["port"] = "1884"
            node["credentials"] = {"user": "alice", "password": password}
    json.dump(flow, open(f"{sys.argv[2]}/{name}", "w"))
EOF
# the broker listener 1884 requires a login, the publisher uses the anonymous listener 1883
: >"$WORK/echo.txt"
mqtt_round_trip mqtt_auth.json "$WORK/mqtt_auth.log"
echo "received on plant/echo: $(cat "$WORK/echo.txt")"
grep -q '"t":21.5' "$WORK/echo.txt" || { cat "$WORK/mqtt_auth.log"; fail "no echo through the authenticated listener"; }

: >"$WORK/echo.txt"
mqtt_round_trip mqtt_badauth.json "$WORK/mqtt_badauth.log"
[ ! -s "$WORK/echo.txt" ] || fail "a wrong password must not get through"
echo "wrong password: refused, as expected"

step "OK"
