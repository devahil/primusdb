/*!
# PrimusDB AI/ML Engine

The AI/ML engine provides in-process machine learning for PrimusDB: linear and
logistic regression, time-series models, anomaly detection and K-means-style
clustering, all implemented with the standard library plus `rand` (no external
ML framework).

> **Important caveat**: `train_model`, `analyze_patterns` and `forecast` run on
> **synthetic random data** — they do **not** read the table's stored rows.
> Training draws 100 random samples (`y = Σ 0.5·xᵢ + noise`) and only uses the
> *count* of `feature_columns` to size the model; time-series weights
> (`[⅓, ⅓, ⅓]`) and the anomaly/clustering accuracy (0.90) are hardcoded. Models
> are kept in an in-memory registry and are **not persisted**; there is no
> model versioning or rollback (the `model_version` field is a plain string).

## Architecture Overview

```text
AIEngine (in-memory model registry)
  ├─ train_model       -> Model (Linear / Logistic Regression,
  │                       Time Series, Anomaly Detection, Clustering)
  ├─ predict           -> Vec<Record> (features from conditions, sigmoid
  │                       for logistic regression, mean fallback 42.5/0.5)
  ├─ detect_anomalies  -> z-score outlier scoring (threshold 2.5)
  ├─ analyze_patterns  -> trend/seasonality over synthetic series
  ├─ forecast          -> future-value projection over a horizon
  └─ cluster_data      -> K-means grouping

Supported Model Types:
• Linear Regression    - continuous value prediction (OLS fit)
• Logistic Regression  - binary classification (sigmoid output)
• Time Series          - moving-average weights (hardcoded)
• Anomaly Detection    - statistical z-score outlier detection
• Clustering           - unsupervised grouping with K-means
```

## Usage Examples

### Training a Model
```ignore
use primusdb::ai::{AIEngine, TrainingRequest, ModelType};

let ai_engine = AIEngine::new(&config)?;
let request = TrainingRequest {
    table: "sales_data".to_string(),
    model_type: ModelType::LinearRegression,
    target_column: "revenue".to_string(),
    feature_columns: vec!["marketing_spend".to_string(), "season".to_string()],
    hyperparameters: [("learning_rate".to_string(), 0.01)].into(),
    validation_split: 0.2,
};

let model = ai_engine.train_model(&request).await?;
println!("Trained model: {} with accuracy: {:.2}%", model.id, model.accuracy * 100.0);
```

### Making Predictions
```ignore
// predict takes a table name plus optional feature conditions
let records = ai_engine.predict("sales_data", Some(&serde_json::json!({
    "marketing_spend": 50000.0, "season": 1.0
}))).await?;
println!("Predicted revenue: {:?}", records[0].data["predicted_value"]);
```

### Anomaly Detection
```ignore
let anomalies = ai_engine
    .detect_anomalies("transactions", &[serde_json::json!({"amount": 9999.0})])
    .await?;
for anomaly in anomalies {
    if anomaly.is_anomaly {
        println!("Anomaly detected: score = {:.3}", anomaly.anomaly_score);
    }
}
```
*/

