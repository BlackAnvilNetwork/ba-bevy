//! Boot forge menu — mirrors ba-stick `boot-forge-shell` before Greymere Yard.

use bevy::ecs::hierarchy::ChildOf;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use crate::dialogue::{HudRoot, SheetText};
use crate::forge_vfx::{ForgeVfx, ForgeVfxLayer};
use crate::player::NoteComposer;
use crate::rpg::{rebuild_hotbar, CombatKits, Fighter, JobId, Progression};

/// Palette from ba-stick `boot-menu-theme` / forge CSS vars.
mod forge {
    use bevy::prelude::Color;
    pub const BRONZE: Color = Color::srgb(0.722, 0.573, 0.290);
    pub const BRONZE_HI: Color = Color::srgb(0.878, 0.753, 0.439);
    pub const BRONZE_DIM: Color = Color::srgb(0.541, 0.439, 0.188);
    pub const CREAM: Color = Color::srgb(0.937, 0.910, 0.863);
    pub const SUB: Color = Color::srgb(0.761, 0.659, 0.471);
    pub const CARD_TOP: Color = Color::srgba(0.133, 0.110, 0.078, 0.94);
    pub const FRAME: Color = Color::srgba(0.071, 0.055, 0.047, 0.88);
    pub const INPUT_BG: Color = Color::srgb(0.110, 0.102, 0.094);
    pub const VOID: Color = Color::srgb(0.024, 0.016, 0.031);
    pub const SELECT: Color = Color::srgba(0.659, 0.471, 0.157, 0.28);
    pub const SELECT_BORDER: Color = Color::srgb(0.878, 0.753, 0.439);
    pub const JOB_BG: Color = Color::srgba(0.08, 0.06, 0.05, 0.85);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Resource)]
pub enum GameFlow {
    #[default]
    ForgeMenu,
    InYard,
}

impl GameFlow {
    pub fn in_menu(self) -> bool {
        matches!(self, Self::ForgeMenu)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Appearance {
    #[default]
    Male,
    Female,
}

impl Appearance {
    pub fn label(self) -> &'static str {
        match self {
            Self::Male => "Male",
            Self::Female => "Female",
        }
    }

    pub fn portrait_path(self) -> &'static str {
        // Source files are JPEG bytes saved with a .png name in ba-stick; ship as .jpg.
        match self {
            Self::Male => "ui/boot/boot-portrait-male.jpg",
            Self::Female => "ui/boot/boot-portrait-female.jpg",
        }
    }

    /// Character Studio Anata pack key (male / female body + face).
    pub fn studio_key(self) -> &'static str {
        match self {
            Self::Male => "male",
            Self::Female => "female",
        }
    }

    pub const ALL: [Appearance; 2] = [Self::Male, Self::Female];
}

#[derive(Resource, Clone, Debug)]
pub struct CharacterDraft {
    pub name: String,
    pub job: JobId,
    pub appearance: Appearance,
    pub name_focused: bool,
}

impl Default for CharacterDraft {
    fn default() -> Self {
        Self {
            name: "Chrona".into(),
            job: JobId::TimeMage,
            appearance: Appearance::Male,
            name_focused: false,
        }
    }
}

#[derive(Component)]
pub struct ForgeMenuRoot;

#[derive(Component)]
pub(crate) struct ForgeNameText;

#[derive(Component)]
pub(crate) struct ForgeHintText;

#[derive(Component)]
pub(crate) struct ForgeSubText;

#[derive(Component)]
pub(crate) struct EnterButton;

#[derive(Component)]
pub(crate) struct JobButton(JobId);

#[derive(Component)]
pub(crate) struct JobLabel;

#[derive(Component)]
pub(crate) struct AppearanceButton(Appearance);

#[derive(Component)]
pub(crate) struct CustomizeAppearanceButton;

#[derive(Component)]
pub(crate) struct NameField;

#[derive(Component)]
pub struct CharacterName(pub String);

#[derive(Component)]
pub struct CharacterAppearance(pub Appearance);

fn ignore_pick() -> Pickable {
    Pickable::IGNORE
}

