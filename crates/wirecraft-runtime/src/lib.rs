//! Application services and adapters shared by desktop, API, and MCP.
pub mod backend;
pub mod jobs;
pub mod lsp;
pub mod mcp;
pub mod service;

pub mod archetypes;
pub mod cli;
pub mod export;

pub mod generation;

pub mod config;

mod process;

mod generated_analysis;
