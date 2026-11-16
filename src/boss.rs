//! Edda — first boss. After the monologue she turns, then fights in real time.

use bevy::ecs::hierarchy::ChildOf;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use crate::cel::CelMaterial;
use crate::dialogue::{DialoguePhase, DialogueSession, PromptText, SubtitleText};
use crate::player::{NoteComposer, Player};
use crate::rpg::{
    apply_ability_side_effects, apply_ws_side_effects, format_hotbar_line, hotbar_slot_pressed,
    mitigate_incoming_physical, mob_kill_xp, physical_damage, resolve_ability,
    resolve_auto_attack, resolve_weapon_skill, tick_rest, tp_from_auto_hit, CastOutcome,
    CombatKits, DamageKind, Fighter, GrantXp, Progression, TargetRef, XpReason, WITNESS_ASH_XP,
};
use crate::scene::Warden;
use crate::vfx::DeathDissolveRequest;

pub const BOSS_MAX_HP: i32 = 160;
const BOSS_SWING_CD: f32 = 1.55;
const BOSS_REACH: f32 = 2.35;
const BOSS_SPEED: f32 = 3.1;
const BOSS_VIT: i32 = 18;
const BOSS_DEF: i32 = 14;
const BOSS_MDB: i32 = 10;
const BOSS_EVA: i32 = 8;

#[derive(Message, Clone, Copy, Debug, Default)]
pub struct WardenPossessRequest;

#[derive(Component)]
pub struct WardenBody {
    pub home: Transform,
    pub laugh_until: f32,
    pub lean_until: f32,
    pub lean_dir: Vec3,
}

#[derive(Component)]
pub struct Possessed;

#[derive(Component)]
pub struct WardenBoss {
    pub hp: i32,
    pub max_hp: i32,
    pub swing_cd: f32,
    pub hit_flash: f32,
}

#[derive(Resource, Default)]
pub struct PossessFx {
    pub until: f32,
    pub started: bool,
    pub darkened: bool,
}

pub fn cue_laugh(body: &mut WardenBody, now: f32) {
    body.laugh_until = now + 1.15;
}

pub fn cue_lean(body: &mut WardenBody, now: f32) {
    body.lean_until = now + 2.4;
}

pub fn on_possess_request(
    mut events: MessageReader<WardenPossessRequest>,
    time: Res<Time>,
    mut session: ResMut<DialogueSession>,
    mut fx: ResMut<PossessFx>,
    warden: Query<Entity, With<Warden>>,
    mut commands: Commands,
    mut subtitle: Query<&mut Text, With<SubtitleText>>,
    mut prompt: Query<&mut Text, (With<PromptText>, Without<SubtitleText>)>,
) {
    for _ in events.read() {
        session.phase = DialoguePhase::Possessing;
        fx.until = time.elapsed_secs() + 2.4;
        fx.started = true;
        fx.darkened = false;
        if let Ok(entity) = warden.single() {
            commands.entity(entity).insert(Possessed);
        }
        if let Ok(mut text) = subtitle.single_mut() {
            **text = "Something takes her.".into();
        }
        if let Ok(mut text) = prompt.single_mut() {
            **text = "".into();
        }
    }
}

pub fn tick_possession(
    time: Res<Time>,
    mut fx: ResMut<PossessFx>,
    mut session: ResMut<DialogueSession>,
    mut warden: Query<(Entity, &mut Transform, &WardenBody), With<Warden>>,
    mut commands: Commands,
    mut subtitle: Query<&mut Text, With<SubtitleText>>,
    mut prompt: Query<&mut Text, (With<PromptText>, Without<SubtitleText>)>,
    parents: Query<&ChildOf>,
    meshes: Query<(Entity, &MeshMaterial3d<CelMaterial>)>,
    mut cels: ResMut<Assets<CelMaterial>>,
) {
    if session.phase != DialoguePhase::Possessing || !fx.started {
        return;
    }
    let now = time.elapsed_secs();
    let shake = (now * 28.0).sin() * 0.04;
    let root = warden.single().ok().map(|(e, _, _)| e);
    if let Ok((_, mut tf, body)) = warden.single_mut() {
        tf.translation = body.home.translation + Vec3::new(shake, 0.0, shake * 0.6);
        tf.rotation = body.home.rotation;
    }
    if !fx.darkened {
        if let Some(root) = root {
            for (entity, handle) in &meshes {
                if !under_root(entity, root, &parents) {
                    continue;
                }
                if let Some(mut mat) = cels.get_mut(handle) {
                    let c = mat.uniforms.color;
                    mat.uniforms.color = LinearRgba::new(
                        (c.red * 0.22).min(0.18),
                        (c.green * 0.1).min(0.07),
                        (c.blue * 0.4).max(0.2),
                        c.alpha,
                    );
                }
            }
        }
        fx.darkened = true;
    }

    if now < fx.until {
        return;
    }
    fx.started = false;
    session.phase = DialoguePhase::BossFight;
    if let Ok((entity, mut tf, body)) = warden.single_mut() {
        *tf = body.home;
        commands.entity(entity).insert(WardenBoss {
            hp: BOSS_MAX_HP,
            max_hp: BOSS_MAX_HP,
            swing_cd: 0.85,
            hit_flash: 0.0,
        });
    }
    if let Ok(mut text) = subtitle.single_mut() {
        **text = "Edda — possessed. She raises her blade.".into();
    }
    if let Ok(mut text) = prompt.single_mut() {
        **text = "LMB strike · WASD move · survive".into();
    }
}