pub fn spawn_forge_menu(
    mut commands: Commands,
    assets: Res<AssetServer>,
    vfx: Res<ForgeVfx>,
) {
    let backdrop = assets.load("ui/boot/forge-backdrop.png");
    let banner = assets.load("ui/black-anvil/slices/a1-title-banner.png");
    let vfx_image = vfx.image.clone();

    commands
        .spawn((
            ForgeMenuRoot,
            Node {
                width: percent(100),
                height: percent(100),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(px(16)),
                ..default()
            },
            BackgroundColor(forge::VOID),
            ZIndex(40),
        ))
        .with_children(|root| {
            // Full-bleed forge photo — must not steal clicks.
            root.spawn((
                ImageNode {
                    image: backdrop,
                    image_mode: bevy::ui::widget::NodeImageMode::Stretch,
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    left: px(0),
                    top: px(0),
                    ..default()
                },
                ignore_pick(),
                ZIndex(0),
            ));

            // Soft vignette under the living pixel pass.
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    left: px(0),
                    top: px(0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.016, 0.012, 0.024, 0.18)),
                ignore_pick(),
                ZIndex(1),
            ));

            // Full-screen content-aware haze + embers (ba-stick photo-pixels).
            root.spawn((
                ForgeVfxLayer,
                ImageNode {
                    image: vfx_image,
                    image_mode: bevy::ui::widget::NodeImageMode::Stretch,
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    left: px(0),
                    top: px(0),
                    ..default()
                },
                ignore_pick(),
                ZIndex(2),
            ));

            // boot-frame card
            root.spawn((
                Node {
                    width: px(560),
                    max_width: percent(96),
                    max_height: percent(96),
                    padding: UiRect::all(px(18)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    border: UiRect::all(px(1)),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                BackgroundColor(forge::FRAME),
                BorderColor::all(Color::srgba(0.816, 0.439, 0.188, 0.55)),
                ZIndex(3),
            ))
            .with_children(|card| {
                card.spawn((
                    ImageNode::new(banner),
                    ignore_pick(),
                    Node {
                        width: percent(100),
                        height: px(56),
                        ..default()
                    },
                ));

                card.spawn((
                    ForgeSubText,
                    Text::new("Create your adventurer"),
                    TextFont::from_font_size(14.0),
                    TextColor(forge::SUB),
                    Node {
                        align_self: AlignSelf::Center,
                        ..default()
                    },
                ));

                // Name
                card.spawn((
                    Text::new("Character name  ·  click field to type"),
                    TextFont::from_font_size(12.0),
                    TextColor(forge::SUB),
                ));
                card.spawn((
                    NameField,
                    Pickable::default(),
                    Node {
                        width: percent(100),
                        padding: UiRect::axes(px(12), px(8)),
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                    BackgroundColor(forge::INPUT_BG),
                    BorderColor::all(forge::BRONZE_DIM),
                ))
                .with_children(|field| {
                    field.spawn((
                        ForgeNameText,
                        Text::new("Chrona"),
                        TextFont::from_font_size(16.0),
                        TextColor(forge::CREAM),
                    ));
                });

                // Appearance
                card.spawn((
                    Text::new("Appearance"),
                    TextFont::from_font_size(12.0),
                    TextColor(forge::SUB),
                ));
                card.spawn(Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    column_gap: px(8),
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|row| {
                    for appearance in Appearance::ALL {
                        let img = assets.load(appearance.portrait_path());
                        let selected = appearance == Appearance::Male;
                        row.spawn((
                            AppearanceButton(appearance),
                            Pickable::default(),
                            Node {
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                padding: UiRect::all(px(4)),
                                border: UiRect::all(px(2)),
                                row_gap: px(2),
                                flex_grow: 1.0,
                                ..default()
                            },
                            BackgroundColor(if selected {
                                forge::SELECT
                            } else {
                                Color::NONE
                            }),
                            BorderColor::all(if selected {
                                forge::SELECT_BORDER
                            } else {
                                Color::srgba(0.541, 0.439, 0.188, 0.35)
                            }),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                ImageNode::new(img),
                                ignore_pick(),
                                Node {
                                    width: px(72),
                                    height: px(72),
                                    ..default()
                                },
                            ));
                            btn.spawn((
                                Text::new(appearance.label()),
                                TextFont::from_font_size(11.0),
                                TextColor(forge::SUB),
                            ));
                        });
                    }
                });

                // Job grid — all classes visible and clickable
                card.spawn((
                    Text::new("Main job  ·  click a class"),
                    TextFont::from_font_size(12.0),
                    TextColor(forge::SUB),
                ));
                card.spawn(Node {
                    width: percent(100),
                    display: Display::Grid,
                    grid_template_columns: RepeatedGridTrack::flex(2, 1.0),
                    row_gap: px(6),
                    column_gap: px(6),
                    ..default()
                })
                .with_children(|grid| {
                    for job in JobId::ALL {
                        let selected = job == JobId::TimeMage;
                        grid.spawn((
                            JobButton(job),
                            Pickable::default(),
                            Node {
                                width: percent(100),
                                min_height: px(40),
                                padding: UiRect::axes(px(10), px(8)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(px(1)),
                                ..default()
                            },
                            BackgroundColor(if selected {
                                forge::SELECT
                            } else {
                                forge::JOB_BG
                            }),
                            BorderColor::all(if selected {
                                forge::SELECT_BORDER
                            } else {
                                forge::BRONZE_DIM
                            }),
                        ))
                        .with_children(|cell| {
                            cell.spawn((
                                JobLabel,
                                Text::new(job.display_name()),
                                TextFont::from_font_size(14.0),
                                TextColor(if selected {
                                    forge::BRONZE_HI
                                } else {
                                    forge::CREAM
                                }),
                            ));
                        });
                    }
                });

                card.spawn((
                    ForgeHintText,
                    Text::new(JobId::TimeMage.blurb()),
                    TextFont::from_font_size(12.0),
                    TextColor(Color::srgb(0.659, 0.596, 0.471)),
                    Node {
                        margin: UiRect::top(px(2)),
                        ..default()
                    },
                ));

                card.spawn((
                    CustomizeAppearanceButton,
                    Pickable::default(),
                    Node {
                        width: percent(100),
                        margin: UiRect::top(px(6)),
                        padding: UiRect::axes(px(16), px(10)),
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                    BackgroundColor(forge::INPUT_BG),
                    BorderColor::all(forge::BRONZE_DIM),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("Customize hair / face / body  ·  Character Studio"),
                        TextFont::from_font_size(13.0),
                        TextColor(forge::CREAM),
                    ));
                });

                card.spawn((
                    EnterButton,
                    Pickable::default(),
                    Node {
                        width: percent(100),
                        margin: UiRect::top(px(8)),
                        padding: UiRect::axes(px(16), px(12)),
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(px(2)),
                        ..default()
                    },
                    BackgroundColor(forge::BRONZE),
                    BorderColor::all(forge::BRONZE_HI),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("Enter Black Anvil"),
                        TextFont::from_font_size(15.0),
                        TextColor(Color::srgb(0.102, 0.078, 0.047)),
                    ));
                });

                card.spawn((
                    Text::new("← → job  ·  1/2 male/female  ·  C customize  ·  Enter start"),
                    TextFont::from_font_size(11.0),
                    TextColor(Color::srgba(0.761, 0.659, 0.471, 0.75)),
                    Node {
                        align_self: AlignSelf::Center,
                        margin: UiRect::top(px(2)),
                        ..default()
                    },
                ));
            });
        });
}

