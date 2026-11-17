use super::*;
use ba_protocol::ashworks::*;
use std::collections::HashSet;

const CINDER: &str = "ashworks:raw_cinder";
const HEART: &str = "ashworks:kiln_heart";
const EDGE: &str = "ashworks:tempered_edge";
const WARD: &str = "ashworks:kiln_ward";

pub(super) struct Kiln {
    pub spec: &'static Construct,
    pub hp: i32,
    strike: Option<u64>,
    ready: u64,
    contributors: HashSet<u32>,
}

impl Kiln {
    pub fn new(spec: &'static Construct) -> Self {
        Self {
            spec,
            hp: spec.hp,
            strike: None,
            ready: 0,
            contributors: HashSet::new(),
        }
    }
    pub fn pose(&self) -> Pose {
        Pose {
            x: self.spec.x,
            z: self.spec.z,
            ..DEFAULT_SPAWN
        }
    }
}

fn supplies(inventory: &Value) -> (u32, bool) {
    let mut cinder = 0;
    let mut ward = false;
    if let Some(items) = inventory.as_array() {
        for item in items {
            match item.as_str() {
                Some(CINDER) => cinder += 1,
                Some(WARD) => ward = true,
                _ => {}
            }
        }
    }
    (cinder, ward)
}

fn owns(inventory: &Value, id: &str) -> bool {
    inventory
        .as_array()
        .is_some_and(|items| items.iter().any(|v| v.as_str() == Some(id)))
}

pub fn edge_bonus(p: &Player) -> f32 {
    if owns(&p.inventory, EDGE) {
        1.25
    } else {
        1.0
    }
}

pub fn forge(p: &mut Player) {
    let site = Pose {
        x: FORGE.0,
        z: FORGE.1,
        ..DEFAULT_SPAWN
    };
    let (stock, ward) = supplies(&p.inventory);
    if p.hp <= 0 || stock < WARD_COST || dist_xz(p.pose, site) > 2.5 {
        return;
    }
    if ward && (!owns(&p.inventory, HEART) || owns(&p.inventory, EDGE)) {
        return;
    }
    let Some(items) = p.inventory.as_array_mut() else {
        return;
    };
    let mut needed = WARD_COST;
    items.retain(|item| {
        if needed > 0 && item.as_str() == Some(CINDER) {
            needed -= 1;
            false
        } else {
            true
        }
    });
    if ward {
        if let Some(slot) = items.iter().position(|v| v.as_str() == Some(HEART)) {
            items.remove(slot);
        }
        items.push(Value::from(EDGE));
    } else {
        items.push(Value::from(WARD));
    }
}

pub fn view(w: &World, viewer: u32) -> AshworksView {
    let (cinder, ward) = w
        .players
        .get(&viewer)
        .map(|p| supplies(&p.inventory))
        .unwrap_or_default();
    let inventory = w.players.get(&viewer).map(|p| &p.inventory);
    AshworksView {
        cinder,
        ward,
        heart: inventory.is_some_and(|v| owns(v, HEART)),
        edge: inventory.is_some_and(|v| owns(v, EDGE)),
        constructs: w
            .kilns
            .iter()
            .map(|k| ConstructState {
                id: k.spec.id,
                hp: k.hp,
                winding_up: k.strike.is_some(),
            })
            .collect(),
    }
}

pub fn nearest(w: &World, pose: Pose) -> Option<u32> {
    w.kilns
        .iter()
        .filter(|k| k.hp > 0)
        .min_by(|a, b| dist_xz(a.pose(), pose).total_cmp(&dist_xz(b.pose(), pose)))
        .map(|k| k.spec.id)
}

pub fn damage(w: &mut World, target: u32, source: u32, damage: i32) -> Option<i32> {
    let k = w
        .kilns
        .iter_mut()
        .find(|k| k.spec.id == target && k.hp > 0)?;
    k.hp = (k.hp - damage.max(0)).max(0);
    if damage > 0 {
        k.contributors.insert(source);
    }
    if k.hp == 0 {
        k.strike = None;
        k.ready = w.tick + 6 * TICK_HZ as u64;
        let site = k.pose();
        for id in k.contributors.drain() {
            let Some(p) = w.players.get_mut(&id) else {
                continue;
            };
            p.engage_target = None;
            if p.hp <= 0 || dist_xz(p.pose, site) > 24.0 {
                continue;
            }
            let (stock, _) = supplies(&p.inventory);
            if stock < 12 {
                if let Some(items) = p.inventory.as_array_mut() {
                    items.push(Value::from(CINDER));
                }
            }
            if k.spec.id == CONSTRUCTS[1].id
                && !owns(&p.inventory, HEART)
                && !owns(&p.inventory, EDGE)
            {
                if let Some(items) = p.inventory.as_array_mut() {
                    items.push(Value::from(HEART));
                }
            }
            p.xp = p.xp.saturating_add(k.spec.xp);
            while p.level < ba_combat::experience::MAX_LEVEL {
                let need = ba_combat::experience::xp_to_next_level(p.level) as u64;
                if p.xp < need {
                    break;
                }
                p.xp -= need;
                p.level += 1;
            }
        }
    }
    Some(k.hp)
}

pub fn tick(w: &mut World) {
    for k in &mut w.kilns {
        if w.tick < k.ready {
            continue;
        }
        if k.hp == 0 {
            k.hp = k.spec.hp;
        }
        let site = k.pose();
        let in_range = |p: &Player| p.hp > 0 && dist_xz(p.pose, site) <= PULSE_RADIUS;
        if let Some(impact) = k.strike {
            if w.tick < impact {
                continue;
            }
            for p in w.players.values_mut().filter(|p| in_range(p)) {
                let ward = supplies(&p.inventory).1;
                p.hp -= if ward { k.spec.pulse / 3 } else { k.spec.pulse };
                if p.hp <= 0 {
                    p.pose = DEFAULT_SPAWN;
                    p.movement = None;
                    p.engage_target = None;
                    p.hp = p.max_hp;
                }
            }
            k.strike = None;
            k.ready = w.tick + 2 * TICK_HZ as u64;
        } else if !k.contributors.is_empty() && w.players.values().any(in_range) {
            k.strike = Some(w.tick + WINDUP_TICKS);
        }
    }
}