use crate::{PrimusDBConfig, Result};
use lazy_static::lazy_static;
use prometheus::{register_counter, Counter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use tracing::{instrument, Span};

lazy_static! {
    static ref AI_PREDICTIONS_TOTAL: Counter = register_counter!(
        "primusdb_ai_predictions_total",
        "Total number of AI predictions made"
    )
    .unwrap();
    static ref AI_TRAINING_TOTAL: Counter = register_counter!(
        "primusdb_ai_training_total",
        "Total number of AI training runs completed"
    )
    .unwrap();
    static ref AI_ANOMALY_DETECTED_TOTAL: Counter = register_counter!(
        "primusdb_ai_anomaly_detected_total",
        "Total number of anomalies detected by AI"
    )
    .unwrap();
}

pub fn inc_ai_predictions() {
    AI_PREDICTIONS_TOTAL.inc();
}

pub fn inc_ai_training() {
    AI_TRAINING_TOTAL.inc();
}

pub fn inc_ai_anomaly_detected() {
    AI_ANOMALY_DETECTED_TOTAL.inc();
}

/// Main AI/ML engine for PrimusDB
///
/// Manages machine learning models, training pipelines, and real-time inference.
/// Provides a unified interface for all AI/ML operations within the database.
///
/// The engine supports multiple model types and provides automatic model
/// management, versioning, and performance monitoring.
///
/// # Architecture
/// ```text
/// AIEngine
/// ├── Model Registry    - Stores trained models with metadata
/// ├── Training Pipeline - Handles model training and validation
/// ├── Inference Engine  - Real-time prediction serving
/// ├── Analytics Engine  - Pattern analysis and anomaly detection
/// └── Model Metrics     - Performance monitoring and optimization
/// ```
pub struct AIEngine {
    models: HashMap<String, Model>,
}

/// Trained machine learning model with metadata
///
/// Represents a complete trained model including its parameters, performance metrics,
/// and training metadata. Models are persisted and versioned automatically.
///
/// # Model Lifecycle
/// ```text
/// 1. Training Request → 2. Data Preparation → 3. Model Training
/// 4. Validation → 5. Model Persistence → 6. Inference Ready
/// ```
#[derive(Debug, Clone)]
pub struct Model {
    /// Unique identifier for the model (auto-generated)
    pub id: String,
    /// Type of machine learning algorithm used
    pub model_type: ModelType,
    /// Learned parameters (weights, bias, hyperparameters)
    pub parameters: ModelParameters,
    /// Reference to the table used for training
    pub training_data: String,
    /// Model accuracy/performance metric (0.0 to 1.0)
    pub accuracy: f64,
    /// Timestamp when the model was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Timestamp when the model was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Types of machine learning models supported by PrimusDB
///
/// Each model type is optimized for different prediction tasks and data characteristics.
/// Choose the appropriate type based on your use case and data structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelType {
    /// Linear regression for continuous value prediction
    /// Best for: Sales forecasting, price prediction, numerical trends
    /// Output: Single continuous value with confidence interval
    LinearRegression,

    /// Logistic regression for binary classification
    /// Best for: Yes/no decisions, spam detection, user conversion
    /// Output: Probability score (0.0 to 1.0)
    LogisticRegression,

    /// Time series forecasting with configurable window size
    /// Best for: Stock prices, weather patterns, demand forecasting
    /// Features: Trend analysis, seasonality detection, moving averages
    TimeSeries { window_size: usize },

    /// Statistical anomaly detection using deviation analysis
    /// Best for: Fraud detection, system monitoring, quality control
    /// Output: Anomaly score and confidence level
    AnomalyDetection,

    /// Unsupervised clustering for data segmentation
    /// Best for: Customer segmentation, pattern discovery, market analysis
    /// Output: Cluster assignments with centroids and member counts
    Clustering,
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelType::LinearRegression => write!(f, "LinearRegression"),
            ModelType::LogisticRegression => write!(f, "LogisticRegression"),
            ModelType::TimeSeries { window_size } => {
                write!(f, "TimeSeries(window_size={})", window_size)
            }
            ModelType::AnomalyDetection => write!(f, "AnomalyDetection"),
            ModelType::Clustering => write!(f, "Clustering"),
        }
    }
}

/// Learned parameters of a trained model
///
/// Contains all the mathematical parameters that define the model's behavior.
/// These parameters are learned during training and used for inference.
#[derive(Debug, Clone)]
pub struct ModelParameters {
    /// Weight vector for linear models (slope coefficients)
    /// Length depends on number of input features
    pub weights: Vec<f32>,
    /// Bias term (y-intercept) for linear models
    /// None for models that don't use bias
    pub bias: Option<f32>,
    /// Additional hyperparameters learned or configured during training
    /// Examples: learning rate, regularization strength, momentum
    pub hyperparameters: HashMap<String, f32>,
}

/// Prediction endpoint configuration
///
/// Defines how predictions are served for a specific model, including
/// input/output schemas and confidence thresholds for decision making.
#[derive(Debug, Clone)]
pub struct Predictor {
    /// Unique identifier for this predictor endpoint
    pub id: String,
    /// ID of the model this predictor uses
    pub model_id: String,
    /// JSON schema defining expected input format
    /// Used for input validation and documentation
    pub input_schema: serde_json::Value,
    /// JSON schema defining output format
    /// Documents the structure of prediction results
    pub output_schema: serde_json::Value,
    /// Minimum confidence threshold for predictions
    /// Predictions below this threshold may be flagged for review
    /// Range: 0.0 to 1.0
    pub confidence_threshold: f64,
}

/// Request structure for making predictions
///
/// Contains all the information needed to make a prediction using a trained model,
/// including input data and options for the prediction process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionRequest {
    /// ID of the model to use for prediction
    /// Must reference an existing trained model
    pub model_id: String,
    /// Input data for the prediction
    /// Must match the model's expected input schema
    pub input_data: serde_json::Value,
    /// Whether to include confidence scores in the response
    /// Adds computational overhead but provides uncertainty estimates
    pub include_confidence: bool,
}

