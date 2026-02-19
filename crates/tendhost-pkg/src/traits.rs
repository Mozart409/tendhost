//! Package manager traits

use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct UpgradablePackage {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct UpdateResult {
    pub success: bool,
    pub upgraded_count: u32,
}

#[async_trait]
pub trait PackageManager: Send + Sync {
    async fn list_upgradable(&self) -> Result<Vec<UpgradablePackage>, String>;
    async fn upgrade_all(&self) -> Result<UpdateResult, String>;
    async fn upgrade_dry_run(&self) -> Result<UpdateResult, String>;
    async fn reboot_required(&self) -> Result<bool, String>;
}
