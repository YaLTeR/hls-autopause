use hooks::*;

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
        let engine = engine::POINTERS.read().unwrap();
        let server = server::POINTERS.read().unwrap();

        AUTOPAUSE.enabled =
            !engine.Cbuf_AddText.is_default()
            && !engine.Host_Spawn_f.is_default()
            && !engine.Host_UnPause_f.is_default();

        CONSOLE_COMMANDS.enabled =
            engine.icvar.is_some()
            && engine.concommand_vtable.is_some();

        AUTOJUMP.enabled =
            !server.CHL1GameMovement__CheckJumpButton.is_default()
            && !server.CGameMovement__FinishGravity.is_default()
    }

    log();
}