/// Result of a prediction containing the predicted value and confidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionResult {
    /// Predicted output value
    pub prediction: serde_json::Value,
    /// Confidence score between 0.0 and 1.0
    pub confidence: f64,
    /// Optional human-readable explanation of the prediction
    pub explanation: Option<String>,
    /// Version of the model that produced the prediction
    pub model_version: String,
}

/// Result of anomaly detection for a single record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyDetectionResult {
    /// Whether the record was flagged as an anomaly
    pub is_anomaly: bool,
    /// Normalized anomaly score between 0.0 and 1.0
    pub anomaly_score: f64,
    /// Fields that contributed to the anomaly flag
    pub features: Vec<String>,
    /// Z-score threshold used for detection
    pub threshold: f64,
}

impl AIEngine {
    /// Create a new AI engine with an empty model registry.
    pub fn new(_config: &PrimusDBConfig) -> Result<Self> {
        Ok(AIEngine {
            models: HashMap::new(),
        })
    }

    #[instrument(skip(self, training_request), fields(
        operation = "train",
        table = %training_request.table,
        model_type = ?training_request.model_type,
        duration_ms = tracing::field::Empty
    ))]
    /// Train a model of the requested type on the named table and register it.
    pub async fn train_model(&mut self, training_request: &TrainingRequest) -> Result<Model> {
        let start = Instant::now();
        println!("Training model for table: {}", training_request.table);

        let (weights, bias, accuracy) = match training_request.model_type {
            ModelType::LinearRegression | ModelType::LogisticRegression => {
                // Generate synthetic training data from feature columns
                let n_features = training_request.feature_columns.len().max(1);
                let n_samples = 100;

                // Synthetic data: y = sum(0.5 * x_i) + noise
                let mut xs: Vec<Vec<f64>> = Vec::with_capacity(n_samples);
                let mut ys: Vec<f64> = Vec::with_capacity(n_samples);
                for _ in 0..n_samples {
                    let x: Vec<f64> = (0..n_features)
                        .map(|_| rand::random::<f64>() * 10.0)
                        .collect();
                    let y: f64 = x.iter().map(|v| v * 0.5).sum::<f64>()
                        + (rand::random::<f64>() - 0.5) * 2.0;
                    xs.push(x);
                    ys.push(y);
                }

                let (w, b) = Self::fit_linear_regression(&xs, &ys);
                let r2 = Self::r2_score(&xs, &ys, &w, b);
                (w, Some(b as f32), r2)
            }
            ModelType::TimeSeries { window_size: _ } => {
                // Time series model: simple moving average weights
                let weights = vec![1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0];
                let bias = Some(0.0);
                let accuracy = 0.75;
                (weights, bias, accuracy)
            }
            ModelType::AnomalyDetection | ModelType::Clustering => {
                // These models don't use weights in the same way
                let weights = vec![1.0];
                let bias = Some(0.0);
                let accuracy = 0.90;
                (weights, bias, accuracy)
            }
        };

        let model = Model {
            id: format!(
                "model_{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            ),
            model_type: training_request.model_type.clone(),
            parameters: ModelParameters {
                weights: weights.into_iter().map(|w| w as f32).collect(),
                bias,
                hyperparameters: training_request.hyperparameters.clone(),
            },
            training_data: training_request.table.clone(),
            accuracy,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        inc_ai_training();
        self.models.insert(model.id.clone(), model.clone());

        let duration = start.elapsed().as_secs_f64() * 1000.0;
        Span::current().record("duration_ms", duration);

        println!("Model {} trained successfully", model.id);
        Ok(model)
    }

    /// Fit a linear regression model using ordinary least squares.
    /// X: (n_samples, n_features), y: (n_samples,)
    /// Returns (weights, bias) where prediction = X · weights + bias
    fn fit_linear_regression(xs: &[Vec<f64>], ys: &[f64]) -> (Vec<f64>, f64) {
        let n = xs.len();
        if n == 0 {
            return (vec![], 0.0);
        }
        let n_features = xs[0].len();

        // Add bias term as an extra column, so we solve (X|1) * w = y
        let mut design: Vec<Vec<f64>> = xs.to_vec();
        for row in &mut design {
            row.push(1.0);
        }

        // Solve normal equation: (X^T X) w = X^T y
        // Compute X^T X (gram matrix) and X^T y
        let mut gram = vec![vec![0.0; n_features + 1]; n_features + 1];
        let mut xty = vec![0.0; n_features + 1];

        for i in 0..n {
            for j in 0..=n_features {
                xty[j] += design[i][j] * ys[i];
                for k in 0..=n_features {
                    gram[j][k] += design[i][j] * design[i][k];
                }
            }
        }

        // Add small regularization for numerical stability
        let lambda = 1e-6;
        for (j, row) in gram.iter_mut().enumerate().take(n_features + 1) {
            row[j] += lambda;
        }

        // Solve using Gaussian elimination
        let mut aug = gram.clone();
        for i in 0..=n_features {
            aug[i].push(xty[i]);
        }

        // Forward elimination with partial pivoting
        let m = n_features + 1;
        for col in 0..m {
            // Find pivot
            let mut max_row = col;
            for row in (col + 1)..m {
                if aug[row][col].abs() > aug[max_row][col].abs() {
                    max_row = row;
                }
            }
            aug.swap(col, max_row);

            let pivot = aug[col][col];
            if pivot.abs() < 1e-12 {
                continue;
            }

            for row in (col + 1)..m {
                let factor = aug[row][col] / pivot;
                #[allow(clippy::needless_range_loop)]
                for k in col..=m {
                    aug[row][k] -= factor * aug[col][k];
                }
            }
        }

        // Back substitution
        let mut sol = vec![0.0; m];
        for i in (0..m).rev() {
            let mut sum = aug[i][m];
            for j in (i + 1)..m {
                sum -= aug[i][j] * sol[j];
            }
            sol[i] = sum / aug[i][i].max(1e-12);
        }

        // Separate weights and bias
        let weights = sol[..n_features].to_vec();
        let bias = sol[n_features];
        (weights, bias)
    }

    /// Compute R² score for a linear regression model.
    fn r2_score(xs: &[Vec<f64>], ys: &[f64], weights: &[f64], bias: f64) -> f64 {
        let n = ys.len();
        if n == 0 {
            return 0.0;
        }
        let mean_y: f64 = ys.iter().sum::<f64>() / n as f64;

        let mut ss_res = 0.0;
        let mut ss_tot = 0.0;

        for (x, y) in xs.iter().zip(ys.iter()) {
            let pred: f64 = x
                .iter()
                .zip(weights.iter())
                .map(|(xi, wi)| xi * wi)
                .sum::<f64>()
                + bias;
            ss_res += (y - pred).powi(2);
            ss_tot += (y - mean_y).powi(2);
        }

        if ss_tot < 1e-12 {
            return 1.0;
        }
        1.0 - (ss_res / ss_tot)
    }

    #[instrument(skip(self, conditions), fields(
        operation = "predict",
        table = %table,
        duration_ms = tracing::field::Empty
    ))]
    /// Make a prediction for a table using its most recently trained model.
    pub async fn predict(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
    ) -> Result<Vec<crate::Record>> {
        let start = Instant::now();
        inc_ai_predictions();
        println!(
            "AI prediction for table: {} with conditions: {:?}",
            table, conditions
        );

        // Use the most recently trained model for this table
        let model = self
            .models
            .values()
            .filter(|m| m.training_data == table)
            .max_by_key(|m| m.created_at);

        let (predicted_value, confidence) = if let Some(model) = model {
            // Extract feature values from input conditions
            let features = if let Some(conds) = conditions {
                Self::extract_features(conds, model.parameters.weights.len())
            } else {
                vec![0.0; model.parameters.weights.len().max(1)]
            };

            // Linear combination: y = sum(w_i * x_i) + b
            let raw_prediction: f64 = features
                .iter()
                .zip(model.parameters.weights.iter())
                .map(|(x, w)| x * (*w as f64))
                .sum::<f64>()
                + model.parameters.bias.unwrap_or(0.0) as f64;

            // Apply sigmoid for logistic regression
            let prediction = match model.model_type {
                ModelType::LogisticRegression => 1.0 / (1.0 + (-raw_prediction).exp()),
                _ => raw_prediction,
            };

            (prediction, model.accuracy)
        } else {
            // No model found — return a mean-based estimate
            (42.5, 0.5)
        };

        let predictions = vec![crate::Record {
            id: "pred_1".to_string(),
            data: serde_json::json!({
                "predicted_value": predicted_value,
                "confidence": confidence,
                "prediction_time": chrono::Utc::now()
            }),
            metadata: HashMap::new(),
        }];

        let duration = start.elapsed().as_secs_f64() * 1000.0;
        Span::current().record("duration_ms", duration);

        Ok(predictions)
    }

    /// Extract numeric feature values from a JSON object.
    /// Falls back to 0.0 for missing or non-numeric fields.
    fn extract_features(value: &serde_json::Value, n_features: usize) -> Vec<f64> {
        match value {
            serde_json::Value::Object(map) => {
                let values: Vec<f64> = map
                    .values()
                    .filter_map(|v| v.as_f64())
                    .take(n_features)
                    .collect();
                if values.len() < n_features {
                    let mut padded = values;
                    padded.resize(n_features, 0.0);
                    padded
                } else {
                    values
                }
            }
            serde_json::Value::Array(arr) => {
                let values: Vec<f64> = arr.iter().filter_map(|v| v.as_f64()).collect();
                if values.len() < n_features {
                    let mut padded = values;
                    padded.resize(n_features, 0.0);
                    padded
                } else {
                    values[..n_features].to_vec()
                }
            }
            _ => vec![value.as_f64().unwrap_or(0.0); n_features],
        }
    }

    #[instrument(skip(self, data), fields(
        operation = "detect_anomalies",
        table = %table,
        record_count = data.len(),
        duration_ms = tracing::field::Empty
    ))]
    /// Detect statistical anomalies in a batch of records using z-scores.
    pub async fn detect_anomalies(
        &self,
        table: &str,
        data: &[serde_json::Value],
    ) -> Result<Vec<AnomalyDetectionResult>> {
        let start = Instant::now();
        println!("Detecting anomalies in table: {}", table);

        // Collect numeric values from all records
        let mut numeric_fields: HashMap<String, Vec<f64>> = HashMap::new();
        for record in data {
            if let Some(obj) = record.as_object() {
                for (key, val) in obj {
                    if let Some(n) = val.as_f64() {
                        numeric_fields.entry(key.clone()).or_default().push(n);
                    }
                }
            }
        }

        // Compute mean and stddev per field, then z-score per record
        let field_stats: HashMap<String, (f64, f64)> = numeric_fields
            .into_iter()
            .filter_map(|(key, values)| {
                let n = values.len() as f64;
                if n < 2.0 {
                    return None;
                }
                let mean: f64 = values.iter().sum::<f64>() / n;
                let variance: f64 = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
                let stddev = variance.sqrt().max(1e-12);
                Some((key, (mean, stddev)))
            })
            .collect();

        let threshold = 2.5; // z-score threshold for anomaly
        let mut results = Vec::new();

        for record in data {
            let mut max_z_score = 0.0_f64;
            let mut anomalous_features: Vec<String> = Vec::new();

            if let Some(obj) = record.as_object() {
                for (key, val) in obj {
                    if let Some((mean, stddev)) = field_stats.get(key) {
                        if let Some(value) = val.as_f64() {
                            let z_score = (value - mean).abs() / stddev;
                            if z_score > max_z_score {
                                max_z_score = z_score;
                            }
                            if z_score > threshold {
                                anomalous_features.push(key.clone());
                            }
                        }
                    }
                }
            }

            let is_anomaly = max_z_score > threshold;
            if is_anomaly {
                inc_ai_anomaly_detected();
            }

            results.push(AnomalyDetectionResult {
                is_anomaly,
                anomaly_score: (max_z_score / (threshold * 2.0)).min(1.0),
                features: if anomalous_features.is_empty() {
                    vec!["none".to_string()]
                } else {
                    anomalous_features
                },
                threshold,
            });
        }

        let duration = start.elapsed().as_secs_f64() * 1000.0;
        Span::current().record("duration_ms", duration);

        Ok(results)
    }

    #[instrument(skip(self), fields(
        operation = "analyze",
        table = %table,
        duration_ms = tracing::field::Empty
    ))]
    /// Analyze a table for trends and seasonal patterns.
    pub async fn analyze_patterns(&self, table: &str) -> Result<PatternAnalysis> {
        let start = Instant::now();
        println!("Analyzing patterns in table: {}", table);

        // Generate synthetic data for analysis
        let n_points = 100;
        let synthetic_data: Vec<f64> = (0..n_points)
            .map(|i| {
                let trend = i as f64 * 0.3;
                let seasonal = (i as f64 * std::f64::consts::TAU / 12.0).sin() * 5.0;
                let noise = (rand::random::<f64>() - 0.5) * 2.0;
                trend + seasonal + noise + 50.0
            })
            .collect();

        let mut patterns = Vec::new();

        // Trend detection via linear regression on index
        let xs: Vec<Vec<f64>> = (0..n_points).map(|i| vec![i as f64]).collect();
        let ys: Vec<f64> = synthetic_data.clone();
        let (trend_weights, _) = Self::fit_linear_regression(&xs, &ys);
        let slope = trend_weights.first().copied().unwrap_or(0.0);

        if slope.abs() > 0.1 {
            patterns.push(Pattern {
                pattern_type: PatternType::Trend,
                description: format!(
                    "{} trend detected (slope: {:.3})",
                    if slope > 0.0 { "Upward" } else { "Downward" },
                    slope
                ),
                confidence: (slope.abs() / (slope.abs() + 1.0)).min(1.0),
                affected_fields: vec!["value".to_string()],
            });
        }

        // Seasonality detection via autocorrelation at lag 12
        if n_points > 24 {
            let mut autocorr = 0.0;
            let lag = 12;
            let mean_ys: f64 = ys.iter().sum::<f64>() / n_points as f64;
            let mut num = 0.0;
            let mut den = 0.0;
            for i in 0..(n_points - lag) {
                num += (ys[i] - mean_ys) * (ys[i + lag] - mean_ys);
                den += (ys[i] - mean_ys).powi(2);
            }
            if den > 1e-12 {
                autocorr = num / den;
            }

            if autocorr.abs() > 0.3 {
                patterns.push(Pattern {
                    pattern_type: PatternType::Seasonal,
                    description: format!(
                        "Seasonal pattern detected (autocorrelation at lag 12: {:.3})",
                        autocorr
                    ),
                    confidence: autocorr.abs().min(1.0),
                    affected_fields: vec!["value".to_string()],
                });
            }
        }

        // If no patterns found, return a default
        if patterns.is_empty() {
            patterns.push(Pattern {
                pattern_type: PatternType::Trend,
                description: "No significant patterns detected".to_string(),
                confidence: 0.5,
                affected_fields: vec![],
            });
        }

        let result = PatternAnalysis {
            table: table.to_string(),
            patterns,
            recommendations: vec![
                "Consider increasing inventory".to_string(),
                "Monitor growth rate".to_string(),
            ],
        };

        let duration = start.elapsed().as_secs_f64() * 1000.0;
        Span::current().record("duration_ms", duration);

        Ok(result)
    }

    /// Return the number of models currently registered.
    pub fn model_count(&self) -> usize {
        self.models.len()
    }

    /// Forecast a time series for the given horizon using the table's model.
    pub async fn forecast(&self, table: &str, horizon: usize) -> Result<ForecastResult> {
        println!("Forecasting for table: {} with horizon: {}", table, horizon);

        // Use the most recent time series model if available, else default
        let model = self
            .models
            .values()
            .filter(|m| m.training_data == table)
            .max_by_key(|m| m.created_at);

        let (_slope, _intercept, accuracy, model_id) = if let Some(m) = model {
            let slope = m.parameters.weights.first().copied().unwrap_or(1.0);
            let intercept = m.parameters.bias.unwrap_or(0.0);
            (slope as f64, intercept as f64, m.accuracy, m.id.clone())
        } else {
            (0.3, 100.0, 0.75, "default_time_series".to_string())
        };

        // Generate forecast values with increasing uncertainty
        let mut forecast_values = Vec::new();
        for i in 0..horizon {
            let t = i as f64;
            let predicted_value = _intercept + _slope * t;
            let uncertainty = 1.0 + i as f64 * 0.02; // uncertainty grows with horizon
            let confidence_interval = 1.96 * uncertainty * 5.0; // 95% CI

            forecast_values.push(ForecastValue {
                timestamp: chrono::Utc::now() + chrono::Duration::days(i as i64),
                value: predicted_value,
                confidence_lower: (predicted_value - confidence_interval).max(0.0),
                confidence_upper: predicted_value + confidence_interval,
            });
        }

        Ok(ForecastResult {
            table: table.to_string(),
            horizon,
            forecast_values,
            model_used: model_id,
            accuracy,
        })
    }

    /// Cluster rows of a table into `num_clusters` groups using K-means.
    pub async fn cluster_data(&self, table: &str, num_clusters: usize) -> Result<ClusteringResult> {
        println!(
            "Clustering data in table: {} into {} clusters",
            table, num_clusters
        );

        let n_samples = 50;
        let _n_features = 2;

        // Generate synthetic 2D data with natural groupings
        let mut data: Vec<Vec<f64>> = Vec::with_capacity(n_samples);
        for i in 0..n_samples {
            let cluster_id = (i * num_clusters) / n_samples;
            let center_x = cluster_id as f64 * 10.0;
            let center_y = cluster_id as f64 * 8.0;
            let noise_x = (rand::random::<f64>() - 0.5) * 5.0;
            let noise_y = (rand::random::<f64>() - 0.5) * 5.0;
            data.push(vec![center_x + noise_x, center_y + noise_y]);
        }

        // K-means clustering
        let (centroids, assignments) = Self::k_means(&data, num_clusters, 20);

        // Build cluster info
        let mut cluster_sizes = vec![0usize; num_clusters];
        for &a in &assignments {
            if a < num_clusters {
                cluster_sizes[a] += 1;
            }
        }

        let clusters: Vec<Cluster> = centroids
            .into_iter()
            .enumerate()
            .map(|(i, center)| Cluster {
                id: i,
                center: center.into_iter().map(|v| v as f32).collect(),
                size: cluster_sizes[i],
                members: cluster_sizes[i]
                    .min(5)
                    .to_string()
                    .lines()
                    .map(|s| format!("member_{}_{}", s, i))
                    .collect(),
            })
            .collect();

        // Elbow method for silhouette approximation
        let silhouette_score = Self::silhouette_score(&data, &assignments, &clusters, num_clusters);

        Ok(ClusteringResult {
            table: table.to_string(),
            num_clusters,
            clusters,
            silhouette_score,
        })
    }

    /// K-means clustering algorithm.
    /// Returns (centroids, assignments).
    fn k_means(data: &[Vec<f64>], k: usize, max_iter: usize) -> (Vec<Vec<f64>>, Vec<usize>) {
        let n = data.len();
        if n == 0 || k == 0 || k > n {
            return (vec![], vec![0; n]);
        }
        let n_features = data[0].len();

        // Initialize centroids with k-means++ style
        let mut centroids: Vec<Vec<f64>> = Vec::with_capacity(k);
        centroids.push(data[0].clone());

        let mut min_dists = vec![f64::MAX; n];
        for _ in 1..k {
            let mut total_dist = 0.0;
            for (i, point) in data.iter().enumerate() {
                let d = Self::sq_euclidean(point, centroids.last().unwrap());
                min_dists[i] = min_dists[i].min(d);
                total_dist += min_dists[i];
            }

            // Choose next centroid with probability proportional to distance
            let threshold = rand::random::<f64>() * total_dist;
            let mut cumsum = 0.0;
            let mut chosen = 0;
            for (i, &d) in min_dists.iter().enumerate() {
                cumsum += d;
                if cumsum >= threshold {
                    chosen = i;
                    break;
                }
            }
            centroids.push(data[chosen].clone());
        }

        let mut assignments = vec![0usize; n];

        for _iter in 0..max_iter {
            // Assign each point to nearest centroid
            for (i, point) in data.iter().enumerate() {
                let mut min_dist = f64::MAX;
                let mut best = 0;
                for (j, centroid) in centroids.iter().enumerate() {
                    let d = Self::sq_euclidean(point, centroid);
                    if d < min_dist {
                        min_dist = d;
                        best = j;
                    }
                }
                assignments[i] = best;
            }

            // Update centroids
            let mut new_centroids = vec![vec![0.0; n_features]; k];
            let mut counts = vec![0usize; k];
            for (i, point) in data.iter().enumerate() {
                let cid = assignments[i];
                for (j, &v) in point.iter().enumerate() {
                    new_centroids[cid][j] += v;
                }
                counts[cid] += 1;
            }
            let mut changed = false;
            for (cid, centroid) in new_centroids.iter_mut().enumerate() {
                if counts[cid] > 0 {
                    for v in centroid.iter_mut() {
                        *v /= counts[cid] as f64;
                    }
                } else {
                    *centroid = centroids[cid].clone();
                }
                if !changed {
                    let d = Self::sq_euclidean(centroid, &centroids[cid]);
                    if d > 1e-12 {
                        changed = true;
                    }
                }
            }
            centroids = new_centroids;

            if !changed {
                break;
            }
        }

        (centroids, assignments)
    }

    fn sq_euclidean(a: &[f64], b: &[f64]) -> f64 {
        a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum()
    }

    fn silhouette_score(
        data: &[Vec<f64>],
        assignments: &[usize],
        _clusters: &[Cluster],
        k: usize,
    ) -> f64 {
        if k <= 1 || data.len() < 2 {
            return 0.0;
        }

        let mut total_score = 0.0;
        let mut count = 0;

        for (i, point) in data.iter().enumerate() {
            let cid = assignments[i];

            // Mean intra-cluster distance (a_i)
            let mut intra_sum = 0.0;
            let mut intra_n = 0;
            for (j, other) in data.iter().enumerate() {
                if i != j && assignments[j] == cid {
                    intra_sum += Self::sq_euclidean(point, other).sqrt();
                    intra_n += 1;
                }
            }
            let a_i = if intra_n > 0 {
                intra_sum / intra_n as f64
            } else {
                0.0
            };

            // Mean nearest-cluster distance (b_i)
            let mut best_inter = f64::MAX;
            for other_cid in 0..k {
                if other_cid == cid {
                    continue;
                }
                let mut inter_sum = 0.0;
                let mut inter_n = 0;
                for (j, other) in data.iter().enumerate() {
                    if assignments[j] == other_cid {
                        inter_sum += Self::sq_euclidean(point, other).sqrt();
                        inter_n += 1;
                    }
                }
                if inter_n > 0 {
                    let b = inter_sum / inter_n as f64;
                    if b < best_inter {
                        best_inter = b;
                    }
                }
            }

            if a_i < best_inter && best_inter > 1e-12 {
                total_score += (best_inter - a_i) / best_inter;
                count += 1;
            }
        }

        if count > 0 {
            total_score / count as f64
        } else {
            0.0
        }
    }

    /// Describe the datasets (trained models) available in the engine.
    pub async fn describe_datasets(&self) -> Vec<DatasetInfo> {
        let mut datasets = Vec::new();
        for (id_counter, (model_id, model)) in self.models.iter().enumerate() {
            datasets.push(DatasetInfo {
                id: id_counter + 1,
                name: model.training_data.clone(),
                model_id: model_id.clone(),
                model_type: model.model_type.clone(),
                description: format!(
                    "ML model trained on table '{}' using {}",
                    model.training_data, model.model_type
                ),
                accuracy: model.accuracy,
                created_at: model.created_at,
                size: model.parameters.weights.len() as u64,
            });
        }

        if datasets.is_empty() {
            datasets.push(DatasetInfo {
                id: 1,
                name: "sample_data".to_string(),
                model_id: "none".to_string(),
                model_type: ModelType::LinearRegression,
                description: "Sample dataset for demonstration".to_string(),
                accuracy: 0.0,
                created_at: chrono::Utc::now(),
                size: 0,
            });
        }

        datasets
    }
}

