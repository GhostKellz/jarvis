use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::error::{JarvisError, JarvisResult};
use crate::types::Environment;
// use jarvis_shell::Environment; // Removed to avoid circular dependency

/// Agent skill system with plugin architecture
pub struct SkillManager {
    pub skills: HashMap<String, Box<dyn Skill>>,
    pub skill_registry: SkillRegistry,
    pub plugin_paths: Vec<PathBuf>,
    pub execution_history: Vec<SkillExecution>,
}

/// Registry of available skills
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillRegistry {
    pub skills: HashMap<String, SkillMetadata>,
    pub categories: HashMap<String, Vec<String>>,
    pub dependencies: HashMap<String, Vec<String>>,
}

/// Skill metadata for registration and discovery
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: SkillCategory,
    pub version: String,
    pub author: String,
    pub enabled: bool,
    pub required_permissions: Vec<Permission>,
    pub dependencies: Vec<String>,
    pub parameters: Vec<SkillParameter>,
    pub examples: Vec<SkillExample>,
    pub tags: Vec<String>,
}

/// Skill categories for organization
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SkillCategory {
    System,        // System administration and monitoring
    Development,   // Code generation and development tools
    Automation,    // Task automation and scripting
    Analysis,      // Data analysis and reporting
    Communication, // Email, messaging, notifications
    Security,      // Security and authentication
    Utility,       // General utility functions
    Custom,        // User-defined skills
}

/// Permission system for skills
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Permission {
    ReadFiles,
    WriteFiles,
    ExecuteCommands,
    NetworkAccess,
    SystemInfo,
    ProcessControl,
    ServiceControl,
    DatabaseAccess,
    EnvironmentVariables,
}

/// Skill parameter definition
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillParameter {
    pub name: String,
    pub parameter_type: ParameterType,
    pub description: String,
    pub required: bool,
    pub default_value: Option<String>,
    pub validation: Option<ParameterValidation>,
}

/// Parameter types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ParameterType {
    String,
    Integer,
    Float,
    Boolean,
    Path,
    Url,
    Email,
    Json,
    Array(Box<ParameterType>),
}

/// Parameter validation rules
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParameterValidation {
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<String>,
    pub allowed_values: Option<Vec<String>>,
}

/// Skill usage example
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillExample {
    pub description: String,
    pub input: HashMap<String, String>,
    pub expected_output: String,
}

/// Skill execution context
#[derive(Clone, Debug)]
pub struct SkillContext {
    pub execution_id: Uuid,
    pub user_id: Option<String>,
    pub environment: Environment,
    pub working_directory: PathBuf,
    pub parameters: HashMap<String, String>,
    pub permissions: Vec<Permission>,
    pub timeout: Option<std::time::Duration>,
}

/// Skill execution result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub metadata: HashMap<String, String>,
    pub execution_time: std::time::Duration,
    pub resources_used: ResourceUsage,
}

/// Resource usage tracking
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_time: std::time::Duration,
    pub memory_peak: usize,
    pub disk_read: usize,
    pub disk_write: usize,
    pub network_read: usize,
    pub network_write: usize,
}

/// Skill execution history
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillExecution {
    pub id: Uuid,
    pub skill_id: String,
    pub timestamp: DateTime<Utc>,
    pub context: HashMap<String, String>, // Simplified context for serialization
    pub result: SkillResult,
    pub user_feedback: Option<UserFeedback>,
}

/// User feedback on skill execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserFeedback {
    pub rating: u8, // 1-5 stars
    pub helpful: bool,
    pub comments: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Core skill trait that all skills must implement
#[async_trait]
pub trait Skill: Send + Sync {
    /// Get skill metadata
    fn metadata(&self) -> &SkillMetadata;
    
    /// Execute the skill with given context
    async fn execute(&self, context: &SkillContext) -> JarvisResult<SkillResult>;
    
    /// Validate parameters before execution
    fn validate_parameters(&self, parameters: &HashMap<String, String>) -> JarvisResult<()>;
    
    /// Check if required permissions are available
    fn check_permissions(&self, available: &[Permission]) -> JarvisResult<()>;
    
    /// Get help text for the skill
    fn help(&self) -> String;
    
    /// Initialize the skill (called when loaded)
    async fn initialize(&mut self) -> JarvisResult<()> {
        Ok(())
    }
    
    /// Cleanup the skill (called when unloaded)
    async fn cleanup(&mut self) -> JarvisResult<()> {
        Ok(())
    }
}

