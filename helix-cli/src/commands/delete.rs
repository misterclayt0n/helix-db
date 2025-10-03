use crate::commands::integrations::ecr::EcrManager;
use crate::commands::integrations::fly::FlyManager;
use crate::config::InstanceInfo;
use crate::docker::DockerManager;
use crate::project::ProjectContext;
use crate::utils::{
    print_confirm, print_error, print_lines, print_newline, print_status, print_success, print_warning,
};
use eyre::{eyre, Result};

pub async fn run(instance_name: String) -> Result<()> {
    let mut project = ProjectContext::find_and_load(None)?;

    let instance_config = project.config.get_instance(&instance_name)?;

    let total_instances = project.config.local.len() + project.config.cloud.len();
    if total_instances <= 1 {
        print_error(&format!(
            "Cannot delete instance '{}' as it is the last instance.\n\
            At least one instance must remain in the configuration.\n\
            If you want to remove this project entirely, delete the helix.toml file instead.",
            instance_name
        ));
        return Err(eyre!("Cannot delete the last instance"));
    }


    // Determine what will be deleted based on instance type
    let deletion_scope = match instance_config {
        InstanceInfo::Local(_) => {
            vec![
                "- Docker containers and images",
                "- Instance workspace directory",
                "- Persistent volumes (databases, files)",
                "- Configuration entry in helix.toml",
            ]
        }
        InstanceInfo::FlyIo(_) => {
            vec![
                "- Fly.io application and associated resources",
                "- Docker containers and images (if present locally)",
                "- Instance workspace directory",
                "- Persistent volumes (databases, files)",
                "- Configuration entry in helix.toml",
            ]
        }
        InstanceInfo::Ecr(_) => {
            vec![
                "- AWS ECR repository and all images",
                "- Docker containers and images (if present locally)",
                "- Instance workspace directory",
                "- Persistent volumes (databases, files)",
                "- Configuration entry in helix.toml",
            ]
        }
        InstanceInfo::Helix(_) => {
            vec![
                "- Helix Cloud instance (requires manual deletion)",
                "- Docker containers and images (if present locally)",
                "- Instance workspace directory",
                "- Persistent volumes (databases, files)",
                "- Configuration entry in helix.toml",
            ]
        }
    };

    print_warning(&format!(
        "This will permanently delete instance '{instance_name}' and ALL its data!"
    ));
    print_lines(&deletion_scope);
    print_lines(&["", "This action cannot be undone."]);
    print_newline();

    let confirmed = print_confirm(&format!(
        "Are you sure you want to delete instance '{instance_name}'?"
    ))?;

    if !confirmed {
        print_status("DELETE", "Deletion cancelled.");
        return Ok(());
    }

    print_newline();
    print_status("DELETE", &format!("Deleting instance '{instance_name}'..."));

    // Delete cloud resources first (if applicable)
    match instance_config {
        InstanceInfo::FlyIo(config) => {
            print_status("FLY", "Deleting Fly.io application...");
            let fly = FlyManager::new(&project, config.auth_type.clone()).await?;
            let app_name = format!("helix-{}-{}", project.config.project.name, instance_name);

            match fly.delete_app(&instance_name).await {
                Ok(_) => print_status("FLY", &format!("Deleted app '{app_name}' from Fly.io")),
                Err(e) => {
                    print_warning(&format!("Failed to delete Fly.io app: {}", e));
                    print_status("FLY", "Continuing with local cleanup...");
                }
            }

            // Clean up fly.toml if it exists
            let fly_toml = project.instance_workspace(&instance_name).join("fly.toml");
            if fly_toml.exists() {
                match std::fs::remove_file(&fly_toml) {
                    Ok(_) => print_status("CLEAN", "Removed fly.toml configuration"),
                    Err(e) => print_warning(&format!("Failed to remove fly.toml: {}", e)),
                }
            }
        }
        InstanceInfo::Ecr(config) => {
            print_status("ECR", "Deleting ECR repository...");
            let ecr = EcrManager::new(&project, config.auth_type.clone()).await?;
            let repo_name = format!("helix-{}-{}", project.config.project.name, instance_name);

            // First, try to remove locally tagged ECR images if Docker is available
            if DockerManager::check_docker_available().is_ok() {
                // Use registry_url from config if available, otherwise construct it
                if let Some(registry_url) = &config.registry_url {
                    // Remove ECR-tagged images for all build modes
                    for tag in ["debug", "latest", "dev"] {
                        let image_name = format!("{}/{repo_name}:{tag}", registry_url);
                        match std::process::Command::new("docker")
                            .args(&["rmi", "-f", &image_name])
                            .output()
                        {
                            Ok(output) if output.status.success() => {
                                print_status("DOCKER", &format!("Removed local ECR image: {tag}"))
                            }
                            _ => {} // Image might not exist locally, that's fine
                        }
                    }
                }
            }

            match ecr.delete_repository(&instance_name).await {
                Ok(_) => print_status("ECR", &format!("Deleted repository '{repo_name}' from AWS ECR")),
                Err(e) => {
                    print_warning(&format!("Failed to delete ECR repository: {}", e));
                    print_status("ECR", "Continuing with local cleanup...");
                }
            }

            // Clean up ecr.toml if it exists
            let ecr_toml = project.instance_workspace(&instance_name).join("ecr.toml");
            if ecr_toml.exists() {
                match std::fs::remove_file(&ecr_toml) {
                    Ok(_) => print_status("CLEAN", "Removed ecr.toml configuration"),
                    Err(e) => print_warning(&format!("Failed to remove ecr.toml: {}", e)),
                }
            }
        }
        InstanceInfo::Helix(config) => {
            print_error(&format!(
                "Helix Cloud instance deletion is not yet implemented.\n\
                Please delete cluster '{}' manually through the Helix dashboard or contact support.\n\
                Local resources will still be cleaned up.",
                config.cluster_id
            ));
        }
        InstanceInfo::Local(_) => {
            // Local instances don't have cloud resources to delete
            print_status("LOCAL", "No cloud resources to delete");
        }
    }

    // Stop and remove Docker containers and volumes
    if DockerManager::check_docker_available().is_ok() {
        let docker = DockerManager::new(&project);

        print_status("DOCKER", "Removing containers and volumes...");
        // Remove containers and Docker volumes
        match docker.prune_instance(&instance_name, true) {
            Ok(_) => print_status("DOCKER", "Removed containers and Docker volumes"),
            Err(e) => print_warning(&format!("Failed to remove containers: {}", e)),
        }

        // Remove Docker images
        print_status("DOCKER", "Removing Docker images...");
        match docker.remove_instance_images(&instance_name) {
            Ok(_) => print_status("DOCKER", "Removed Docker images"),
            Err(e) => print_warning(&format!("Failed to remove images: {}", e)),
        }
    } else {
        print_status("DOCKER", "Docker not available, skipping container cleanup");
    }

    // Remove instance workspace
    let workspace = project.instance_workspace(&instance_name);
    if workspace.exists() {
        print_status("CLEAN", "Removing workspace directory...");
        match std::fs::remove_dir_all(&workspace) {
            Ok(_) => print_status("CLEAN", "Removed workspace directory"),
            Err(e) => {
                print_warning(&format!("Failed to remove workspace directory: {}", e));
                print_status("CLEAN", &format!("You may need to manually remove: {}", workspace.display()));
            }
        }
    }

    // Remove instance volumes (permanent data loss)
    let volume = project.instance_volume(&instance_name);
    if volume.exists() {
        print_status("CLEAN", "Removing persistent volumes...");
        match std::fs::remove_dir_all(&volume) {
            Ok(_) => print_status("CLEAN", "Removed persistent volumes"),
            Err(e) => {
                print_warning(&format!("Failed to remove volume directory: {}", e));
                print_status("CLEAN", &format!("You may need to manually remove: {}", volume.display()));
            }
        }
    }

    // Remove instance from config and save
    print_status("CONFIG", "Removing instance from helix.toml...");

    // Use the remove_instance method which validates we're not removing the last instance
    match project.config.remove_instance(&instance_name) {
        Ok(_) => {},
        Err(e) if e.to_string().contains("last instance") => {
            // This should have been caught earlier, but if we get here,
            // we've already deleted resources. This is a critical error.
            print_error(&format!(
                "CRITICAL: {}\n\
                 WARNING: Cloud resources and local files have already been deleted!",
                e
            ));
            return Err(e);
        }
        Err(e) => {
            // Instance not found or other error - shouldn't happen as we validated earlier
            print_warning(&format!("Failed to remove instance from config: {}", e));
        }
    }

    // Save the updated config
    let config_path = project.root.join("helix.toml");
    match project.config.save_to_file(&config_path) {
        Ok(_) => print_status("CONFIG", "Updated helix.toml"),
        Err(e) => {
            print_error(&format!(
                "Failed to save updated config to helix.toml: {}\n\
                 The instance has been deleted but the config file was not updated.\n\
                 You may need to manually remove the instance from helix.toml",
                e
            ));
            // Don't return error here as deletion was successful
        }
    }

    print_newline();
    print_success(&format!("Instance '{instance_name}' deleted successfully"));

    Ok(())
}
