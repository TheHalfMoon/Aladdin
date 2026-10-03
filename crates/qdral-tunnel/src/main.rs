use qdral_tunnel::{run, status, TunnelConfig};

fn main() {
    let command = std::env::args().nth(1).unwrap_or_else(|| "status".into());
    let config = match TunnelConfig::from_env() {
        Ok(config) => config,
        Err(error) => exit_error(&error),
    };

    match command.as_str() {
        "run" => match run(&config) {
            Ok(status) if status.success() => {}
            Ok(status) => {
                eprintln!(
                    "qdral-tunnel: tunnel-client exited with {}",
                    status
                        .code()
                        .map_or_else(|| "signal".into(), |v| v.to_string())
                );
                std::process::exit(3);
            }
            Err(error) => exit_error(&error),
        },
        "status" => println!("{}", status(&config)),
        "doctor" => println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "tunnel_id": config.tunnel_id,
                "credential_source": "file",
                "runtime_key_in_environment": false,
                "health_listen": "127.0.0.1:0"
            })
        ),
        _ => exit_error("usage: qdral-tunnel [run|status|doctor]"),
    }
}

fn exit_error(message: &str) -> ! {
    eprintln!("qdral-tunnel: {message}");
    std::process::exit(2)
}