impl SkillManager {
    pub fn new(plugin_paths: Vec<PathBuf>) -> Self {
        Self {
            skills: HashMap::new(),
            skill_registry: SkillRegistry::new(),
            plugin_paths,
            execution_history: Vec::new(),
        }
    }

    /// Register a skill
    pub async fn register_skill(&mut self, mut skill: Box<dyn Skill>) -> JarvisResult<()> {
        skill.initialize().await?;
        let metadata = skill.metadata().clone();
        let skill_id = metadata.id.clone();
        
        self.skill_registry.register_skill(metadata)?;
        self.skills.insert(skill_id, skill);
        
        Ok(())
    }

    /// Unregister a skill
    pub async fn unregister_skill(&mut self, skill_id: &str) -> JarvisResult<()> {
        if let Some(mut skill) = self.skills.remove(skill_id) {
            skill.cleanup().await?;
        }
        self.skill_registry.unregister_skill(skill_id);
        Ok(())
    }

    /// Execute a skill by ID
    pub async fn execute_skill(
        &mut self,
        skill_id: &str,
        context: SkillContext,
    ) -> JarvisResult<SkillResult> {
        let skill = self.skills.get(skill_id)
            .ok_or_else(|| JarvisError::Plugin(format!("Skill not found: {}", skill_id)))?;

        // Validate parameters
        skill.validate_parameters(&context.parameters)?;
        
        // Check permissions
        skill.check_permissions(&context.permissions)?;

        // Execute skill
        let start_time = std::time::Instant::now();
        let execution_id = context.execution_id;
        
        let result = skill.execute(&context).await;
        
        // Record execution
        let execution = SkillExecution {
            id: execution_id,
            skill_id: skill_id.to_string(),
            timestamp: Utc::now(),
            context: self.serialize_context(&context),
            result: result.clone().unwrap_or_else(|e| SkillResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
                metadata: HashMap::new(),
                execution_time: start_time.elapsed(),
                resources_used: ResourceUsage::default(),
            }),
            user_feedback: None,
        };
        
        self.execution_history.push(execution);
        
