mod atmosphere;
mod boss;
mod cel;
mod character;
mod data;
mod net;
mod debug_shot;
mod dialogue;
mod forge_menu;
mod forge_vfx;
mod pixel_present;
mod player;
mod rpg;
mod scene;
mod sky;
mod tree;
mod vfx;
mod well;

use std::path::PathBuf;

use bevy::prelude::*;

use atmosphere::{flicker_well_fire, tick_atmosphere, StickAtmosphere};
use boss::{
    boss_fight, on_possess_request, tick_possession, tick_warden_acting, PossessFx,
    WardenPossessRequest,
};
use cel::{cel_shade_figures, CelMaterial};
use character::{attach_player_avatar, despawn_stale_avatars, load_character_profile};
use data::load_game_data;
use net::{
    apply_zone_snapshots, connect_zone_on_enter, maybe_enable_zone_net, send_local_move, ZoneNet,
};
use debug_shot::{
    capture_debug_shot, spawn_debug_hud, toggle_debug_hud, type_note, update_debug_hud,
};
use dialogue::{
    announce_level_up, handle_interact, spawn_hud, update_proximity, update_sheet_hud,
};
use forge_menu::{
    ensure_sheet_root_hidden, forge_menu_clicks, forge_menu_keys, free_cursor_on_menu,
    hide_yard_hud_while_menu, spawn_forge_menu, type_forge_name, CharacterDraft, GameFlow,
};
use forge_vfx::{setup_forge_vfx, tick_forge_vfx};
use pixel_present::{attach_pixel_camera, fit_pixel_present, point_ui_at_present};
use player::{grab_cursor, look_player, move_player, spawn_player, NoteComposer};
use rpg::{
    apply_xp_grants, cycle_job, ensure_fighter, load_combat_kits, sync_hotbar_on_level, GrantXp,
    PlayerLeveledUp,
};
use scene::{spawn_opening_scene, tick_fireflies};
use sky::{follow_sky_to_player, spawn_sky_dome, sync_sky_uniforms, StickSkyMaterial};
use vfx::{
    spawn_level_up_vfx, start_dissolve, tick_ash, tick_book_fall, tick_dissolve, tick_level_up_fx,
    DeathDissolveRequest,
};
use well::{tick_well, tick_well_splash, WellPour};

fn main() {
    let asset_root = {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
        if manifest.is_dir() {
            manifest
        } else {
            PathBuf::from("assets")
        }
    };

    App::new()
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Black Anvil — Greymere Yard".into(),
                        resolution: (1280, 720).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: asset_root.to_string_lossy().into_owned(),
                    ..default()
                }),
            MaterialPlugin::<StickSkyMaterial>::default(),
            MaterialPlugin::<CelMaterial>::default(),
        ))
        .init_resource::<StickAtmosphere>()
        .init_resource::<NoteComposer>()
        .init_resource::<WellPour>()
        .init_resource::<PossessFx>()
        .init_resource::<GameFlow>()
        .init_resource::<CharacterDraft>()
        .init_resource::<ZoneNet>()
        .insert_resource(ClearColor(atmosphere::hex_rgb(atmosphere::SKY_TOP)))
        .add_message::<DeathDissolveRequest>()
        .add_message::<WardenPossessRequest>()
        .add_message::<GrantXp>()
        .add_message::<PlayerLeveledUp>()
        .add_systems(
            Startup,
            (
                load_game_data,
                load_combat_kits,
                load_character_profile,
                spawn_opening_scene.after(load_game_data),
                spawn_player.after(load_game_data),
                attach_pixel_camera.after(spawn_player),
                spawn_sky_dome.after(spawn_player),
                spawn_hud,
                spawn_debug_hud,
                setup_forge_vfx,
                spawn_forge_menu
                    .after(attach_pixel_camera)
                    .after(setup_forge_vfx)
                    .after(load_character_profile),
            ),
        )
        .add_systems(
            Update,
            (
                despawn_stale_avatars,
                attach_player_avatar.after(despawn_stale_avatars),
                maybe_enable_zone_net,
                connect_zone_on_enter,
                send_local_move.run_if(in_yard),
                apply_zone_snapshots.run_if(in_yard),
            ),
        )
        .add_systems(
            Update,
            (
                tick_atmosphere,
                flicker_well_fire,
                follow_sky_to_player,
                sync_sky_uniforms,
                fit_pixel_present,
                point_ui_at_present,
                free_cursor_on_menu,
                hide_yard_hud_while_menu,
                ensure_sheet_root_hidden,
                tick_forge_vfx,
                type_forge_name,
                forge_menu_clicks,
                forge_menu_keys,
            ),
        )
        .add_systems(
            Update,
            (
                grab_cursor.run_if(in_yard),
                look_player.run_if(in_yard),
                move_player.run_if(in_yard),
                update_proximity.run_if(in_yard),
                handle_interact.run_if(in_yard),
                on_possess_request.after(handle_interact).run_if(in_yard),
                tick_possession.after(on_possess_request).run_if(in_yard),
                tick_warden_acting.run_if(in_yard),
                ensure_fighter.run_if(in_yard),
                sync_hotbar_on_level.run_if(in_yard),
            ),
        )
        .add_systems(
            Update,
            (
                cycle_job.run_if(in_yard),
                boss_fight
                    .after(tick_possession)
                    .after(ensure_fighter)
                    .run_if(in_yard),
                apply_xp_grants
                    .after(handle_interact)
                    .after(boss_fight)
                    .run_if(in_yard),
                announce_level_up.after(apply_xp_grants).run_if(in_yard),
                update_sheet_hud.after(apply_xp_grants),
                start_dissolve.after(boss_fight).run_if(in_yard),
            ),
        )
        .add_systems(
            Update,
            (
                spawn_level_up_vfx.after(apply_xp_grants).run_if(in_yard),
                tick_dissolve.run_if(in_yard),
                tick_book_fall.run_if(in_yard),
                tick_ash.run_if(in_yard),
                tick_level_up_fx.run_if(in_yard),
                tick_fireflies,
                tick_well.run_if(in_yard),
                tick_well_splash.run_if(in_yard),
                cel_shade_figures,
                toggle_debug_hud,
                update_debug_hud,
                capture_debug_shot,
                type_note
                    .before(move_player)
                    .before(look_player)
                    .before(handle_interact)
                    .run_if(in_yard),
            ),
        )
        .run();
}

fn in_yard(flow: Res<GameFlow>) -> bool {
    !flow.in_menu()
}
