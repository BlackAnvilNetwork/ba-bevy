use super::*;
use ba_protocol::stillwater::*;
use serde_json::json;

#[derive(Clone)]
pub struct Journey {
    pub stamina: u8,
    pub roll_until: u64,
    pub roll: MoveAxes,
    pub immune_until: u64,
    pub revive: u64,
    pub warm_until: u64,
    pub warm_ticks: u64,
    pub rest_ready: u64,
    pub slow: u64,
    pub freeze: u64,
    next_action: u64,
    pub notice: String,
}
impl Default for Journey {
    fn default() -> Self {
        Self {
            stamina: 100,
            roll_until: 0,
            roll: MoveAxes {
                forward: 1.,
                strafe: 0.,
                yaw: 0.,
                sprint: true,
            },
            immune_until: 0,
            revive: 0,
            warm_until: 0,
            warm_ticks: 0,
            rest_ready: 0,
            slow: 0,
            freeze: 0,
            next_action: 0,
            notice: "Pell: Don't stand in the water. The eels are polite, but they're still eels."
                .into(),
        }
    }
}
pub fn opening() -> Pose {
    if std::env::var("BA_ZONE_START").as_deref() == Ok("greymere") {
        DEFAULT_SPAWN
    } else {
        POST
    }
}
fn count(p: &Player, item: &str) -> usize {
    p.inventory
        .as_array()
        .into_iter()
        .flatten()
        .filter(|v| v.as_str() == Some(item))
        .count()
}
fn add(p: &mut Player, item: &str, n: usize) {
    let n = n.min(64usize.saturating_sub(count(p, item)));
    if let Some(items) = p.inventory.as_array_mut() {
        items.extend((0..n).map(|_| Value::from(item)));
    }
}
fn remove(p: &mut Player, item: &str, n: usize) {
    if let Some(items) = p.inventory.as_array_mut() {
        let mut n = n;
        items.retain(|v| {
            if n > 0 && v.as_str() == Some(item) {
                n -= 1;
                false
            } else {
                true
            }
        });
    }
}
fn flag(p: &mut Player, name: &str) -> bool {
    if count(p, name) > 0 {
        return false;
    }
    add(p, name, 1);
    true
}
fn clock() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
fn gold(p: &mut Player, amount: u32) {
    if let Some(items) = p.inventory.as_array_mut() {
        if let Some(wallet) = items.iter_mut().find(|x| x.get("marsh_gold").is_some()) {
            wallet["marsh_gold"] = json!(wallet["marsh_gold"]
                .as_u64()
                .unwrap_or(0)
                .saturating_add(amount as u64));
        } else {
            items.push(json!({"marsh_gold":amount}));
        }
    }
}
fn quest(p: &Player) -> &'static str {
    if count(p, "@pell") == 0 {
        "Waking · Meet Pell at the blue lantern"
    } else if count(p, "@hounds") == 0 {
        "Hounds at the Post · Defeat 4 hounds, then visit Pell"
    } else if count(p, "@toads") == 0 {
        "Toads in the Shallows · Defeat 5 toads, collect 3 hides, visit Pell"
    } else if count(p, "@bones") == 0 {
        "Bones for the Keeper · Bring 6 splinters, 4 peat and 3 iron to the Fen Shrine"
    } else if count(p, YOKE) == 0 {
        "The Ox That Wouldn't Lie Down · Find the elite east of the ribs"
    } else {
        "The Drowned Gate · Level 3, three iron, the yoke and Edda's Book"
    }
}
fn speak(p: &mut Player) {
    if flag(p, "@pell") {
        for item in ["Dried Fish", "Pell's Worn Hatchet", "Pell's Cracked Vest"] {
            add(p, item, 1);
        }
        add(p, "Minor Healing Potion", 2);
        gold(p, 10);
    }
    if count(p, "@hound") >= 4 && flag(p, "@hounds") {
        add(p, "Minor Healing Potion", 1);
        add(p, "Beast Hide", 1);
        gold(p, 25);
    }
    if count(p, "@hounds") > 0
        && count(p, "@toad") >= 5
        && count(p, "Toad Hide") >= 3
        && flag(p, "@toads")
    {
        add(p, "Toad Fat", 1);
        gold(p, 40);
    }
    if dist_xz(p.pose, SHRINE) < 5.
        && count(p, "@toads") > 0
        && count(p, "Bone Splinters") >= 6
        && count(p, "Peat") >= 4
        && count(p, "Cold Iron") >= 3
        && flag(p, "@bones")
    {
        add(p, "Pell's Starter Pouch", 1);
        add(p, "Cold Iron", 3);
        gold(p, 75);
    }
    p.journey.notice = format!(
        "Pell: {}. Keep what you gather. Cael has the forge.",
        quest(p)
    );
}
pub fn warmth(p: &mut Player, tick: u64) {
    if dist_xz(p.pose, POST) < 4.5 {
        p.journey.warm_ticks += 1;
        if p.journey.warm_ticks >= 100 {
            p.journey.warm_until = tick + 24000;
        }
    } else {
        p.journey.warm_ticks = 0;
    }
    let base = starter_max_hp(p.job);
    let warm = p.journey.warm_until > tick;
    p.max_hp = if warm { base + base / 20 } else { base };
    p.hp = p.hp.min(p.max_hp);
    if warm && tick % 100 == 0 && p.hp > 0 && p.engage_target.is_none() {
        p.hp = (p.hp + 3).min(p.max_hp);
    }
}
pub fn reward(p: &mut Player, id: u32, site: Pose, roll: u16, rain: bool) {
    let (mut loot, rare) = match id {
        70000..=70001 | 70012..=70013 => (vec!["Beast Hide"], false),
        70002..=70004 => (vec!["Toad Hide", "Toad Fat"], false),
        70005..=70007 => (vec!["Bone Splinters"], false),
        70008..=70009 => (vec!["Eel Skin"], false),
        OX => (
            vec![
                YOKE,
                "Drowned Ox Hide",
                "Cold Iron",
                "Cold Iron",
                "Toad Fat",
            ],
            true,
        ),
        EDDA => (vec![BOOK], true),
        u32::MAX => (vec!["ashworks:raw_cinder"], false),
        id if id == u32::MAX - 1 => (
            vec!["ashworks:raw_cinder", "ashworks:kiln_heart", "Star Iron"],
            true,
        ),
        _ => return,
    };
    if marsh(site.x) {
        if matches!(id, 70000..=70001) {
            add(p, "@hound", 1);
        }
        if matches!(id, 70002..=70004) {
            add(p, "@toad", 1);
        }
        let coins = match id {
            OX => {
                if flag(p, "@ox") {
                    150
                } else {
                    35
                }
            }
            70000..=70001 => 5 + roll as u32 % 6,
            70002..=70004 => 8 + roll as u32 % 5,
            _ => 3 + roll as u32 % 4,
        };
        gold(p, coins);
    }
    let extra = roll < if id == u32::MAX { 800 } else { 1200 };
    if extra {
        loot.push(if marsh(site.x) {
            "Stillwater Essence"
        } else {
            "Bright Cinder"
        });
    }
    if id == OX {
        if let Some(amber) = match roll {
            0..=4 => Some("Primal Bog Amber"),
            5..=34 => Some("Ancient Bog Amber"),
            35..=199 => Some("Golden Bog Amber"),
            _ => None,
        } {
            loot.push(amber);
        }
    }
    if rain && loot.contains(&"Stillwater Essence") {
        loot.push("Stillwater Essence");
    }
    if let Some(items) = p.inventory.as_array_mut() {
        // Persist one bundle per source with this character, including unclaimed quest drops.
        let same = items
            .iter()
            .position(|v| v.get("source").and_then(Value::as_u64) == Some(id as u64));
        if let Some(i) = same {
            // Stack later kills into the same visible bundle without unbounded save growth.
            if extra || rare {
                items[i]["rare"] = json!(true);
            }
            if let Some(old) = items[i]["ground"].as_array_mut() {
                for name in loot {
                    if old.len() < 64 {
                        old.push(json!(name));
                    }
                }
            }
        } else {
            items.push(json!({"source":id,"ground":loot,"x":site.x,"z":site.z,"rare":rare||extra,"born":clock()}));
        }
    }
}
fn drop_from(v: &Value, now: f64) -> Option<Drop> {
    Some(Drop {
        x: v["x"].as_f64()? as f32,
        z: v["z"].as_f64()? as f32,
        items: v["ground"]
            .as_array()?
            .iter()
            .filter_map(|x| x.as_str().map(str::to_owned))
            .collect(),
        rare: v["rare"].as_bool()?,
        age: (now - v["born"].as_f64()?).clamp(0., 10.) as f32,
    })
}
pub fn view(w: &World, id: u32) -> MarshView {
    let Some(p) = w.players.get(&id) else {
        return MarshView::default();
    };
    let mut v = MarshView {
        rain: (w.tick / 20) % 360 >= 310,
        roll_progress: if p.journey.roll_until > w.tick {
            1. - (p.journey.roll_until - w.tick) as f32 / 9.
        } else {
            0.
        },
        stamina: p.journey.stamina,
        rolling: p.journey.roll_until > w.tick,
        dead: p.hp <= 0,
        crossed: rest(p.pose.x),
        notice: p.journey.notice.clone(),
        ..Default::default()
    };
    let now = clock();
    for item in p.inventory.as_array().into_iter().flatten() {
        if let Some(name) = item.as_str() {
            if !name.starts_with('@') {
                *v.inventory.entry(name.into()).or_default() += 1;
            }
        } else if let Some(coins) = item["marsh_gold"].as_u64() {
            v.inventory
                .insert("Gold".into(), coins.min(u32::MAX as u64) as u32);
        } else if let Some(drop) = drop_from(item, now) {
            v.drops.push(drop);
        }
    }
    v.quest = quest(p).into();
    for k in &w.kilns {
        if marsh(k.spec.x) && marsh(p.pose.x) {
            v.enemies.push(k.wire(w.tick));
        }
    }
    v
}
pub fn dodge(p: &mut Player, tick: u64, mut axes: MoveAxes) {
    if !axes.valid() || p.hp <= 0 || p.journey.stamina < 35 || tick < p.journey.roll_until + 5 {
        return;
    }
    if axes.forward.abs() + axes.strafe.abs() < 0.01 {
        axes.forward = 1.;
    }
    p.journey.stamina -= 35;
    p.journey.immune_until = tick + 5;
    p.journey.roll_until = tick + 9;
    p.journey.roll = axes;
}
pub fn interact(p: &mut Player, tick: u64, blind_clear: bool) {
    if p.hp <= 0 || tick < p.journey.next_action {
        return;
    }
    p.journey.next_action = tick + 5;
    let now = clock();
    if let Some((i, drop)) = p
        .inventory
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(i, v)| Some((i, drop_from(v, now)?)))
        .filter(|(_, d)| d.age >= 0.75 && dist_xz(p.pose, at(d.x, d.z)) < 3.6)
        .min_by(|(_, a), (_, b)| {
            dist_xz(p.pose, at(a.x, a.z)).total_cmp(&dist_xz(p.pose, at(b.x, b.z)))
        })
    {
        p.inventory.as_array_mut().unwrap().remove(i);
        p.journey.notice = format!("Collected {}", drop.items.join(", "));
        for item in drop.items {
            if (item == BOOK || item == YOKE || item == "ashworks:kiln_heart")
                && count(p, &item) > 0
            {
                continue;
            }
            add(p, &item, 1);
        }
        return;
    }
    let here = p.pose;
    let near = |pt| dist_xz(here, pt) < 4.5;
    if near(at(28., -1.5)) {
        p.pose = POST;
        p.movement = None;
        p.engage_target = None;
        return;
    }
    if near(GATE) {
        if count(p, BOOK) > 0 && count(p, YOKE) > 0 && count(p, "Cold Iron") >= 3 && p.level >= 3 {
            if flag(p, "@crossed") {
                gold(p, 200);
                for (item, n) in [
                    ("Cold Iron", 2),
                    ("Beast Hide", 2),
                    ("Stillwater Essence", 1),
                    ("Peat", 2),
                ] {
                    add(p, item, n);
                }
            }
            p.pose = REST;
            p.movement = None;
            p.engage_target = None;
            p.journey.notice =
                "The ward falls silent. The ferryman poles north. Sable Rest — Cael's field forge."
                    .into();
        } else {
            p.journey.notice =
                "The Drowned Gate awaits level 3, the Ox's Yoke, three Cold Iron and Edda's Book."
                    .into();
        }
    } else if near(REST) {
        if count(p, "Peat") >= 1 && count(p, "Cold Iron") >= 4 && count(p, "Cael's Iron Edge") == 0
        {
            remove(p, "Cold Iron", 4);
            remove(p, "Peat", 1);
            add(p, "Cael's Iron Edge", 1);
            p.journey.notice = "Cael quenches your iron edge. +25% auto-attack power.".into();
        } else {
            p.journey.notice = "Cael: Four cold iron, one peat. Bring them to my fire.".into();
        }
    } else if near(POST) {
        speak(p);
    } else if near(SHRINE) {
        if tick >= p.journey.rest_ready {
            p.hp = p.max_hp;
            p.journey.rest_ready = tick + 6000;
        }
        speak(p);
    } else if near(BLIND) && !blind_clear {
        p.journey.notice = "Three toads guard the rack. Clear the blind first.".into();
    } else if near(BLIND) && flag(p, "@cache") {
        add(p, "Minor Healing Potion", 2);
        add(p, "Cold Iron", 3);
        p.journey.notice = "Under the hide rack: two potions and three cold iron.".into();
    } else if let Some(i) = HERONS.iter().position(|pt| near(*pt)) {
        flag(p, &format!("@heron{i}"));
        if (0..3).all(|n| count(p, &format!("@heron{n}")) > 0) && flag(p, "@patience") {
            add(p, "Stillwater Essence", 1);
            p.journey.notice =
                "The Keeper whispers: Even the water remembers. Heron's Patience.".into();
        } else {
            p.journey.notice = "The heron tilts its head and listens with you.".into();
        }
    } else if let Some((i, (_, mat))) = NODES.iter().enumerate().find(|(_, (pt, _))| near(*pt)) {
        if flag(p, &format!("@node{i}")) {
            add(p, mat, 3);
            p.journey.notice = format!("Gathered 3 {mat}.");
        } else {
            p.journey.notice = "You have gathered this bank.".into();
        }
    }
}
pub fn drink(p: &mut Player, tick: u64) {
    if p.hp <= 0 || p.hp == p.max_hp || tick < p.journey.next_action {
        return;
    }
    for item in ["Dried Fish", "Minor Healing Potion"] {
        if count(p, item) > 0 {
            remove(p, item, 1);
            p.hp = (p.hp + if item == "Dried Fish" { 20 } else { 55 }).min(p.max_hp);
            p.journey.next_action = tick + 20;
            p.journey.notice = format!("Used {item}.");
            break;
        }
    }
}
