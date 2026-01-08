// MCP Service Main Entry Point
// This service implements the Model Context Protocol (MCP) for AI agents
// It provides tools and resources that agents can use to query ConHub data
use anyhow::Result;
use mcp_service::{McpConfig, connectors::ConnectorManager, protocol::McpServer, db};
use actix_web::{web, App, HttpResponse, HttpServer};
use confuse_observability::{init_tracing, TracingConfig, info};

async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "mcp-service"
    }))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize observability with structured logging
    init_tracing(TracingConfig::for_service("mcp-service"));

    info!("Starting ConHub MCP Service");

    // Load configuration
    dotenv::dotenv().ok();
    let config = McpConfig::from_env()?;

    // Initialize database stub
    let db_config = db::DatabaseConfig::from_env();
    info!("Initializing database");
    let database = db::Database::new(&db_config).await?;
    info!("Database initialized");

    // Initialize connector manager
    let connector_manager = ConnectorManager::new(database, &config).await?;

    info!(
        connectors = connector_manager.connector_count(),
        "Initialized connectors"
    );

    // Start minimal HTTP server for health checks only
    let port = std::env::var("MCP_PORT").unwrap_or_else(|_| "3004".to_string());
    let port_num: u16 = port.parse().unwrap_or(3004);
    
    let http_handle = tokio::spawn(async move {
        tracing::info!("🚀 [MCP Service] Starting health check server on port {}", port_num);
        HttpServer::new(move || {
            App::new()
                .route("/health", web::get().to(health))
        })
        .bind(("0.0.0.0", port_num))
        .expect("Failed to bind MCP HTTP server")
        .run()
        .await
        .expect("MCP HTTP server failed");
    });

    // Start MCP server on stdio (main protocol)
    let server = McpServer::new(connector_manager, config);
    let mcp_handle = tokio::spawn(async move {
        match server.run().await {
            Ok(_) => {
                tracing::warn!("MCP server finished");
            }
            Err(e) => {
                tracing::error!("MCP server error: {}", e);
            }
        }
    });

    tracing::info!("✅ MCP service running");
    tracing::info!("   MCP Protocol: stdio");
    tracing::info!("   Health Check: http://0.0.0.0:{}", port_num);
    tracing::info!("   Tools: vector_search, graph_query, data_fetch");
    
    // Keep service running as long as HTTP health server is alive
    // Continue even if MCP stdio server exits (e.g., no attached client)
    if let Err(e) = http_handle.await {
        tracing::error!("HTTP server task error: {}", e);
    }
    
    Ok(())
}
