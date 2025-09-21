use crate::{
    CloudAction,
    cloud_client::CloudClient,
    commands::auth::Credentials,
    config::CloudConfig,
    project::ProjectContext,
    utils::{print_info, print_status, print_success, print_warning},
};
use eyre::{Result, eyre};

pub async fn run(action: CloudAction) -> Result<()> {
    match action {
        CloudAction::Status { instance } => status(instance).await,
        CloudAction::Claim { instance } => claim(instance).await,
    }
}

async fn status(instance_name: String) -> Result<()> {
    // Load project context
    let project = ProjectContext::find_and_load(None)?;
    
    // Get instance config
    let _instance_config = project.config.get_instance(&instance_name)?;
    
    // Check if it's an anonymous cloud instance
    let config = project.config.cloud.get(&instance_name)
        .ok_or_else(|| eyre!("Instance '{instance_name}' is not a cloud instance"))?;
    
    match config {
        CloudConfig::AnonymousCloud(config) => {
            print_status("STATUS", &format!("Getting status for instance '{instance_name}'"));
            
            let cloud_client = CloudClient::new()?;
            let status = cloud_client.get_instance_status(&config.instance_id, &config.api_key).await?;
            
            print_info(&format!("Instance ID: {}", status.instance_id));
            print_info(&format!("Status: {}", status.status));
            if let Some(app_name) = &status.app_name {
                print_info(&format!("App Name: {app_name}"));
            }
            if let Some(app_url) = &status.app_url {
                print_info(&format!("App URL: {app_url}"));
            }
        }
        _ => {
            print_warning(&format!("Instance '{instance_name}' is not an anonymous cloud instance"));
        }
    }
    
    Ok(())
}

async fn claim(instance_name: String) -> Result<()> {
    // Load project context
    let project = ProjectContext::find_and_load(None)?;
    
    // Get instance config
    let _instance_config = project.config.get_instance(&instance_name)?;
    
    // Check if it's an anonymous cloud instance
    let config = project.config.cloud.get(&instance_name)
        .ok_or_else(|| eyre!("Instance '{instance_name}' is not a cloud instance"))?;
    
    match config {
        CloudConfig::AnonymousCloud(config) => {
            print_status("CLAIM", &format!("Claiming instance '{instance_name}'"));
            
            // Check if user is authenticated
            let home = dirs::home_dir().ok_or_else(|| eyre!("Cannot find home directory"))?;
            let cred_path = home.join(".helix").join("credentials");
            
            let credentials = Credentials::try_read_from_file(&cred_path)
                .ok_or_else(|| eyre!("Not authenticated. Please run 'helix auth login' first"))?;
            
            if !credentials.is_authenticated() {
                return Err(eyre!("Invalid credentials. Please run 'helix auth login' again"));
            }
            
            // Claim the instance
            let cloud_client = CloudClient::new()?;
            cloud_client.claim_instance(
                &config.instance_id,
                &config.deployment_key,
                &credentials.helix_admin_key,
            ).await?;
            
            print_success(&format!("Successfully claimed instance '{instance_name}'"));
            print_info("This instance is now associated with your account");
        }
        _ => {
            print_warning(&format!("Instance '{instance_name}' is not an anonymous cloud instance"));
            print_info("Only anonymous cloud instances can be claimed");
        }
    }
    
    Ok(())
}