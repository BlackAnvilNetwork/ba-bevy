use bevy::prelude::*;

use crate::boss::{cue_laugh, cue_lean, WardenBody, WardenPossessRequest};
use crate::data::GameData;
use crate::player::Player;
use crate::rpg::{GrantXp, PlayerLeveledUp, Progression, XpReason, BOOK_MEMORY_XP};
use crate::scene::Warden;
use crate::vfx::{BookFall, DroppedBook};
use crate::well::{start_pour, WellPour};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DialoguePhase {
    #[default]
    ApproachNpc,
    Monologue,
    /// Dark turn before the fight.
    Possessing,
    /// Real-time fight with possessed Edda.
    BossFight,
    /// Ash playing; wait for book to settle / player to approach.
    AfterAsh,
    ReadingBook,
    PathOpen,
}

#[derive(Resource, Default)]
pub struct DialogueSession {
    pub phase: DialoguePhase,
    pub near_npc: bool,
    pub near_book: bool,
    pub on_exit: bool,
    pub line_index: usize,
    pub book_line_index: usize,
}

#[derive(Component)]
pub struct PromptText;

#[derive(Component)]
pub struct SubtitleText;

#[derive(Component)]
pub struct HudRoot;

#[derive(Component)]
pub struct SheetText;

const INTERACT_RANGE: f32 = 3.2;
const BOOK_RANGE: f32 = 2.4;

pub fn spawn_hud(mut commands: Commands) {
    commands
        .spawn((
            HudRoot,
            Node {
                width: percent(100),
                height: percent(100),
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
                padding: UiRect::all(px(28)),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                PromptText,
                Text::new("Click to look · WASD move · V/F5 camera · E talk · T note"),
                TextFont::from_font_size(18.0),
                TextColor(Color::srgba(0.85, 0.78, 0.65, 0.9)),
                Node {
                    margin: UiRect::bottom(px(10)),
                    ..default()
                },
            ));
            root.spawn((
                SubtitleText,
                Text::new(""),
                TextFont::from_font_size(26.0),
                TextColor(Color::srgb(0.95, 0.9, 0.8)),
                Node {
                    max_width: px(820),
                    margin: UiRect::bottom(px(12)),
                    ..default()
                },
            ));
        });

    // Top-left character sheet strip (ba-stick style glance).
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(16),
                left: px(18),
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                SheetText,
                Text::new(""),
                TextFont::from_font_size(16.0),
                TextColor(Color::srgba(0.9, 0.86, 0.72, 0.95)),
            ));
        });

    commands.insert_resource(DialogueSession::default());
}

pub fn update_sheet_hud(
    flow: Res<crate::forge_menu::GameFlow>,
    prog: Query<(&Progression, Option<&crate::forge_menu::CharacterName>), With<Player>>,
    fighter: Query<&crate::rpg::Fighter, With<Player>>,
    kits: Option<Res<crate::rpg::CombatKits>>,
    mut sheet: Query<&mut Text, With<SheetText>>,
) {
    if flow.in_menu() {
        return;
    }
    let Ok((p, name)) = prog.single() else {
        return;
    };
    let Ok(mut text) = sheet.single_mut() else {
        return;
    };
    let who = name.map(|n| n.0.as_str()).unwrap_or("Wanderer");
    let weapon = kits
        .as_ref()
        .map(|k| k.weapon_for(p.job).as_str())
        .unwrap_or("—");
    let bar = if let (Ok(f), Some(k)) = (fighter.single(), kits.as_ref()) {
        let slots: Vec<String> = f
            .hotbar
            .iter()
            .enumerate()
            .map(|(i, id)| format!("{} {}", i + 1, k.label_of(id)))
            .collect();
        if slots.is_empty() {
            String::new()
        } else {
            format!("\n{}", slots.join(" · "))
        }
    } else {
        String::new()
    };
    **text = format!(
        "{who}  ·  {}  Lv {}  ·  {}/{} XP  ·  {}\nHP {}/{}  MP {}/{}  ·  DEF {}  MDB {}  ·  [ ] switch job{}",
        p.job.display_name(),
        p.level,
        p.xp,
        p.xp_need(),
        weapon,
        p.hp,
        p.vitals.max_hp,
        p.mp,
        p.vitals.max_mp,
        p.defense(),
        p.magic_defense(),
        bar,
    )
    .into();
}

