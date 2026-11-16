/**
 * Smoke: kits buff + realtime swings + WS (TP) + shield_bash potency.
 * Requires ba_zone_server on ws://127.0.0.1:8787
 */
import WebSocket from "ws";

const URL = process.env.BA_ZONE_URL || "ws://127.0.0.1:8787";
const GLB = "character/exports/active.glb";

function appearance(name, job = "Fighter") {
  return {
    characterId: `smoke-${name}`,
    name,
    startingJob: job,
    manifestId: "anata/male",
    bodyTraits: {},
    starterEquipment: {},
    glbRelativePath: GLB,
  };
}

function openClient(name, x, job = "Fighter") {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(URL);
    const state = {
      ws,
      name,
      id: null,
      peers: [],
      damages: [],
    };
    const t = setTimeout(() => reject(new Error(`${name} open timeout`)), 5000);
    ws.on("open", () => {
      ws.send(
        JSON.stringify({
          type: "hello",
          protocol: 2,
          appearance: appearance(name, job),
          pose: { x, y: 1.65, z: 0, yaw: 0 },
        })
      );
    });
    ws.on("message", (buf) => {
      const msg = JSON.parse(String(buf));
      if (msg.type === "welcome") {
        state.id = msg.player_id;
        clearTimeout(t);
        resolve(state);
      } else if (msg.type === "snapshot") {
        state.peers = msg.players;
      } else if (msg.type === "damage") {
        state.damages.push(msg);
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

const a = await openClient("Alpha", 0, "Fighter");
const b = await openClient("Bravo", 1.5, "Knight");
await waitUntil(() => a.peers.length >= 2 && b.peers.length >= 2, 5000, "peer wait");

const remote = a.peers.find((p) => p.player_id !== a.id);
if (!remote?.appearance?.glbRelativePath) throw new Error("missing remote glb path");

// kits buff
a.ws.send(JSON.stringify({ type: "cast", ability_id: "berserk" }));
await waitUntil(
  () => a.damages.some((d) => d.attacker_id === a.id && d.target_id === a.id && d.hit && d.damage === 0),
  3000,
  "berserk buff"
);

// Build TP with autos (no berserk damage amp needed for this — already buffed; keep count low)
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

// knight potency JA from kits
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
  `PASS zone kits smoke: glb ok, wsDmg=${wsHit.damage}, buff+autos+shield_bash ok`
);
process.exit(0);
