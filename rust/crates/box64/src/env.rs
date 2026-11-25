use std::path::Path;
use androidpc_configuration::{is_android_bionic_path, Box64EnvironmentConfig};

/// Builds guest-scoped `BOX64_*` environment variables for a single execution.
///
/// Strictly avoids touching global guest environment files and strips any Android/Bionic paths.
pub fn build_box64_env_vars(
    config: &Box64EnvironmentConfig,
    executable_path: Option<&str>,
    rcfile_guest_path: Option<&str>,
) -> Vec<(String, String)> {
    let exe_basename = executable_path
        .and_then(|p| Path::new(p).file_name())
        .and_then(|n| n.to_str());

    let app_override = exe_basename.and_then(|name| config.app_overrides.get(name));

    let dynarec = app_override
        .and_then(|o| o.dynarec_enabled)
        .unwrap_or(config.dynarec_enabled);
    let safeflags = app_override
        .and_then(|o| o.dynarec_safeflags)
        .unwrap_or(config.dynarec_safeflags)
        .min(2);
    let strongmem = app_override
        .and_then(|o| o.dynarec_strongmem)
        .unwrap_or(config.dynarec_strongmem)
        .min(3);
    let bigblock = app_override
        .and_then(|o| o.dynarec_bigblock)
        .unwrap_or(config.dynarec_bigblock)
        .min(3);
    let log_level = app_override
        .and_then(|o| o.log_level)
        .unwrap_or(config.log_level);

    let mut vars = vec![
        ("BOX64_DYNAREC".to_string(), if dynarec { "1" } else { "0" }.to_string()),
        ("BOX64_DYNAREC_SAFEFLAGS".to_string(), safeflags.to_string()),
        ("BOX64_DYNAREC_STRONGMEM".to_string(), strongmem.to_string()),
        ("BOX64_DYNAREC_BIGBLOCK".to_string(), bigblock.to_string()),
        ("BOX64_LOG".to_string(), log_level.as_box64_env_value().to_string()),
        ("BOX64_DYNAREC_LOG".to_string(), config.dynarec_log_level.min(3).to_string()),
        ("BOX64_SHOWSEGV".to_string(), if config.show_segv { "1" } else { "0" }.to_string()),
        ("BOX64_SHOWBT".to_string(), if config.show_bt { "1" } else { "0" }.to_string()),
        // Conservative default: keep guest glibc + Box64 wrapped libs separated from host Android libs.
        ("BOX64_NOBANNER".to_string(), "0".to_string()),
    ];

    let safe_lib_paths: Vec<&str> = config
        .library_search_paths
        .iter()
        .map(String::as_str)
        .filter(|p| p.starts_with('/') && !p.contains("..") && !is_android_bionic_path(p))
        .collect();

    if !safe_lib_paths.is_empty() {
        vars.push(("BOX64_LD_LIBRARY_PATH".to_string(), safe_lib_paths.join(":")));
    }

    if let Some(rc) = rcfile_guest_path {
        if rc.starts_with('/') && !is_android_bionic_path(rc) {
            vars.push(("BOX64_RCFILE".to_string(), rc.to_string()));
        }
    }

    for (k, v) in &config.env_vars {
        if !k.eq_ignore_ascii_case("LD_PRELOAD") {
            vars.push((k.clone(), v.clone()));
        }
    }

    if let Some(ov) = app_override {
        for (k, v) in &ov.extra_env {
            if !k.eq_ignore_ascii_case("LD_PRELOAD") {
                vars.push((k.clone(), v.clone()));
            }
        }
    }

    vars
}

/// Renders a per-environment `.box64rc` file containing conservative defaults and application overrides.
pub fn render_box64_rcfile(config: &Box64EnvironmentConfig) -> String {
    let mut out = String::from(
        "# Android PC Environment — Per-environment Box64 configuration\n\
         # Generated automatically with conservative defaults.\n\n\
         [*]\n",
    );
    out.push_str(&format!(
        "BOX64_DYNAREC={}\n",
        if config.dynarec_enabled { 1 } else { 0 }
    ));
    out.push_str(&format!(
        "BOX64_DYNAREC_SAFEFLAGS={}\n",
        config.dynarec_safeflags.min(2)
    ));
    out.push_str(&format!(
        "BOX64_DYNAREC_STRONGMEM={}\n",
        config.dynarec_strongmem.min(3)
    ));
    out.push_str(&format!(
        "BOX64_DYNAREC_BIGBLOCK={}\n",
        config.dynarec_bigblock.min(3)
    ));
    out.push_str(&format!(
        "BOX64_LOG={}\n",
        config.log_level.as_box64_env_value()
    ));
    out.push_str(&format!(
        "BOX64_SHOWSEGV={}\n",
        if config.show_segv { 1 } else { 0 }
    ));
    out.push_str(&format!(
        "BOX64_SHOWBT={}\n",
        if config.show_bt { 1 } else { 0 }
    ));

    for (app, ov) in &config.app_overrides {
        out.push_str(&format!("\n[{app}]\n"));
        if let Some(d) = ov.dynarec_enabled {
            out.push_str(&format!("BOX64_DYNAREC={}\n", if d { 1 } else { 0 }));
        }
        if let Some(sf) = ov.dynarec_safeflags {
            out.push_str(&format!("BOX64_DYNAREC_SAFEFLAGS={}\n", sf.min(2)));
        }
        if let Some(sm) = ov.dynarec_strongmem {
            out.push_str(&format!("BOX64_DYNAREC_STRONGMEM={}\n", sm.min(3)));
        }
        if let Some(bb) = ov.dynarec_bigblock {
            out.push_str(&format!("BOX64_DYNAREC_BIGBLOCK={}\n", bb.min(3)));
        }
        if let Some(ll) = ov.log_level {
            out.push_str(&format!("BOX64_LOG={}\n", ll.as_box64_env_value()));
        }
        for (k, v) in &ov.extra_env {
            out.push_str(&format!("{k}={v}\n"));
        }
    }

    out
}