pub fn hide_yard_hud_while_menu(
    flow: Res<GameFlow>,
    mut hud: Query<&mut Visibility, Or<(With<HudRoot>, With<SheetText>)>>,
) {
    let vis = if flow.in_menu() {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut v in &mut hud {
        *v = vis;
    }
}

pub fn ensure_sheet_root_hidden(
    flow: Res<GameFlow>,
    sheets: Query<&ChildOf, With<SheetText>>,
    mut vis: Query<&mut Visibility>,
) {
    let want = if flow.in_menu() {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for child_of in &sheets {
        if let Ok(mut v) = vis.get_mut(child_of.parent()) {
            *v = want;
        }
    }
}

pub fn free_cursor_on_menu(flow: Res<GameFlow>, mut cursor: Query<&mut CursorOptions>) {
    if !flow.in_menu() {
        return;
    }
    for mut c in &mut cursor {
        c.visible = true;
        c.grab_mode = CursorGrabMode::None;
    }
}

pub fn type_forge_name(
    mut events: MessageReader<KeyboardInput>,
    mut draft: ResMut<CharacterDraft>,
    flow: Res<GameFlow>,
    note: Res<NoteComposer>,
    mut name_q: Query<&mut Text, With<ForgeNameText>>,
) {
    if !flow.in_menu() || note.active || !draft.name_focused {
        return;
    }
    for ev in events.read() {
        if ev.state != ButtonState::Pressed {
            continue;
        }
        match ev.key_code {
            KeyCode::Backspace => {
                draft.name.pop();
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {}
            _ => {
                if let Some(text) = &ev.text {
                    for ch in text.chars() {
                        if ch.is_control() {
                            continue;
                        }
                        if draft.name.chars().count() >= 16 {
                            break;
                        }
                        if ch.is_ascii_alphanumeric() || ch == ' ' || ch == '-' || ch == '_' {
                            if draft.name.is_empty() {
                                draft.name.push(ch.to_ascii_uppercase());
                            } else {
                                draft.name.push(ch);
                            }
                        }
                    }
                }
            }
        }
    }
    if let Ok(mut text) = name_q.single_mut() {
        **text = if draft.name.is_empty() {
            " ".into()
        } else {
            draft.name.clone()
        };
    }
}

pub fn forge_menu_clicks(
    mut draft: ResMut<CharacterDraft>,
    mut flow: ResMut<GameFlow>,
    mut commands: Commands,
    kits: Res<CombatKits>,
    mut clicks: MessageReader<PointerClick>,
    markers: Query<(
        Option<&EnterButton>,
        Option<&CustomizeAppearanceButton>,
        Option<&JobButton>,
        Option<&AppearanceButton>,
        Option<&NameField>,
    )>,
    mut hint_text: Query<&mut Text, (With<ForgeHintText>, Without<ForgeNameText>)>,
    mut style: ParamSet<(
        Query<(&JobButton, &mut BorderColor, &mut BackgroundColor, &Children)>,
        Query<(&AppearanceButton, &mut BorderColor, &mut BackgroundColor)>,
        Query<&mut TextColor, With<JobLabel>>,
    )>,
    menu_root: Query<Entity, With<ForgeMenuRoot>>,
    mut player: Query<(Entity, &mut Progression, Option<&mut Fighter>), With<crate::player::Player>>,
    parents: Query<&ChildOf>,
) {
    if !flow.in_menu() {
        return;
    }

    let mut enter = false;
    let mut open_studio = false;
    for click in clicks.read() {
        if click.button != PointerButton::Primary {
            continue;
        }
        let mut entity = click.entity;
        for _ in 0..10 {
            if let Ok((is_enter, customize, job, appear, name_field)) = markers.get(entity) {
                if is_enter.is_some() {
                    enter = true;
                    break;
                }
                if customize.is_some() {
                    open_studio = true;
                    draft.name_focused = false;
                    break;
                }
                if let Some(JobButton(job)) = job {
                    draft.job = *job;
                    draft.name_focused = false;
                    break;
                }
                if let Some(AppearanceButton(a)) = appear {
                    draft.appearance = *a;
                    draft.name_focused = false;
                    break;
                }
                if name_field.is_some() {
                    draft.name_focused = true;
                    break;
                }
            }
            let Ok(child_of) = parents.get(entity) else {
                break;
            };
            entity = child_of.parent();
        }
    }

    if open_studio {
        open_character_studio(draft.appearance);
    }

    sync_draft_visuals(&draft, &mut hint_text, &mut style);

    if enter {
        enter_yard(
            &draft,
            &mut flow,
            &mut commands,
            &kits,
            &menu_root,
            &mut player,
        );
    }
}

pub fn forge_menu_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut draft: ResMut<CharacterDraft>,
    mut flow: ResMut<GameFlow>,
    mut commands: Commands,
    kits: Res<CombatKits>,
    note: Res<NoteComposer>,
    menu_root: Query<Entity, With<ForgeMenuRoot>>,
    mut player: Query<(Entity, &mut Progression, Option<&mut Fighter>), With<crate::player::Player>>,
    mut hint_text: Query<&mut Text, (With<ForgeHintText>, Without<ForgeNameText>)>,
    mut style: ParamSet<(
        Query<(&JobButton, &mut BorderColor, &mut BackgroundColor, &Children)>,
        Query<(&AppearanceButton, &mut BorderColor, &mut BackgroundColor)>,
        Query<&mut TextColor, With<JobLabel>>,
    )>,
) {
    if !flow.in_menu() || note.active {
        return;
    }

    if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyQ) {
        let all = JobId::ALL;
        let idx = all.iter().position(|j| *j == draft.job).unwrap_or(0);
        draft.job = all[(idx + all.len() - 1) % all.len()];
        draft.name_focused = false;
    }
    if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyE) {
        let all = JobId::ALL;
        let idx = all.iter().position(|j| *j == draft.job).unwrap_or(0);
        draft.job = all[(idx + 1) % all.len()];
        draft.name_focused = false;
    }
    if keys.just_pressed(KeyCode::Digit1) {
        draft.appearance = Appearance::Male;
        draft.name_focused = false;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        draft.appearance = Appearance::Female;
        draft.name_focused = false;
    }
    if keys.just_pressed(KeyCode::KeyC) && !draft.name_focused {
        open_character_studio(draft.appearance);
    }

    sync_draft_visuals(&draft, &mut hint_text, &mut style);

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter) {
        enter_yard(
            &draft,
            &mut flow,
            &mut commands,
            &kits,
            &menu_root,
            &mut player,
        );
    }
}

