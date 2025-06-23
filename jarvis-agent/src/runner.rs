use anyhow::Result;
use jarvis_core::{MemoryStore, LLMRouter, types::*};
use crate::tools::SystemTools;

pub struct AgentRunner {
    memory: MemoryStore,
    llm: LLMRouter,
    tools: SystemTools,
}

impl AgentRunner {
    pub async fn new(memory: MemoryStore, llm: LLMRouter) -> Result<Self> {
        let tools = SystemTools::new().await?;
        
        Ok(Self {
            memory,
            llm,
            tools,
        })
    }

    pub async fn explain(&self, query: &str, environment: &jarvis_shell::Environment) -> Result<()> {
        println!("🤖 Jarvis: Let me explain '{}'...", query);
        
        // Gather context
        let context = self.gather_context(query, environment).await?;
        
        // Generate explanation
        let prompt = format!(
            "Explain this query in the context of an Arch Linux system: {}\n\nSystem Context:\n{}",
            query, context
        );
        
        let response = self.llm.generate(&prompt, None).await?;
        println!("\n📚 Explanation:\n{}", response);
        
        Ok(())
    }

    pub async fn diagnose(&self, target: &str, environment: &jarvis_shell::Environment) -> Result<()> {
        println!("🔍 Jarvis: Diagnosing '{}'...", target);
        
        // Run diagnostic tools
        let diagnostic_info = self.tools.diagnose(target).await?;
        
        let prompt = format!(
            "Diagnose this system issue: {}\n\nDiagnostic Information:\n{}",
            target, diagnostic_info
        );
        
        let response = self.llm.generate(&prompt, None).await?;
        println!("\n🔍 Diagnosis:\n{}", response);
        
        Ok(())
    }

    pub async fn write_code(&self, description: &str, environment: &jarvis_shell::Environment) -> Result<()> {
        println!("✍️ Jarvis: Writing code for '{}'...", description);
        
        let prompt = format!(
            "Write code based on this description: {}\n\nEnvironment: Arch Linux, Rust ecosystem",
            description
        );
        
        let response = self.llm.generate(&prompt, None).await?;
        println!("\n💻 Generated Code:\n{}", response);
        
        Ok(())
    }

    pub async fn check_status(&self, target: &str, environment: &jarvis_shell::Environment) -> Result<()> {
        println!("✅ Jarvis: Checking status of '{}'...", target);
        
        let status_info = self.tools.check_status(target).await?;
        println!("\n📊 Status:\n{}", status_info);
        
        Ok(())
    }

    pub async fn fix_issue(&self, issue: &str, environment: &jarvis_shell::Environment) -> Result<()> {
        println!("🔧 Jarvis: Attempting to fix '{}'...", issue);
        
        // This would analyze the issue and propose fixes
        let prompt = format!(
            "Analyze this issue and suggest fixes for an Arch Linux system: {}",
            issue
        );
        
        let response = self.llm.generate(&prompt, None).await?;
        println!("\n🔧 Suggested Fix:\n{}", response);
        
        Ok(())
    }

    pub async fn train_model(&self, model_name: &str, data_path: &str) -> Result<()> {
        println!("🧠 Training model '{}' with data from '{}'", model_name, data_path);
        // TODO: Implement model training
        Ok(())
    }

    pub async fn list_models(&self) -> Result<()> {
        println!("📋 Available Models:");
        // TODO: List available models
        Ok(())
    }

    pub async fn load_model(&self, model_name: &str) -> Result<()> {
        println!("📥 Loading model '{}'", model_name);
        // TODO: Load specific model
        Ok(())
    }

    pub async fn interactive_chat(&self, environment: &jarvis_shell::Environment) -> Result<()> {
        println!("💬 Entering interactive chat mode. Type 'exit' to quit.");
        
        use std::io::{self, Write};
        
        loop {
            print!("You: ");
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();
            
            if input == "exit" {
                break;
            }
            
            let response = self.llm.generate(input, None).await?;
            println!("Jarvis: {}\n", response);
        }
        
        Ok(())
    }

    async fn gather_context(&self, query: &str, environment: &jarvis_shell::Environment) -> Result<String> {
        let mut context = String::new();
        
        // Add system information
        context.push_str(&format!("System: {}\n", environment.system_info()));
        
        // Add current directory context
        if let Some(git_info) = &environment.git_context {
            context.push_str(&format!("Git Repository: {}\n", git_info.repo_path));
            context.push_str(&format!("Branch: {}\n", git_info.current_branch));
        }
        
        // Add relevant file contents based on query
        // TODO: Implement smart file detection based on query
        
        Ok(context)
    }
}
