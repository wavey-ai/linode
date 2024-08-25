use linode::regions::{RegionInfo, REGIONS};
use linode::LinodeClient;
use std::error::Error;
use structopt::StructOpt;
use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Debug, StructOpt)]
#[structopt(name = "linode", about = "Linode API abstractions")]
struct Command {
    #[structopt(long, env = "LINODE_API_TOKEN")]
    token: String,

    #[structopt(long, env = "LINODE_PUB_KEY")]
    pub_key: String,

    #[structopt(subcommand)]
    action: Action,
}

#[derive(Debug, StructOpt)]
enum Action {
    Dns(DnsAction),
    Scale(ScaleAction),
}

#[derive(Debug, StructOpt)]
enum DnsAction {
    Ls {
        #[structopt(long)]
        domain_id: u64,
    },
}

#[derive(Debug, StructOpt)]
enum ScaleAction {
    Up {
        #[structopt(long)]
        image_id: String,

        #[structopt(long)]
        instance_type: String,

        #[structopt(long)]
        domain_id: u64,

        #[structopt(long)]
        region: String,

        #[structopt(long)]
        tag: String,

        #[structopt(long, default_value = "1")]
        n: u32,
    },
    Down {
        #[structopt(long)]
        domain_id: u64,

        #[structopt(long)]
        region: String,

        #[structopt(long)]
        tag: String,

        #[structopt(long, default_value = "1")]
        n: u32,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::new("info"))
        .with(tracing_subscriber::fmt::Layer::default());

    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to set global default subscriber");

    let args = Command::from_args();

    let client = LinodeClient::new(args.token, args.pub_key)?;

    match args.action {
        Action::Scale(action) => match action {
            ScaleAction::Up {
                image_id,
                instance_type,
                domain_id,
                region,
                tag,
                n,
            } => {
                if let Some(region_info) = REGIONS.get(region.as_str()) {
                    for _ in 0..n {
                        client
                            .scale_up_one(&image_id, &instance_type, domain_id, region_info, &tag)
                            .await
                            .map_err(|e| format!("Failed to scale up: {}", e))?;
                    }
                    println!(
                        "Scaled up {} instance(s) in region: {}",
                        n, region_info.region
                    );
                } else {
                    eprintln!("Region code '{}' not found.", region);
                }
            }
            ScaleAction::Down {
                domain_id,
                region,
                tag,
                n,
            } => {
                if let Some(region_info) = REGIONS.get(region.as_str()) {
                    for _ in 0..n {
                        client
                            .scale_down_one(domain_id, region_info, &tag)
                            .await
                            .map_err(|e| format!("Failed to scale down: {}", e))?;
                    }
                    println!(
                        "Scaled down {} instance(s) in region: {}",
                        n, region_info.region
                    );
                } else {
                    eprintln!("Region code '{}' not found.", region);
                }
            }
        },
        Action::Dns(DnsAction::Ls { domain_id }) => {
            if let Ok(records) = client.fetch_records(domain_id).await {
                for rec in &records {
                    if rec.record_type == "A" {
                        dbg!(rec);
                    }
                }
            }
        }
    }

    Ok(())
}
