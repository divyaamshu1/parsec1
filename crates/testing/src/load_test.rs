//! Load and performance testing

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use tokio::sync::{Mutex, RwLock};
use tokio::time;
use tracing::{info, warn, debug};

use crate::TestingConfig;

/// Load tester
pub struct LoadTester {
    config: TestingConfig,
    active_tests: Arc<RwLock<HashMap<String, LoadTest>>>,
}

/// Load test configuration
#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    pub name: String,
    pub duration: Duration,
    pub users: usize,
    pub spawn_rate: usize,
    pub stages: Vec<LoadTestStage>,
    pub script: Option<String>,
    pub endpoints: Vec<String>,
    pub headers: HashMap<String, String>,
    pub timeout: Duration,
}

/// Load test stage
#[derive(Debug, Clone)]
pub struct LoadTestStage {
    pub duration: Duration,
    pub target_users: usize,
}

/// Load test
pub struct LoadTest {
    pub id: String,
    pub config: LoadTestConfig,
    pub start_time: std::time::Instant,
    pub stats: Arc<RwLock<LoadTestStats>>,
    pub results: Arc<RwLock<Vec<RequestResult>>>,
}

/// Load test statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoadTestStats {
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub total_response_time: Duration,
    pub min_response_time: Duration,
    pub max_response_time: Duration,
    pub avg_response_time: Duration,
    pub p50_response_time: Duration,
    pub p90_response_time: Duration,
    pub p95_response_time: Duration,
    pub p99_response_time: Duration,
    pub requests_per_second: f64,
}

