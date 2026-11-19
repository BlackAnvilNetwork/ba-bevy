use super::*;
use ba_protocol::ashworks::*;

const CINDER: &str = "ashworks:raw_cinder";
const HEART: &str = "ashworks:kiln_heart";
const EDGE: &str = "ashworks:tempered_edge";
const WARD: &str = "ashworks:kiln_ward";

pub(super) struct Kiln {
    pub spec: &'static Construct,
    pub hp: i32,
    strike: Option<u64>,
    x: f32,
    z: f32,
    yaw: f32,
    sequence: u32,
    duration: u64,
    tell: sw::Tell,
    line: bool,
    radius: f32,
    grasp: bool,
    hidden_until: u64,
    ready: u64,
    contributors: HashMap<u32, u64>,
}

impl Kiln {
    pub fn new(spec: &'static Construct) -> Self {
        Self {
            spec,
            hp: if matches!(spec.id, 70012..=70013) {
                0
            } else {
                spec.hp
            },
            strike: None,
            x: spec.x,
            z: spec.z,
            yaw: 0.,
            sequence: 0,
            duration: 20,
            tell: sw::Tell::Normal,
            line: false,
            radius: PULSE_RADIUS,
            grasp: false,
            hidden_until: 0,
            ready: if matches!(spec.id, 70012..=70013) {
                u64::MAX
            } else {
                0
            },
            contributors: HashMap::new(),
        }
    }
    pub fn targetable(&self, tick: u64) -> bool {
        self.hp > 0 && tick >= self.hidden_until
    }
    pub fn wire(&self, tick: u64) -> sw::Enemy {
        sw::Enemy {
            submerged: self.hidden_until > tick,
            grasp: self.grasp,
            id: self.spec.id,
            x: self.x,
            z: self.z,
            yaw: self.yaw,
            hp: self.hp,
            windup: self.strike.map_or(0., |t| {
                1. - t.saturating_sub(tick) as f32 / (self.duration + 1) as f32
            }),
            tell: self.tell,
            radius: self.radius,
            line: self.line,
        }
    }
    pub fn pose(&self) -> Pose {
        Pose {
            x: self.x,
            z: self.z,
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
    if owns(&p.inventory, EDGE) || owns(&p.inventory, "Cael's Iron Edge") {
        1.25
    } else {
        1.0
    }
}

pub fn forge(p: &mut Player, recipe: Recipe) {
    let site = Pose {
        x: FORGE.0,
        z: FORGE.1,
        ..DEFAULT_SPAWN
    };
    let (stock, ward) = supplies(&p.inventory);
    if p.hp <= 0 || stock < WARD_COST || dist_xz(p.pose, site) > 2.5 {
        return;
    }
    if (recipe == Recipe::TemperedEdge) != ward {
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
            .filter(|k| !sw::marsh(k.spec.x))
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
        .filter(|k| k.targetable(w.tick))
        .min_by(|a, b| dist_xz(a.pose(), pose).total_cmp(&dist_xz(b.pose(), pose)))
        .map(|k| k.spec.id)
}

pub fn damage(w: &mut World, target: u32, source: u32, damage: i32) -> Option<i32> {
    let k = w
        .kilns
        .iter_mut()
        .find(|k| k.spec.id == target && k.targetable(w.tick))?;
    k.hp = (k.hp - damage.max(0)).max(0);
    if damage > 0 {
        k.contributors.insert(source, w.tick);
    }
    if k.hp == 0 {
        k.strike = None;
        k.ready = w.tick
            + TICK_HZ as u64
                * match k.spec.id {
                    70000..=70001 | 70005..=70007 => 30,
                    70002..=70004 => 40,
                    70008..=70009 | sw::EDDA => 60,
                    sw::OX => 600,
                    _ => 6,
                };
        if matches!(k.spec.id, 70012..=70013) {
            k.ready = u64::MAX;
        }
        let site = k.pose();
        for (id, last_hit) in k.contributors.drain() {
            if sw::marsh(site.x) && w.tick.saturating_sub(last_hit) > 200 {
                continue;
            }
            let Some(p) = w.players.get_mut(&id) else {
                continue;
            };
            p.engage_target = None;
            if p.hp <= 0 || dist_xz(p.pose, site) > 24.0 {
                continue;
            }
            stillwater::reward(
                p,
                k.spec.id,
                site,
                w.rng.random_range(0..10_000),
                (w.tick / 20) % 360 >= 310,
            );
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

pub fn tick(w: &mut World) -> Vec<ServerMessage> {
    let mut events = Vec::new();
    let mut summon = None;
    let mut flood = false;
    for k in &mut w.kilns {
        if w.tick < k.ready {
            continue;
        }
        if k.hp == 0 {
            k.hp = k.spec.hp;
            k.x = k.spec.x;
            k.z = k.spec.z;
            k.sequence = 0;
        }
        let id = k.spec.id;
        let marsh = sw::marsh(k.spec.x);
        let site = k.pose();
        if let Some(impact) = k.strike {
            if w.tick < impact {
                if k.grasp {
                    for p in w.players.values_mut() {
                        let d = dist_xz(site, p.pose);
                        if p.hp > 0 && d > 1.0 && d < k.radius && w.tick >= p.journey.roll_until {
                            p.pose.x += (site.x - p.pose.x) / d * 0.11;
                            p.pose.z += (site.z - p.pose.z) / d * 0.11;
                        }
                    }
                }
                continue;
            }
            for (pid, p) in &mut w.players {
                let dx = p.pose.x - k.x;
                let dz = p.pose.z - k.z;
                let (sx, cz) = k.yaw.sin_cos();
                let inside = if k.line {
                    let along = dx * sx + dz * cz;
                    along >= -0.6 && along <= k.radius && (dx * cz - dz * sx).abs() < 1.2
                } else {
                    dist_xz(site, p.pose) <= k.radius
                };
                if k.grasp && dist_xz(site, p.pose) > 2.5 {
                    continue;
                }
                if marsh && (dist_xz(p.pose, sw::POST) < 6. || dist_xz(p.pose, sw::SHRINE) < 5.) {
                    continue;
                }
                if !inside || p.hp <= 0 || w.tick < p.journey.immune_until {
                    continue;
                }
                let amount = if supplies(&p.inventory).1 {
                    k.spec.pulse / 3
                } else {
                    k.spec.pulse
                };
                p.hp = (p.hp - amount).max(0);
                p.journey.freeze = w.tick + if id == sw::OX || id == sw::EDDA { 2 } else { 1 };
                if k.tell != sw::Tell::Normal {
                    p.journey.slow = w.tick + 16;
                    let d = dist_xz(site, p.pose).max(0.01);
                    let (a, b, c, e) = sw::bounds(p.pose.x);
                    p.pose.x = (p.pose.x + dx / d * 1.4).clamp(a, b);
                    p.pose.z = (p.pose.z + dz / d * 1.4).clamp(c, e);
                }
                events.push(ServerMessage::Damage {
                    tick: w.tick,
                    attacker_id: id,
                    target_id: *pid,
                    damage: amount,
                    hit: true,
                    crit: false,
                    target_hp: p.hp,
                });
            }
            // Melee beasts plant after a swing. Only long-body lunges (eels, ox)
            // carry through the line — hounds circling/hopping read as bouncing.
            if marsh && k.line && lunges_after_strike(id) {
                let (sx, cz) = k.yaw.sin_cos();
                k.x += sx * k.radius * 0.7;
                k.z += cz * k.radius * 0.7;
            }
            if id == sw::OX && k.hp > k.spec.hp * 6 / 10 && k.sequence % 5 == 0 {
                summon = Some(site);
            }
            if id == sw::OX
                && k.hp <= k.spec.hp * 6 / 10
                && k.hp > k.spec.hp * 3 / 10
                && k.sequence % 3 == 0
            {
                flood = true;
            }
            if matches!(id, 70008..=70009) {
                k.hidden_until = w.tick + 80;
            }
            k.strike = None;
            k.ready = w.tick
                + match id {
                    sw::OX if k.hp < k.spec.hp / 3 && k.sequence % 4 != 0 => 4,
                    sw::OX => 60,
                    70008..=70009 => 80,
                    _ => 35,
                };
            continue;
        }
        let target = w
            .players
            .values()
            .filter(|p| p.hp > 0)
            .min_by(|a, b| dist_xz(a.pose, site).total_cmp(&dist_xz(b.pose, site)));
        let Some(p) = target else { continue };
        let d = dist_xz(p.pose, site);
        let safe = marsh && (dist_xz(p.pose, sw::POST) < 6. || dist_xz(p.pose, sw::SHRINE) < 5.);
        if marsh && (d > 14. || safe || dist_xz(site, sw::at(k.spec.x, k.spec.z)) > 20.) {
            let t = w.tick as f32 * 0.018;
            let r = if id == sw::OX { 3. } else { 0.6 };
            k.x += (k.spec.x + r * t.sin() - k.x).clamp(-0.1, 0.1);
            k.z += (k.spec.z + r * t.cos() - k.z).clamp(-0.1, 0.1);
            continue;
        }
        k.yaw = (p.pose.x - k.x).atan2(p.pose.z - k.z);
        let middle = id == sw::OX && k.hp <= k.spec.hp * 6 / 10 && k.hp > k.spec.hp * 3 / 10;
        let likes_range = matches!(id,70002..=70004|70008..=70009|sw::OX);
        if marsh && !middle && d > if likes_range { 5.5 } else { 2.4 } {
            let speed = if id == sw::OX { 0.09 } else { 0.15 };
            k.x += (p.pose.x - k.x) / d * speed;
            k.z += (p.pose.z - k.z) / d * speed;
            continue;
        }
        if !marsh && (k.contributors.is_empty() || d > PULSE_RADIUS) {
            continue;
        }
        k.sequence += 1;
        k.line = marsh
            && (matches!(id,70000..=70004|70008..=70009) || id == sw::OX && k.sequence % 3 != 0);
        k.tell = if id == sw::EDDA && k.sequence % 3 == 0 || id == sw::OX && k.hp < k.spec.hp / 3 {
            sw::Tell::Unblockable
        } else if id == sw::OX || id == sw::EDDA || id == u32::MAX - 1 {
            sw::Tell::Heavy
        } else {
            sw::Tell::Normal
        };
        k.grasp = middle && k.sequence % 3 == 1;
        if middle {
            k.line = !k.grasp && k.sequence % 3 == 2;
        }
        if k.grasp {
            k.tell = sw::Tell::Unblockable;
        }
        k.radius = if k.grasp {
            12.
        } else if k.line {
            7.
        } else if id == sw::OX {
            7.5
        } else {
            PULSE_RADIUS
        };
        k.duration = match id {
            70000..=70001 => 12,
            70002..=70004 => 24,
            70005..=70007 => 9,
            sw::OX if k.hp < k.spec.hp / 3 => 14,
            sw::OX | sw::EDDA => 26,
            _ => 20,
        };
        if k.grasp {
            k.duration = 40;
        }
        k.strike = Some(w.tick + k.duration);
    }
    if let Some(pt) = summon {
        for k in w
            .kilns
            .iter_mut()
            .filter(|k| matches!(k.spec.id, 70012..=70013))
        {
            k.x = pt.x + if k.spec.id == 70012 { -5. } else { 5. };
            k.z = pt.z + 4.;
            k.hp = k.spec.hp;
            k.ready = w.tick + 10;
            k.hidden_until = 0;
        }
    }
    if flood {
        for k in w
            .kilns
            .iter_mut()
            .filter(|k| matches!(k.spec.id, 70008..=70009))
        {
            k.hidden_until = k.hidden_until.min(w.tick + 20);
            k.ready = k.ready.min(w.tick + 20);
        }
    }
    events
}

fn lunges_after_strike(id: u32) -> bool {
    matches!(id, 70008..=70009 | sw::OX)
}

#[cfg(test)]
mod fight_stance_tests {
    use super::*;

    #[test]
    fn melee_hounds_plant_instead_of_lunging() {
        assert!(!lunges_after_strike(70000));
        assert!(!lunges_after_strike(70001));
        assert!(!lunges_after_strike(70002));
        assert!(lunges_after_strike(70008));
        assert!(lunges_after_strike(sw::OX));
    }
}