/// Opens the local Character Studio (Anata male/female + face blendshapes).
/// Expects `scripts/run-character-studio.ps1` / `npm run dev` on :5173.
fn open_character_studio(appearance: Appearance) {
    let url = format!(
        "http://127.0.0.1:5173/?ba_gender={}",
        appearance.studio_key()
    );
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", &url])
        .spawn();
}

fn sync_draft_visuals(
    draft: &CharacterDraft,
    hint_text: &mut Query<&mut Text, (With<ForgeHintText>, Without<ForgeNameText>)>,
    style: &mut ParamSet<(
        Query<(&JobButton, &mut BorderColor, &mut BackgroundColor, &Children)>,
        Query<(&AppearanceButton, &mut BorderColor, &mut BackgroundColor)>,
        Query<&mut TextColor, With<JobLabel>>,
    )>,
) {
    if let Ok(mut t) = hint_text.single_mut() {
        **t = draft.job.blurb().into();
    }

    let mut label_updates: Vec<(Entity, bool)> = Vec::new();
    {
        let mut job_btns = style.p0();
        for (JobButton(job), mut border, mut bg, children) in job_btns.iter_mut() {
            let selected = *job == draft.job;
            *border = BorderColor::all(if selected {
                forge::SELECT_BORDER
            } else {
                forge::BRONZE_DIM
            });
            *bg = BackgroundColor(if selected {
                forge::SELECT
            } else {
                forge::JOB_BG
            });
            for child in children.iter() {
                label_updates.push((child, selected));
            }
        }
    }
    {
        let mut labels = style.p2();
        for (child, selected) in label_updates {
            if let Ok(mut color) = labels.get_mut(child) {
                *color = TextColor(if selected {
                    forge::BRONZE_HI
                } else {
                    forge::CREAM
                });
            }
        }
    }
    {
        let mut appear_btns = style.p1();
        for (AppearanceButton(a), mut border, mut bg) in appear_btns.iter_mut() {
            let selected = *a == draft.appearance;
            *border = BorderColor::all(if selected {
                forge::SELECT_BORDER
            } else {
                Color::srgba(0.541, 0.439, 0.188, 0.35)
            });
            *bg = BackgroundColor(if selected {
                forge::SELECT
            } else {
                Color::NONE
            });
        }
    }
}

fn enter_yard(
    draft: &CharacterDraft,
    flow: &mut GameFlow,
    commands: &mut Commands,
    kits: &CombatKits,
    menu_root: &Query<Entity, With<ForgeMenuRoot>>,
    player: &mut Query<(Entity, &mut Progression, Option<&mut Fighter>), With<crate::player::Player>>,
) {
    let name = {
        let trimmed = draft.name.trim();
        if trimmed.is_empty() {
            "Chrona".to_string()
        } else {
            trimmed.to_string()
        }
    };

    if let Ok((entity, mut prog, fighter)) = player.single_mut() {
        prog.job = draft.job;
        prog.sync_sheet();
        prog.hp = prog.vitals.max_hp;
        prog.mp = prog.vitals.max_mp;
        if let Some(mut f) = fighter {
            rebuild_hotbar(&mut f, kits, prog.job, prog.level);
        }
        commands
            .entity(entity)
            .insert((CharacterName(name.clone()), CharacterAppearance(draft.appearance)));
    }

    for entity in menu_root.iter() {
        commands.entity(entity).despawn();
    }
    *flow = GameFlow::InYard;
    info!("Entered Greymere Yard as {name} ({})", draft.job.display_name());
}
