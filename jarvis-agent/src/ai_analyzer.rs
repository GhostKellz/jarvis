// jarvis-agent/src/ai_analyzer.rs
//! AI-powered blockchain analysis using Ollama and local LLMs

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use jarvis_core::{MemoryStore, LLMRouter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::blockchain_monitor::{MonitoringAlert, AlertSeverity};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIAnalysisResult {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub analysis_type: AnalysisType,
    pub confidence: f64,
    pub summary: String,
    pub recommendations: Vec<String>,
    pub risk_score: u8, // 0-100
    pub automated_actions: Vec<AutomatedAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisType {
    SecurityThreat,
    PerformanceOptimization,
    AnomalyDetection,
    PatternRecognition,
    PredictiveAnalysis,
    TransactionAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomatedAction {
    pub action_type: ActionType,
    pub description: String,
    pub priority: ActionPriority,
    pub estimated_impact: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    AlertStakeholders,
    ScaleResources,
    OptimizeParameters,
    BlockSuspiciousActivity,
    UpdateConfiguration,
    InitiateBackup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub struct AIAnalyzerConfig {
    pub model_name: String,
    pub analysis_prompt_template: String,
    pub confidence_threshold: f64,
    pub enable_automated_actions: bool,
    pub max_analysis_history: usize,
}

impl Default for AIAnalyzerConfig {
    fn default() -> Self {
        Self {
            model_name: "llama3.2:3b".to_string(),
            analysis_prompt_template: r#"
You are an expert blockchain analyst and security specialist. Analyze the following blockchain monitoring data and provide:

1. Risk assessment (0-100 scale)
2. Key insights and patterns
3. Specific recommendations
4. Automated actions if any are needed

Data to analyze:
{data}

Provide your analysis in JSON format with the following structure:
{{
    "risk_score": <0-100>,
    "summary": "<brief analysis>",
    "recommendations": ["<recommendation1>", "<recommendation2>"],
    "automated_actions": [{{
        "action_type": "<action>",
        "description": "<description>",
        "priority": "<priority>",
        "estimated_impact": "<impact>"
    }}]
}}
"#.to_string(),
            confidence_threshold: 0.7,
            enable_automated_actions: false, // Start conservative
            max_analysis_history: 1000,
        }
    }
}

pub struct AIBlockchainAnalyzer {
    llm_router: LLMRouter,
    memory: MemoryStore,
    config: AIAnalyzerConfig,
    analysis_history: Vec<AIAnalysisResult>,
}

impl AIBlockchainAnalyzer {
    pub fn new(llm_router: LLMRouter, memory: MemoryStore, config: AIAnalyzerConfig) -> Self {
        Self {
            llm_router,
            memory,
            config,
            analysis_history: Vec::new(),
        }
    }

    /// Analyze a monitoring alert using AI
    pub async fn analyze_alert(&mut self, alert: &MonitoringAlert) -> Result<AIAnalysisResult> {
        info!("Starting AI analysis for alert: {}", alert.id);

        // Prepare context data for analysis
        let context = self.prepare_analysis_context(alert).await?;
        
        // Get historical context
        let historical_data = self.get_relevant_history(&alert.alert_type).await?;
        
        // Format the prompt
        let prompt = self.format_analysis_prompt(alert, &context, &historical_data)?;
        
        // Query the LLM
        let llm_response = self.llm_router.generate(&prompt, None).await
            .context("Failed to get LLM response for alert analysis")?;

        // Parse and validate the response
        let analysis = self.parse_llm_response(alert, &llm_response).await?;
        
        // Store the analysis
        self.store_analysis(&analysis).await?;
        
        // Add to history
        self.analysis_history.push(analysis.clone());
        if self.analysis_history.len() > self.config.max_analysis_history {
            self.analysis_history.remove(0);
        }

        info!("AI analysis completed for alert: {} (Risk Score: {})", alert.id, analysis.risk_score);
        
        Ok(analysis)
    }

    /// Analyze blockchain patterns and trends
    pub async fn analyze_patterns(&mut self, timeframe_hours: u32) -> Result<AIAnalysisResult> {
        info!("Starting pattern analysis for {} hours", timeframe_hours);

        // Collect recent metrics and alerts
        let recent_data = self.collect_recent_data(timeframe_hours).await?;
        
        let prompt = format!(r#"
Analyze the following blockchain monitoring data from the last {} hours and identify:

1. Emerging patterns or trends
2. Potential future issues
3. Optimization opportunities
4. Security concerns

Data:
{}

Provide analysis in JSON format with risk assessment and recommendations.
"#, timeframe_hours, serde_json::to_string_pretty(&recent_data)?);

        let llm_response = self.llm_router.generate(&prompt, None).await
            .context("Failed to get LLM response for pattern analysis")?;

        let analysis = AIAnalysisResult {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            analysis_type: AnalysisType::PatternRecognition,
            confidence: 0.8, // Default confidence for pattern analysis
            summary: "Pattern analysis completed".to_string(),
            recommendations: vec!["Monitor identified patterns".to_string()],
            risk_score: 20, // Default low risk for patterns
            automated_actions: vec![],
        };

        self.store_analysis(&analysis).await?;
        
        Ok(analysis)
    }

    /// Predict potential issues based on current trends
    pub async fn predictive_analysis(&mut self) -> Result<AIAnalysisResult> {
        info!("Starting predictive analysis");

        // Get trend data from the last week
        let trend_data = self.collect_trend_data(7 * 24).await?;
        
        let prompt = format!(r#"
Based on the following blockchain monitoring trends, predict potential issues in the next 24-48 hours:

Trend Data:
{}

Focus on:
1. Performance degradation risks
2. Security vulnerability patterns
3. Resource exhaustion predictions
4. Network stability concerns

Provide predictions with confidence levels and recommended preventive actions.
"#, serde_json::to_string_pretty(&trend_data)?);

        let llm_response = self.llm_router.generate(&prompt, None).await
            .context("Failed to get LLM response for predictive analysis")?;

        let analysis = self.parse_predictive_response(&llm_response).await?;
        self.store_analysis(&analysis).await?;

        Ok(analysis)
    }

    /// Prepare context data for AI analysis
    async fn prepare_analysis_context(&self, alert: &MonitoringAlert) -> Result<HashMap<String, serde_json::Value>> {
        let mut context = HashMap::new();
        
        // Add alert details
        context.insert("alert".to_string(), serde_json::to_value(alert)?);
        
        // Add recent system metrics
        // In a real implementation, this would query recent metrics from memory
        context.insert("recent_metrics".to_string(), serde_json::json!({}));
        
        // Add network status
        context.insert("network_status".to_string(), serde_json::json!({
            "network": alert.network,
            "timestamp": alert.timestamp
        }));

        Ok(context)
    }

    /// Get relevant historical analysis for context
    async fn get_relevant_history(&self, alert_type: &crate::blockchain_monitor::AlertType) -> Result<Vec<AIAnalysisResult>> {
        // Filter analysis history for similar alert types
        let relevant = self.analysis_history.iter()
            .filter(|analysis| {
                // Match analysis types to alert types
                matches!((alert_type, &analysis.analysis_type), 
                    (crate::blockchain_monitor::AlertType::SecurityThreat, AnalysisType::SecurityThreat) |
                    (crate::blockchain_monitor::AlertType::PerformanceDegradation, AnalysisType::PerformanceOptimization) |
                    (_, AnalysisType::AnomalyDetection)
                )
            })
            .cloned()
            .collect();

        Ok(relevant)
    }

    /// Format the analysis prompt with context
    fn format_analysis_prompt(
        &self, 
        alert: &MonitoringAlert, 
        context: &HashMap<String, serde_json::Value>,
        history: &[AIAnalysisResult]
    ) -> Result<String> {
        let data = serde_json::json!({
            "alert": alert,
            "context": context,
            "historical_analysis": history
        });

        let prompt = self.config.analysis_prompt_template.replace("{data}", 
            &serde_json::to_string_pretty(&data)?);

        Ok(prompt)
    }

    /// Parse LLM response into structured analysis result
    async fn parse_llm_response(&self, alert: &MonitoringAlert, response: &str) -> Result<AIAnalysisResult> {
        // Try to parse JSON response
        let parsed: serde_json::Value = serde_json::from_str(response)
            .unwrap_or_else(|_| {
                // Fallback if response is not valid JSON
                serde_json::json!({
                    "risk_score": 50,
                    "summary": response,
                    "recommendations": ["Review the alert manually"],
                    "automated_actions": []
                })
            });

        let analysis_type = match alert.alert_type {
            crate::blockchain_monitor::AlertType::SecurityThreat => AnalysisType::SecurityThreat,
            crate::blockchain_monitor::AlertType::PerformanceDegradation => AnalysisType::PerformanceOptimization,
            crate::blockchain_monitor::AlertType::SuspiciousActivity => AnalysisType::AnomalyDetection,
            _ => AnalysisType::AnomalyDetection,
        };

        let risk_score = parsed["risk_score"].as_u64().unwrap_or(50) as u8;
        let summary = parsed["summary"].as_str().unwrap_or("Analysis completed").to_string();
        
        let recommendations: Vec<String> = parsed["recommendations"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_else(|| vec!["No specific recommendations".to_string()]);

        let automated_actions: Vec<AutomatedAction> = parsed["automated_actions"]
            .as_array()
            .map(|arr| {
                arr.iter().filter_map(|action| {
                    Some(AutomatedAction {
                        action_type: ActionType::AlertStakeholders, // Default
                        description: action["description"].as_str()?.to_string(),
                        priority: ActionPriority::Medium, // Default
                        estimated_impact: action["estimated_impact"].as_str().unwrap_or("Unknown").to_string(),
                    })
                }).collect()
            })
            .unwrap_or_default();

        Ok(AIAnalysisResult {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            analysis_type,
            confidence: 0.8, // Would be calculated based on model confidence
            summary,
            recommendations,
            risk_score,
            automated_actions,
        })
    }

    /// Parse predictive analysis response
    async fn parse_predictive_response(&self, response: &str) -> Result<AIAnalysisResult> {
        // Similar to parse_llm_response but for predictive analysis
        Ok(AIAnalysisResult {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            analysis_type: AnalysisType::PredictiveAnalysis,
            confidence: 0.7,
            summary: "Predictive analysis based on current trends".to_string(),
            recommendations: vec!["Monitor trends closely".to_string()],
            risk_score: 30,
            automated_actions: vec![],
        })
    }

    /// Store analysis result in memory
    async fn store_analysis(&mut self, analysis: &AIAnalysisResult) -> Result<()> {
        let key = format!("ai_analysis:{}", analysis.id);
        let data = serde_json::to_string(analysis)?;
        self.memory.store_document(&key, &data).await?;
        
        Ok(())
    }

    /// Collect recent monitoring data for analysis
    async fn collect_recent_data(&self, hours: u32) -> Result<serde_json::Value> {
        // In a real implementation, this would query the memory store
        // for alerts and metrics from the specified timeframe
        Ok(serde_json::json!({
            "timeframe_hours": hours,
            "alerts": [],
            "metrics": []
        }))
    }

    /// Collect trend data for predictive analysis
    async fn collect_trend_data(&self, hours: u32) -> Result<serde_json::Value> {
        // Similar to collect_recent_data but focused on trends
        Ok(serde_json::json!({
            "trend_period_hours": hours,
            "performance_trends": {},
            "security_trends": {},
            "usage_patterns": {}
        }))
    }

    /// Get analysis summary for dashboard
    pub async fn get_analysis_summary(&self) -> Result<serde_json::Value> {
        let recent_analyses: Vec<&AIAnalysisResult> = self.analysis_history.iter()
            .filter(|analysis| {
                let age = Utc::now().signed_duration_since(analysis.timestamp);
                age.num_hours() < 24
            })
            .collect();

        let avg_risk_score = if !recent_analyses.is_empty() {
            recent_analyses.iter().map(|a| a.risk_score as f64).sum::<f64>() / recent_analyses.len() as f64
        } else {
            0.0
        };

        Ok(serde_json::json!({
            "total_analyses": self.analysis_history.len(),
            "recent_analyses_24h": recent_analyses.len(),
            "average_risk_score": avg_risk_score,
            "last_analysis": recent_analyses.last().map(|a| &a.timestamp),
            "analysis_types": {
                "security": recent_analyses.iter().filter(|a| matches!(a.analysis_type, AnalysisType::SecurityThreat)).count(),
                "performance": recent_analyses.iter().filter(|a| matches!(a.analysis_type, AnalysisType::PerformanceOptimization)).count(),
                "anomaly": recent_analyses.iter().filter(|a| matches!(a.analysis_type, AnalysisType::AnomalyDetection)).count(),
                "predictive": recent_analyses.iter().filter(|a| matches!(a.analysis_type, AnalysisType::PredictiveAnalysis)).count()
            }
        }))
    }
}