        result
    }

    /// Find skills by category
    pub fn find_skills_by_category(&self, category: &SkillCategory) -> Vec<&SkillMetadata> {
        self.skill_registry.skills
            .values()
            .filter(|skill| skill.category == *category)
            .collect()
    }

    /// Search skills by query
    pub fn search_skills(&self, query: &str) -> Vec<&SkillMetadata> {
        let query_lower = query.to_lowercase();
        self.skill_registry.skills
            .values()
            .filter(|skill| {
                skill.name.to_lowercase().contains(&query_lower) ||
                skill.description.to_lowercase().contains(&query_lower) ||
                skill.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// Get skill recommendations based on context
    pub fn recommend_skills(&self, environment: &Environment, task_description: &str) -> Vec<&SkillMetadata> {
        let mut recommendations = Vec::new();
        let task_lower = task_description.to_lowercase();

        // Simple keyword-based recommendations
        let keywords = [
            ("file", SkillCategory::System),
            ("git", SkillCategory::Development),
            ("code", SkillCategory::Development),
            ("monitor", SkillCategory::System),
            ("analyze", SkillCategory::Analysis),
            ("security", SkillCategory::Security),
            ("automate", SkillCategory::Automation),
        ];

        for (keyword, category) in &keywords {
            if task_lower.contains(keyword) {
                let category_skills = self.find_skills_by_category(category);
                recommendations.extend(category_skills);
            }
        }

        // Remove duplicates and limit to top 5
        recommendations.sort_by(|a, b| a.name.cmp(&b.name));
        recommendations.dedup_by(|a, b| a.id == b.id);
        recommendations.truncate(5);

        recommendations
    }

    /// Load skills from plugin directories
    pub async fn load_plugins(&mut self) -> JarvisResult<()> {
        // Register built-in skills first
        self.register_builtin_skills().await?;
        
        // TODO: Implement dynamic plugin loading from filesystem
        // This would involve:
        // 1. Scanning plugin directories for .so/.dll files
        // 2. Loading shared libraries
        // 3. Calling plugin initialization functions
        // 4. Registering skills from plugins
        
        Ok(())
    }

    /// Register built-in skills
    async fn register_builtin_skills(&mut self) -> JarvisResult<()> {
        // System information skill
        let system_info_skill = Box::new(SystemInfoSkill::new());
        self.register_skill(system_info_skill).await?;

        // File operations skill
        let file_ops_skill = Box::new(FileOpsSkill::new());
        self.register_skill(file_ops_skill).await?;

        // Git operations skill
        let git_skill = Box::new(GitSkill::new());
        self.register_skill(git_skill).await?;

        Ok(())
    }

    fn serialize_context(&self, context: &SkillContext) -> HashMap<String, String> {
        let mut serialized = HashMap::new();
        serialized.insert("execution_id".to_string(), context.execution_id.to_string());
        serialized.insert("working_directory".to_string(), context.working_directory.to_string_lossy().to_string());
        
        for (key, value) in &context.parameters {
            serialized.insert(format!("param_{}", key), value.clone());
        }
        
        serialized
    }

    /// Get execution statistics
    pub fn get_execution_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        
        for execution in &self.execution_history {
            let counter = stats.entry(execution.skill_id.clone()).or_insert(0);
            *counter += 1;
        }
        
        stats
    }

    /// Get skill performance metrics
    pub fn get_skill_performance(&self, skill_id: &str) -> Option<SkillPerformanceMetrics> {
        let executions: Vec<_> = self.execution_history
            .iter()
            .filter(|e| e.skill_id == skill_id)
            .collect();

        if executions.is_empty() {
            return None;
        }

        let total_executions = executions.len();
        let successful_executions = executions.iter().filter(|e| e.result.success).count();
        let success_rate = (successful_executions as f32 / total_executions as f32) * 100.0;
        
        let avg_execution_time = executions
            .iter()
            .map(|e| e.result.execution_time)
            .sum::<std::time::Duration>() / total_executions as u32;

        Some(SkillPerformanceMetrics {
            skill_id: skill_id.to_string(),
            total_executions,
            successful_executions,
            success_rate,
            avg_execution_time,
            last_executed: executions.last().map(|e| e.timestamp),
        })
    }
}

/// Skill performance metrics
#[derive(Clone, Debug)]
pub struct SkillPerformanceMetrics {
    pub skill_id: String,
    pub total_executions: usize,
    pub successful_executions: usize,
    pub success_rate: f32,
    pub avg_execution_time: std::time::Duration,
    pub last_executed: Option<DateTime<Utc>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            categories: HashMap::new(),
            dependencies: HashMap::new(),
        }
    }

    pub fn register_skill(&mut self, metadata: SkillMetadata) -> JarvisResult<()> {
        let skill_id = metadata.id.clone();
        let category = format!("{:?}", metadata.category);
        
        // Add to category index
        self.categories.entry(category).or_insert_with(Vec::new).push(skill_id.clone());
        
        // Store dependencies
        if !metadata.dependencies.is_empty() {
            self.dependencies.insert(skill_id.clone(), metadata.dependencies.clone());
        }
        
        // Store metadata
        self.skills.insert(skill_id, metadata);
        
        Ok(())
    }

    pub fn unregister_skill(&mut self, skill_id: &str) {
        if let Some(metadata) = self.skills.remove(skill_id) {
            let category = format!("{:?}", metadata.category);
            if let Some(category_skills) = self.categories.get_mut(&category) {
                category_skills.retain(|id| id != skill_id);
            }
        }
        self.dependencies.remove(skill_id);
    }
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            cpu_time: std::time::Duration::new(0, 0),
            memory_peak: 0,
            disk_read: 0,
            disk_write: 0,
            network_read: 0,
            network_write: 0,
        }
    }
}

// Built-in skills implementation

/// System information skill
pub struct SystemInfoSkill {
    metadata: SkillMetadata,
}

impl SystemInfoSkill {
    pub fn new() -> Self {
        Self {
            metadata: SkillMetadata {
                id: "system_info".to_string(),
                name: "System Information".to_string(),
                description: "Get comprehensive system information".to_string(),
                category: SkillCategory::System,
                version: "1.0.0".to_string(),
                author: "Jarvis Core".to_string(),
                enabled: true,
                required_permissions: vec![Permission::SystemInfo],
                dependencies: vec![],
                parameters: vec![
                    SkillParameter {
                        name: "component".to_string(),
                        parameter_type: ParameterType::String,
                        description: "System component to query (cpu, memory, disk, all)".to_string(),
                        required: false,
                        default_value: Some("all".to_string()),
                        validation: Some(ParameterValidation {
                            allowed_values: Some(vec!["cpu".to_string(), "memory".to_string(), "disk".to_string(), "all".to_string()]),
                            ..Default::default()
                        }),
                    }
                ],
                examples: vec![
                    SkillExample {
                        description: "Get all system information".to_string(),
                        input: [("component".to_string(), "all".to_string())].iter().cloned().collect(),
                        expected_output: "System: Linux, CPU: 4 cores, Memory: 16GB, Disk: 512GB".to_string(),
                    }
                ],
                tags: vec!["system".to_string(), "info".to_string(), "monitoring".to_string()],
            },
        }
    }
}

