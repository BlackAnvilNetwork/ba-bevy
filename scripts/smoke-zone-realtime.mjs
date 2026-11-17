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
        state.ashworks = msg.ashworks;state.marsh=msg.marsh;
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

const imp = a.ashworks.constructs[0].id;
const send = (client, msg) => client.ws.send(JSON.stringify(msg));
const strike = (client, target_id) => send(client, { type: "attack", target_id });
const cast = (client, ability_id) => send(client, { type: "cast", ability_id });

strike(a, b.id);
await sleep(900);
if (a.damages.some(d => d.attacker_id === a.id)) throw new Error("player target accepted");
cast(a, "ws_gs_hard_slash");
await waitUntil(() => a.rejects.some(r => r.reason === "tp"), 3000, "weapon skill must require TP");
cast(b, "shield_bash");
await waitUntil(() => b.damages.some(d => d.attacker_id === b.id && d.target_id === imp), 3000, "bash construct");
cast(a, "berserk");
await waitUntil(() => a.buffs.some(d => d.ability_id === "berserk"), 3000, "berserk buff");

async function hunt() {
  await waitUntil(() => a.ashworks.constructs[0].hp > 0, 9000, "imp rekindles");
  const stock = a.ashworks.cinder;
  strike(a, imp);
  await waitUntil(() => a.marsh.drops.some(d=>d.age>=0.8),30000,"ground bundle");
  send(a,{type:"interact"});send(b,{type:"interact"});
  await waitUntil(() => a.ashworks.cinder > stock, 30000, "earn raw cinder");
}
await hunt();
if (b.ashworks.cinder !== 1) await waitUntil(() => b.ashworks.cinder === 1, 2000, "shared contribution reward");
await hunt();
send(a, { type: "forge", recipe: "kiln_ward" });
await sleep(300);
if (a.ashworks.ward || a.ashworks.cinder !== 2) throw new Error("crafted away from forge");

let seq = 83;
async function walkTo(x, z) {
  const deadline = Date.now() + 10000;
  while (Date.now() < deadline) {
    const pose = a.peers.find(p => pid(p) === a.id).pose;
    const dx = x - pose.x, dz = z - pose.z;
    const distance = Math.hypot(dx,dz);
    if (distance < 0.9) {
      send(a, {type:"move_intent",seq:seq++,forward:0,strafe:0,yaw:0,sprint:false});
      return;
    }
    send(a, {type:"move_intent",seq:seq++,forward:-dz/distance,strafe:dx/distance,yaw:0,sprint:true});
    await sleep(90);
  }
  throw new Error("could not reach waypoint");
}
await walkTo(-2.05,0.35);
send(a, { type: "forge", recipe: "kiln_ward" });
send(a, { type: "forge", recipe: "kiln_ward" });
await waitUntil(() => a.ashworks.ward && a.ashworks.cinder === 0, 2000, "forge ward");
if (b.ashworks.ward) throw new Error("ward leaked to other character");
// Finish the forge order with the boss heart. The extra imp also supplies
// the second cinder and enough TP to exercise the weapon skill.
await walkTo(1.6, 4.8);
await hunt();
await walkTo(22,-1.5);
const boss = a.ashworks.constructs[1].id;
strike(a,boss);
await waitUntil(() => a.damages.filter(d => d.attacker_id === a.id && d.hit).length >= 10, 16000, "earn weapon skill");
cast(a,"ws_gs_hard_slash");
// The heavy pulse knocks attackers away. Re-enter melee range after impact.
const untilHeart = Date.now() + 60000;
while (!a.marsh.drops.some(d=>d.age>=0.8)) {
  if (Date.now()>untilHeart) throw new Error("ground heart");
  await walkTo(22,-1.5);
  await sleep(200);
}
await walkTo(22,-1.5);
send(a,{type:"interact"});
await waitUntil(() => a.ashworks.heart, 45000, "kiln heart");
if (a.rejects.filter(r => r.ability_id === "ws_gs_hard_slash").length !== 1) throw new Error("earned weapon skill rejected");
if (a.peers.find(p => pid(p) === a.id).level < 2) throw new Error("XP did not advance level");
await walkTo(-2.05,0.35);
send(a, { type: "forge", recipe: "kiln_ward" });
await sleep(250);
if (a.ashworks.edge || !a.ashworks.heart) throw new Error("replayed ward craft consumed kiln heart");
send(a, { type: "forge", recipe: "tempered_edge" });
await waitUntil(() => a.ashworks.edge && !a.ashworks.heart && a.ashworks.cinder === 0, 2000, "temper edge");
a.ws.close();
b.ws.close();
console.log("PASS cooperative hunt, XP materials, forge distance, crafting and character ownership");
process.exit(0);
