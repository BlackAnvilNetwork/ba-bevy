// Walk the live zone and use its travel points through ordinary player inputs.
import assert from "node:assert/strict";
import WebSocket from "ws";
const ws = new WebSocket(process.env.BA_ZONE_URL || "ws://127.0.0.1:8787");
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let welcome,
  snap,
  seq = 0;
ws.on("message", (data) => {
  const m = JSON.parse(String(data));
  if (m.type === "welcome") welcome = m;
  if (m.type === "snapshot") snap = m;
});
const send = (type, fields = {}) =>
  ws.send(JSON.stringify({ type, ...fields }));
async function wait(check, desc, limit = 7000) {
  const start = Date.now();
  while (!check()) {
    if (Date.now() - start > limit)
      throw Error(desc + " · " + (snap?.marsh?.notice || ""));
    await sleep(50);
  }
}
await new Promise((resolve) => ws.once("open", resolve));
send("hello", {
  protocol: 5,
  appearance: {
    characterId: "marsh-smoke",
    name: "Marsh walker",
    startingJob: "Fighter",
    manifestId: "anata/male",
  },
  pose: { x: 0, y: 1.65, z: 0, yaw: 0 },
});
try {
  await wait(() => welcome && snap, "join");
  assert.equal(welcome.spawn.x, 300);
  const me = () => snap.players.find((p) => p.playerId === welcome.player_id);
  // A moving enemy can turn mid-step. Steer from the latest snapshot on each tick.
  function steer(x, z, tolerance) {
    const pose = me().pose;
    const dx = x - pose.x;
    const dz = z - pose.z;
    const distance = Math.hypot(dx, dz);
    const arrived = distance < tolerance;
    send("move_intent", {
      seq: ++seq,
      forward: arrived ? 0 : -dz / distance,
      strafe: arrived ? 0 : dx / distance,
      yaw: 0,
      sprint: !arrived,
    });
    return arrived;
  }
  async function walk(x, z, tolerance = 1) {
    const deadline = Date.now() + 55000;
    while (!steer(x, z, tolerance)) {
      if (Date.now() > deadline) throw Error("walking to " + [x, z]);
      await sleep(100);
    }
  }
  const inventory = () => snap.marsh.inventory;
  const enemy = (id) => snap.marsh.enemies.find((e) => e.id === id);
  const act = async () => {
    send("interact");
    await sleep(360);
  };
  async function hunt(id) {
    send("attack", { target_id: id });
    const deadline = Date.now() + 100000;
    let nextSkill = 0;
    let nextRoll = 0;
    if (id === 70011) send("cast", { ability_id: "berserk" });
    while (enemy(id).hp > 0) {
      const now = Date.now();
      const target = enemy(id);
      if (now > deadline) throw Error(`hunt ${id}: enemy HP ${target.hp}, player HP ${me().hp}`);
      const inReach = steer(target.x, target.z, 1.6);
      send("attack", { target_id: id });
      if (target.windup > 0.8 && now >= nextRoll && snap.marsh.stamina >= 35) {
        send("dodge", { forward: 0, strafe: 1, yaw: target.yaw });
        nextRoll = now + 1000;
      }
      if (inReach && now >= nextSkill) {
        send("cast", { ability_id: "ws_gs_hard_slash" });
        nextSkill = now + 5000;
      }
      if (me().hp < me().maxHp * 0.65) send("drink");
      await sleep(110);
    }
    console.log(`Defeated ${id}`);
    await wait(() => snap.marsh.drops.some((d) => d.age >= 0.8), "drop reveal");
    const drop = snap.marsh.drops[0];
    await walk(drop.x, drop.z);
    await act();
  }
  await act();
  assert(inventory()["Pell's Worn Hatchet"]);
  await walk(300, 103);
  await act();
  await wait(() => me().pose.x < 30, "Stillwater to Greymere");
  await walk(28, -1.5);
  await act();
  await wait(() => me().pose.x === 300, "Greymere to the Lantern Post");
  assert.equal(inventory()["Pell's Worn Hatchet"], 1);
  send("dodge", { forward: 1, strafe: 0, yaw: 0 });
  await wait(() => snap.marsh.stamina < 100, "paid roll");
  await walk(278.5, 62);
  await act();
  assert.equal(inventory()["Cold Iron"], 3);
  await walk(284, 56);
  await act();
  assert.equal(inventory().Peat, 3);
  await hunt(70000);
  await hunt(70001);
  await walk(247, -28);
  await act();
  await walk(298, -98);
  await act();
  assert.equal(snap.marsh.crossed, false);
  await hunt(70011);
  assert.equal(inventory()["Edda's Book"], 1);
  await walk(300, -98);
  await act();
  assert.equal(snap.marsh.crossed, false);
  await walk(246, -29);
  await act();
  await hunt(70010);
  assert.equal(inventory()["Ox's Yoke"], 1);
  for (const foe of [70005, 70006, 70007, 70002, 70003]) {
    if (me().level >= 3) break;
    if (enemy(foe).hp > 0) await hunt(foe);
  }
  await walk(300, -98);
  await act();
  await wait(() => snap.marsh.crossed, "book opens road");
  await act();
  await wait(() => inventory()["Cael's Iron Edge"], "field forge");
  const packAfterCraft = { ...inventory() };
  await walk(600, 15);
  await act();
  await wait(() => !snap.marsh.crossed && me().pose.z < -98, "return ferry");
  await walk(300, -98);
  await act();
  await wait(() => snap.marsh.crossed, "unlocked passage after crafting");
  assert.deepEqual(inventory(), packAfterCraft, "crossing twice must preserve the pack and rewards");
  console.log(
    "PASS Stillwater: both travel round trips, roll cost, gathering, hunts, book gate, forge, and one-time rewards",
  );
} finally {
  ws.close();
}
