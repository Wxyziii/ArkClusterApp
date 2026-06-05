use std::path::Path;

use serde::Serialize;

use crate::config::Config;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub ready: bool,
    pub steamcmd: Check,
    pub ark_server: Check,
    pub shared_config: Check,
    pub cluster_dir: Check,
    pub backup_root: Check,
    pub ark_root: String,
    pub server_root: String,
    pub executable: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub ok: bool,
    pub path: String,
    pub message: String,
}

pub async fn status(config: &Config) -> RuntimeStatus {
    let ark_root = if config.paths.ark_root.trim().is_empty() {
        "/srv/ark"
    } else {
        &config.paths.ark_root
    };
    let server_root = format!("{ark_root}/server");
    let executable = format!("{server_root}/ShooterGame/Binaries/Linux/ShooterGameServer");
    let config_dir = format!("{server_root}/ShooterGame/Saved/Config/LinuxServer");
    let gus = format!("{config_dir}/GameUserSettings.ini");
    let game_ini = format!("{config_dir}/Game.ini");
    let cluster_dir = if config.paths.cluster_dir.trim().is_empty() {
        config.cluster.directory.clone()
    } else {
        config.paths.cluster_dir.clone()
    };
    let backup_root = if config.paths.backup_root.trim().is_empty() {
        config.backup_policy.directory.clone()
    } else {
        config.paths.backup_root.clone()
    };

    let steamcmd = steamcmd_check().await;
    let ark_server = path_check(&executable, "ARK dedicated server executable");
    let shared_config = if Path::new(&gus).is_file() && Path::new(&game_ini).is_file() {
        Check {
            ok: true,
            path: config_dir,
            message: "shared config present".into(),
        }
    } else {
        Check {
            ok: false,
            path: config_dir,
            message: "GameUserSettings.ini or Game.ini missing".into(),
        }
    };
    let cluster_dir_check = path_check(&cluster_dir, "shared cluster directory");
    let backup_root_check = path_check(&backup_root, "backup root");
    let ready = steamcmd.ok
        && ark_server.ok
        && shared_config.ok
        && cluster_dir_check.ok
        && backup_root_check.ok;

    RuntimeStatus {
        ready,
        steamcmd,
        ark_server,
        shared_config,
        cluster_dir: cluster_dir_check,
        backup_root: backup_root_check,
        ark_root: ark_root.into(),
        server_root,
        executable,
    }
}

fn path_check(path: &str, label: &str) -> Check {
    let ok = Path::new(path).exists();
    Check {
        ok,
        path: path.into(),
        message: if ok {
            format!("{label} present")
        } else {
            format!("{label} missing")
        },
    }
}

async fn steamcmd_check() -> Check {
    #[cfg(target_os = "linux")]
    {
        match tokio::process::Command::new("sh")
            .args([
                "-lc",
                "command -v steamcmd || test -x /usr/games/steamcmd && echo /usr/games/steamcmd",
            ])
            .output()
            .await
        {
            Ok(out) if out.status.success() => Check {
                ok: true,
                path: String::from_utf8_lossy(&out.stdout).trim().into(),
                message: "SteamCMD installed".into(),
            },
            _ => Check {
                ok: false,
                path: "steamcmd".into(),
                message: "SteamCMD missing".into(),
            },
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        Check {
            ok: false,
            path: "steamcmd".into(),
            message: "SteamCMD only checked on Linux".into(),
        }
    }
}