fn under_root(mut entity: Entity, root: Entity, parents: &Query<&ChildOf>) -> bool {
    if entity == root {
        return true;
    }
    while let Ok(child_of) = parents.get(entity) {
        entity = child_of.parent();
        if entity == root {
            return true;
        }
    }
    false
}

pub fn tick_warden_acting(
    time: Res<Time>,
    session: Res<DialogueSession>,
    mut set: ParamSet<(
        Query<&Transform, With<Player>>,
        Query<(&mut Transform, &mut WardenBody), (With<Warden>, Without<WardenBoss>)>,
    )>,
) {
    if matches!(
        session.phase,
        DialoguePhase::BossFight | DialoguePhase::Possessing | DialoguePhase::AfterAsh
    ) {
        return;
    }
    let now = time.elapsed_secs();
    let player_pos = {
        let q = set.p0();
        q.single().ok().map(|tf| tf.translation)
    };

    let mut warden_q = set.p1();
    let Ok((mut tf, mut body)) = warden_q.single_mut() else {
        return;
    };
    let mut offset = Vec3::ZERO;
    let mut rot = body.home.rotation;

    if now < body.laugh_until {
        let t = body.laugh_until - now;
        offset.y = (t * 22.0).sin().abs() * 0.08;
        rot = body.home.rotation
            * Quat::from_rotation_z((t * 30.0).sin() * 0.12)
            * Quat::from_rotation_x((t * 18.0).sin() * 0.06);
    }
    if now < body.lean_until {
        let pull = ((body.lean_until - now) / 2.2).clamp(0.0, 1.0);
        let toward = if let Some(player_tf) = player_pos {
            let d = (player_tf - body.home.translation).normalize_or_zero();
            Vec3::new(d.x, 0.0, d.z)
        } else {
            body.lean_dir
        };
        body.lean_dir = toward;
        offset += toward * 0.45 * pull;
        if toward.length_squared() > 0.01 {
            rot = Quat::from_rotation_arc(Vec3::NEG_Z, toward)
                * Quat::from_rotation_x(-0.18 * pull);
        }
    }

    if now < body.laugh_until || now < body.lean_until {
        tf.translation = body.home.translation + offset;
        tf.rotation = rot;
    } else {
        tf.translation = body.home.translation;
        tf.rotation = body.home.rotation;
    }
}