#[async_trait]
impl Skill for SystemInfoSkill {
    fn metadata(&self) -> &SkillMetadata {
        &self.metadata
    }

    async fn execute(&self, context: &SkillContext) -> JarvisResult<SkillResult> {
        let start_time = std::time::Instant::now();
        let component = context.parameters.get("component").unwrap_or(&"all".to_string());
        
        let mut output = Vec::new();
        
        match component.as_str() {
            "cpu" => {
                output.push(format!("CPU: {} cores", context.environment.system_stats.cpu_count));
                output.push(format!("Load: {:.2}, {:.2}, {:.2}", 
                    context.environment.system_stats.load_avg_1min,
                    context.environment.system_stats.load_avg_5min,
                    context.environment.system_stats.load_avg_15min));
            },
            "memory" => {
                output.push("Memory information not available in current environment".to_string());
            },
            "disk" => {
                output.push("Disk information not available in current environment".to_string());
            },
            "all" | _ => {
                output.push(format!("System: {} {}", context.environment.os_info.os_type, context.environment.os_info.version));
                output.push(format!("Hostname: {}", context.environment.os_info.hostname));
                output.push(format!("Architecture: {}", context.environment.os_info.arch));
                output.push(format!("CPU: {} cores", context.environment.system_stats.cpu_count));
                output.push(format!("Load: {:.2}, {:.2}, {:.2}", 
                    context.environment.system_stats.load_avg_1min,
                    context.environment.system_stats.load_avg_5min,
                    context.environment.system_stats.load_avg_15min));
            }
        }

        Ok(SkillResult {
            success: true,
            output: output.join("\\n"),
            error: None,
            metadata: HashMap::new(),
            execution_time: start_time.elapsed(),
            resources_used: ResourceUsage::default(),
        })
    }

    fn validate_parameters(&self, parameters: &HashMap<String, String>) -> JarvisResult<()> {
        if let Some(component) = parameters.get("component") {
            let valid_components = ["cpu", "memory", "disk", "all"];
            if !valid_components.contains(&component.as_str()) {
                return Err(JarvisError::Plugin(format!("Invalid component: {}", component)));
            }
        }
        Ok(())
    }

    fn check_permissions(&self, available: &[Permission]) -> JarvisResult<()> {
        if !available.contains(&Permission::SystemInfo) {
            return Err(JarvisError::Plugin("SystemInfo permission required".to_string()));
        }
        Ok(())
    }

    fn help(&self) -> String {
        "Get system information including CPU, memory, disk, and OS details".to_string()
    }
}

/// File operations skill
pub struct FileOpsSkill {
    metadata: SkillMetadata,
}

impl FileOpsSkill {
    pub fn new() -> Self {
        Self {
            metadata: SkillMetadata {
                id: "file_ops".to_string(),
                name: "File Operations".to_string(),
                description: "Perform file and directory operations".to_string(),
                category: SkillCategory::System,
                version: "1.0.0".to_string(),
                author: "Jarvis Core".to_string(),
                enabled: true,
                required_permissions: vec![Permission::ReadFiles, Permission::WriteFiles],
                dependencies: vec![],
                parameters: vec![
                    SkillParameter {
                        name: "operation".to_string(),
                        parameter_type: ParameterType::String,
                        description: "File operation to perform (list, read, write, delete)".to_string(),
                        required: true,
                        default_value: None,
                        validation: Some(ParameterValidation {
                            allowed_values: Some(vec!["list".to_string(), "read".to_string(), "write".to_string(), "delete".to_string()]),
                            ..Default::default()
                        }),
                    },
                    SkillParameter {
                        name: "path".to_string(),
                        parameter_type: ParameterType::Path,
                        description: "File or directory path".to_string(),
                        required: true,
                        default_value: None,
                        validation: None,
                    }
                ],
                examples: vec![],
                tags: vec!["file".to_string(), "directory".to_string(), "filesystem".to_string()],
            },
        }
    }
}

#[async_trait]
impl Skill for FileOpsSkill {
    fn metadata(&self) -> &SkillMetadata {
        &self.metadata
    }

