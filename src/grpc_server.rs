//! MCP Server gRPC Server
//! Handles gRPC requests for MCP tool operations

use std::sync::Arc;
use tonic::{transport::Server, Request, Response, Status};
use std::collections::HashMap;

use crate::core::McpService;
use crate::core::Config;
use crate::proto::confuse::mcp::v1::{
    mcp_server::Mcp,
    ListToolsRequest, ListToolsResponse, CallToolRequest, CallToolResponse,
    ToolSchemaRequest, ToolSchema, Tool,
};

pub struct McpGrpcService {
    service: Arc<McpService>,
    config: Arc<Config>,
}

impl McpGrpcService {
    pub fn new(service: Arc<McpService>, config: Arc<Config>) -> Self {
        Self {
            service,
            config,
        }
    }
}

#[tonic::async_trait]
impl Mcp for McpGrpcService {
    async fn list_tools(
        &self,
        request: Request<ListToolsRequest>,
    ) -> Result<Response<ListToolsResponse>, Status> {
        let req = request.into_inner();
        
        tracing::info!("Listing tools for category: {:?}", req.category);
        
        match self.service.list_tools(req.category).await {
            Ok(tools) => {
                let proto_tools: Vec<Tool> = tools.into_iter().map(|t| Tool {
                    tool_id: t.tool_id,
                    name: t.name,
                    description: t.description,
                    category: t.category,
                    parameters_schema: t.parameters_schema,
                }).collect();

                Ok(Response::new(ListToolsResponse {
                    tools: proto_tools,
                }))
            }
            Err(e) => {
                tracing::error!("Failed to list tools: {}", e);
                Err(Status::internal(format!("Tool listing failed: {}", e)))
            }
        }
    }

    async fn call_tool(
        &self,
        request: Request<CallToolRequest>,
    ) -> Result<Response<CallToolResponse>, Status> {
        let req = request.into_inner();
        
        tracing::info!("Calling tool: {} for user: {}", req.tool_id, req.user_id);
        
        match self.service.call_tool(&req.tool_id, req.parameters, &req.user_id, &req.session_id).await {
            Ok(result) => {
                Ok(Response::new(CallToolResponse {
                    success: result.success,
                    result: result.result,
                    error: result.error,
                    metadata: result.metadata,
                }))
            }
            Err(e) => {
                tracing::error!("Failed to call tool {}: {}", req.tool_id, e);
                Err(Status::internal(format!("Tool call failed: {}", e)))
            }
        }
    }

    async fn get_tool_schema(
        &self,
        request: Request<ToolSchemaRequest>,
    ) -> Result<Response<ToolSchema>, Status> {
        let req = request.into_inner();
        
        tracing::info!("Getting schema for tool: {}", req.tool_id);
        
        match self.service.get_tool_schema(&req.tool_id).await {
            Ok(schema) => {
                Ok(Response::new(ToolSchema {
                    tool_id: schema.tool_id,
                    json_schema: schema.json_schema,
                }))
            }
            Err(e) => {
                tracing::error!("Failed to get tool schema: {}", e);
                Err(Status::internal(format!("Schema retrieval failed: {}", e)))
            }
        }
    }
}

pub async fn start_grpc_server(
    service: Arc<McpService>,
    config: Arc<Config>,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", config.grpc_host, config.grpc_port).parse()?;
    
    tracing::info!("Starting gRPC server on {}", addr);
    
    let grpc_service = McpGrpcService::new(service, config);
    
    Server::builder()
        .add_service(
            crate::proto::confuse::mcp::v1::mcp_server::McpServer::new(grpc_service)
        )
        .serve(addr)
        .await?;
    
    Ok(())
}