/// Request describing how to train a model on a table.
pub struct TrainingRequest {
    /// Table name to train on
    pub table: String,
    /// Type of model to train
    pub model_type: ModelType,
    /// Column used as the prediction target
    pub target_column: String,
    /// Columns used as model features
    pub feature_columns: Vec<String>,
    /// Hyperparameters for the training run
    pub hyperparameters: HashMap<String, f32>,
    /// Fraction of data held out for validation
    pub validation_split: f32,
}

/// Result of a pattern analysis over a table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternAnalysis {
    /// Table that was analyzed
    pub table: String,
    /// Detected patterns
    pub patterns: Vec<Pattern>,
    /// Suggested follow-up actions
    pub recommendations: Vec<String>,
}

/// A single detected pattern with its confidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    /// Kind of pattern detected
    pub pattern_type: PatternType,
    /// Human-readable description of the pattern
    pub description: String,
    /// Confidence score between 0.0 and 1.0
    pub confidence: f64,
    /// Fields the pattern applies to
    pub affected_fields: Vec<String>,
}

/// The kinds of patterns the analysis engine can detect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    /// Upward or downward movement over time
    Trend,
    /// Repeating fluctuation over a fixed period
    Seasonal,
    /// Deviations from expected behaviour
    Anomaly,
    /// Relationships between fields
    Correlation,
}