pub fn boss_fight(
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    cursor: Query<&CursorOptions>,
    note: Res<NoteComposer>,
    kits: Res<CombatKits>,
    mut session: ResMut<DialogueSession>,
    mut set: ParamSet<(
        Query<(&Transform, &mut Progression, &mut Fighter), With<Player>>,
        Query<(&mut Transform, &mut WardenBoss, &WardenBody), With<Warden>>,
        Query<&mut Text, With<SubtitleText>>,
        Query<&mut Text, (With<PromptText>, Without<SubtitleText>)>,
    )>,
    mut dissolve: MessageWriter<DeathDissolveRequest>,
    mut xp: MessageWriter<GrantXp>,
    mut rng: Local<Option<SmallRng>>,
) {
    if session.phase != DialoguePhase::BossFight || note.active {
        return;
    }
    if rng.is_none() {
        *rng = Some(SmallRng::seed_from_u64(0xEDDA_u64));
    }
    let rng = rng.as_mut().unwrap();
    let dt = time.delta_secs();
    let now = time.elapsed_secs();
    let grabbed = cursor
        .single()
        .map(|c| c.grab_mode != CursorGrabMode::None)
        .unwrap_or(false);

    let (player_pos, defense, level, want_auto, want_slot, want_ws, cast_note) = {
        let mut player_q = set.p0();
        let Ok((player_tf, mut prog, mut fighter)) = player_q.single_mut() else {
            return;
        };
        fighter.swing_cd = (fighter.swing_cd - dt).max(0.0);
        fighter.tick_buffs(now);
        tick_rest(&mut prog, &mut fighter, dt);
        if prog.hp <= 0 {
            prog.hp = prog.vitals.max_hp;
        }
        let cast = if now < fighter.last_cast_until {
            fighter.last_cast_label.clone()
        } else {
            String::new()
        };
        (
            player_tf.translation,
            prog.defense(),
            prog.level,
            grabbed && mouse.just_pressed(MouseButton::Left) && fighter.swing_cd <= 0.0,
            hotbar_slot_pressed(&keys),
            keys.just_pressed(KeyCode::KeyR),
            cast,
        )
    };

    let (mut boss_hp, mut boss_max, mut boss_dead, hit_player, dist) = {
        let mut boss_q = set.p1();
        let Ok((mut boss_tf, mut boss, body)) = boss_q.single_mut() else {
            return;
        };
        boss.swing_cd = (boss.swing_cd - dt).max(0.0);
        boss.hit_flash = (boss.hit_flash - dt).max(0.0);

        let to_player = player_pos - boss_tf.translation;
        let flat = Vec3::new(to_player.x, 0.0, to_player.z);
        let dist = flat.length();
        if dist > 0.4 {
            let dir = flat.normalize_or_zero();
            boss_tf.translation += dir * BOSS_SPEED * dt;
            boss_tf.translation.y = body.home.translation.y;
            if dir.length_squared() > 0.01 {
                boss_tf.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, dir);
            }
        }
        if boss.hit_flash > 0.0 {
            boss_tf.translation.y = body.home.translation.y + boss.hit_flash * 0.12;
        }

        let hit_player = boss.swing_cd <= 0.0 && dist <= BOSS_REACH && boss.hp > 0;
        if hit_player {
            boss.swing_cd = BOSS_SWING_CD;
        }

        (boss.hp, boss.max_hp, boss.hp <= 0, hit_player, dist)
    };

    // Player actions + apply damage to boss in a second boss touch.
    let mut apply_boss_dmg = 0i32;
    let mut rose_again = false;
    let mut hud_cast = cast_note;
    let mut hp = 0;
    let mut max_hp = 1;
    let mut mp = 0;
    let mut max_mp = 0;
    let mut tp = 0.0;
    {
        let mut player_q = set.p0();
        let Ok((_, mut prog, mut fighter)) = player_q.single_mut() else {
            return;
        };
        let target = TargetRef {
            vit: BOSS_VIT,
            defense: BOSS_DEF,
            magic_defense: BOSS_MDB,
            evasion: BOSS_EVA,
            distance: dist,
        };

        if want_auto {
            let outcome = resolve_auto_attack(
                prog.job,
                &prog.stats,
                &fighter,
                target,
                rng.random::<f32>(),
                rng.random::<f32>(),
            );
            match outcome {
                CastOutcome::Damage { amount, .. } => {
                    fighter.swing_cd = fighter.swing_period();
                    fighter.add_tp(tp_from_auto_hit(240.0));
                    apply_boss_dmg += amount;
                    fighter.note_cast(&format!("Strike {amount}"), now);
                }
                CastOutcome::Miss => {
                    fighter.swing_cd = fighter.swing_period();
                    fighter.note_cast("Miss", now);
                }
                CastOutcome::Fail { reason } => {
                    fighter.note_cast(reason, now);
                }
                _ => {}
            }
        }

        if want_ws {
            let ws_id = fighter
                .hotbar
                .iter()
                .find(|id| kits.weapon_skill(id).is_some())
                .cloned();
            if let Some(id) = ws_id {
                if let Some(ws) = kits.weapon_skill(&id) {
                    let outcome = resolve_weapon_skill(
                        prog.job,
                        &prog.stats,
                        &fighter,
                        ws,
                        target,
                        rng.random::<f32>(),
                    );
                    match outcome {
                        CastOutcome::Damage { amount, .. } => {
                            apply_ws_side_effects(&mut fighter, ws, now);
                            apply_boss_dmg += amount;
                            fighter.note_cast(&format!("{} {amount}", ws.label), now);
                        }
                        CastOutcome::Miss => {
                            apply_ws_side_effects(&mut fighter, ws, now);
                            fighter.note_cast(&format!("{} miss", ws.label), now);
                        }
                        CastOutcome::Fail { reason } => fighter.note_cast(reason, now),
                        _ => {}
                    }
                }
            } else {
                fighter.note_cast("No weapon skill", now);
            }
        }

        if let Some(slot) = want_slot {
            if let Some(id) = fighter.hotbar.get(slot).cloned() {
                if let Some(ws) = kits.weapon_skill(&id) {
                    let outcome = resolve_weapon_skill(
                        prog.job,
                        &prog.stats,
                        &fighter,
                        ws,
                        target,
                        rng.random::<f32>(),
                    );
                    match outcome {
                        CastOutcome::Damage { amount, .. } => {
                            apply_ws_side_effects(&mut fighter, ws, now);
                            apply_boss_dmg += amount;
                            fighter.note_cast(&format!("{} {amount}", ws.label), now);
                        }
                        CastOutcome::Miss => {
                            apply_ws_side_effects(&mut fighter, ws, now);
                            fighter.note_cast(&format!("{} miss", ws.label), now);
                        }
                        CastOutcome::Fail { reason } => fighter.note_cast(reason, now),
                        _ => {}
                    }
                } else if let Some(def) = kits.ability(&id) {
                    let outcome = resolve_ability(
                        prog.job,
                        &prog.stats,
                        prog.mp,
                        &fighter,
                        def,
                        Some(target),
                        now,
                    );
                    match &outcome {
                        CastOutcome::Damage { amount, kind } => {
                            apply_ability_side_effects(
                                &mut fighter,
                                &mut prog,
                                def,
                                &outcome,
                                now,
                            );
                            apply_boss_dmg += *amount;
                            let tag = match kind {
                                DamageKind::Magic => "mag",
                                DamageKind::Physical => "phy",
                            };
                            fighter.note_cast(&format!("{} {amount}{tag}", def.label), now);
                        }
                        CastOutcome::Heal { amount } => {
                            apply_ability_side_effects(
                                &mut fighter,
                                &mut prog,
                                def,
                                &outcome,
                                now,
                            );
                            prog.hp = (prog.hp + amount).min(prog.vitals.max_hp);
                            fighter.note_cast(&format!("{} +{amount}", def.label), now);
                        }
                        CastOutcome::Buff { label } => {
                            apply_ability_side_effects(
                                &mut fighter,
                                &mut prog,
                                def,
                                &outcome,
                                now,
                            );
                            fighter.note_cast(label, now);
                        }
                        CastOutcome::Miss => fighter.note_cast("Resist", now),
                        CastOutcome::Fail { reason } => fighter.note_cast(reason, now),
                        CastOutcome::None => {}
                    }
                }
            }
        }

        if hit_player {
            let blade = physical_damage(26, defense, 5, false);
            let taken = mitigate_incoming_physical(blade, &fighter);
            prog.hp = (prog.hp - taken).max(0);
            fighter.resting = false;
        }
        if now < fighter.last_cast_until {
            hud_cast = fighter.last_cast_label.clone();
        }
        if prog.hp <= 0 {
            prog.hp = prog.vitals.max_hp;
            rose_again = true;
        }
        hp = prog.hp;
        max_hp = prog.vitals.max_hp;
        mp = prog.mp;
        max_mp = prog.vitals.max_mp;
        tp = fighter.tp;
    }

    if apply_boss_dmg > 0 {
        let mut boss_q = set.p1();
        if let Ok((_, mut boss, _)) = boss_q.single_mut() {
            boss.hp = (boss.hp - apply_boss_dmg).max(0);
            boss.hit_flash = 0.18;
            boss_hp = boss.hp;
            boss_max = boss.max_hp;
            boss_dead = boss.hp <= 0;
        }
    } else {
        let boss_q = set.p1();
        if let Ok((_, boss, _)) = boss_q.single() {
            boss_hp = boss.hp;
            boss_max = boss.max_hp;
            boss_dead = boss.hp <= 0;
        }
    }

    if rose_again {
        let mut sub = set.p2();
        if let Ok(mut text) = sub.single_mut() {
            **text =
                "You fall — and rise again. The yard will not let you forget her yet.".into();
        }
    }

    let bar = {
        // Rebuild a display fighter snapshot for hotbar formatting.
        let player_q = set.p0();
        if let Ok((_, _, fighter)) = player_q.single() {
            format_hotbar_line(&kits, fighter, now)
        } else {
            String::new()
        }
    };

    {
        let mut prompt = set.p3();
        if let Ok(mut text) = prompt.single_mut() {
            let cast = if hud_cast.is_empty() {
                String::new()
            } else {
                format!("  ·  {hud_cast}")
            };
            **text = format!(
                "Edda {:>3}/{:<3}  ·  You {:>3}/{:<3} HP  {:>3}/{:<3} MP  TP {:>4.0}{}\nLMB auto  1-8 abilities  R WS  [ ] job\n{bar}",
                boss_hp, boss_max, hp, max_hp, mp, max_mp, tp, cast
            )
            .into();
        }
    }

    if boss_dead {
        session.phase = DialoguePhase::AfterAsh;
        dissolve.write(DeathDissolveRequest);
        xp.write(GrantXp {
            amount: WITNESS_ASH_XP.max(mob_kill_xp(5, level)),
            reason: XpReason::WitnessAsh,
        });
        let mut sub = set.p2();
        if let Ok(mut text) = sub.single_mut() {
            **text = "Edda falls. Ash, and a book in the dust.".into();
        }
    }
}
