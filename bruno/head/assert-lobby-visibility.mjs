#!/usr/bin/env node

// Verify that both matched players can join the assigned lobby and observe
// each other in world updates over the public game-server WebSocket.

const DEFAULT_TIMEOUT_MS = 8_000;

function ensureRuntimeSupportsWebSocket() {
  const nodeVersion = process.versions?.node || "unknown";
  const nodeMajor = Number.parseInt(nodeVersion.split(".")[0], 10);

  if (Number.isNaN(nodeMajor) || nodeMajor < 21) {
    throw new Error(
      `assert-lobby-visibility.mjs requires Node >=21; current runtime is ${nodeVersion}`,
    );
  }

  if (typeof WebSocket !== "function") {
    throw new Error(
      "assert-lobby-visibility.mjs requires a runtime with built-in WebSocket support",
    );
  }
}

function parseArgs() {
  const payload = process.argv[2];
  if (!payload) {
    throw new Error("expected JSON payload argument");
  }

  const parsed = JSON.parse(payload);
  const requiredFields = [
    "wsUrl",
    "lobbyId",
    "playerAId",
    "playerASessionToken",
    "playerBId",
    "playerBSessionToken",
  ];

  for (const field of requiredFields) {
    if (typeof parsed[field] !== "string" || parsed[field].trim() === "") {
      throw new Error(`missing required field: ${field}`);
    }
  }

  return {
    wsUrl: parsed.wsUrl,
    lobbyId: parsed.lobbyId,
    playerAId: parsed.playerAId,
    playerASessionToken: parsed.playerASessionToken,
    playerBId: parsed.playerBId,
    playerBSessionToken: parsed.playerBSessionToken,
    timeoutMs:
      typeof parsed.timeoutMs === "number" && Number.isFinite(parsed.timeoutMs)
        ? parsed.timeoutMs
        : DEFAULT_TIMEOUT_MS,
  };
}

function buildLobbyUrl(wsUrl, lobbyId) {
  const url = new URL(wsUrl);
  url.searchParams.set("lobby_id", lobbyId);
  return url.toString();
}

function closeSocket(socket) {
  try {
    socket.close(1000, "visibility-check-complete");
  } catch (_error) {
    // Best-effort close only. The process exits immediately after verification.
  }
}

function connectAndObserve({
  name,
  wsUrl,
  lobbyId,
  sessionToken,
  expectedIdentity,
  visiblePlayerIds,
  timeoutMs,
}) {
  const socket = new WebSocket(buildLobbyUrl(wsUrl, lobbyId));
  let settled = false;
  let identityVerified = false;

  const completion = new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      finish(
        new Error(
          `${name} did not observe players ${visiblePlayerIds.join(", ")} within ${timeoutMs}ms`,
        ),
      );
    }, timeoutMs);

    function finish(error, result) {
      if (settled) {
        return;
      }

      settled = true;
      clearTimeout(timeout);

      if (error) {
        closeSocket(socket);
        reject(error);
        return;
      }

      resolve(result);
    }

    socket.addEventListener("open", () => {
      socket.send(
        JSON.stringify({
          type: "Join",
          data: {
            session_token: sessionToken,
          },
        }),
      );
    });

    socket.addEventListener("message", async (event) => {
      try {
        const raw =
          typeof event.data === "string" ? event.data : await event.data.text();
        const message = JSON.parse(raw);

        if (message.type === "Identity") {
          const actualIdentity = String(message.data.player_id);
          if (actualIdentity !== expectedIdentity) {
            finish(
              new Error(
                `${name} joined as ${actualIdentity}, expected ${expectedIdentity}`,
              ),
            );
            return;
          }

          identityVerified = true;
          return;
        }

        if (message.type !== "WorldUpdate") {
          return;
        }

        if (!identityVerified) {
          finish(new Error(`${name} received world update before identity ack`));
          return;
        }

        const entityIds = new Set(
          (message.data.entities || []).map((entity) => String(entity.id)),
        );
        const sawAllPlayers = visiblePlayerIds.every((playerId) =>
          entityIds.has(playerId),
        );

        if (!sawAllPlayers) {
          return;
        }

        finish(null, {
          player: name,
          tick: message.data.tick,
          visiblePlayerIds,
        });
      } catch (error) {
        finish(error);
      }
    });

    socket.addEventListener("error", () => {
      finish(new Error(`${name} WebSocket connection failed`));
    });

    socket.addEventListener("close", (event) => {
      if (settled) {
        return;
      }

      finish(
        new Error(
          `${name} WebSocket closed before visibility check completed (${event.code})`,
        ),
      );
    });
  });

  return {
    completion,
    close() {
      closeSocket(socket);
    },
  };
}

async function main() {
  ensureRuntimeSupportsWebSocket();

  const args = parseArgs();
  const visiblePlayerIds = [args.playerAId, args.playerBId];
  const observers = [
    connectAndObserve({
      name: "player-a",
      wsUrl: args.wsUrl,
      lobbyId: args.lobbyId,
      sessionToken: args.playerASessionToken,
      expectedIdentity: args.playerAId,
      visiblePlayerIds,
      timeoutMs: args.timeoutMs,
    }),
    connectAndObserve({
      name: "player-b",
      wsUrl: args.wsUrl,
      lobbyId: args.lobbyId,
      sessionToken: args.playerBSessionToken,
      expectedIdentity: args.playerBId,
      visiblePlayerIds,
      timeoutMs: args.timeoutMs,
    }),
  ];

  try {
    const results = await Promise.all(
      observers.map((observer) => observer.completion),
    );
    process.stdout.write(`${JSON.stringify({ ok: true, results })}\n`);
  } finally {
    for (const observer of observers) {
      observer.close();
    }
  }
}

main().catch((error) => {
  process.stderr.write(`${error.message}\n`);
  process.exit(1);
});
