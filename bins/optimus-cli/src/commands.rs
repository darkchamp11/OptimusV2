// CLI commands for managing Optimus
use anyhow::{Context, Result, bail};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageExecution {
    pub command: String,
    pub args: Vec<String>,
    pub file_extension: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequests {
    pub memory: String,
    pub cpu: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub memory: String,
    pub cpu: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resources {
    pub requests: ResourceRequests,
    pub limits: ResourceLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concurrency {
    pub max_parallel_jobs: u32,
    pub max_parallel_tests: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageConfig {
    pub name: String,
    pub version: String,
    pub image: String,
    pub dockerfile_path: String,
    pub execution: LanguageExecution,
    pub queue_name: String,
    pub memory_limit_mb: u32,
    pub cpu_limit: f32,
    pub resources: Resources,
    pub concurrency: Concurrency,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LanguagesJson {
    pub languages: Vec<LanguageConfig>,
}

/// Load languages configuration
fn load_languages_config() -> Result<LanguagesJson> {
    let config_path = Path::new("config/languages.json");
    if !config_path.exists() {
        return Ok(LanguagesJson { languages: vec![] });
    }

    let content = fs::read_to_string(config_path)
        .context("Failed to read languages.json")?;
    serde_json::from_str(&content)
        .context("Failed to parse languages.json")
}

/// Save languages configuration
fn save_languages_config(config: &LanguagesJson) -> Result<()> {
    let config_path = Path::new("config/languages.json");
    
    // Ensure config directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    let json_content = serde_json::to_string_pretty(&config)
        .context("Failed to serialize languages.json")?;
    
    fs::write(config_path, json_content)
        .context("Failed to write languages.json")?;
    
    Ok(())
}

/// Add a new language to Optimus
pub async fn add_language(
    name: &str,
    ext: &str,
    version: &str,
    base_image: Option<&str>,
    command: Option<&str>,
    queue: Option<&str>,
    memory: u32,
    cpu: f32,
    build_docker: bool,
) -> Result<()> {
    println!("🚀 Adding language: {}", name);

    // Validate inputs
    if name.is_empty() || ext.is_empty() {
        bail!("Language name and extension cannot be empty");
    }

    // Load existing config
    let mut languages_json = load_languages_config()?;

    // Check if language already exists
    if languages_json.languages.iter().any(|l| l.name == name) {
        bail!("Language '{}' already exists in config", name);
    }

    // Determine defaults
    let exec_command = command.unwrap_or(name).to_string();
    let queue_name = queue.map(|q| q.to_string())
        .unwrap_or_else(|| format!("optimus:queue:{}", name));
    let file_extension = if ext.starts_with('.') {
        ext.to_string()
    } else {
        format!(".{}", ext)
    };

    // Calculate resource allocations
    let (resources, concurrency) = calculate_resources(memory, cpu);

    // Create new language config
    let new_lang = LanguageConfig {
        name: name.to_string(),
        version: version.to_string(),
        image: format!("optimus-{}:{}-v1", name, version),
        dockerfile_path: format!("dockerfiles/{}/Dockerfile", name),
        execution: LanguageExecution {
            command: exec_command,
            args: vec![],
            file_extension,
        },
        queue_name,
        memory_limit_mb: memory,
        cpu_limit: cpu,
        resources,
        concurrency,
    };

    // Add to languages
    languages_json.languages.push(new_lang);

    // Save config
    println!("📝 Updating config/languages.json...");
    save_languages_config(&languages_json)?;

    // Generate Dockerfile
    let dockerfile_dir = PathBuf::from(format!("dockerfiles/{}", name));
    let dockerfile_path = dockerfile_dir.join("Dockerfile");
    println!("🐳 Generating Dockerfile...");
    generate_dockerfile(&dockerfile_path, name, version, base_image)?;

    // Generate runner script if needed
    if matches!(name, "python" | "java" | "rust" | "cpp" | "go") {
        println!("📜 Generating runner script...");
        generate_runner_script(&dockerfile_dir, name)?;
    }

    println!("✅ Language '{}' added successfully!", name);

    // Build Docker image if requested
    if build_docker {
        println!("\n🔨 Building Docker image...");
        build_docker_image(name, false).await?;
        
        println!("\n📋 Next steps:");
        println!("  1. Render K8s manifests: optimus-cli render-k8s");
        println!("  2. Deploy to cluster: kubectl apply -f k8s/worker-deployment-{}.yaml", name);
    } else {
        println!("\n⚠️  Docker image not built - the language won't work until you build it!");
        println!("\n📋 Next steps:");
        println!("  1. Build Docker image: optimus-cli build-image --name {}", name);
        println!("  2. Render K8s manifests: optimus-cli render-k8s");
        println!("  3. Deploy to cluster: kubectl apply -f k8s/");
    }

    Ok(())
}

/// Calculate resource allocations based on memory and CPU
fn calculate_resources(memory_mb: u32, cpu: f32) -> (Resources, Concurrency) {
    // Resource requests are 50% of limits
    let memory_request = format!("{}Mi", memory_mb * 2);
    let memory_limit = format!("{}Gi", (memory_mb as f32 * 4.0 / 1024.0).ceil() as u32);
    let cpu_request = format!("{}m", (cpu * 1000.0) as u32);
    let cpu_limit = format!("{}m", (cpu * 4000.0) as u32);

    let resources = Resources {
        requests: ResourceRequests {
            memory: memory_request,
            cpu: cpu_request,
        },
        limits: ResourceLimits {
            memory: memory_limit,
            cpu: cpu_limit,
        },
    };

    // Concurrency based on memory
    let concurrency = if memory_mb >= 512 {
        Concurrency {
            max_parallel_jobs: 2,
            max_parallel_tests: 3,
        }
    } else {
        Concurrency {
            max_parallel_jobs: 3,
            max_parallel_tests: 5,
        }
    };

    (resources, concurrency)
}

/// Remove a language from Optimus
pub async fn remove_language(name: &str, yes: bool) -> Result<()> {
    println!("🗑️  Removing language: {}", name);

    // Load existing config
    let mut languages_json = load_languages_config()?;

    // Find language
    let lang_index = languages_json.languages.iter()
        .position(|l| l.name == name)
        .ok_or_else(|| anyhow::anyhow!("Language '{}' not found in config", name))?;

    let lang_version = languages_json.languages[lang_index].version.clone();
    let lang_dockerfile_path = languages_json.languages[lang_index].dockerfile_path.clone();

    // Confirm deletion
    if !yes {
        print!("⚠️  This will remove:\n");
        print!("  - Config entry in languages.json\n");
        print!("  - Dockerfile at {}\n", lang_dockerfile_path);
        print!("  - K8s manifests (worker-deployment-{}.yaml, KEDA ScaledObjects)\n", name);
        print!("\nContinue? (y/N): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("❌ Aborted");
            return Ok(());
        }
    }

    // Remove from config
    languages_json.languages.remove(lang_index);
    println!("📝 Removing from config/languages.json...");
    save_languages_config(&languages_json)?;

    // Remove Dockerfile directory
    let dockerfile_dir = PathBuf::from(format!("dockerfiles/{}", name));
    if dockerfile_dir.exists() {
        println!("🐳 Removing {}...", dockerfile_dir.display());
        fs::remove_dir_all(&dockerfile_dir)
            .with_context(|| format!("Failed to remove {}", dockerfile_dir.display()))?;
    }

    // Remove K8s manifests
    let manifests = vec![
        format!("k8s/worker-deployment-{}.yaml", name),
        format!("k8s/keda/scaled-object-{}.yaml", name),
        format!("k8s/keda/scaled-object-{}-retry.yaml", name),
    ];

    for manifest_path in manifests {
        let path = Path::new(&manifest_path);
        if path.exists() {
            println!("📊 Removing {}...", manifest_path);
            fs::remove_file(path)
                .with_context(|| format!("Failed to remove {}", manifest_path))?;
        }
    }

    println!("✅ Language '{}' removed successfully!", name);
    println!("\n📋 Next steps:");
    println!("  1. Remove Docker image: docker rmi optimus-{}:{}-v1", name, lang_version);
    println!("  2. Apply changes to K8s cluster if deployed");

    Ok(())
}

/// List all configured languages
pub async fn list_languages() -> Result<()> {
    let languages_json = load_languages_config()?;

    if languages_json.languages.is_empty() {
        println!("No languages configured.");
        println!("\n💡 Add a language with: optimus-cli add-lang --name <name> --ext <ext>");
        return Ok(());
    }

    println!("📋 Configured Languages:\n");
    println!("{:<12} {:<10} {:<30} {:<20} {:<10}",
             "Name", "Version", "Image", "Queue", "CPU/Mem");
    println!("{}", "─".repeat(100));

    for lang in &languages_json.languages {
        println!("{:<12} {:<10} {:<30} {:<20} {:.1}/{} MB",
                 lang.name,
                 lang.version,
                 lang.image,
                 lang.queue_name,
                 lang.cpu_limit,
                 lang.memory_limit_mb);
    }

    println!("\n✅ Total: {} language(s)", languages_json.languages.len());

    Ok(())
}

/// Render Kubernetes manifests from templates
pub async fn render_k8s_manifests(output_dir: Option<&str>) -> Result<()> {
    println!("📊 Rendering Kubernetes manifests from templates...");

    let languages_json = load_languages_config()?;

    if languages_json.languages.is_empty() {
        bail!("No languages configured. Add a language first with: optimus-cli add-lang");
    }

    let output_base = output_dir.unwrap_or("k8s");
    let output_path = Path::new(output_base);
    let keda_path = output_path.join("keda");

    // Ensure output directories exist
    fs::create_dir_all(&output_path)?;
    fs::create_dir_all(&keda_path)?;

    // Load templates
    let worker_template = fs::read_to_string("config/templates/worker-deployment.yaml.tmpl")
        .context("Failed to read worker-deployment.yaml.tmpl")?;
    let scaledobject_template = fs::read_to_string("config/templates/scaled-object.yaml.tmpl")
        .context("Failed to read scaled-object.yaml.tmpl")?;
    let scaledobject_retry_template = fs::read_to_string("config/templates/scaled-object-retry.yaml.tmpl")
        .context("Failed to read scaled-object-retry.yaml.tmpl")?;

    // Initialize handlebars
    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);

    println!("\n🔧 Generating manifests:");

    for lang in &languages_json.languages {
        // Prepare template data
        let mut data = HashMap::new();
        data.insert("language", &lang.name);
        data.insert("queue_name", &lang.queue_name);
        data.insert("image", &lang.image);
        
        let memory_request = &lang.resources.requests.memory;
        let memory_limit = &lang.resources.limits.memory;
        let cpu_request = &lang.resources.requests.cpu;
        let cpu_limit = &lang.resources.limits.cpu;
        
        data.insert("memory_request", memory_request);
        data.insert("memory_limit", memory_limit);
        data.insert("cpu_request", cpu_request);
        data.insert("cpu_limit", cpu_limit);
        
        let max_parallel_jobs = lang.concurrency.max_parallel_jobs.to_string();
        let max_parallel_tests = lang.concurrency.max_parallel_tests.to_string();
        
        data.insert("max_parallel_jobs", &max_parallel_jobs);
        data.insert("max_parallel_tests", &max_parallel_tests);

        // Render worker deployment
        let worker_yaml = handlebars.render_template(&worker_template, &data)
            .context("Failed to render worker-deployment template")?;
        let worker_path = output_path.join(format!("worker-deployment-{}.yaml", lang.name));
        fs::write(&worker_path, worker_yaml)
            .with_context(|| format!("Failed to write {}", worker_path.display()))?;
        println!("  ✅ {}", worker_path.display());

        // Render KEDA ScaledObject
        let scaledobject_yaml = handlebars.render_template(&scaledobject_template, &data)
            .context("Failed to render scaled-object template")?;
        let scaledobject_path = keda_path.join(format!("scaled-object-{}.yaml", lang.name));
        fs::write(&scaledobject_path, scaledobject_yaml)
            .with_context(|| format!("Failed to write {}", scaledobject_path.display()))?;
        println!("  ✅ {}", scaledobject_path.display());

        // Render KEDA ScaledObject (retry)
        let scaledobject_retry_yaml = handlebars.render_template(&scaledobject_retry_template, &data)
            .context("Failed to render scaled-object-retry template")?;
        let scaledobject_retry_path = keda_path.join(format!("scaled-object-{}-retry.yaml", lang.name));
        fs::write(&scaledobject_retry_path, scaledobject_retry_yaml)
            .with_context(|| format!("Failed to write {}", scaledobject_retry_path.display()))?;
        println!("  ✅ {}", scaledobject_retry_path.display());
    }

    println!("\n✅ All manifests rendered successfully!");
    println!("📂 Output directory: {}", output_path.display());
    println!("\n📋 Next steps:");
    println!("  1. Review generated manifests");
    println!("  2. Apply to cluster: kubectl apply -f {}/", output_path.display());

    Ok(())
}

/// Generate Dockerfile for the language
fn generate_dockerfile(
    dockerfile_path: &Path,
    name: &str,
    version: &str,
    base_image: Option<&str>,
) -> Result<()> {
    // Create directory
    if let Some(parent) = dockerfile_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let dockerfile_content = match name {
        "python" => generate_python_dockerfile(version),
        "java" => generate_java_dockerfile(version),
        "rust" => generate_rust_dockerfile(version),
        "cpp" => generate_cpp_dockerfile(version),
        "go" => generate_go_dockerfile(version),
        "javascript" | "node" => generate_node_dockerfile(version),
        _ => {
            // Generic Dockerfile
            let default_base = format!("{}:{}", name, version);
            let base = base_image.unwrap_or(&default_base);
            format!(
                r#"# GENERATED BY optimus-cli — DO NOT EDIT
FROM {}

WORKDIR /app

# Copy runner script (if exists)
COPY runner.* /app/

# Set execution command
CMD ["{}"]
"#,
                base, name
            )
        }
    };

    fs::write(dockerfile_path, dockerfile_content)
        .context("Failed to write Dockerfile")?;

    Ok(())
}

/// Generate Python Dockerfile
fn generate_python_dockerfile(version: &str) -> String {
    format!(
        r#"# GENERATED BY optimus-cli — DO NOT EDIT
FROM python:{}

WORKDIR /app

# Copy runner script
COPY runner.py /app/runner.py

# Make runner executable
RUN chmod +x /app/runner.py

# Set Python to run in unbuffered mode
ENV PYTHONUNBUFFERED=1

CMD ["python", "/app/runner.py"]
"#,
        version
    )
}

/// Generate Java Dockerfile
fn generate_java_dockerfile(version: &str) -> String {
    format!(
        r#"# GENERATED BY optimus-cli — DO NOT EDIT
FROM openjdk:{}

WORKDIR /app

# Install necessary tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    && rm -rf /var/lib/apt/lists/*

# Copy runner script (if needed)
# COPY runner.sh /app/runner.sh
# RUN chmod +x /app/runner.sh

CMD ["java"]
"#,
        version
    )
}

/// Generate C++ Dockerfile
fn generate_cpp_dockerfile(version: &str) -> String {
    format!(
        r#"# GENERATED BY optimus-cli — DO NOT EDIT
FROM gcc:{}

WORKDIR /app

# Install necessary build tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

CMD ["g++"]
"#,
        version
    )
}

/// Generate Go Dockerfile
fn generate_go_dockerfile(version: &str) -> String {
    format!(
        r#"# GENERATED BY optimus-cli — DO NOT EDIT
FROM golang:{}

WORKDIR /app

# Set Go environment
ENV GO111MODULE=on
ENV CGO_ENABLED=0

CMD ["go"]
"#,
        version
    )
}

/// Generate Node.js Dockerfile
fn generate_node_dockerfile(version: &str) -> String {
    format!(
        r#"# GENERATED BY optimus-cli — DO NOT EDIT
FROM node:{}

WORKDIR /app

# Install necessary tools
RUN npm install -g typescript ts-node

CMD ["node"]
"#,
        version
    )
}

/// Generate Rust Dockerfile
fn generate_rust_dockerfile(version: &str) -> String {
    format!(
        r#"# GENERATED BY optimus-cli — DO NOT EDIT
# Rust Execution Environment - Optimized for Code Execution
FROM rust:{}-slim

# Set environment variables for performance
ENV CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH=/usr/local/cargo/bin:$PATH \
    RUSTFLAGS="-C opt-level=2 -C debuginfo=0"

WORKDIR /code

# Install required packages
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy runner script
COPY runner.sh /code/runner.sh
RUN chmod +x /code/runner.sh

# Create non-root user for security
RUN useradd -m -u 1000 optimus && \
    chown -R optimus:optimus /code

USER optimus

# Set entrypoint to runner script
ENTRYPOINT ["/code/runner.sh"]
"#,
        version
    )
}

/// Generate language-specific runner script
fn generate_runner_script(dockerfile_dir: &Path, name: &str) -> Result<()> {
    match name {
        "rust" => {
            let runner_path = dockerfile_dir.join("runner.sh");
            let runner_content = r#"#!/bin/bash
# GENERATED BY optimus-cli — DO NOT EDIT
# Optimus Rust Runner
# Executes Rust code with given input and captures output

set -e

# Read source code from environment (base64 encoded)
SOURCE_CODE_B64="${SOURCE_CODE:-}"
TEST_INPUT_B64="${TEST_INPUT:-}"

if [ -z "$SOURCE_CODE_B64" ]; then
    echo "Error: SOURCE_CODE environment variable not set" >&2
    exit 1
fi

# Decode source code and input
SOURCE_CODE=$(echo "$SOURCE_CODE_B64" | base64 -d)
TEST_INPUT=$(echo "$TEST_INPUT_B64" | base64 -d)

# Write source code to file
echo "$SOURCE_CODE" > /code/main.rs

# Compile the Rust code
rustc /code/main.rs -o /code/main 2>&1

if [ $? -ne 0 ]; then
    echo "Compilation failed" >&2
    exit 1
fi

# Execute with test input
echo "$TEST_INPUT" | /code/main
"#;
            fs::write(runner_path, runner_content)?;
        }
        "python" => {
            let runner_path = dockerfile_dir.join("runner.py");
            let runner_content = r#"#!/usr/bin/env python3
# GENERATED BY optimus-cli — DO NOT EDIT
"""
Python Runner for Optimus
Executes Python code with given input and captures output
"""

import sys
import subprocess
import tempfile
import os

def main():
    # Read source code from environment or stdin
    source_code = os.environ.get('SOURCE_CODE', '')
    if not source_code:
        source_code = sys.stdin.read()
    
    # Read input
    test_input = os.environ.get('TEST_INPUT', '')
    
    # Create temporary file
    with tempfile.NamedTemporaryFile(mode='w', suffix='.py', delete=False) as f:
        f.write(source_code)
        temp_file = f.name
    
    try:
        # Execute Python code
        result = subprocess.run(
            ['python', '-u', temp_file],
            input=test_input,
            capture_output=True,
            text=True,
            timeout=60
        )
        
        # Output results
        print(result.stdout, end='')
        if result.stderr:
            print(result.stderr, file=sys.stderr, end='')
        
        sys.exit(result.returncode)
    finally:
        # Cleanup
        if os.path.exists(temp_file):
            os.remove(temp_file)

if __name__ == '__main__':
    main()
"#;
            fs::write(runner_path, runner_content)?;
        }
        _ => {
            // For other languages, create a placeholder
            println!("  ⚠️  No default runner for {}. You may need to create one manually.", name);
        }
    }

    Ok(())
}

/// Initialize a new Optimus project
pub async fn init_project(path: &str) -> Result<()> {
    println!("🚀 Initializing Optimus project at: {}", path);
    
    let project_path = Path::new(path);
    
    // Create directories
    let dirs = [
        "config",
        "config/templates",
        "dockerfiles",
        "k8s",
        "k8s/keda",
        "examples",
    ];
    
    for dir in &dirs {
        let dir_path = project_path.join(dir);
        fs::create_dir_all(&dir_path)
            .with_context(|| format!("Failed to create directory: {}", dir))?;
        println!("  ✅ Created: {}", dir);
    }
    
    // Create default languages.json
    let languages_json_path = project_path.join("config/languages.json");
    if !languages_json_path.exists() {
        let default_config = LanguagesJson {
            languages: vec![],
        };
        let json_content = serde_json::to_string_pretty(&default_config)?;
        fs::write(languages_json_path, json_content)?;
        println!("  ✅ Created: config/languages.json");
    }
    
    // Create template files
    create_template_files(project_path)?;
    
    println!("✅ Project initialized successfully!");
    println!("\n📋 Next steps:");
    println!("  1. Add a language: optimus-cli add-lang --name python --ext py");
    println!("  2. Configure Redis and API settings");
    println!("  3. Deploy to Kubernetes");
    
    Ok(())
}

/// Create template files for K8s manifest generation
fn create_template_files(project_path: &Path) -> Result<()> {
    let templates_dir = project_path.join("config/templates");
    
    // Worker deployment template
    let worker_template = include_str!("../../../config/templates/worker-deployment.yaml.tmpl");
    let worker_path = templates_dir.join("worker-deployment.yaml.tmpl");
    if !worker_path.exists() {
        fs::write(&worker_path, worker_template)?;
        println!("  ✅ Created: config/templates/worker-deployment.yaml.tmpl");
    }
    
    // ScaledObject template
    let scaledobject_template = include_str!("../../../config/templates/scaled-object.yaml.tmpl");
    let scaledobject_path = templates_dir.join("scaled-object.yaml.tmpl");
    if !scaledobject_path.exists() {
        fs::write(&scaledobject_path, scaledobject_template)?;
        println!("  ✅ Created: config/templates/scaled-object.yaml.tmpl");
    }
    
    // ScaledObject retry template
    let scaledobject_retry_template = include_str!("../../../config/templates/scaled-object-retry.yaml.tmpl");
    let scaledobject_retry_path = templates_dir.join("scaled-object-retry.yaml.tmpl");
    if !scaledobject_retry_path.exists() {
        fs::write(&scaledobject_retry_path, scaledobject_retry_template)?;
        println!("  ✅ Created: config/templates/scaled-object-retry.yaml.tmpl");
    }
    
    Ok(())
}

/// Build Docker image for a language
pub async fn build_docker_image(name: &str, no_cache: bool) -> Result<()> {
    println!("🐳 Building Docker image for: {}", name);
    
    // Read languages.json to get version info
    let languages_json = load_languages_config()?;
    
    let lang_config = languages_json.languages.iter()
        .find(|l| l.name == name)
        .ok_or_else(|| anyhow::anyhow!("Language '{}' not found in config", name))?;
    
    let dockerfile_dir = PathBuf::from(format!("dockerfiles/{}", name));
    let dockerfile_path = dockerfile_dir.join("Dockerfile");
    
    if !dockerfile_path.exists() {
        bail!("Dockerfile not found at {}. Generate it first with add-lang command.", dockerfile_path.display());
    }
    
    // Build image tags
    let image_versioned = format!("optimus-{}:{}-v1", name, lang_config.version);
    let image_latest = format!("optimus-{}:latest", name);
    
    println!("📦 Building tags:");
    println!("  - {}", image_versioned);
    println!("  - {}", image_latest);
    println!("📂 Context: {}", dockerfile_dir.display());
    println!("📄 Dockerfile: {}", dockerfile_path.display());
    
    // Build docker command
    let mut docker_args = vec![
        "build".to_string(),
        "-t".to_string(),
        image_versioned.clone(),
        "-t".to_string(),
        image_latest.clone(),
        "-f".to_string(),
        dockerfile_path.to_string_lossy().to_string(),
    ];
    
    if no_cache {
        docker_args.push("--no-cache".to_string());
    }
    
    docker_args.push(dockerfile_dir.to_string_lossy().to_string());
    
    println!("\n🔨 Running: docker {}", docker_args.join(" "));
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    // Execute docker build
    let status = Command::new("docker")
        .args(&docker_args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("Failed to execute docker build. Is Docker installed and running?")?;
    
    if !status.success() {
        bail!("Docker build failed with exit code: {:?}", status.code());
    }
    
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Docker image built successfully!");
    println!("\n📦 Available images:");
    println!("  - {}", image_versioned);
    println!("  - {}", image_latest);
    
    // Verify images exist
    println!("\n🔍 Verifying images...");
    let verify_status = Command::new("docker")
        .args(&["images", &image_latest, "--format", "{{.Repository}}:{{.Tag}}"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();
    
    if verify_status.is_ok() {
        println!("✅ Image verification complete!");
    }
    
    Ok(())
}
