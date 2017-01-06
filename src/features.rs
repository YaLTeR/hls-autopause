use hooks::{engine, server};
use libc::*;

struct Feature {
    name: &'static str,
    enabled: bool,
}

define_features! {
    (autopause, AUTOPAUSE, "autopause"),
    (console_commands, CONSOLE_COMMANDS, "console commands"),
    (autojump, AUTOJUMP, "autojump")
}

fn log() {
    info!(target: "", "Features:");

    for feature in FEATURES {
        if feature.enabled {
            info!(target: "", "✔ {}", feature.name);
        } else {
            warn!(target: "", "❌ {}", feature.name);
        }
    }
}

pub fn refresh() {
    unsafe {
        AUTOPAUSE.enabled =
            engine::engine.Cbuf_AddText != engine::Engine::Cbuf_AddText_default
            && engine::engine.Host_Spawn_f != engine::Engine::Host_Spawn_f_default
            && engine::engine.Host_UnPause_f != engine::Engine::Host_UnPause_f_default;

        CONSOLE_COMMANDS.enabled =
            engine::engine.icvar != 0 as *mut engine::icvar::ICVar
            && engine::engine.concommand_vtable != 0 as *mut c_void;

        AUTOJUMP.enabled =
            server::server.CHL1GameMovement__CheckJumpButton as *const () != server::Server::CHL1GameMovement__CheckJumpButton_default as *const ()
            && server::server.CGameMovement__FinishGravity as *const () != server::Server::CGameMovement__FinishGravity_default as *const ();
    }

    log();
}