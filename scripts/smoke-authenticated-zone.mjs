/**
 * End-to-end authenticated zone smoke with two different Discord-backed
 * (isolated CI fixture) accounts. No mock login endpoint exists in production.
 * Verifies: account ownership, character upload, content-hash download,
 * single-use tickets, server-owned appearance, reconnect and DB persistence.
 */
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import WebSocket from "ws";

const API = process.env.BA_GATEWAY_PUBLIC_URL || "http://127.0.0.1:8790";
const WS = process.env.BA_ZONE_URL || "ws://127.0.0.1:8787";
const file = process.env.BA_SMOKE_SESSION_FILE;
if (!file) throw new Error("BA_SMOKE_SESSION_FILE required");
const sessions = JSON.parse(readFileSync(file, "utf8"));

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
async function api(path, token, method = "GET", json) {
  const res = await fetch(API + path, {
    method,
    headers: {
      ...(token ? { Authorization: "Bearer " + token } : {}),
      ...(json ? { "Content-Type": "application/json" } : {}),
    },
    ...(json ? { body: JSON.stringify(json) } : {}),
  });
  const result = await res.json();
  if (!res.ok) throw new Error(`API ${method} ${path} => ${res.status}: ${JSON.stringify(result)}`);
  return result;
}

function makeGLB(label) {
  const json = Buffer.from(JSON.stringify({
    asset: { version: "2.0", generator: "ba-gateway-smoke" },
    scenes: [{ nodes: [0] }], scene: 0,
    nodes: [{ name: "Player" + label }],
  }));
  const padded = Buffer.concat([json, Buffer.alloc((4 - (json.length % 4)) % 4, 0x20)]);
  const out = Buffer.alloc(20 + padded.length);
  out.write("glTF", 0);
  out.writeUInt32LE(2, 4);
  out.writeUInt32LE(out.length, 8);
  out.writeUInt32LE(padded.length, 12);
  out.write("JSON", 16);
  padded.copy(out, 20);
  return out;
}

function character(label, id) {
  return {
    schemaVersion: 1, revision: 1, characterId: id, name: "Smoke " + label,
    startingJob: label === "A" ? "TimeMage" : "Fighter",
    pack: "anata", manifestId: "anata/male",
    catalogVersion: "anata-2026-11", bodyTraits: {}, starterEquipment: {},
    runtime: { format: "glb", relativePath: "this-is-ignored-by-server" },
  };
}

function makeHello(id, ticket, reconnectToken) {
  return {
    type: "hello", protocol: 5,
    appearance: {
      characterId: id, name: "HACKED CLIENT NAME",
      startingJob: "Knight", manifestId: "anata/female",
      bodyTraits: {}, starterEquipment: {},
      glbContentHash: "f".repeat(64),
    },
    pose: { x: 99999, y: 1.65, z: 99999, yaw: 0 },
    ...(ticket ? { ticket } : {}),
    ...(reconnectToken ? { reconnect_token: reconnectToken } : {}),
  };
}

function openZone(hello) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(WS);
    let connected = false;
    const snapshots = [];
    const timer = setTimeout(() => {
      ws.terminate();
      reject(new Error("Zone handshake timed out"));
    }, 7000);
    ws.on("open", () => ws.send(JSON.stringify(hello)));
    ws.on("message", (frame) => {
      let msg;
      try { msg = JSON.parse(String(frame)); } catch { return; }
      if (msg.type === "snapshot") {
        snapshots.push(msg);
        if (snapshots.length > 150) snapshots.shift();
      } else if (msg.type === "welcome") {
        connected = true;
        clearTimeout(timer);
        resolve({ ws, welcome: msg, snapshots });
      } else if (msg.type === "reject" && !connected) {
        clearTimeout(timer);
        ws.close();
        reject(new Error("Zone rejected: " + msg.reason));
      }
    });
    ws.on("error", (error) => {
      if (!connected) { clearTimeout(timer); reject(error); }
    });
    ws.on("close", () => {
      if (!connected) { clearTimeout(timer); reject(new Error("Closed before Welcome")); }
    });
  });
}

async function eventually(predicate, timeout = 4500) {
  const until = Date.now() + timeout;
  while (Date.now() < until) {
    const result = predicate();
    if (result) return result;
    await sleep(50);
  }
  throw new Error("Timed out waiting for zone snapshot predicate");
}