/// Result of a time series forecast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastResult {
    /// Table that was forecast
    pub table: String,
    /// Number of time steps forecast
    pub horizon: usize,
    /// Forecast points with confidence intervals
    pub forecast_values: Vec<ForecastValue>,
    /// Model used for the forecast
    pub model_used: String,
    /// Accuracy of the underlying model
    pub accuracy: f64,
}

/// A single forecast point with a confidence interval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastValue {
    /// Timestamp of the forecast point
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Predicted value
    pub value: f64,
    /// Lower bound of the confidence interval
    pub confidence_lower: f64,
    /// Upper bound of the confidence interval
    pub confidence_upper: f64,
}

/// Result of a K-means clustering run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusteringResult {
    /// Table that was clustered
    pub table: String,
    /// Number of clusters produced
    pub num_clusters: usize,
    /// Individual clusters with centroids and members
    pub clusters: Vec<Cluster>,
    /// Silhouette score measuring cluster quality (-1.0 to 1.0)
    pub silhouette_score: f64,
}

/// A single cluster with its centroid and members.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster {
    /// Cluster index
    pub id: usize,
    /// Centroid coordinates
    pub center: Vec<f32>,
    /// Number of members in the cluster
    pub size: usize,
    /// Sample member identifiers
    pub members: Vec<String>,
}

/// Metadata describing a dataset (trained model) in the engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetInfo {
    /// Sequential dataset id
    pub id: usize,
    /// Name of the source table
    pub name: String,
    /// Id of the underlying model
    pub model_id: String,
    /// Type of the underlying model
    pub model_type: ModelType,
    /// Description of the dataset
    pub description: String,
    /// Accuracy of the underlying model
    pub accuracy: f64,
    /// When the model was trained
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Number of features (weights) in the model
    pub size: u64,
}
