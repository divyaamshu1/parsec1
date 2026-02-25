//! Unit testing framework

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use futures::future::BoxFuture;
use tokio::sync::Mutex;
use tracing::{info, warn, debug};

use crate::{TestingConfig, TestSuite, TestDefinition, TestResult, TestStatus, TestType, TestReport, TestSummary};

/// Unit test runner
pub struct UnitTestRunner {
    config: TestingConfig,
    suites: Arc<Mutex<HashMap<String, TestSuite>>>,
    results: Arc<Mutex<Vec<TestResult>>>,
}

impl UnitTestRunner {
    /// Create new unit test runner
    pub fn new(config: TestingConfig) -> Result<Self> {
        Ok(Self {
            config,
            suites: Arc::new(Mutex::new(HashMap::new())),
            results: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Register a test suite
    pub async fn register_suite(&self, suite: TestSuite) {
        self.suites.lock().await.insert(suite.name.clone(), suite);
    }

    /// Run unit tests
    pub async fn run(&self, filter: Option<&str>) -> Result<TestReport> {
        let start = std::time::Instant::now();
        let suites = self.suites.lock().await;
        let mut results = Vec::new();
        let mut summary = TestSummary::default();

        for (name, suite) in suites.iter() {
            if let Some(filter) = filter {
                if !name.contains(filter) {
                    continue;
                }
            }

            info!("Running test suite: {}", name);

            // Run setup
            if let Some(setup) = &suite.setup {
                if let Err(e) = setup().await {
                    warn!("Setup failed for suite {}: {}", name, e);
                }
            }

            // Run tests
            for test in &suite.tests {
                if test.skip {
                    summary.skipped += 1;
                    continue;
                }

                if let Some(filter) = filter {
                    if !test.name.contains(filter) {
                        continue;
                    }
                }

                let test_start = std::time::Instant::now();
                let status = match test.handler().await {
                    Ok(_) => TestStatus::Passed,
                    Err(e) => {
                        summary.failed += 1;
                        TestStatus::Failed
                    }
                };

                let duration = test_start.elapsed();
                summary.total += 1;
                if status == TestStatus::Passed {
                    summary.passed += 1;
                }

                results.push(TestResult {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: test.name.clone(),
                    suite: name.clone(),
                    test_type: TestType::Unit,
                    status,
                    duration,
                    assertions: 0,
                    passed: if status == TestStatus::Passed { 1 } else { 0 },
                    failed: if status == TestStatus::Failed { 1 } else { 0 },
                    skipped: 0,
                    error: None,
                    stack_trace: None,
                    timestamp: chrono::Utc::now(),
                    logs: vec![],
                });
            }

            // Run teardown
            if let Some(teardown) = &suite.teardown {
                if let Err(e) = teardown().await {
                    warn!("Teardown failed for suite {}: {}", name, e);
                }
            }
        }

        summary.duration = start.elapsed();

        Ok(TestReport {
            name: "unit".to_string(),
            timestamp: chrono::Utc::now(),
            duration: start.elapsed(),
            summary,
            results,
            coverage: None,
            artifacts: vec![],
        })
    }

    /// Create a test
    pub fn create_test<F, Fut>(name: &str, handler: F) -> TestDefinition
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send,
    {
        let handler = Arc::new(move || {
            let fut = handler();
            Box::pin(fut) as BoxFuture<'static, Result<()>>
        });

        TestDefinition {
            name: name.to_string(),
            handler,
            tags: vec![],
            timeout: None,
            retries: 0,
            skip: false,
            only: false,
        }
    }

    /// Create a test suite
    pub fn create_suite(name: &str, tests: Vec<TestDefinition>) -> TestSuite {
        TestSuite {
            name: name.to_string(),
            tests,
            setup: None,
            teardown: None,
            timeout: None,
            parallel: false,
        }
    }
}

/// Property-based testing with proptest
#[cfg(feature = "unit")]
pub mod property {
    use proptest::prelude::*;
    use anyhow::Result;

    /// Property test
    pub fn property_test<F, Input>(name: &str, f: F) -> Result<()>
    where
        F: Fn(Input) -> bool + 'static,
        Input: Arbitrary + std::fmt::Debug,
    {
        proptest!(|(input: Input)| {
            prop_assert!(f(input));
        });
        Ok(())
    }

    /// Generate test data
    pub fn generate_data<T: Arbitrary>(count: usize) -> Vec<T> {
        let strategy = any::<T>();
        let mut runner = TestRunner::default();
        (0..count).map(|_| strategy.new_value(&mut runner).unwrap().current()).collect()
    }
}

/// QuickCheck testing
#[cfg(feature = "unit")]
pub mod quickcheck {
    use quickcheck::{QuickCheck, Testable};
    use anyhow::Result;

    /// QuickCheck property test
    pub fn quickcheck<A: Testable>(f: A) -> Result<()> {
        QuickCheck::new().quickcheck(f);
        Ok(())
    }
}

/// Benchmarking with criterion
#[cfg(feature = "unit")]
pub mod benchmark {
    use criterion::{Criterion, BenchmarkId, black_box};
    use std::time::Duration;

    /// Run benchmark
    pub fn benchmark<F>(name: &str, f: F) -> Result<()>
    where
        F: Fn() + 'static,
    {
        let mut criterion = Criterion::default()
            .warm_up_time(Duration::from_secs(3))
            .measurement_time(Duration::from_secs(5));

        criterion.bench_function(name, |b| b.iter(|| black_box(f())));
        Ok(())
    }

    /// Benchmark with different inputs
    pub fn benchmark_with_inputs<F, T>(name: &str, inputs: Vec<T>, mut f: F) -> Result<()>
    where
        F: FnMut(&T) + 'static,
        T: std::fmt::Display,
    {
        let mut criterion = Criterion::default();
        
        for input in inputs {
            criterion.bench_with_input(
                BenchmarkId::new(name, input),
                &input,
                |b, i| b.iter(|| black_box(f(i)))
            );
        }

        Ok(())
    }
}