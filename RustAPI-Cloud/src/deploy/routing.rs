use std::path::Path;

/// DNS-safe slug from a Cargo package / project name.
pub fn slugify_project_name(name: &str) -> String {
    let mut slug = String::new();
    let mut prev_hyphen = false;
    for ch in name.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            slug.push(lower);
            prev_hyphen = false;
        } else if !prev_hyphen && !slug.is_empty() {
            slug.push('-');
            prev_hyphen = true;
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "app".into()
    } else {
        slug
    }
}

/// Stable per-user project subdomain (same project name redeploys keep the same host).
pub fn deploy_subdomain(project_name: &str, user_id: &str) -> String {
    let name_part = slugify_project_name(project_name);
    let user_prefix: String = user_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(8)
        .collect();
    let user_prefix = if user_prefix.is_empty() {
        "user".into()
    } else {
        user_prefix
    };
    format!("{name_part}-{user_prefix}")
}

pub fn deploy_hostname(public_host: &str, project_name: &str, user_id: &str) -> String {
    format!(
        "{}.{}",
        deploy_subdomain(project_name, user_id),
        public_host.trim().trim_end_matches('.')
    )
}

pub fn public_deploy_url(scheme: &str, public_host: &str, project_name: &str, user_id: &str) -> String {
    let host = deploy_hostname(public_host, project_name, user_id);
    format!(
        "{}://{}",
        scheme.trim_end_matches("://"),
        host
    )
}

/// Write one nginx map entry file consumed by `include /path/*.conf` inside a `map` block.
pub fn upsert_nginx_map_entry(map_dir: &Path, hostname: &str, port: u16) -> std::io::Result<()> {
    std::fs::create_dir_all(map_dir)?;
    let file_key = hostname.split('.').next().unwrap_or("app");
    let path = map_dir.join(format!("{file_key}.conf"));
    std::fs::write(path, format!("{hostname} {port};\n"))?;
    Ok(())
}

pub fn reload_nginx() -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        if std::process::Command::new("nginx")
            .arg("-s")
            .arg("reload")
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
        {
            return Ok(());
        }
        let status = std::process::Command::new("sudo")
            .args(["nginx", "-s", "reload"])
            .status()?;
        if !status.success() {
            return Err(std::io::Error::other("nginx reload failed"));
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = ();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_normalizes_project_names() {
        assert_eq!(slugify_project_name("My_RustAPI App"), "my-rustapi-app");
        assert_eq!(slugify_project_name("---"), "app");
    }

    #[test]
    fn deploy_subdomain_is_stable_per_user_project() {
        let user = "a5c231e4-4bbd-4c99-8993-771aacd3dd35";
        assert_eq!(
            deploy_subdomain("listener-app", user),
            "listener-app-a5c231e4"
        );
        assert_eq!(
            deploy_subdomain("listener-app", user),
            deploy_subdomain("listener-app", user)
        );
    }

    #[test]
    fn public_deploy_url_uses_https_and_wildcard_host() {
        let url = public_deploy_url(
            "https",
            "rustapi.tunayinbayramharcligi.com",
            "my-api",
            "a5c231e4-4bbd-4c99-8993-771aacd3dd35",
        );
        assert_eq!(
            url,
            "https://my-api-a5c231e4.rustapi.tunayinbayramharcligi.com"
        );
    }

    #[test]
    fn upsert_nginx_map_entry_writes_host_port_mapping() {
        let dir = tempfile::tempdir().expect("tempdir");
        upsert_nginx_map_entry(
            dir.path(),
            "my-api-a5c231e4.rustapi.tunayinbayramharcligi.com",
            30123,
        )
        .expect("write map");
        let content =
            std::fs::read_to_string(dir.path().join("my-api-a5c231e4.conf")).expect("read");
        assert_eq!(
            content,
            "my-api-a5c231e4.rustapi.tunayinbayramharcligi.com 30123;\n"
        );
    }
}