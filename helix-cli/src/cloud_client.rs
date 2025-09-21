use eyre::{Result, eyre};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::time::Duration;

const DEFAULT_CLOUD_API_URL: &str = "https://api.helix-db.com";

#[derive(Debug, Serialize, Deserialize)]
pub struct CloudInitRequest {
    pub instance_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CloudInitResponse {
    pub instance_id: String,
    pub deployment_key: String,
    pub api_key: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CloudDeployRequest {
    pub instance_id: String,
    pub deployment_key: String,
    pub docker_image: String,
    pub region: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CloudDeployResponse {
    pub app_name: String,
    pub app_url: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CloudClaimRequest {
    pub instance_id: String,
    pub deployment_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CloudStatusResponse {
    pub instance_id: String,
    pub status: String,
    pub app_name: Option<String>,
    pub app_url: Option<String>,
}

pub struct CloudClient {
    client: Client,
    base_url: String,
}

impl CloudClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        
        let base_url = std::env::var("HELIX_CLOUD_API_URL")
            .unwrap_or_else(|_| DEFAULT_CLOUD_API_URL.to_string());
        
        Ok(Self { client, base_url })
    }

    pub async fn init_cloud_instance(&self) -> Result<CloudInitResponse> {
        let url = format!("{}/cloud/init", self.base_url);
        
        let request = CloudInitRequest {
            instance_type: "free".to_string(),
        };
        
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(eyre!("Failed to initialize cloud instance: {} - {}", status, error_text));
        }
        
        let init_response: CloudInitResponse = response.json().await?;
        Ok(init_response)
    }

    pub async fn deploy_to_cloud(
        &self,
        instance_id: &str,
        deployment_key: &str,
        docker_image: &str,
        region: &str,
    ) -> Result<CloudDeployResponse> {
        let url = format!("{}/cloud/deploy", self.base_url);
        
        let request = CloudDeployRequest {
            instance_id: instance_id.to_string(),
            deployment_key: deployment_key.to_string(),
            docker_image: docker_image.to_string(),
            region: region.to_string(),
        };
        
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(eyre!("Failed to deploy to cloud: {} - {}", status, error_text));
        }
        
        let deploy_response: CloudDeployResponse = response.json().await?;
        Ok(deploy_response)
    }

    pub async fn claim_instance(
        &self,
        instance_id: &str,
        deployment_key: &str,
        auth_token: &str,
    ) -> Result<()> {
        let url = format!("{}/cloud/claim", self.base_url);
        
        let request = CloudClaimRequest {
            instance_id: instance_id.to_string(),
            deployment_key: deployment_key.to_string(),
        };
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {auth_token}"))
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(eyre!("Failed to claim instance: {} - {}", status, error_text));
        }
        
        Ok(())
    }

    pub async fn get_instance_status(
        &self,
        instance_id: &str,
        api_key: &str,
    ) -> Result<CloudStatusResponse> {
        let url = format!("{}/cloud/status/{}", self.base_url, instance_id);
        
        let response = self.client
            .get(&url)
            .header("x-api-key", api_key)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(eyre!("Failed to get instance status: {} - {}", status, error_text));
        }
        
        let status_response: CloudStatusResponse = response.json().await?;
        Ok(status_response)
    }
}