/// Request result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestResult {
    pub id: usize,
    pub user_id: usize,
    pub endpoint: String,
    pub status_code: u16,
    pub response_time: Duration,
    pub success: bool,
    pub error: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Load test report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestReport {
    pub id: String,
    pub name: String,
    pub config: LoadTestConfig,
    pub duration: Duration,
    pub stats: LoadTestStats,
    pub results: Vec<RequestResult>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl LoadTester {
    /// Create new load tester
    pub fn new(config: TestingConfig) -> Result<Self> {
        Ok(Self {
            config,
            active_tests: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Run load test
    pub async fn run(&self, config: LoadTestConfig) -> Result<LoadTestReport> {
        let test_id = uuid::Uuid::new_v4().to_string();
        let start = std::time::Instant::now();

        info!("Starting load test: {}", config.name);
        info!("  Users: {}", config.users);
        info!("  Duration: {:?}", config.duration);

        let stats = Arc::new(RwLock::new(LoadTestStats::default()));
        let results = Arc::new(RwLock::new(Vec::new()));
        let client = reqwest::Client::new();

        // Spawn user tasks
        let mut handles = Vec::new();
        for user_id in 0..config.users {
            let stats = stats.clone();
            let results = results.clone();
            let config = config.clone();
            let client = client.clone();

            let handle = tokio::spawn(async move {
                let user_start = std::time::Instant::now();
                
                while user_start.elapsed() < config.duration {
                    for endpoint in &config.endpoints {
                        let request_start = std::time::Instant::now();
                        
                        let response = client.get(endpoint)
                            .headers(config.headers.clone().into_iter().map(|(k, v)| {
                                (reqwest::header::HeaderName::from_bytes(k.as_bytes()).unwrap(), v.parse().unwrap())
                            }).collect())
                            .timeout(config.timeout)
                            .send()
                            .await;

                        let response_time = request_start.elapsed();
                        let (success, status_code, error) = match response {
                            Ok(resp) => (resp.status().is_success(), resp.status().as_u16(), None),
                            Err(e) => (false, 0, Some(e.to_string())),
                        };

                        // Update stats
                        {
                            let mut stats = stats.write().await;
                            stats.total_requests += 1;
                            if success {
                                stats.successful_requests += 1;
                            } else {
                                stats.failed_requests += 1;
                            }
                            stats.total_response_time += response_time;
                            if response_time < stats.min_response_time || stats.min_response_time == Duration::from_secs(0) {
                                stats.min_response_time = response_time;
                            }
                            if response_time > stats.max_response_time {
                                stats.max_response_time = response_time;
                            }
                        }

                        // Record result
                        results.write().await.push(RequestResult {
                            id: results.read().await.len(),
                            user_id,
                            endpoint: endpoint.clone(),
                            status_code,
                            response_time,
                            success,
                            error,
                            timestamp: chrono::Utc::now(),
                        });
                    }

                    // Random think time
                    time::sleep(Duration::from_millis(rand::random::<u64>() % 1000)).await;
                }
            });

            handles.push(handle);
        }

        // Wait for all users to complete
        for handle in handles {
            handle.await?;
        }

        let duration = start.elapsed();

        // Calculate final stats
        let final_stats = {
            let mut stats = stats.write().await;
            if stats.total_requests > 0 {
                stats.avg_response_time = stats.total_response_time / stats.total_requests as u32;
            }
            stats.requests_per_second = stats.total_requests as f64 / duration.as_secs_f64();

            // Calculate percentiles
            let mut response_times: Vec<_> = results.read().await.iter()
                .map(|r| r.response_time)
                .collect();
            response_times.sort();

            let len = response_times.len();
            if len > 0 {
                stats.p50_response_time = response_times[len * 50 / 100];
                stats.p90_response_time = response_times[len * 90 / 100];
                stats.p95_response_time = response_times[len * 95 / 100];
                stats.p99_response_time = response_times[len * 99 / 100];
            }

            stats.clone()
        };

        let report = LoadTestReport {
            id: test_id.clone(),
            name: config.name.clone(),
            config,
            duration,
            stats: final_stats,
            results: results.read().await.clone(),
            timestamp: chrono::Utc::now(),
        };

        // Save report
        self.save_report(&report).await?;

        Ok(report)
    }

    /// Save load test report
    async fn save_report(&self, report: &LoadTestReport) -> Result<()> {
        let path = self.config.reports_dir.join(format!(
            "loadtest_{}_{}.json",
            report.name,
            report.timestamp.format("%Y%m%d_%H%M%S")
        ));
        let json = serde_json::to_string_pretty(report)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }

    /// Generate HTML report
    pub async fn generate_html_report(&self, report: &LoadTestReport) -> Result<String> {
        let mut html = String::new();
        
        html.push_str(r#"<!DOCTYPE html>
<html>
<head>
    <title>Load Test Report</title>
    <style>
        body { font-family: monospace; margin: 20px; background: #1e1e1e; color: #d4d4d4; }
        .stats { margin: 20px 0; padding: 10px; background: #2d2d2d; border-radius: 5px; }
        .success { color: #6a9955; }
        .failure { color: #f48771; }
        table { border-collapse: collapse; width: 100%; }
        th, td { border: 1px solid #3c3c3c; padding: 8px; text-align: left; }
        th { background: #007acc; }
    </style>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
</head>
<body>
    <h1>Load Test Report: {}</h1>
    <div class="stats">
        <h2>Summary</h2>
        <p>Duration: {:?}</p>
        <p>Total Requests: {}</p>
        <p>Successful: <span class="success">{}</span></p>
        <p>Failed: <span class="failure">{}</span></p>
        <p>Requests/sec: {:.2}</p>
        <p>Avg Response: {:?}</p>
        <p>P95 Response: {:?}</p>
        <p>P99 Response: {:?}</p>
    </div>
    
    <canvas id="responseChart"></canvas>
    
    <script>
        const ctx = document.getElementById('responseChart').getContext('2d');
        new Chart(ctx, {
            type: 'line',
            data: {
                labels: [...Array({}).keys()],
                datasets: [{
                    label: 'Response Time (ms)',
                    data: {},
                    borderColor: '#007acc'
                }]
            }
        });
    </script>
</body>
</html>"#,
            report.name,
            report.duration,
            report.stats.total_requests,
            report.stats.successful_requests,
            report.stats.failed_requests,
            report.stats.requests_per_second,
            report.stats.avg_response_time,
            report.stats.p95_response_time,
            report.stats.p99_response_time
        );

        Ok(html)
    }
}