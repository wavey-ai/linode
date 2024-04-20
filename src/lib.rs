pub mod regions;

use crate::regions::{RegionInfo, REGIONS};
use rand::{distributions::Alphanumeric, Rng};
use reqwest::{Client, Error};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use svix_ksuid::*;

const API_HOST: &str = "https://api.linode.com/v4/";

#[derive(Debug, Serialize, Deserialize)]
pub struct LinodeInstance {
    pub id: u64,
    pub label: String,
    group: String,
    status: String,
    created: String,
    updated: String,
    #[serde(rename = "type")]
    pub instance_type: String,
    pub ipv4: Vec<String>,
    pub ipv6: String,
    image: Option<String>,
    pub region: String,
    specs: InstanceSpecs,
    alerts: InstanceAlerts,
    backups: InstanceBackups,
    hypervisor: String,
    watchdog_enabled: bool,
    pub tags: Vec<String>,
    host_uuid: String,
    has_user_data: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct InstanceSpecs {
    disk: u32,
    memory: u32,
    vcpus: u32,
    gpus: u32,
    transfer: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct InstanceAlerts {
    cpu: u32,
    network_in: u32,
    network_out: u32,
    transfer_quota: u32,
    io: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct InstanceBackups {
    enabled: bool,
    available: bool,
    schedule: Option<BackupSchedule>,
    last_successful: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BackupSchedule {
    day: Option<String>,
    window: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LinodeResponse {
    data: Vec<LinodeInstance>,
    page: u32,
    pages: u32,
    results: u32,
}

#[derive(Serialize, Deserialize)]
struct LinodeInstanceCreateOptions {
    image: String,
    tags: Vec<String>,
    label: String,
    region: String,
    #[serde(rename = "type")]
    instance_type: String,
    root_pass: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Helpers {
    updatedb_disabled: bool,
    distro: bool,
    modules_dep: bool,
    network: bool,
    devtmpfs_automount: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Devices {
    sda: Option<DiskInfo>,
    sdb: Option<DiskInfo>,
    sdc: Option<DiskInfo>,
    sdd: Option<DiskInfo>,
    sde: Option<DiskInfo>,
    sdf: Option<DiskInfo>,
    sdg: Option<DiskInfo>,
    sdh: Option<DiskInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiskInfo {
    disk_id: u64,
    volume_id: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Configuration {
    pub id: u64,
    label: String,
    helpers: Helpers,
    kernel: String,
    comments: String,
    memory_limit: u64,
    created: String,
    updated: String,
    root_device: String,
    devices: Devices,
    initrd: Option<String>,
    run_level: String,
    virt_mode: String,
    interfaces: Vec<Interface>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Interfaces {
    pub interfaces: Vec<Interface>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Interface {
    pub purpose: String,
    pub ipam_address: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DomainRecordOptions {
    #[serde(rename = "type")]
    record_type: String,
    name: String,
    target: String,
    ttl_sec: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DomainRecord {
    pub id: u64,
    #[serde(rename = "type")]
    pub record_type: String,
    pub name: String,
    pub target: String,
    priority: Option<i32>,
    weight: Option<i32>,
    port: Option<i32>,
    service: Option<String>,
    protocol: Option<String>,
    ttl_sec: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct DomainRecordsResponse {
    data: Vec<DomainRecord>,
    page: u64,
    pages: u64,
    results: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct InstanceConfigurationsResponse {
    data: Vec<Configuration>,
    page: u64,
    pages: u64,
    results: u64,
}

pub struct LinodeClient {
    token: String,
    client: Client,
}

impl LinodeClient {
    pub fn new(token: String) -> Self {
        LinodeClient {
            token,
            client: Client::new(),
        }
    }

    pub async fn fetch_records(&self, domain: u64) -> Result<Vec<DomainRecord>, Error> {
        let response = self
            .client
            .get(format!("{}/domains/{}/records", API_HOST, domain))
            .bearer_auth(&self.token)
            .send()
            .await?
            .json::<DomainRecordsResponse>()
            .await?;

        Ok(response.data)
    }

    pub async fn delete_record(&self, domain: u64, id: u64) -> Result<(), Error> {
        self.client
            .delete(format!("{}/domains/{}/records/{}", API_HOST, domain, id))
            .bearer_auth(&self.token)
            .send()
            .await?;

        Ok(())
    }

    pub async fn create_a_record(
        &self,
        domain: u64,
        name: String,
        target: String,
    ) -> Result<(), Error> {
        let options = DomainRecordOptions {
            record_type: "A".to_owned(),
            name,
            target,
            ttl_sec: 3600,
        };
        self.client
            .post(format!("{}/domains/{}/records", API_HOST, domain))
            .bearer_auth(&self.token)
            .json(&options)
            .send()
            .await?;

        Ok(())
    }

    pub async fn fetch_instances(&self) -> Result<Vec<LinodeInstance>, Error> {
        let response = self
            .client
            .get(format!("{}/linode/instances?page_size=500", API_HOST))
            .bearer_auth(&self.token)
            .send()
            .await?
            .json::<LinodeResponse>()
            .await?;

        Ok(response.data)
    }

    pub async fn get_instance_configurations(&self, id: u64) -> Result<Vec<Configuration>, Error> {
        let response = self
            .client
            .get(format!("{}/linode/instances/{}/configs", API_HOST, id))
            .bearer_auth(&self.token)
            .send()
            .await?
            .json::<InstanceConfigurationsResponse>()
            .await?;

        Ok(response.data)
    }

    pub async fn get_instances_by_tag(
        &self,
        tags: Vec<&str>,
    ) -> Result<Vec<LinodeInstance>, Error> {
        let instances = self.fetch_instances().await?;
        let filtered_instances = instances
            .into_iter()
            .filter(|instance| {
                tags.iter()
                    .all(|tag| instance.tags.contains(&tag.to_string()))
            })
            .collect();

        Ok(filtered_instances)
    }

    pub async fn set_interfaces(
        &self,
        id: u64,
        config_id: u64,
        interfaces: Interfaces,
    ) -> Result<(), Error> {
        self.client
            .put(format!(
                "{}/linode/instances/{}/configs/{}",
                API_HOST, id, config_id
            ))
            .bearer_auth(&self.token)
            .json(&interfaces)
            .send()
            .await?;

        Ok(())
    }

    pub async fn destroy_instance(&self, id: u64) -> Result<(), Error> {
        self.client
            .delete(format!("{}/linode/instances/{}", API_HOST, id,))
            .bearer_auth(&self.token)
            .send()
            .await?;

        Ok(())
    }

    pub async fn reboot_instance(&self, id: u64) -> Result<(), Error> {
        self.client
            .post(format!("{}/linode/instances/{}/reboot", API_HOST, id,))
            .bearer_auth(&self.token)
            .send()
            .await?;

        Ok(())
    }

    pub async fn create_linode_instance(
        &self,
        image: String,
        tags: Vec<String>,
        label: String,
        region: String,
    ) -> Result<LinodeInstance, Error> {
        let instance_type = if self.is_legacy_region(&region) {
            "g6-dedicated-2"
        } else {
            "g7-premium-2"
        }
        .to_string();

        let password = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect::<String>();

        let options = LinodeInstanceCreateOptions {
            image,
            tags,
            label,
            region,
            instance_type,
            root_pass: password,
        };

        let response = self
            .client
            .post(format!("{}/linode/instances", API_HOST))
            .bearer_auth(&self.token)
            .json(&options)
            .send()
            .await?
            .json::<LinodeInstance>()
            .await?;
        Ok(response)
    }

    fn is_legacy_region(&self, region: &str) -> bool {
        REGIONS
            .get(region)
            .map(|info| info.is_legacy)
            .unwrap_or(false)
    }

    // remove an instance that has a particular tag
    pub async fn scale_down_one(
        &self,
        domain: u64,
        region: &RegionInfo,
        tag: &str,
    ) -> Result<(), Error> {
        let instances = self.get_instances_by_tag(vec![tag, region.code]).await?;
        let records = self.fetch_records(domain).await.unwrap();

        let mut a_records = HashMap::new();
        for record in records {
            a_records.insert(record.name, record.id);
        }

        for instance in instances {
            if let Some(id) = a_records.get(&instance.label) {
                self.delete_record(domain, *id).await?;
                self.destroy_instance(instance.id).await?;

                break;
            }
        }

        Ok(())
    }

    // add an instance to the same VLAN as other linodes in a region
    pub async fn scale_up_one(
        &self,
        image_id: &str,
        domain: u64,
        region: &RegionInfo,
        tag: &str,
    ) -> Result<(), Error> {
        let instances = self.get_instances_by_tag(vec![tag, region.code]).await?;

        let mut cidrs: Vec<u8> = Vec::new();
        for instance in instances {
            let configs = self.get_instance_configurations(instance.id).await?;
            for config in &configs {
                for interface in &config.interfaces {
                    if let Some(label) = &interface.label {
                        if label == tag {
                            if let Some(ipam) = &interface.ipam_address {
                                let parts: Vec<&str> = ipam.split('/').collect();
                                let ip_parts: Vec<&str> = parts[0].split('.').collect();
                                match ip_parts[3].parse::<u8>() {
                                    Ok(n) => cidrs.push(n),
                                    Err(e) => eprintln!("Parsing error: {}", e),
                                }
                            }
                        }
                    }
                }
            }
        }

        let kid = Ksuid::new(None, None);
        let label = format!("{}-{}", region.code, kid.to_string());
        let instance = self
            .create_linode_instance(
                image_id.to_string(),
                vec![tag.to_string(), region.code.to_string()],
                label,
                region.region.to_string(),
            )
            .await?;

        let configs = self.get_instance_configurations(instance.id).await?;
        let config_id = configs[0].id;

        let cidr = if let Some(max) = cidrs.iter().max() {
            max + 1
        } else {
            1
        };

        // TODO: > 254
        let ipam = format!("10.0.0.{}/24", cidr);

        let new_interfaces = Interfaces {
            interfaces: vec![
                Interface {
                    purpose: "public".to_string(),
                    ipam_address: None,
                    label: None,
                },
                Interface {
                    label: Some(tag.to_string()),
                    ipam_address: Some(ipam),
                    purpose: "vlan".to_string(),
                },
            ],
        };

        self.set_interfaces(instance.id, config_id, new_interfaces)
            .await?;

        self.reboot_instance(instance.id).await?;
        self.create_a_record(domain, instance.label, instance.ipv4[0].clone())
            .await
    }
}
