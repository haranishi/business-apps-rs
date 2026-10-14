// SPDX-License-Identifier: Apache-2.0
use business_apps_server::{local_address, router, store::Store};
use std::{env, path::Path};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address =
        local_address(&env::var("BIZAPP_BIND").unwrap_or_else(|_| "127.0.0.1:8080".into()))?;
    let database = env::var("BIZAPP_DB").unwrap_or_else(|_| ".data/business-apps.sqlite".into());
    if let Some(parent) = Path::new(&database)
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let store = Store::open(database)?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    println!(
        "business-apps-rs local preview: http://{}",
        listener.local_addr()?
    );
    axum::serve(listener, router(store))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
