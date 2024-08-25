pub mod regions;

use crate::regions::{RegionInfo, REGIONS};
use rand::{distributions::Alphanumeric, Rng};
use regex::Regex;
use reqwest::{Client, Error};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, f32::consts::LOG2_E};
use svix_ksuid::*;
use tls_helpers::from_base64_raw;
use tracing::{error, info};

const A_RECORD: &str = "A";
const API_HOST: &str = "https://api.linode.com/v4/";
const LOCALHOST: &str = "127.0.0.1";

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

#[derive(Serialize, Deserialize, Debug)]
struct LinodeInstanceCreateOptions {
    image: String,
    tags: Vec<String>,
    label: String,
    region: String,
    #[serde(rename = "type")]
    instance_type: String,
    root_pass: String,
    authorized_keys: Vec<String>,
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
pub struct DomainRecordUpdateOptions {
    target: String,
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
    pub_key: String,
}

impl LinodeClient {
    pub fn new(
        token: String,
        pub_key: String,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut decoded_pub_key: Vec<u8> = from_base64_raw(&pub_key)?;

        // Strip any newline or whitespace characters from the end
        while let Some(&last_byte) = decoded_pub_key.last() {
            if last_byte == b'\n' || last_byte == b'\r' || last_byte == b' ' {
                decoded_pub_key.pop();
            } else {
                break;
            }
        }

        Ok(LinodeClient {
            token,
            pub_key: String::from_utf8_lossy(&decoded_pub_key).into_owned(),
            client: Client::new(),
        })
    }

    pub async fn fetch_records(&self, domain: u64) -> Result<Vec<DomainRecord>, Error> {
        info!("Fetching domain records for domain ID: {}", domain);
        let response = self
            .client
            .get(format!("{}/domains/{}/records", API_HOST, domain))
            .bearer_auth(&self.token)
            .send()
            .await?;

        info!("Parsing response into DomainRecordsResponse");
        let records = response.json::<DomainRecordsResponse>().await?;
        info!(
            "Fetched {} records for domain ID: {}",
            records.data.len(),
            domain
        );

        Ok(records.data)
    }

    pub async fn delete_record(&self, domain: u64, id: u64) -> Result<(), Error> {
        info!("Deleting record with ID: {} in domain ID: {}", id, domain);
        self.client
            .delete(format!("{}/domains/{}/records/{}", API_HOST, domain, id))
            .bearer_auth(&self.token)
            .send()
            .await?;

        info!("Record ID: {} deleted successfully", id);
        Ok(())
    }

    pub async fn update_record_target(
        &self,
        domain: u64,
        id: u64,
        target: &str,
    ) -> Result<(), Error> {
        info!(
            "Updating record ID: {} in domain ID: {} with new target: {}",
            id, domain, target
        );
        let options = DomainRecordUpdateOptions {
            target: target.to_owned(),
        };
        self.client
            .put(format!("{}/domains/{}/records/{}", API_HOST, domain, id))
            .bearer_auth(&self.token)
            .json(&options)
            .send()
            .await?;

        info!(
            "Record ID: {} updated successfully to target: {}",
            id, target
        );
        Ok(())
    }

    pub async fn create_a_record(
        &self,
        domain: u64,
        name: String,
        target: String,
    ) -> Result<(), Error> {
        info!(
            "Creating new A record in domain ID: {} with name: {} and target: {}",
            domain, name, target
        );
        let options = DomainRecordOptions {
            record_type: "A".to_owned(),
            name: name.clone(),
            target: target.clone(),
            ttl_sec: 3600,
        };
        self.client
            .post(format!("{}/domains/{}/records", API_HOST, domain))
            .bearer_auth(&self.token)
            .json(&options)
            .send()
            .await?;

        info!(
            "A record created successfully with name: {} in domain ID: {}",
            name, domain
        );
        Ok(())
    }

    pub async fn fetch_instances(&self) -> Result<Vec<LinodeInstance>, Error> {
        info!("Fetching all Linode instances");
        let response = self
            .client
            .get(format!("{}/linode/instances?page_size=500", API_HOST))
            .bearer_auth(&self.token)
            .send()
            .await?;

        info!("Parsing response into LinodeResponse");
        let instances = response.json::<LinodeResponse>().await?;
        info!("Fetched {} instances", instances.data.len());

        Ok(instances.data)
    }

    pub async fn get_instance_configurations(&self, id: u64) -> Result<Vec<Configuration>, Error> {
        info!("Fetching configurations for instance ID: {}", id);
        let response = self
            .client
            .get(format!("{}/linode/instances/{}/configs", API_HOST, id))
            .bearer_auth(&self.token)
            .send()
            .await?;

        info!("Parsing response into InstanceConfigurationsResponse");
        let configs = response.json::<InstanceConfigurationsResponse>().await?;
        info!(
            "Fetched {} configurations for instance ID: {}",
            configs.data.len(),
            id
        );

        Ok(configs.data)
    }

    pub async fn get_instances_by_tag(
        &self,
        tags: Vec<&str>,
    ) -> Result<Vec<LinodeInstance>, Error> {
        info!("Filtering instances by tags: {:?}", tags);
        let instances = self.fetch_instances().await?;
        let filtered_instances = instances
            .into_iter()
            .filter(|instance| {
                tags.iter()
                    .all(|tag| instance.tags.contains(&tag.to_string()))
            })
            .collect::<Vec<_>>();

        info!(
            "Found {} instances with tags: {:?}",
            filtered_instances.len(),
            tags
        );
        Ok(filtered_instances)
    }

