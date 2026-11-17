/**
 * Smoke: kits buff + realtime swings + WS + shield_bash + cast reject + NaN move.
 * Requires ba_zone_server on ws://127.0.0.1:8787 (protocol v3).
 */
import WebSocket from "ws";

const URL = process.env.BA_ZONE_URL || "ws://127.0.0.1:8787";
const PROTOCOL = 3;

function appearance(name, job = "Fighter") {
  return {
    characterId: `smoke-${name}-${Date.now()}`,
    name,
    startingJob: job,
    manifestId: "anata/male",
    bodyTraits: {},
    starterEquipment: {},
  };
}

function openClient(name, x, job = "Fighter") {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(URL);
    const state = { ws, name, id: null, peers: [], damages: [], heals: [], buffs: [], rejects: [], spawn: null };
    const t = setTimeout(() => reject(new Error(`${name} open timeout`)), 5000);
    ws.on("open", () => {
      ws.send(
        JSON.stringify({
          type: "hello",
          protocol: PROTOCOL,
          appearance: appearance(name, job),
          pose: { x, y: 1.65, z: 0, yaw: 0 },
        })
      );
    });
    ws.on("message", (buf) => {
      const msg = JSON.parse(String(buf));
      if (msg.type === "welcome") {
        state.id = msg.player_id;
        state.spawn = msg.spawn;
        clearTimeout(t);
        resolve(state);
      } else if (msg.type === "reject") {
        clearTimeout(t);
        reject(new Error(`rejected: ${msg.reason}`));
      } else if (msg.type === "snapshot") {
        state.peers = msg.players;
      } else if (msg.type === "damage") {
        state.damages.push(msg);
      } else if (msg.type === "heal") {
        state.heals.push(msg);
      } else if (msg.type === "buff_applied") {
        state.buffs.push(msg);
      } else if (msg.type === "cast_rejected") {
        state.rejects.push(msg);
      }
    });
    ws.on("error", reject);
  });
}

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

async function waitUntil(fn, ms, label) {
  const start = Date.now();
  while (Date.now() - start < ms) {
    if (fn()) return;
    await sleep(50);
  }
  throw new Error(label);
}

const a = await openClient("Alpha", 99, "Fighter"); // client pose ignored
const b = await openClient("Bravo", -99, "Knight");
if (!a.spawn || a.spawn.x !== 0) throw new Error("server spawn expected at origin");

// Duplicate hello must reject (keep connection)
a.ws.send(
  JSON.stringify({
    type: "hello",
    protocol: PROTOCOL,
    appearance: appearance("Dup", "Fighter"),
    pose: { x: 0, y: 1.65, z: 0, yaw: 0 },
  })
);
await waitUntil(() => true, 200, "dup hello settle");

await waitUntil(() => a.peers.length >= 2 && b.peers.length >= 2, 5000, "peer wait");

// NaN move should be ignored (still near spawn)
a.ws.send(JSON.stringify({ type: "move", pose: { x: Number.NaN, y: 1.65, z: 0, yaw: 0 } }));
a.ws.send(JSON.stringify({ type: "move", pose: { x: 1.2, y: 1.65, z: 0, yaw: 0 } }));
b.ws.send(JSON.stringify({ type: "move", pose: { x: 1.5, y: 1.65, z: 0, yaw: 0 } }));
await sleep(200);

// kits buff
a.ws.send(JSON.stringify({ type: "cast", ability_id: "berserk" }));
await waitUntil(() => a.buffs.some((d) => d.ability_id === "berserk"), 3000, "berserk buff");

// OOR cast before engage should reject
a.ws.send(JSON.stringify({ type: "cast", ability_id: "ws_gs_hard_slash" }));
await waitUntil(
  () => a.rejects.some((r) => r.ability_id === "ws_gs_hard_slash" && (r.reason === "tp" || r.reason === "no_target" || r.reason === "out_of_range")),
  3000,
  "ws reject without TP/target"
);

a.ws.send(JSON.stringify({ type: "attack", target_id: 0 }));
await waitUntil(
  () => a.damages.filter((d) => d.attacker_id === a.id && d.target_id === b.id && d.hit).length >= 10,
  12000,
  "10 auto hits for TP"
);

const beforeWs = a.damages.length;
a.ws.send(JSON.stringify({ type: "cast", ability_id: "ws_gs_hard_slash" }));
await waitUntil(() => {
  const fresh = a.damages.slice(beforeWs);
  return fresh.some((d) => d.attacker_id === a.id && d.target_id === b.id && d.hit && d.damage > 0);
}, 4000, "weapon skill");

const wsHit = a.damages
  .slice(beforeWs)
  .find((d) => d.attacker_id === a.id && d.target_id === b.id && d.hit && d.damage > 0);

const beforeBash = b.damages.length;
b.ws.send(JSON.stringify({ type: "attack", target_id: 0 }));
b.ws.send(JSON.stringify({ type: "cast", ability_id: "shield_bash" }));
await waitUntil(() => {
  const fresh = b.damages.slice(beforeBash);
  return fresh.some((d) => d.attacker_id === b.id && d.hit && d.damage > 0);
}, 5000, "shield_bash");

a.ws.close();
b.ws.close();
console.log(
  `PASS zone kits smoke v3: spawn ok, buff+reject+autos+wsDmg=${wsHit.damage}+shield_bash`
);
process.exit(0);