fn flat_distance(a: Vec3, b: Vec3) -> f32 {
    Vec2::new(a.x - b.x, a.z - b.z).length()
}

pub fn update_proximity(
    player: Single<&Transform, With<Player>>,
    data: Res<GameData>,
    npc: Query<&Transform, With<Warden>>,
    book: Query<&Transform, (With<DroppedBook>, Without<BookFall>)>,
    mut session: ResMut<DialogueSession>,
) {
    if let Ok(npc_tf) = npc.single() {
        session.near_npc = flat_distance(player.translation, npc_tf.translation) <= INTERACT_RANGE;
    } else {
        session.near_npc = false;
    }

    if let Ok(book_tf) = book.single() {
        session.near_book = flat_distance(player.translation, book_tf.translation) <= BOOK_RANGE;
    } else {
        session.near_book = false;
    }

    let x = player.translation.x;
    let z = player.translation.z;
    session.on_exit = data.zone.exits.iter().any(|exit| {
        x >= exit.min[0] && x <= exit.max[0] && z >= exit.min[1] && z <= exit.max[1]
    });
}

pub fn handle_interact(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    note: Res<crate::player::NoteComposer>,
    data: Res<GameData>,
    mut session: ResMut<DialogueSession>,
    mut possess: MessageWriter<WardenPossessRequest>,
    mut xp: MessageWriter<GrantXp>,
    mut commands: Commands,
    mut pour: ResMut<WellPour>,
    mut warden_body: Query<&mut WardenBody, With<Warden>>,
    book: Query<Entity, With<DroppedBook>>,
    mut prompt: Query<&mut Text, (With<PromptText>, Without<SubtitleText>)>,
    mut subtitle: Query<&mut Text, (With<SubtitleText>, Without<PromptText>)>,
) {
    if note.active {
        return;
    }
    let beats = &data.cutscene.beats;
    let book_beats = &data.cutscene.book_beats;
    let Ok(mut prompt_text) = prompt.single_mut() else {
        return;
    };
    let Ok(mut subtitle_text) = subtitle.single_mut() else {
        return;
    };

    match session.phase {
        DialoguePhase::Possessing | DialoguePhase::BossFight => {
            // Boss systems own the HUD during the turn and the fight.
            return;
        }
        DialoguePhase::PathOpen => {
            **prompt_text = exit_hint(&data).into();
            return;
        }
        DialoguePhase::AfterAsh => {
            if session.near_book {
                **prompt_text = "[E] Pick up the book in the ash".into();
            } else {
                **prompt_text = "She left a book in the ash.".into();
            }
            **subtitle_text = "".into();
            if keys.just_pressed(KeyCode::KeyE) && session.near_book {
                session.phase = DialoguePhase::ReadingBook;
                session.book_line_index = 0;
                if let Some(beat) = book_beats.first() {
                    **subtitle_text = format!("{}: \"{}\"", beat.speaker, beat.line).into();
                }
                **prompt_text = book_prompt(book_beats.len(), 0);
                // Claim the physical book
                if let Ok(entity) = book.single() {
                    commands.entity(entity).despawn();
                }
                session.near_book = false;
                xp.write(GrantXp {
                    amount: BOOK_MEMORY_XP,
                    reason: XpReason::BookMemory,
                });
            }
            return;
        }
        DialoguePhase::ReadingBook => {
            if !keys.just_pressed(KeyCode::KeyE) {
                if let Some(beat) = book_beats.get(session.book_line_index) {
                    **subtitle_text = format!("{}: \"{}\"", beat.speaker, beat.line).into();
                }
                **prompt_text = book_prompt(book_beats.len(), session.book_line_index);
                return;
            }
            session.book_line_index += 1;
            if session.book_line_index >= book_beats.len() {
                session.phase = DialoguePhase::PathOpen;
                **subtitle_text = "".into();
                **prompt_text = exit_hint(&data).into();
                return;
            }
            let beat = &book_beats[session.book_line_index];
            **subtitle_text = format!("{}: \"{}\"", beat.speaker, beat.line).into();
            **prompt_text = book_prompt(book_beats.len(), session.book_line_index);
            return;
        }
        DialoguePhase::ApproachNpc | DialoguePhase::Monologue => {}
    }

    if session.phase == DialoguePhase::ApproachNpc {
        if session.on_exit {
            **prompt_text = exit_hint(&data).into();
            **subtitle_text = "".into();
            return;
        }
        if !session.near_npc {
            **prompt_text = format!("Approach the {} · E to speak", data.npc.display_name).into();
            **subtitle_text = "".into();
            return;
        }
        **prompt_text = format!("[E] Speak with {}", data.npc.display_name).into();
        if !keys.just_pressed(KeyCode::KeyE) {
            return;
        }
        session.phase = DialoguePhase::Monologue;
        session.line_index = 0;
        let beat = &beats[0];
        **subtitle_text = format!("{}: \"{}\"", beat.speaker, beat.line).into();
        **prompt_text = monologue_prompt(beats.len(), 0);
        apply_stage(
            beat.stage.as_deref(),
            time.elapsed_secs(),
            &mut warden_body,
            &mut pour,
        );
        return;
    }

    // Monologue advance
    if !keys.just_pressed(KeyCode::KeyE) {
        if let Some(beat) = beats.get(session.line_index) {
            **subtitle_text = format!("{}: \"{}\"", beat.speaker, beat.line).into();
        }
        **prompt_text = monologue_prompt(beats.len(), session.line_index);
        return;
    }

    session.line_index += 1;
    if session.line_index >= beats.len() {
        **subtitle_text = "".into();
        **prompt_text = "".into();
        possess.write(WardenPossessRequest);
        return;
    }

    let beat = &beats[session.line_index];
    **subtitle_text = format!("{}: \"{}\"", beat.speaker, beat.line).into();
    **prompt_text = monologue_prompt(beats.len(), session.line_index);
    apply_stage(
        beat.stage.as_deref(),
        time.elapsed_secs(),
        &mut warden_body,
        &mut pour,
    );
}