    pub async fn set_interfaces(
        &self,
        id: u64,
        config_id: u64,
        interfaces: Interfaces,
    ) -> Result<(), Error> {
        info!(
            "Setting interfaces for instance ID: {} with config ID: {}",
            id, config_id
        );
        self.client
            .put(format!(
                "{}/linode/instances/{}/configs/{}",
                API_HOST, id, config_id
            ))
            .bearer_auth(&self.token)
            .json(&interfaces)
            .send()
            .await?;

        info!(
            "Interfaces set successfully for instance ID: {} with config ID: {}",
            id, config_id
        );
        Ok(())
    }

    pub async fn destroy_instance(&self, id: u64) -> Result<(), Error> {
        info!("Destroying instance ID: {}", id);
        self.client
            .delete(format!("{}/linode/instances/{}", API_HOST, id,))
            .bearer_auth(&self.token)
            .send()
            .await?;

        info!("Instance ID: {} destroyed successfully", id);
        Ok(())
    }

    pub async fn reboot_instance(&self, id: u64) -> Result<(), Error> {
        info!("Rebooting instance ID: {}", id);
        self.client
            .post(format!("{}/linode/instances/{}/reboot", API_HOST, id,))
            .bearer_auth(&self.token)
            .send()
            .await?;

        info!("Instance ID: {} rebooted successfully", id);
        Ok(())
    }

    pub async fn create_linode_instance(
        &self,
        image: String,
        tags: Vec<String>,
        label: String,
        region: String,
        instance_type: String,
    ) -> Result<LinodeInstance, Error> {
        info!(
            "Creating Linode instance with label: {}, region: {}, instance type: {}",
            label, region, instance_type
        );
        let password = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect::<String>();

        info!("Generated password for instance: {}", password);
        let authorized_keys = vec![self.pub_key.clone()];
        let options = LinodeInstanceCreateOptions {
            authorized_keys,
            image,
            tags,
            label: label.clone(),
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
            .await?;

        info!("Parsing response into LinodeInstance");
        let instance = response.json::<LinodeInstance>().await?;
        info!("Created instance ID: {} with label: {}", instance.id, label);

        Ok(instance)
    }

    fn is_legacy_region(&self, region: &str) -> bool {
        info!("Checking if region: {} is a legacy region", region);
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
        info!(
            "Scaling down an instance in region: {} with tag: {}",
            region.code, tag
        );
        let instances = self.get_instances_by_tag(vec![tag, region.code]).await?;
        let records = self.fetch_records(domain).await?;

        let mut a_records = HashMap::new();
        for record in records {
            a_records.insert(record.target, record.id);
        }

        for instance in instances {
            if let Some(id) = a_records.get(&instance.ipv4[0]) {
                self.update_record_target(domain, *id, LOCALHOST).await?;
                self.destroy_instance(instance.id).await?;

                info!(
                    "Scaled down instance ID: {} with label: {} in region: {}",
                    instance.id, instance.label, region.code
                );
                break;
            }
        }

        Ok(())
    }

    // add an instance to the same VLAN as other linodes in a region
    // assigns instance to a sequential subdomain
    pub async fn scale_up_one(
        &self,
        image_id: &str,
        instance_type: &str,
        domain: u64,
        region: &RegionInfo,
        tag: &str,
    ) -> Result<(), Error> {
        info!(
            "Scaling up an instance in region: {} with tag: {}",
            region.code, tag
        );
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
                                    Err(e) => {
                                        error!("Parsing error in scale_up_one: {}", e);
                                    }
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
                label.clone(),
                region.region.to_string(),
                instance_type.to_string(),
            )
            .await?;

        let configs = self.get_instance_configurations(instance.id).await?;
        let config_id = configs[0].id;

        let cidr = if let Some(max) = cidrs.iter().max() {
            max + 1
        } else {
            1
        };

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

        info!("Rebooting the newly created instance ID: {}", instance.id);
        self.reboot_instance(instance.id).await?;

        let records = self.fetch_records(domain).await?;
        let prefix = format!("{}-{}", tag, region.code);
        let mut dns_done = false;
        let mut seqs = Vec::new();

        for rec in &records {
            if rec.name.starts_with(&prefix) && rec.record_type == A_RECORD {
                if rec.target == LOCALHOST {
                    // found a free slot, claim it
                    self.update_record_target(domain, rec.id, &instance.ipv4[0])
                        .await?;

                    dns_done = true;
                    break;
                } else {
                    if let Some(n) = extract_number(&rec.name) {
                        seqs.push(n);
                    }
                }
            }
        }

        if !dns_done {
            seqs.sort();
            seqs.reverse();
            let n = if !seqs.is_empty() { seqs[0] + 1 } else { 1 };
            self.create_a_record(
                domain,
                format!("{}-{}", prefix, n),
                instance.ipv4[0].clone(),
            )
            .await?;
        }

        info!(
            "Scaled up instance ID: {} with label: {} in region: {}",
            instance.id, label, region.code
        );
        Ok(())
    }
}

fn extract_number(input: &str) -> Option<i32> {
    let parts: Vec<&str> = input.split('-').collect();

    if let Some(last_part) = parts.last() {
        return last_part.parse().ok();
    }
    None
}
