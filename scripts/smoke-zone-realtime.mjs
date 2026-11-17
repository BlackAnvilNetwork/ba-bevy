/**
 * Smoke: kits buff + realtime swings + WS + negative cases (dup hello, flood move, cast reject).
 * Requires ba_zone_server on ws://127.0.0.1:8787 (protocol v5).
 */
import WebSocket from "ws";

const URL = process.env.BA_ZONE_URL || "ws://127.0.0.1:8787";
const PROTOCOL = 5;

function appearance(name, job = "Fighter", characterId) {
  return {
    characterId: characterId || `smoke-${name}-${Date.now()}`.replace(/[^A-Za-z0-9_-]/g, "-"),
    name,
    startingJob: job,
    manifestId: "anata/male",
    bodyTraits: {},
    starterEquipment: {},
  };
}

function openClient(name, job = "Fighter") {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(URL);
    const state = {
      ws,
      name,
      id: null,
      peers: [],
      damages: [],
      buffs: [],
      rejects: [],
      serverRejects: [],
      spawn: null,
    };
    const t = setTimeout(() => reject(new Error(`${name} open timeout`)), 5000);
    ws.on("open", () => {
      ws.send(
        JSON.stringify({
          type: "hello",
          protocol: PROTOCOL,
          appearance: appearance(name, job),
          pose: { x: 99, y: 1.65, z: 0, yaw: 0 },
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
        state.serverRejects.push(msg);
        if (state.id == null) {
          clearTimeout(t);
          reject(new Error(`rejected: ${msg.reason}`));
        }
      } else if (msg.type === "snapshot") {
        state.peers = msg.players;
      } else if (msg.type === "damage") {
        state.damages.push(msg);
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

const a = await openClient("Alpha", "Fighter");
const b = await openClient("Bravo", "Knight");
if (!a.spawn || Math.abs(a.spawn.x - 0.2) > 0.001 || Math.abs(a.spawn.z - 6.4) > 0.001) throw new Error("server spawn must match Greymere Yard zone data");

const idBeforeDup = a.id;
a.ws.send(
  JSON.stringify({
    type: "hello",
    protocol: PROTOCOL,
    appearance: appearance("Dup", "Fighter"),
    pose: { x: 0, y: 1.65, z: 0, yaw: 0 },
  })
);
await waitUntil(
  () => a.serverRejects.some((r) => String(r.reason).includes("duplicate")),
  3000,
  "duplicate hello reject"
);
if (a.id !== idBeforeDup) throw new Error("player id changed after duplicate hello");

await waitUntil(() => a.peers.length >= 2 && b.peers.length >= 2, 5000, "peer wait");

// A forged destination is no longer a valid protocol message.
a.ws.send('{"type":"move","pose":{"x":20,"y":1.65,"z":0,"yaw":0}}');
// NaN is not a valid JSON number and must never mutate the player.
a.ws.send('{"type":"move_intent","seq":1,"forward":NaN,"strafe":0,"yaw":0,"sprint":true}');
// The next 80 intents arrive in a burst. The server must not integrate 80 moves.
for (let i = 2; i <= 81; i++) {
  a.ws.send(JSON.stringify({
    type: "move_intent", seq: i, forward: 1, strafe: 0, yaw: 0, sprint: true,
  }));
}
await sleep(220);
const pid = (p) => p.playerId ?? p.player_id;
const me = a.peers.find((p) => pid(p) === a.id);
if (!me) throw new Error(`self missing from snapshot (peers=${a.peers.length})`);
const distance = Math.hypot(me.pose.x - a.spawn.x, me.pose.z - a.spawn.z);
if (distance > 1.9) {
  throw new Error(`packet burst advanced too far: ${distance.toFixed(3)}m`);
}
const ack = me.lastInputSeq ?? me.last_input_seq;
if (!(ack >= 2 && ack <= 81)) {
  throw new Error(`input acknowledgement missing or invalid: ${ack}`);
}
// Stop moving with an explicit neutral command; late duplicate commands are ignored.
a.ws.send(JSON.stringify({ type: "move_intent", seq: 82, forward: 0, strafe: 0, yaw: 0, sprint: false }));
b.ws.send(JSON.stringify({ type: "move_intent", seq: 1, forward: 0, strafe: 0, yaw: 0, sprint: false }));
await sleep(100);

// Test Knight's shield skill while both combatants are still alive. The
// Fighter's Berserk + 10 hits + weapon skill below can otherwise kill the
// Knight before this assertion, correctly making the server reject casts.
const beforeBash = b.damages.length;
b.ws.send(JSON.stringify({ type: "attack", target_id: 0 }));
b.ws.send(JSON.stringify({ type: "cast", ability_id: "shield_bash" }));
await waitUntil(
  () => b.damages.slice(beforeBash).some(
    (d) => d.attacker_id === b.id && d.target_id === a.id
  ) || b.rejects.some((r) => r.ability_id === "shield_bash"),
  5000, "shield_bash outcome"
);
const bashReject = b.rejects.find((r) => r.ability_id === "shield_bash");
if (bashReject) throw new Error(`shield_bash unexpectedly rejected: ${bashReject.reason}`);

a.ws.send(JSON.stringify({ type: "cast", ability_id: "berserk" }));
await waitUntil(() => a.buffs.some((d) => d.ability_id === "berserk"), 3000, "berserk buff");

a.ws.send(JSON.stringify({ type: "cast", ability_id: "ws_gs_hard_slash" }));
await waitUntil(
  () =>
    a.rejects.some(
      (r) =>
        r.ability_id === "ws_gs_hard_slash" &&
        (r.reason === "tp" || r.reason === "no_target" || r.reason === "out_of_range")
    ),
  3000,
  "ws reject without TP/target"
);

a.ws.send(JSON.stringify({ type: "attack", target_id: 0 }));
await waitUntil(
  () => a.damages.filter((d) => d.attacker_id === a.id && d.target_id === b.id && d.hit).length >= 10,
  14000,
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

a.ws.close();
b.ws.close();
console.log(
  `PASS zone kits smoke v5: dup-hello reject, tick-bound movement, buff+wsDmg=${wsHit.damage}+shield_bash`
);
process.exit(0);
