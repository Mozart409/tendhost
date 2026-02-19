use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use kameo::actor::Spawn;
use tokio::sync::broadcast;

use tendhost_core::*;
use tendhost_exec::traits::RemoteExecutor;
use tendhost_pkg::traits::{PackageManager, UpdateResult as PkgUpdateResult, UpgradablePackage};

// Mock implementations
struct MockExecutor;

#[async_trait]
impl RemoteExecutor for MockExecutor {
    async fn run(&self, _cmd: &str) -> Result<String, String> {
        Ok("ok".to_string())
    }

    async fn run_with_timeout(&self, cmd: &str, _timeout: Duration) -> Result<String, String> {
        self.run(cmd).await
    }
}

struct MockPackageManager {
    packages: Vec<String>,
    reboot_required: bool,
}

#[async_trait]
impl PackageManager for MockPackageManager {
    async fn list_upgradable(&self) -> Result<Vec<UpgradablePackage>, String> {
        Ok(self
            .packages
            .iter()
            .map(|name| UpgradablePackage {
                name: name.clone(),
                version: "1.0.0".to_string(),
            })
            .collect())
    }

    async fn upgrade_all(&self) -> Result<PkgUpdateResult, String> {
        #[allow(clippy::cast_possible_truncation)]
        let count = self.packages.len() as u32;
        Ok(PkgUpdateResult {
            success: true,
            upgraded_count: count,
        })
    }

    async fn upgrade_dry_run(&self) -> Result<PkgUpdateResult, String> {
        self.upgrade_all().await
    }

    async fn reboot_required(&self) -> Result<bool, String> {
        Ok(self.reboot_required)
    }
}

struct TestHostFactory;

#[async_trait]
impl HostActorFactory for TestHostFactory {
    async fn create_executor(&self, _config: &HostConfig) -> Arc<dyn RemoteExecutor> {
        Arc::new(MockExecutor)
    }

    async fn create_package_manager(
        &self,
        _config: &HostConfig,
        _executor: Arc<dyn RemoteExecutor>,
    ) -> Arc<dyn PackageManager> {
        Arc::new(MockPackageManager {
            packages: vec!["vim".to_string(), "curl".to_string()],
            reboot_required: false,
        })
    }
}

#[tokio::test]
async fn test_host_actor_query_inventory() {
    let (tx, _rx) = broadcast::channel(100);

    let config = HostConfig {
        name: "test-host".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        ssh_key: None,
        compose_paths: vec![],
        tags: vec![],
        policy: HostPolicy::default(),
    };

    let args = HostActorArgs {
        config,
        executor: Arc::new(MockExecutor),
        package_manager: Arc::new(MockPackageManager {
            packages: vec!["vim".to_string(), "curl".to_string()],
            reboot_required: false,
        }),
        event_tx: tx,
    };

    let actor_ref = HostActor::spawn(args);

    let inventory = actor_ref.ask(QueryInventory).await.unwrap();

    assert_eq!(inventory.pending_updates, 2);
    assert_eq!(inventory.packages, vec!["vim", "curl"]);

    actor_ref.stop_gracefully().await.unwrap();
}

#[tokio::test]
async fn test_orchestrator_register_host() {
    let args = OrchestratorActorArgs {
        event_channel_capacity: 100,
        host_factory: Arc::new(TestHostFactory),
    };

    let orchestrator = OrchestratorActor::spawn(args);

    let config = HostConfig {
        name: "test-host".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        ssh_key: None,
        compose_paths: vec![],
        tags: vec!["test".to_string()],
        policy: HostPolicy::default(),
    };

    orchestrator.ask(RegisterHost { config }).await.unwrap();

    let hosts = orchestrator.ask(ListHosts).await.unwrap();
    assert_eq!(hosts.len(), 1);
    assert_eq!(hosts[0].name, "test-host");

    orchestrator.stop_gracefully().await.unwrap();
}
