// Walk the live zone using ordinary intents. No fixtures, grants or teleports.
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
  async function walk(x, z, tol = 1.0) {
    const until = Date.now() + 55000;
    while (true) {
      const dx = x - me().pose.x,
        dz = z - me().pose.z,
        n = Math.hypot(dx, dz);
      if (n < tol) break;
      if (Date.now() > until) throw Error("walking to " + [x, z]);
      send("move_intent", {
        seq: ++seq,
        forward: -dz / n,
        strafe: dx / n,
        yaw: 0,
        sprint: true,
      });
      await sleep(100);
    }
    send("move_intent", {
      seq: ++seq,
      forward: 0,
      strafe: 0,
      yaw: 0,
      sprint: false,
    });
  }
  const inventory = () => snap.marsh.inventory;
  const enemy = (id) => snap.marsh.enemies.find((e) => e.id === id);
  const act = async () => {
    send("interact");
    await sleep(360);
  };
  async function hunt(id) {
    send("attack", { target_id: id });
    const end = Date.now() + 85000;
    while (enemy(id).hp > 0) {
      if (Date.now() > end) throw Error("hunt " + id);
      let e = enemy(id);
      await walk(e.x, e.z, 1.7);
      send("attack", { target_id: id });
      if (me().hp < me().maxHp * 0.55) send("drink");
      await sleep(300);
    }
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
    "PASS Stillwater: personal gathering, roll cost, hunts, physical book, locked gate, crossing, first craft",
  );
} finally {
  ws.close();
}
