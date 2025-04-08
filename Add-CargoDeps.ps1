# Add dependencies
cargo add eyre color-eyre
cargo add tracing
cargo add tracing-subscriber --features fmt,env-filter
cargo add chrono --features serde
cargo add tokio --features full
cargo add cloud_terrastodon_core_user_input --git https://github.com/AAFC-Cloud/Cloud-Terrastodon --rev 1332247ae3fa97e9ff29b4320666782c86d020e5
cargo add ollama-rs
cargo add serde --features derive
cargo add serde_json
cargo add regex
cargo add itertools

# Create init.rs if missing
if (Test-Path "src/init.rs") {
    Write-Host -ForegroundColor Yellow "init.rs already exists, skipping"
}
else {
    $initContent = @"
pub fn init() -> eyre::Result<()> {
    color_eyre::install()?;

    let env_filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(tracing::level_filters::LevelFilter::INFO.into())
        .from_env_lossy();
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_file(true)
        .with_line_number(true)
        .without_time()
        .init();

    Ok(())
}
"@
    Set-Content -Path "src/init.rs" -Value $initContent -Encoding utf8
}

# Check and update main.rs
$mainPath = "src/main.rs"
$mainContent = Get-Content $mainPath -Raw

$defaultContent = @"
fn main() {
    println!("Hello, world!");
}
"@

$desiredContent = @"
use tracing::info;

pub mod init;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    init::init()?;
    info!("Hello, world!");
    Ok(())
}
"@

# Normalize and compare
$normalizedMain = $mainContent -replace "`r", "" -replace "`n", "`n" -replace "\s+$", "" | Out-String
$normalizedDefault = $defaultContent -replace "`r", "" -replace "`n", "`n" -replace "\s+$", "" | Out-String

if ($normalizedMain.Trim() -eq $normalizedDefault.Trim()) {
    Set-Content -Path $mainPath -Value $desiredContent -Encoding utf8
    Write-Host -ForegroundColor Green "main.rs replaced with new content"
}
else {
    Write-Host -ForegroundColor Yellow "main.rs has been modified, skipping"
}
