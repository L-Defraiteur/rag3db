#!/bin/sh
set -eu
# Before PG start: watchdog is inside this container's PID namespace/cgroup.
(
  sleep 1740
  printf '{"event":"TTL_TERM","nonce":"%s"}\n' "$G_PG_FIXTURE_NONCE" >> /fixture-lifecycle/ttl.jsonl
  kill -TERM 1
  sleep 10
  printf '{"event":"TTL_KILL","nonce":"%s"}\n' "$G_PG_FIXTURE_NONCE" >> /fixture-lifecycle/ttl.jsonl
  kill -KILL 1
) &
printf '{"event":"TTL_ARMED","nonce":"%s"}\n' "$G_PG_FIXTURE_NONCE" >> /fixture-lifecycle/ttl.jsonl
exec /usr/local/bin/docker-entrypoint.sh "$@"