const names = ["A", "B"];
const receipt = {};
for (const suffix of names) {
  const key = suffix.toLowerCase();
  const token = sessions[key].sessionToken;
  const me = await api("/v1/me", token);
  if (me.accountId !== sessions[key].accountId) throw new Error("Wrong Discord owner");
  const c = character(suffix, "smoke-player-" + key);
  const glb = makeGLB(suffix);
  receipt[key] = await api("/v1/characters", token, "POST", {
    character: c, avatarBase64: glb.toString("base64"),
  });
  const expected = createHash("sha256").update(glb).digest("hex");
  if (receipt[key].contentHash !== expected) throw new Error("Wrong GLB hash");
  const asset = await fetch(API + "/v1/assets/" + expected);
  if (!asset.ok) throw new Error("Approved asset unavailable to other clients");
  const downloaded = Buffer.from(await asset.arrayBuffer());
  if (!downloaded.equals(glb)) throw new Error("Downloaded GLB differs from uploaded");
  const selected = await api("/v1/characters/selected", token);
  if (selected.character.characterId !== c.characterId) throw new Error("Wrong account-owned character");
}

const aTicket = await api("/v1/tickets", sessions.a.sessionToken, "POST", { characterId: "smoke-player-a" });
const bTicket = await api("/v1/tickets", sessions.b.sessionToken, "POST", { characterId: "smoke-player-b" });

// One account cannot request a ticket for another account's character.
let ownershipRejected = false;
try {
  await api("/v1/tickets", sessions.a.sessionToken, "POST", { characterId: "smoke-player-b" });
} catch { ownershipRejected = true; }
if (!ownershipRejected) throw new Error("Cross-account character ticket issued");

const a = await openZone(makeHello("smoke-player-a", aTicket.ticket));
const b = await openZone(makeHello("smoke-player-b", bTicket.ticket));
if (!a.welcome.reconnect_token || !b.welcome.reconnect_token) {
  throw new Error("Missing one-time reconnect credentials");
}
const peers = await eventually(() =>
  a.snapshots.find((snapshot) =>
    snapshot.players.some((p) => p.playerId === b.welcome.player_id)
  )
);
const remote = peers.players.find((p) => p.playerId === b.welcome.player_id);
if (remote.appearance.name !== "Smoke B"
    || remote.appearance.startingJob !== "Fighter"
    || remote.appearance.glbContentHash !== receipt.b.contentHash) {
  throw new Error("Server trusted forged client appearance/job/hash");
}
if (Math.abs(a.welcome.spawn.x - 0.2) > .01 || Math.abs(a.welcome.spawn.z - 6.4) > .01) {
  throw new Error("Client-controlled spawn was accepted");
}

// Consumed ticket cannot be redeemed again, regardless of the claimed ID.
let replayFailed = false;
try {
  const replay = await openZone(makeHello("smoke-player-a", aTicket.ticket));
  replay.ws.close();
} catch { replayFailed = true; }
if (!replayFailed) throw new Error("Ticket replay accepted");

for (let seq = 1; seq <= 6; seq++) {
  a.ws.send(JSON.stringify({
    type: "move_intent", seq, forward: 1, strafe: 0, yaw: 0, sprint: false,
  }));
  await sleep(55);
}
a.ws.send(JSON.stringify({
  type: "move_intent", seq: 7, forward: 0, strafe: 0, yaw: 0, sprint: false,
}));
const moved = await eventually(() => a.snapshots.find((snapshot) => {
  const p = snapshot.players.find((entry) => entry.playerId === a.welcome.player_id);
  return p && p.pose.z < a.welcome.spawn.z - 0.12;
}));
const last = moved.players.find((p) => p.playerId === a.welcome.player_id);
a.ws.send(JSON.stringify({ type: "attack", target_id: 0 }));
await eventually(() => a.snapshots.at(-1)?.ashworks?.cinder === 1, 30000);
const closed = new Promise((resolve) => a.ws.once("close", resolve));
a.ws.close();
await closed;
await sleep(500); // permit disconnect's durable state flush

const selectedA = await api("/v1/characters/selected", sessions.a.sessionToken);
if (selectedA.stateRevision < 1) throw new Error("Disconnect did not persist world state");
const rejoined = await openZone(makeHello("smoke-player-a", null, a.welcome.reconnect_token));
if (rejoined.welcome.reconnect_token === a.welcome.reconnect_token) {
  throw new Error("Reconnect token was not rotated");
}
if (rejoined.welcome.spawn.z >= a.welcome.spawn.z - 0.08) {
  throw new Error("Reconnect did not restore server-owned position");
}
await eventually(() => rejoined.snapshots.at(-1)?.ashworks?.cinder === 1);
if (selectedA.xp !== 125) throw new Error("Kill XP did not persist across reconnect");
let oldReconnectRejected = false;
try {
  const old = await openZone(makeHello("smoke-player-a", null, a.welcome.reconnect_token));
  old.ws.close();
} catch { oldReconnectRejected = true; }
if (!oldReconnectRejected) throw new Error("Replayed reconnect credential accepted");

rejoined.ws.close();
b.ws.close();
console.log("PASS authenticated gateway + zone: Discord account ownership, immutable avatars, one-use tickets, secure reconnect and persisted movement");