fn apply_stage(
    stage: Option<&str>,
    now: f32,
    warden_body: &mut Query<&mut WardenBody, With<Warden>>,
    pour: &mut WellPour,
) {
    let Some(stage) = stage else {
        return;
    };
    match stage {
        "laugh" => {
            if let Ok(mut body) = warden_body.single_mut() {
                cue_laugh(&mut body, now);
            }
        }
        "lean" => {
            if let Ok(mut body) = warden_body.single_mut() {
                cue_lean(&mut body, now);
            }
        }
        "pour" => start_pour(pour),
        _ => {}
    }
}

fn exit_hint(data: &GameData) -> String {
    data.zone
        .exits
        .first()
        .map(|exit| exit.hint.clone())
        .unwrap_or_else(|| "The yard ends here.".into())
}

fn monologue_prompt(total: usize, index: usize) -> String {
    let remaining = total.saturating_sub(index + 1);
    if remaining == 0 {
        "[E] …finish".into()
    } else {
        format!("[E] Continue ({remaining} left)")
    }
}

fn book_prompt(total: usize, index: usize) -> String {
    let remaining = total.saturating_sub(index + 1);
    if remaining == 0 {
        "[E] Close the book".into()
    } else {
        format!("[E] Turn page ({remaining} left)")
    }
}

pub fn announce_level_up(
    mut events: MessageReader<PlayerLeveledUp>,
    mut subtitle: Query<&mut Text, With<SubtitleText>>,
) {
    for ev in events.read() {
        if let Ok(mut text) = subtitle.single_mut() {
            **text = if ev.levels_gained == 1 {
                format!("Level up! You are now level {}.", ev.new_level)
            } else {
                format!(
                    "Level up ×{}! You are now level {}.",
                    ev.levels_gained, ev.new_level
                )
            }
            .into();
        }
    }
}