    async fn execute(&self, context: &SkillContext) -> JarvisResult<SkillResult> {
        let start_time = std::time::Instant::now();
        let operation = context.parameters.get("operation")
            .ok_or_else(|| JarvisError::Plugin("operation parameter required".to_string()))?;
        let path = context.parameters.get("path")
            .ok_or_else(|| JarvisError::Plugin("path parameter required".to_string()))?;

        let output = match operation.as_str() {
            "list" => {
                let entries = std::fs::read_dir(path)
                    .map_err(|e| JarvisError::System(e.to_string()))?;
                
                let mut files = Vec::new();
                for entry in entries {
                    let entry = entry.map_err(|e| JarvisError::System(e.to_string()))?;
                    files.push(entry.file_name().to_string_lossy().to_string());
                }
                files.join("\\n")
            },
            "read" => {
                std::fs::read_to_string(path)
                    .map_err(|e| JarvisError::System(e.to_string()))?
            },
            _ => return Err(JarvisError::Plugin(format!("Operation {} not implemented", operation))),
        };

        Ok(SkillResult {
            success: true,
            output,
            error: None,
            metadata: HashMap::new(),
            execution_time: start_time.elapsed(),
            resources_used: ResourceUsage::default(),
        })
    }

    fn validate_parameters(&self, parameters: &HashMap<String, String>) -> JarvisResult<()> {
        if !parameters.contains_key("operation") {
            return Err(JarvisError::Plugin("operation parameter required".to_string()));
        }
        if !parameters.contains_key("path") {
            return Err(JarvisError::Plugin("path parameter required".to_string()));
        }
        Ok(())
    }

    fn check_permissions(&self, available: &[Permission]) -> JarvisResult<()> {
        if !available.contains(&Permission::ReadFiles) {
            return Err(JarvisError::Plugin("ReadFiles permission required".to_string()));
        }
        Ok(())
    }

    fn help(&self) -> String {
        "Perform file operations like listing directories and reading files".to_string()
    }
}

/// Git operations skill
pub struct GitSkill {
    metadata: SkillMetadata,
}

impl GitSkill {
    pub fn new() -> Self {
        Self {
            metadata: SkillMetadata {
                id: "git_ops".to_string(),
                name: "Git Operations".to_string(),
                description: "Perform Git repository operations".to_string(),
                category: SkillCategory::Development,
                version: "1.0.0".to_string(),
                author: "Jarvis Core".to_string(),
                enabled: true,
                required_permissions: vec![Permission::ExecuteCommands],
                dependencies: vec![],
                parameters: vec![
                    SkillParameter {
                        name: "operation".to_string(),
                        parameter_type: ParameterType::String,
                        description: "Git operation (status, log, branch)".to_string(),
                        required: true,
                        default_value: None,
                        validation: None,
                    }
                ],
                examples: vec![],
                tags: vec!["git".to_string(), "version-control".to_string(), "development".to_string()],
            },
        }
    }
}

#[async_trait]
impl Skill for GitSkill {
    fn metadata(&self) -> &SkillMetadata {
        &self.metadata
    }

    async fn execute(&self, context: &SkillContext) -> JarvisResult<SkillResult> {
        let start_time = std::time::Instant::now();
        
        if let Some(git_context) = &context.environment.git_context {
            let output = format!(
                "Repository: {}\\nBranch: {}\\nDirty: {}\\nCommits: {}",
                git_context.repository_name,
                git_context.current_branch,
                git_context.is_dirty,
                git_context.recent_commits.len()
            );

            Ok(SkillResult {
                success: true,
                output,
                error: None,
                metadata: HashMap::new(),
                execution_time: start_time.elapsed(),
                resources_used: ResourceUsage::default(),
            })
        } else {
            Ok(SkillResult {
                success: false,
                output: String::new(),
                error: Some("Not in a Git repository".to_string()),
                metadata: HashMap::new(),
                execution_time: start_time.elapsed(),
                resources_used: ResourceUsage::default(),
            })
        }
    }

    fn validate_parameters(&self, _parameters: &HashMap<String, String>) -> JarvisResult<()> {
        Ok(())
    }

    fn check_permissions(&self, available: &[Permission]) -> JarvisResult<()> {
        if !available.contains(&Permission::ExecuteCommands) {
            return Err(JarvisError::Plugin("ExecuteCommands permission required".to_string()));
        }
        Ok(())
    }

    fn help(&self) -> String {
        "Get Git repository information and status".to_string()
    }
}

impl Default for ParameterValidation {
    fn default() -> Self {
        Self {
            min_length: None,
            max_length: None,
            pattern: None,
            allowed_values: None,
        }
    }
}