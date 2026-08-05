/**
 * PrimusDB Node.js Driver
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: MIT - See LICENSE file for details
 * Version: 1.3.1-alpha - Added: Resource Governor API methods
 */

import axios, { AxiosInstance } from 'axios';

/**
 * # PrimusDB Node.js Driver
 *
 * Node.js client library for PrimusDB - Hybrid Database Engine supporting columnar, vector, document, and relational storage with AI/ML capabilities.
 *
 * ## Features
 *
 * - **Native Performance**: Direct HTTP client with connection pooling
 * - **Async/Await Support**: Full async/await compatibility with promises
 * - **Complete CRUD**: Create, Read, Update, Delete operations
 * - **AI/ML Integration**: Built-in predictions and clustering
 * - **Vector Search**: High-performance similarity search
 * - **Type Safety**: Full TypeScript support with type definitions
 * - **Connection Pooling**: Efficient connection management
 *
 * ## Quick Start
 *
 * ```typescript
 * import { PrimusDB } from 'primusdb';
 *
 * async function main() {
 *   const db = new PrimusDB('localhost', 8080);
 *
 *   // Create a table
 *   await db.createTable('document', 'users', {
 *     name: 'string',
 *     email: 'string',
 *     age: 'integer'
 *   });
 *
 *   // Insert data
 *   await db.insert('document', 'users', {
 *     name: 'John Doe',
 *     email: 'john@example.com',
 *     age: 30
 *   });
 *
 *   // Query data
 *   const users = await db.select('document', 'users', {
 *     age: { $gte: 25 }
 *   });
 *
 *   console.log(users);
 * }
 *
 * main();
 * ```
 */

export interface PrimusDBConfig {
  host: string;
  port: number;
  timeout?: number;
  maxRetries?: number;
}

export interface Schema {
  [key: string]: string;
}

export interface QueryConditions {
  [key: string]: any;
}

export interface InsertData {
  [key: string]: any;
}

export interface UpdateData {
  [key: string]: any;
}

export interface PredictParams {
  [key: string]: any;
}

export interface ClusterParams {
  [key: string]: any;
}

export interface VectorSearchResult {
  id: string;
  score: number;
  data: any;
}

export interface AnalysisResult {
  [key: string]: any;
}

export interface PredictionResult {
  [key: string]: any;
}

export interface ClusterResult {
  [key: string]: any;
}

export interface CacheConfig {
  maxMemory: number;
  compressionEnabled: boolean;
  compressionLevel: 'Fast' | 'Balanced' | 'High';
  enableSearch: boolean;
  corruptionCheck: boolean;
  lruEnabled: boolean;
  bloomFilterEnabled: boolean;
}

export interface CacheStatistics {
  entries: number;
  memoryUsed: number;
  memoryPeak: number;
  hits: number;
  misses: number;
  hitRate: number;
  compressionRatio: number;
  avgAccessTimeUs: number;
  evictions: number;
  corruptionsDetected: number;
}

export interface ClusterConfig {
  nodes: string[];
  replicationFactor: number;
  consensusQuorum: number;
  enableEncryption: boolean;
  heartbeatInterval: number;
}

export interface ClusterHealth {
  overallHealth: number;
  totalNodes: number;
  healthyNodes: number;
  unhealthyNodes: number;
  failedNodes: number;
  averageResponseTime: number;
  dataConsistencyScore: number;
}

export interface ClusterStatistics {
  totalOperations: number;
  successfulOperations: number;
  failedOperations: number;
  successRate: number;
  avgValidationTimeMs: number;
  activeValidators: number;
  totalValidators: number;
}

export interface GovernorStatus {
  enabled: boolean;
  active_executions: number;
  total_violations: number;
  blocked_count: number;
  throttled_count: number;
  policies_loaded: number;
  uptime_seconds: number;
}

export interface GovernorMetrics {
  active_executions: number;
  blocked_total: number;
  throttled_total: number;
  policy_violations_total: number;
  memory_usage_bytes: number;
  cpu_time_ms: number;
  ffi_calls_total: number;
}

export interface GovernorExecution {
  execution_id: string;
  namespace: string;
  workload_type: string;
  action: string;
  created_at: string;
  elapsed_ms: number;
}

export interface GovernorViolation {
  id: string;
  timestamp: string;
  execution_id: string;
  namespace: string;
  workload_type: string;
  limit_name: string;
  limit_value: string;
  usage_value: string;
  action: string;
}

export interface GovernorPolicy {
  name: string;
  scope: string;
  scope_name: string;
  action: string;
  max_memory_mb: number | null;
  max_execution_steps: number | null;
}

export interface GovernorCheckResult {
  action: string;
  message: string | null;
}

export interface GovernorStartResult {
  execution_id: string;
  action: string;
}

export interface ServerCapabilities {
  protocol_version: number;
  server: ServerInfo;
  engines: EngineCapabilities[];
  features: string[];
}

export interface ServerInfo {
  version: string;
  node_id: string;
  instance_id: string;
  uptime_seconds: number;
}

export interface EngineCapabilities {
  storage_type: string;
  tables: string[];
}

/**
 * PrimusDB Node.js Client
 */
export class PrimusDB {
  private config: PrimusDBConfig;
  private httpClient: AxiosInstance;
  private connected: boolean = false;

  /**
   * Create a new PrimusDB client instance
   *
   * @param host - Server hostname or IP address
   * @param port - Server port number
   * @param config - Additional configuration options
   */
  constructor(host: string = 'localhost', port: number = 8080, config: Partial<PrimusDBConfig> = {}) {
    this.config = {
      host,
      port,
      timeout: 30000,
      maxRetries: 3,
      ...config
    };

    this.httpClient = axios.create({
      baseURL: `http://${this.config.host}:${this.config.port}`,
      timeout: this.config.timeout,
      headers: {
        'Content-Type': 'application/json',
        'Accept': 'application/json'
      }
    });
  }

  /**
   * Connect to the PrimusDB server
   */
  async connect(): Promise<void> {
    try {
      const response = await this.httpClient.get('/health');
      if (response.status === 200) {
        this.connected = true;
      } else {
        throw new Error('Server health check failed');
      }
    } catch (error) {
      throw new Error(`Failed to connect to PrimusDB server: ${error}`);
    }
  }

  /**
   * Disconnect from the server
   */
  async disconnect(): Promise<void> {
    this.connected = false;
  }

  /**
   * Check if connected to server
   */
  isConnected(): boolean {
    return this.connected;
  }

  /**
   * Create a new table/collection
   *
   * @param storageType - Storage type: 'document', 'columnar', 'vector', 'relational'
   * @param table - Table/collection name
   * @param schema - Schema definition
   */
  async createTable(storageType: string, table: string, schema: Schema): Promise<void> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post(`/api/v1/crud/${storageType}/${table}`, {
        operation: 'CreateTable',
        schema: schema
      });

      if (response.status !== 200) {
        throw new Error(`Failed to create table: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Create table failed: ${error}`);
    }
  }

  /**
   * Insert data into a table
   *
   * @param storageType - Storage type
   * @param table - Table/collection name
   * @param data - Data to insert
   * @returns Number of records inserted
   */
  async insert(storageType: string, table: string, data: InsertData): Promise<number> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post(`/api/v1/crud/${storageType}/${table}`, data);

      if (response.status === 200) {
        return 1; // Assume single record insert
      } else {
        throw new Error(`Insert failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Insert failed: ${error}`);
    }
  }

  /**
   * Query data from a table
   *
   * @param storageType - Storage type
   * @param table - Table/collection name
   * @param conditions - Query conditions
   * @param limit - Maximum number of results
   * @param offset - Number of results to skip
   * @returns Array of matching records
   */
  async select(
    storageType: string,
    table: string,
    conditions?: QueryConditions,
    limit?: number,
    offset?: number
  ): Promise<any[]> {
    this.checkConnection();

    try {
      let url = `/api/v1/crud/${storageType}/${table}`;
      const params = new URLSearchParams();

      if (conditions) {
        params.append('conditions', JSON.stringify(conditions));
      }
      if (limit) {
        params.append('limit', limit.toString());
      }
      if (offset) {
        params.append('offset', offset.toString());
      }

      if (params.toString()) {
        url += '?' + params.toString();
      }

      const response = await this.httpClient.get(url);

      if (response.status === 200) {
        return response.data || [];
      } else {
        throw new Error(`Select failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Select failed: ${error}`);
    }
  }

  /**
   * Update existing records
   *
   * @param storageType - Storage type
   * @param table - Table/collection name
   * @param conditions - Update conditions
   * @param data - New data
   * @returns Number of records updated
   */
  async update(
    storageType: string,
    table: string,
    conditions: QueryConditions,
    data: UpdateData
  ): Promise<number> {
    this.checkConnection();

    try {
      const response = await this.httpClient.put(`/api/v1/crud/${storageType}/${table}`, {
        conditions,
        data
      });

      if (response.status === 200) {
        return response.data.count || 0;
      } else {
        throw new Error(`Update failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Update failed: ${error}`);
    }
  }

  /**
   * Delete records from a table
   *
   * @param storageType - Storage type
   * @param table - Table/collection name
   * @param conditions - Delete conditions
   * @returns Number of records deleted
   */
  async delete(storageType: string, table: string, conditions: QueryConditions): Promise<number> {
    this.checkConnection();

    try {
      const response = await this.httpClient.delete(`/api/v1/crud/${storageType}/${table}`, {
        data: { conditions }
      });

      if (response.status === 200) {
        return response.data.count || 0;
      } else {
        throw new Error(`Delete failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Delete failed: ${error}`);
    }
  }

  /**
   * Perform data analysis
   *
   * @param storageType - Storage type
   * @param table - Table/collection name
   * @param conditions - Analysis conditions
   * @returns Analysis results
   */
  async analyze(
    storageType: string,
    table: string,
    conditions?: QueryConditions
  ): Promise<AnalysisResult> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post('/api/v1/advanced/analyze', {
        storage_type: storageType,
        table,
        conditions
      });

      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`Analysis failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Analysis failed: ${error}`);
    }
  }

  /**
   * Make AI predictions
   *
   * @param storageType - Storage type
   * @param table - Table/collection name
   * @param data - Input data for prediction
   * @param predictionType - Type of prediction
   * @returns Prediction results
   */
  async predict(
    storageType: string,
    table: string,
    data: PredictParams,
    predictionType: string
  ): Promise<PredictionResult> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post('/api/v1/advanced/predict', {
        storage_type: storageType,
        table,
        data,
        prediction_type: predictionType
      });

      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`Prediction failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Prediction failed: ${error}`);
    }
  }

  /**
   * Perform vector similarity search
   *
   * @param table - Table/collection name
   * @param queryVector - Query vector
   * @param limit - Maximum number of results
   * @returns Search results
   */
  async vectorSearch(
    table: string,
    queryVector: number[],
    limit: number = 10
  ): Promise<VectorSearchResult[]> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post('/api/v1/advanced/vector-search', {
        table,
        query_vector: queryVector,
        limit
      });

      if (response.status === 200) {
        return response.data.results || [];
      } else {
        throw new Error(`Vector search failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Vector search failed: ${error}`);
    }
  }

  /**
   * Perform data clustering
   *
   * @param storageType - Storage type
   * @param table - Table/collection name
   * @param params - Clustering parameters
   * @returns Clustering results
   */
  async cluster(
    storageType: string,
    table: string,
    params: ClusterParams
  ): Promise<ClusterResult> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post('/api/v1/advanced/cluster', {
        storage_type: storageType,
        table,
        params
      });

      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`Clustering failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Clustering failed: ${error}`);
    }
  }

  /**
   * Get server health status
   */
  async health(): Promise<any> {
    try {
      const response = await this.httpClient.get('/health');
      return response.data;
    } catch (error) {
      throw new Error(`Health check failed: ${error}`);
    }
  }

  /**
   * Get detailed server status
   */
  async status(): Promise<any> {
    try {
      const response = await this.httpClient.get('/status');
      return response.data;
    } catch (error) {
      throw new Error(`Status check failed: ${error}`);
    }
  }

  // ==================== Capability Negotiation ====================

  /**
   * Fetch the server capabilities snapshot
   */
  async capabilities(): Promise<ServerCapabilities> {
    this.checkConnection();
    const response = await this.httpClient.get('/api/v1/capabilities');
    if (response.data.success) return response.data.data;
    throw new Error(response.data.error || 'Capabilities fetch failed');
  }

  /**
   * Negotiate with the server: fetch capabilities and validate that the node
   * supports all required features and storage engines
   *
   * @param requiredFeatures - Feature flags the server must advertise
   * @param requiredEngines - Storage engines the server must advertise
   * @returns Server capabilities snapshot
   */
  async negotiate(requiredFeatures?: string[], requiredEngines?: string[]): Promise<ServerCapabilities> {
    const caps = await this.capabilities();
    if (requiredFeatures && requiredFeatures.length > 0) {
      const missing = requiredFeatures.filter((f) => !caps.features.includes(f));
      if (missing.length > 0) {
        throw new Error(`Missing required features: ${missing.join(', ')}`);
      }
    }
    if (requiredEngines && requiredEngines.length > 0) {
      const available = caps.engines.map((e) => e.storage_type);
      const missing = requiredEngines.filter((e) => !available.includes(e));
      if (missing.length > 0) {
        throw new Error(`Missing required engines: ${missing.join(', ')}`);
      }
    }
    return caps;
  }

  /// Cache management methods

  /**
   * Enable or disable caching
   */
  async enableCache(enabled: boolean = true): Promise<void> {
    try {
      const response = await this.httpClient.post('/api/v1/cache/enable', { enabled });
      if (response.status !== 200) {
        throw new Error(`Failed to ${enabled ? 'enable' : 'disable'} cache: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Cache ${enabled ? 'enable' : 'disable'} failed: ${error}`);
    }
  }

  /**
   * Configure cache settings
   */
  async configureCache(config: Partial<CacheConfig>): Promise<void> {
    try {
      const response = await this.httpClient.post('/api/v1/cache/configure', config);
      if (response.status !== 200) {
        throw new Error(`Failed to configure cache: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Cache configuration failed: ${error}`);
    }
  }

  /**
   * Get cache statistics
   */
  async getCacheStatistics(): Promise<CacheStatistics> {
    try {
      const response = await this.httpClient.get('/api/v1/cache/statistics');
      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`Failed to get cache statistics: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Get cache statistics failed: ${error}`);
    }
  }

  /**
   * Clear all cache entries
   */
  async clearCache(): Promise<void> {
    try {
      const response = await this.httpClient.post('/api/v1/cache/clear');
      if (response.status !== 200) {
        throw new Error(`Failed to clear cache: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Clear cache failed: ${error}`);
    }
  }

  /**
   * Warm up cache with data
   */
  async warmupCache(data: Record<string, any>): Promise<void> {
    try {
      const response = await this.httpClient.post('/api/v1/cache/warmup', { data });
      if (response.status !== 200) {
        throw new Error(`Failed to warmup cache: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Cache warmup failed: ${error}`);
    }
  }

  /**
   * Search in cached data
   */
  async searchCache(pattern: string, limit: number = 100): Promise<any[]> {
    try {
      const response = await this.httpClient.get('/api/v1/cache/search', {
        params: { pattern, limit }
      });
      if (response.status === 200) {
        return response.data.results || [];
      } else {
        throw new Error(`Cache search failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Cache search failed: ${error}`);
    }
  }

  /// Distributed Cache Cluster APIs

  /**
   * Join a distributed cache cluster
   */
  async joinCacheCluster(config: ClusterConfig): Promise<void> {
    try {
      const response = await this.httpClient.post('/api/v1/cache/cluster/join', config);
      if (response.status !== 200) {
        throw new Error(`Failed to join cache cluster: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Join cache cluster failed: ${error}`);
    }
  }

  /**
   * Leave the distributed cache cluster
   */
  async leaveCacheCluster(): Promise<void> {
    try {
      const response = await this.httpClient.post('/api/v1/cache/cluster/leave');
      if (response.status !== 200) {
        throw new Error(`Failed to leave cache cluster: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Leave cache cluster failed: ${error}`);
    }
  }

  /**
   * Get cluster health status
   */
  async getClusterHealth(): Promise<ClusterHealth> {
    try {
      const response = await this.httpClient.get('/api/v1/cache/cluster/health');
      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`Failed to get cluster health: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Get cluster health failed: ${error}`);
    }
  }

  /**
   * Get cluster statistics
   */
  async getClusterStatistics(): Promise<ClusterStatistics> {
    try {
      const response = await this.httpClient.get('/api/v1/cache/cluster/statistics');
      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`Failed to get cluster statistics: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Get cluster statistics failed: ${error}`);
    }
  }

  /**
   * Add a node to the cache cluster
   */
  async addClusterNode(nodeAddress: string): Promise<void> {
    try {
      const response = await this.httpClient.post('/api/v1/cache/cluster/add-node', {
        nodeAddress
      });
      if (response.status !== 200) {
        throw new Error(`Failed to add cluster node: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Add cluster node failed: ${error}`);
    }
  }

  /**
   * Remove a node from the cache cluster
   */
  async removeClusterNode(nodeAddress: string): Promise<void> {
    try {
      const response = await this.httpClient.post('/api/v1/cache/cluster/remove-node', {
        nodeAddress
      });
      if (response.status !== 200) {
        throw new Error(`Failed to remove cluster node: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Remove cluster node failed: ${error}`);
    }
  }

  /**
   * Scale cluster to specified number of nodes
   */
  async scaleCluster(targetNodes: number): Promise<void> {
    try {
      const response = await this.httpClient.post('/api/v1/cache/cluster/scale', {
        targetNodes
      });
      if (response.status !== 200) {
        throw new Error(`Failed to scale cluster: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Scale cluster failed: ${error}`);
    }
  }

  /**
   * Get consensus validation statistics
   */
  async getConsensusStatistics(): Promise<any> {
    try {
      const response = await this.httpClient.get('/api/v1/cache/cluster/consensus');
      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`Failed to get consensus statistics: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Get consensus statistics failed: ${error}`);
    }
  }

  /**
   * Drop (delete) a table/collection
   *
   * @param storageType - Storage type
   * @param table - Table/collection name to drop
   */
  async dropTable(storageType: string, table: string): Promise<void> {
    this.checkConnection();

    try {
      const response = await this.httpClient.delete(`/api/v1/crud/${storageType}/${table}`);
      if (response.status !== 200) {
        throw new Error(`Failed to drop table: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Drop table failed: ${error}`);
    }
  }

  /**
   * Truncate (empty) a table/collection
   *
   * @param storageType - Storage type
   * @param table - Table/collection name to truncate
   * @param cascade - If true, also truncate dependent tables (default: false)
   */
  async truncateTable(storageType: string, table: string, cascade: boolean = false): Promise<void> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post(`/api/v1/crud/${storageType}/${table}/truncate`, { cascade });
      if (response.status !== 200) {
        throw new Error(`Failed to truncate table: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Truncate table failed: ${error}`);
    }
  }

  // ==================== DDL / ER Model Operations ====================

  /**
   * Add a column to a relational table
   */
  async addColumn(storageType: string, table: string, field: any): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.post(`/api/v1/ddl/${storageType}/${table}/column/add`, field);
      if (response.status === 200) { return response.data; }
      else { throw new Error(`Add column failed: ${response.statusText}`); }
    } catch (error) { throw new Error(`Add column failed: ${error}`); }
  }

  /**
   * Drop a column from a relational table
   */
  async dropColumn(storageType: string, table: string, columnName: string): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.delete(`/api/v1/ddl/${storageType}/${table}/column/${columnName}`);
      if (response.status === 200) { return response.data; }
      else { throw new Error(`Drop column failed: ${response.statusText}`); }
    } catch (error) { throw new Error(`Drop column failed: ${error}`); }
  }

  /**
   * Modify a column definition
   */
  async modifyColumn(storageType: string, table: string, field: any): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.put(`/api/v1/ddl/${storageType}/${table}/column`, field);
      if (response.status === 200) { return response.data; }
      else { throw new Error(`Modify column failed: ${response.statusText}`); }
    } catch (error) { throw new Error(`Modify column failed: ${error}`); }
  }

  /**
   * Add a constraint to a relational table
   */
  async addConstraint(storageType: string, table: string, constraint: any): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.post(`/api/v1/ddl/${storageType}/${table}/constraint`, constraint);
      if (response.status === 200) { return response.data; }
      else { throw new Error(`Add constraint failed: ${response.statusText}`); }
    } catch (error) { throw new Error(`Add constraint failed: ${error}`); }
  }

  /**
   * Drop a constraint from a relational table
   */
  async dropConstraint(storageType: string, table: string, constraintName: string): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.delete(`/api/v1/ddl/${storageType}/${table}/constraint/${constraintName}`);
      if (response.status === 200) { return response.data; }
      else { throw new Error(`Drop constraint failed: ${response.statusText}`); }
    } catch (error) { throw new Error(`Drop constraint failed: ${error}`); }
  }

  /**
   * Rename a relational table
   */
  async renameTable(storageType: string, table: string, newName: string): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.post(`/api/v1/ddl/${storageType}/${table}/rename`, { new_name: newName });
      if (response.status === 200) { return response.data; }
      else { throw new Error(`Rename table failed: ${response.statusText}`); }
    } catch (error) { throw new Error(`Rename table failed: ${error}`); }
  }

  /**
   * Create a sequence
   */
  async createSequence(storageType: string, sequence: any): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.post(`/api/v1/sequence/${storageType}`, sequence);
      if (response.status === 200) { return response.data; }
      else { throw new Error(`Create sequence failed: ${response.statusText}`); }
    } catch (error) { throw new Error(`Create sequence failed: ${error}`); }
  }

  /**
   * Drop a sequence
   */
  async dropSequence(storageType: string, name: string): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.delete(`/api/v1/sequence/${storageType}/${name}`);
      if (response.status === 200) { return response.data; }
      else { throw new Error(`Drop sequence failed: ${response.statusText}`); }
    } catch (error) { throw new Error(`Drop sequence failed: ${error}`); }
  }

  /**
   * Get next value from a sequence
   */
  async nextval(storageType: string, name: string): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.post(`/api/v1/sequence/${storageType}/${name}/nextval`);
      if (response.status === 200) { return response.data; }
      else { throw new Error(`Nextval failed: ${response.statusText}`); }
    } catch (error) { throw new Error(`Nextval failed: ${error}`); }
  }

  /**
   * Get current value of a sequence
   */
  async currval(storageType: string, name: string): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.get(`/api/v1/sequence/${storageType}/${name}/currval`);
      if (response.status === 200) { return response.data; }
      else { throw new Error(`Currval failed: ${response.statusText}`); }
    } catch (error) { throw new Error(`Currval failed: ${error}`); }
  }

  /**
   * Set a sequence value
   */
  async setval(storageType: string, name: string, value: number): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.post(`/api/v1/sequence/${storageType}/${name}/setval`, { value });
      if (response.status === 200) { return response.data; }
      else { throw new Error(`Setval failed: ${response.statusText}`); }
    } catch (error) { throw new Error(`Setval failed: ${error}`); }
  }

  /**
   * Create a view
   */
  async createView(storageType: string, view: any): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.post(`/api/v1/view/${storageType}`, view);
      if (response.status === 200) { return response.data; }
      else { throw new Error(`Create view failed: ${response.statusText}`); }
    } catch (error) { throw new Error(`Create view failed: ${error}`); }
  }

  /**
   * Drop a view
   */
  async dropView(storageType: string, name: string): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.delete(`/api/v1/view/${storageType}/${name}`);
      if (response.status === 200) { return response.data; }
      else { throw new Error(`Drop view failed: ${response.statusText}`); }
    } catch (error) { throw new Error(`Drop view failed: ${error}`); }
  }

  /**
   * Refresh a view's cached data
   */
  async refreshView(storageType: string, name: string): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.post(`/api/v1/view/${storageType}/${name}/refresh`);
      if (response.status === 200) { return response.data; }
      else { throw new Error(`Refresh view failed: ${response.statusText}`); }
    } catch (error) { throw new Error(`Refresh view failed: ${error}`); }
  }

  /**
   * Create a trigger on a table
   */
  async createTrigger(storageType: string, table: string, trigger: any): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.post(`/api/v1/trigger/${storageType}/${table}`, trigger);
      if (response.status === 200) { return response.data; }
      else { throw new Error(`Create trigger failed: ${response.statusText}`); }
    } catch (error) { throw new Error(`Create trigger failed: ${error}`); }
  }

  /**
   * Drop a trigger from a table
   */
  async dropTrigger(storageType: string, table: string, triggerName: string): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.delete(`/api/v1/trigger/${storageType}/${table}/${triggerName}`);
      if (response.status === 200) { return response.data; }
      else { throw new Error(`Drop trigger failed: ${response.statusText}`); }
    } catch (error) { throw new Error(`Drop trigger failed: ${error}`); }
  }

  /**
   * Get information schema tables listing
   */
  async infoSchemaTables(storageType: string): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.get(`/api/v1/info-schema/${storageType}/tables`);
      if (response.status === 200) { return response.data; }
      else { throw new Error(`Info schema tables failed: ${response.statusText}`); }
    } catch (error) { throw new Error(`Info schema tables failed: ${error}`); }
  }

  /**
   * Get information schema columns for a table
   */
  async infoSchemaColumns(storageType: string, table: string): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.get(`/api/v1/info-schema/${storageType}/${table}/columns`);
      if (response.status === 200) { return response.data; }
      else { throw new Error(`Info schema columns failed: ${response.statusText}`); }
    } catch (error) { throw new Error(`Info schema columns failed: ${error}`); }
  }

  /**
   * Get information schema constraints for a table
   */
  async infoSchemaConstraints(storageType: string, table: string): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.get(`/api/v1/info-schema/${storageType}/${table}/constraints`);
      if (response.status === 200) { return response.data; }
      else { throw new Error(`Info schema constraints failed: ${response.statusText}`); }
    } catch (error) { throw new Error(`Info schema constraints failed: ${error}`); }
  }

  private checkConnection(): void {
    if (!this.connected) {
      throw new Error('Not connected to PrimusDB server. Call connect() first.');
    }
  }

  // ==================== Authentication Methods ====================

  /**
   * Login with username and password
   *
   * @param username - User's username
   * @param password - User's password
   * @returns Login result with user info and token
   */
  async login(username: string, password: string): Promise<any> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post('/api/v1/auth/login', {
        username,
        password
      });

      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`Login failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Login failed: ${error}`);
    }
  }

  /**
   * Register a new user
   *
   * @param username - New user's username
   * @param password - New user's password
   * @param email - User's email (optional)
   * @param roles - User roles (default: ['readonly'])
   * @returns Registration result
   */
  async register(username: string, password: string, email?: string, roles?: string[]): Promise<any> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post('/api/v1/auth/register', {
        username,
        password,
        email,
        roles: roles || ['readonly']
      });

      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`Registration failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Registration failed: ${error}`);
    }
  }

  /**
   * Create an API token
   *
   * @param authorization - Login token or credentials
   * @param name - Token name
   * @param scopes - Token permissions
   * @param expiresInHours - Token expiration time
   * @returns Created token info
   */
  async createToken(authorization: string, name: string, scopes: any[], expiresInHours: number = 8760): Promise<any> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post('/api/v1/auth/token/create', {
        authorization,
        name,
        scopes,
        expires_in_hours: expiresInHours
      });

      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`Token creation failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Token creation failed: ${error}`);
    }
  }

  /**
   * Revoke an API token
   *
   * @param tokenId - Token ID to revoke
   */
  async revokeToken(tokenId: string): Promise<void> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post(`/api/v1/auth/token/revoke/${tokenId}`);
      if (response.status !== 200) {
        throw new Error(`Token revocation failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Token revocation failed: ${error}`);
    }
  }

  /**
   * List user's API tokens
   *
   * @returns List of user's tokens
   */
  async listTokens(): Promise<any[]> {
    this.checkConnection();

    try {
      const response = await this.httpClient.get('/api/v1/auth/tokens');
      if (response.status === 200) {
        return response.data.tokens || [];
      } else {
        throw new Error(`List tokens failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`List tokens failed: ${error}`);
    }
  }

  /**
   * List available roles
   *
   * @returns List of available roles
   */
  async listRoles(): Promise<any[]> {
    this.checkConnection();

    try {
      const response = await this.httpClient.get('/api/v1/auth/roles');
      if (response.status === 200) {
        return response.data.roles || [];
      } else {
        throw new Error(`List roles failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`List roles failed: ${error}`);
    }
  }

  /**
   * List all users (admin only)
   *
   * @returns List of users
   */
  async listUsers(): Promise<any[]> {
    this.checkConnection();

    try {
      const response = await this.httpClient.get('/api/v1/auth/users');
      if (response.status === 200) {
        return response.data.users || [];
      } else {
        throw new Error(`List users failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`List users failed: ${error}`);
    }
  }

  /**
   * Create a data segment for multi-tenancy
   *
   * @param name - Segment name
   * @param description - Segment description
   * @param parentSegment - Parent segment ID (optional)
   * @returns Created segment info
   */
  async createSegment(name: string, description: string, parentSegment?: string): Promise<any> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post('/api/v1/auth/segment/create', {
        name,
        description,
        parent_segment: parentSegment
      });

      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`Create segment failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Create segment failed: ${error}`);
    }
  }

  // ==================== Collection Encryption Methods ====================

  /**
   * Enable encryption for a document collection
   *
   * @param collection - Collection name
   * @returns Encryption status
   */
  async enableCollectionEncryption(collection: string): Promise<any> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post(`/api/v1/collection/${collection}/encrypt`);
      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`Enable encryption failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Enable encryption failed: ${error}`);
    }
  }

  /**
   * Disable encryption for a document collection
   *
   * @param collection - Collection name
   * @returns Encryption status
   */
  async disableCollectionEncryption(collection: string): Promise<any> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post(`/api/v1/collection/${collection}/decrypt`);
      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`Disable encryption failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Disable encryption failed: ${error}`);
    }
  }

  // ==================== Transaction Methods ====================

  /**
   * Begin a new transaction
   *
   * @param isolationLevel - Transaction isolation level
   * @returns Transaction ID
   */
  async beginTransaction(isolationLevel: string = 'read_committed'): Promise<string> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post('/api/v1/transaction/begin', {
        isolation_level: isolationLevel
      });

      if (response.status === 200) {
        return response.data.transaction_id;
      } else {
        throw new Error(`Begin transaction failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Begin transaction failed: ${error}`);
    }
  }

  /**
   * Commit a transaction
   *
   * @param transactionId - Transaction ID to commit
   */
  async commitTransaction(transactionId: string): Promise<void> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post(`/api/v1/transaction/${transactionId}/commit`);
      if (response.status !== 200) {
        throw new Error(`Commit transaction failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Commit transaction failed: ${error}`);
    }
  }

  /**
   * Rollback a transaction
   *
   * @param transactionId - Transaction ID to rollback
   */
  async rollbackTransaction(transactionId: string): Promise<void> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post(`/api/v1/transaction/${transactionId}/rollback`);
      if (response.status !== 200) {
        throw new Error(`Rollback transaction failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Rollback transaction failed: ${error}`);
    }
  }

  // ==================== Key-Value (CouchDB-compatible) Methods ====================

  /**
   * Get database information
   *
   * @param db - Database name
   * @returns Database info
   */
  async kvGetDbInfo(db: string): Promise<any> {
    this.checkConnection();

    try {
      const response = await this.httpClient.get(`/api/v1/kv/${db}`);
      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`Get DB info failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Get DB info failed: ${error}`);
    }
  }

  /**
   * Create a Key-Value database
   *
   * @param db - Database name
   */
  async kvCreateDatabase(db: string): Promise<void> {
    this.checkConnection();

    try {
      const response = await this.httpClient.put(`/api/v1/kv/${db}`);
      if (response.status !== 200) {
        throw new Error(`Create database failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Create database failed: ${error}`);
    }
  }

  /**
   * Delete a Key-Value database
   *
   * @param db - Database name
   */
  async kvDeleteDatabase(db: string): Promise<void> {
    this.checkConnection();

    try {
      const response = await this.httpClient.delete(`/api/v1/kv/${db}`);
      if (response.status !== 200) {
        throw new Error(`Delete database failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Delete database failed: ${error}`);
    }
  }

  /**
   * Get all documents from a Key-Value database
   *
   * @param db - Database name
   * @param includeDocs - Include document content
   * @param limit - Maximum results
   * @param skip - Number of results to skip
   * @returns All documents
   */
  async kvAllDocs(db: string, includeDocs: boolean = false, limit?: number, skip?: number): Promise<any> {
    this.checkConnection();

    try {
      const params: any = {};
      if (includeDocs) params.include_docs = 'true';
      if (limit) params.limit = limit.toString();
      if (skip) params.skip = skip.toString();

      const response = await this.httpClient.get(`/api/v1/kv/${db}/_all_docs`, { params });
      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`Get all docs failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Get all docs failed: ${error}`);
    }
  }

  /**
   * Find documents using Mango query syntax
   *
   * @param db - Database name
   * @param selector - MongoDB-style selector
   * @param limit - Maximum results
   * @param skip - Number of results to skip
   * @returns Matching documents
   */
  async kvFind(db: string, selector: any, limit?: number, skip?: number): Promise<any> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post(`/api/v1/kv/${db}/_find`, {
        selector,
        limit,
        skip
      });
      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`Find failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Find failed: ${error}`);
    }
  }

  /**
   * Get a document by ID
   *
   * @param db - Database name
   * @param docId - Document ID
   * @returns Document
   */
  async kvGetDocument(db: string, docId: string): Promise<any> {
    this.checkConnection();

    try {
      const response = await this.httpClient.get(`/api/v1/kv/${db}/${docId}`);
      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`Get document failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Get document failed: ${error}`);
    }
  }

  /**
   * Create or update a document
   *
   * @param db - Database name
   * @param docId - Document ID
   * @param data - Document data
   * @returns Result with revision
   */
  async kvPutDocument(db: string, docId: string, data: any): Promise<any> {
    this.checkConnection();

    try {
      const response = await this.httpClient.put(`/api/v1/kv/${db}/${docId}`, data);
      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`Put document failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Put document failed: ${error}`);
    }
  }

  /**
   * Delete a document
   *
   * @param db - Database name
   * @param docId - Document ID
   * @param rev - Revision to delete
   */
  async kvDeleteDocument(db: string, docId: string, rev: string): Promise<any> {
    this.checkConnection();

    try {
      const response = await this.httpClient.delete(`/api/v1/kv/${db}/${docId}?rev=${rev}`);
      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`Delete document failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Delete document failed: ${error}`);
    }
  }

  /**
   * Bulk document operations
   *
   * @param db - Database name
   * @param docs - Array of documents
   * @param allOrNothing - All or nothing mode
   * @returns Bulk operation results
   */
  async kvBulkDocs(db: string, docs: any[], allOrNothing: boolean = false): Promise<any> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post(`/api/v1/kv/${db}/_bulk_docs`, {
        docs,
        all_or_nothing: allOrNothing
      });
      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`Bulk docs failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Bulk docs failed: ${error}`);
    }
  }

  /**
   * Create an index
   *
   * @param db - Database name
   * @param name - Index name
   * @param fields - Fields to index
   * @returns Index creation result
   */
  async kvCreateIndex(db: string, name: string, fields: string[]): Promise<any> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post(`/api/v1/kv/${db}/_index`, {
        index: { fields },
        name
      });
      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`Create index failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Create index failed: ${error}`);
    }
  }

  /**
   * Compact a database
   *
   * @param db - Database name
   */
  async kvCompact(db: string): Promise<void> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post(`/api/v1/kv/${db}/_compact`);
      if (response.status !== 200) {
        throw new Error(`Compact failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Compact failed: ${error}`);
    }
  }

  // ==================== UQL (Unified Query Language) Methods ====================

  /**
   * Execute a UQL query across multiple storage engines
   *
   * @param query - UQL query string
   * @param language - Query language: 'sql', 'mongodb', 'mango', 'uql' (default: 'sql')
   * @param params - Optional query parameters
   * @returns Query results
   */
  async executeUql(
    query: string,
    language: string = 'sql',
    params?: Record<string, any>
  ): Promise<any> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post('/api/v1/uql', {
        query,
        language,
        params
      });

      if (response.status === 200) {
        return response.data;
      } else {
        throw new Error(`UQL query failed: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`UQL query failed: ${error}`);
    }
  }

  /**
   * Execute a SQL query using UQL
   *
   * @param sql - SQL query string
   * @param params - Optional query parameters
   * @returns Query results
   */
  async executeSql(sql: string, params?: Record<string, any>): Promise<any> {
    return this.executeUql(sql, 'sql', params);
  }

  /**
   * Execute a MongoDB-style query using UQL
   *
   * @param query - MongoDB-style query
   * @param params - Optional query parameters
   * @returns Query results
   */
  async executeMongoDb(query: any, params?: Record<string, any>): Promise<any> {
    return this.executeUql(JSON.stringify(query), 'mongodb', params);
  }

  /**
   * Execute a Mango query (CouchDB-style) using UQL
   *
   * @param selector - Mango selector
   * @param params - Optional query parameters
   * @returns Query results
   */
  async executeMango(selector: any, params?: Record<string, any>): Promise<any> {
    return this.executeUql(JSON.stringify(selector), 'mango', params);
  }

  // ==================== ER Model Features (v1.2.2+) ====================

  /**
   * Insert data and return specified columns
   *
   * @param storageType - Storage type
   * @param table - Table/collection name
   * @param data - Data to insert
   * @param returning - Columns to return
   * @returns Array of returned records
   */
  async insertReturning(storageType: string, table: string, data: Record<string, any>, returning: string[]): Promise<any[]> {
    const cols = Object.keys(data).join(', ');
    const vals = Object.values(data).map(v => typeof v === 'string' ? `'${v}'` : v).join(', ');
    const ret = returning.join(', ');
    const sql = `INSERT INTO ${table} (${cols}) VALUES (${vals}) RETURNING ${ret}`;
    const result = await this.executeSql(sql);
    return result?.records || [];
  }

  /**
   * Update data and return specified columns
   *
   * @param storageType - Storage type
   * @param table - Table/collection name
   * @param conditions - Update conditions
   * @param data - New data
   * @param returning - Columns to return
   * @returns Array of returned records
   */
  async updateReturning(storageType: string, table: string, conditions: Record<string, any>, data: Record<string, any>, returning: string[]): Promise<any[]> {
    const setClause = Object.entries(data).map(([k, v]) => typeof v === 'string' ? `${k} = '${v}'` : `${k} = ${v}`).join(', ');
    const conds = Object.entries(conditions).map(([k, v]) => typeof v === 'string' ? `${k} = '${v}'` : `${k} = ${v}`).join(' AND ');
    const ret = returning.join(', ');
    const sql = `UPDATE ${table} SET ${setClause} WHERE ${conds} RETURNING ${ret}`;
    const result = await this.executeSql(sql);
    return result?.records || [];
  }

  /**
   * Delete data and return specified columns
   *
   * @param storageType - Storage type
   * @param table - Table/collection name
   * @param conditions - Delete conditions
   * @param returning - Columns to return
   * @returns Array of returned records
   */
  async deleteReturning(storageType: string, table: string, conditions: Record<string, any>, returning: string[]): Promise<any[]> {
    const conds = Object.entries(conditions).map(([k, v]) => typeof v === 'string' ? `${k} = '${v}'` : `${k} = ${v}`).join(' AND ');
    const ret = returning.join(', ');
    const sql = `DELETE FROM ${table} WHERE ${conds} RETURNING ${ret}`;
    const result = await this.executeSql(sql);
    return result?.records || [];
  }

  /**
   * Select data with GROUP BY, HAVING, DISTINCT, ORDER BY support
   *
   * @param storageType - Storage type
   * @param table - Table/collection name
   * @param columns - Columns to select (default: *)
   * @param conditions - WHERE conditions
   * @param groupBy - GROUP BY columns
   * @param having - HAVING conditions
   * @param distinct - Whether to use DISTINCT
   * @param orderBy - ORDER BY columns
   * @param limit - Maximum results
   * @param offset - Results offset
   * @returns Array of matching records
   */
  async selectGrouped(
    storageType: string, table: string,
    columns?: string[], conditions?: Record<string, any>,
    groupBy?: string[], having?: Record<string, any>,
    distinct?: boolean, orderBy?: string[],
    limit?: number, offset?: number
  ): Promise<any[]> {
    let sql = 'SELECT ';
    if (distinct) sql += 'DISTINCT ';
    sql += (columns ? columns.join(', ') : '*') + ` FROM ${table}`;
    if (conditions) {
      const conds = Object.entries(conditions).map(([k, v]) => typeof v === 'string' ? `${k} = '${v}'` : `${k} = ${v}`).join(' AND ');
      sql += ` WHERE ${conds}`;
    }
    if (groupBy) sql += ` GROUP BY ${groupBy.join(', ')}`;
    if (having) {
      const havingConds = Object.entries(having).map(([k, v]) => typeof v === 'string' ? `${k} = '${v}'` : `${k} = ${v}`).join(' AND ');
      sql += ` HAVING ${havingConds}`;
    }
    if (orderBy) sql += ` ORDER BY ${orderBy.join(', ')}`;
    if (limit) sql += ` LIMIT ${limit}`;
    if (offset) sql += ` OFFSET ${offset}`;
    const result = await this.executeSql(sql);
    return result?.records || [];
  }

  /**
   * Truncate a table with cascade support
   *
   * @param storageType - Storage type
   * @param table - Table/collection name
   * @param cascade - If true, also truncate dependent tables
   * @returns Result
   */
  async truncateTableCascade(storageType: string, table: string, cascade: boolean = true): Promise<any> {
    return this.truncateTable(storageType, table, cascade);
  }

  /**
   * Add a foreign key constraint to a relational table
   *
   * @param storageType - Storage type
   * @param table - Table/collection name
   * @param name - Constraint name
   * @param column - Local column name
   * @param referencesTable - Referenced table name
   * @param referencesColumn - Referenced column name
   * @param onDelete - ON DELETE action (Restrict, Cascade, SetNull, SetDefault, NoAction)
   * @param onUpdate - ON UPDATE action
   * @returns Result
   */
  async addForeignKey(
    storageType: string, table: string,
    name: string, column: string,
    referencesTable: string, referencesColumn: string,
    onDelete: string = 'Restrict', onUpdate: string = 'Restrict'
  ): Promise<any> {
    return this.addConstraint(storageType, table, {
      name,
      constraint_type: 'ForeignKey',
      fields: [column],
      definition: {
        references_table: referencesTable,
        references_field: referencesColumn,
        on_delete: onDelete,
        on_update: onUpdate
      }
    });
  }

  /**
   * Drop a foreign key constraint
   *
   * @param storageType - Storage type
   * @param table - Table/collection name
   * @param constraintName - Constraint name to drop
   * @returns Result
   */
  async dropForeignKey(storageType: string, table: string, constraintName: string): Promise<any> {
    return this.dropConstraint(storageType, table, constraintName);
  }

  // ==================== Cluster Gateway Methods ====================

  /**
   * Get cluster status
   */
  async clusterStatus(): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.get('/api/v1/cluster/status');
      return response.data;
    } catch (error) {
      throw new Error(`Cluster status failed: ${error}`);
    }
  }

  /**
   * List cluster nodes with health and latency
   */
  async clusterNodes(): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.get('/api/v1/cluster/nodes');
      return response.data;
    } catch (error) {
      throw new Error(`Cluster nodes failed: ${error}`);
    }
  }

  /**
   * Get route decision for a shard key
   *
   * @param shardKey - Shard key for routing (optional)
   * @param preferredNodes - Preferred node list (optional)
   */
  async routeRequest(shardKey?: string, preferredNodes?: string[]): Promise<any> {
    this.checkConnection();
    try {
      const body: Record<string, any> = {};
      if (shardKey !== undefined) body.shard_key = shardKey;
      if (preferredNodes !== undefined) body.preferred_nodes = preferredNodes;
      const response = await this.httpClient.post('/api/v1/cluster/route', body);
      return response.data;
    } catch (error) {
      throw new Error(`Route request failed: ${error}`);
    }
  }

  /**
   * Get gateway metrics
   */
  async clusterMetrics(): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.get('/api/v1/cluster/metrics');
      return response.data;
    } catch (error) {
      throw new Error(`Cluster metrics failed: ${error}`);
    }
  }

  // ==================== Federation Methods ====================

  /**
   * Get federation status
   */
  async federationStatus(): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.get('/api/v1/federation/status');
      return response.data;
    } catch (error) {
      throw new Error(`Federation status failed: ${error}`);
    }
  }

  /**
   * List federated clusters
   */
  async federationClusters(): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.get('/api/v1/federation/clusters');
      return response.data;
    } catch (error) {
      throw new Error(`Federation clusters failed: ${error}`);
    }
  }

  /**
   * List federated data domains
   */
  async federationDomains(): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.get('/api/v1/federation/domains');
      return response.data;
    } catch (error) {
      throw new Error(`Federation domains failed: ${error}`);
    }
  }

  /**
   * Create a new data domain
   *
   * @param name - Domain name
   * @param description - Domain description (optional)
   * @param replicationMode - Replication mode (optional)
   * @param storageTypes - Allowed storage types (optional)
   * @param collections - Collections schema (optional)
   * @param tables - Tables schema (optional)
   * @param memberClusters - Member cluster IDs (optional)
   */
  async createDataDomain(
    name: string,
    description?: string,
    replicationMode?: string,
    storageTypes?: string[],
    collections?: Record<string, any>[],
    tables?: Record<string, any>[],
    memberClusters?: string[]
  ): Promise<any> {
    this.checkConnection();
    try {
      const body: Record<string, any> = { name };
      if (description !== undefined) body.description = description;
      if (replicationMode !== undefined) body.replication_mode = replicationMode;
      if (storageTypes !== undefined) body.storage_types = storageTypes;
      if (collections !== undefined) body.collections = collections;
      if (tables !== undefined) body.tables = tables;
      if (memberClusters !== undefined) body.member_clusters = memberClusters;
      const response = await this.httpClient.post('/api/v1/federation/domains', body);
      return response.data;
    } catch (error) {
      throw new Error(`Create data domain failed: ${error}`);
    }
  }

  /**
   * Join an existing data domain
   *
   * @param name - Domain name
   * @param collections - Collections schema for this member (optional)
   * @param storageTypes - Storage types for this member (optional)
   * @param replicationMode - Replication mode for this member (optional)
   */
  async joinDomain(
    name: string,
    collections?: Record<string, any>[],
    storageTypes?: string[],
    replicationMode?: string
  ): Promise<any> {
    this.checkConnection();
    try {
      const body: Record<string, any> = {};
      if (collections !== undefined) body.collections = collections;
      if (storageTypes !== undefined) body.storage_types = storageTypes;
      if (replicationMode !== undefined) body.replication_mode = replicationMode;
      const response = await this.httpClient.post(`/api/v1/federation/domains/${name}/join`, body);
      return response.data;
    } catch (error) {
      throw new Error(`Join domain failed: ${error}`);
    }
  }

  /**
   * Leave a data domain
   *
   * @param name - Domain name
   */
  async leaveDomain(name: string): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.post(`/api/v1/federation/domains/${name}/leave`);
      return response.data;
    } catch (error) {
      throw new Error(`Leave domain failed: ${error}`);
    }
  }

  /**
   * Balance a data domain across member clusters
   *
   * @param name - Domain name
   */
  async balanceDomain(name: string): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.post(`/api/v1/federation/domains/${name}/balance`);
      return response.data;
    } catch (error) {
      throw new Error(`Balance domain failed: ${error}`);
    }
  }

  async governorStartExecution(
    namespace: string,
    workloadType: string,
    user?: string,
    role?: string
  ): Promise<GovernorStartResult> {
    this.checkConnection();
    const payload: Record<string, unknown> = { namespace, workload_type: workloadType };
    if (user !== undefined) payload.user = user;
    if (role !== undefined) payload.role = role;
    const response = await this.httpClient.post('/api/v1/governor/executions/start', payload);
    if (response.data.success) return response.data.data;
    throw new Error(response.data.error || 'Governor start failed');
  }

  async governorFinishExecution(executionId: string): Promise<void> {
    this.checkConnection();
    const response = await this.httpClient.post(`/api/v1/governor/executions/${executionId}/finish`, {});
    if (!response.data.success) throw new Error(response.data.error || 'Governor finish failed');
  }

  async governorCheckLimit(executionId: string, checkType: string, value: number): Promise<GovernorCheckResult> {
    this.checkConnection();
    const response = await this.httpClient.post(`/api/v1/governor/executions/${executionId}/check`, {
      check_type: checkType,
      value,
    });
    if (response.data.success) return response.data.data;
    throw new Error(response.data.error || 'Governor check failed');
  }

  async governorStatus(): Promise<GovernorStatus> {
    this.checkConnection();
    const response = await this.httpClient.get('/api/v1/governor/status');
    if (response.data.success) return response.data.data;
    throw new Error(response.data.error || 'Governor status failed');
  }

  async governorMetrics(): Promise<GovernorMetrics> {
    this.checkConnection();
    const response = await this.httpClient.get('/api/v1/governor/metrics');
    if (response.data.success) return response.data.data;
    throw new Error(response.data.error || 'Governor metrics failed');
  }

  async governorListExecutions(): Promise<GovernorExecution[]> {
    this.checkConnection();
    const response = await this.httpClient.get('/api/v1/governor/executions');
    if (response.data.success) return response.data.data;
    throw new Error(response.data.error || 'Governor list executions failed');
  }

  async governorListViolations(): Promise<GovernorViolation[]> {
    this.checkConnection();
    const response = await this.httpClient.get('/api/v1/governor/violations');
    if (response.data.success) return response.data.data;
    throw new Error(response.data.error || 'Governor list violations failed');
  }

  async governorPolicies(): Promise<GovernorPolicy[]> {
    this.checkConnection();
    const response = await this.httpClient.get('/api/v1/governor/policies');
    if (response.data.success) return response.data.data;
    throw new Error(response.data.error || 'Governor policies failed');
  }

  async governorUpdatePolicy(name: string, limits: Record<string, unknown>, action: string, scope: string): Promise<void> {
    this.checkConnection();
    const response = await this.httpClient.post('/api/v1/governor/policies/update', { name, limits, action, scope });
    if (!response.data.success) throw new Error(response.data.error || 'Governor update policy failed');
  }

  // ==================== TimeSeries Methods ====================

  /**
   * List all time-series metrics
   */
  async tsListMetrics(): Promise<any[]> {
    this.checkConnection();
    const response = await this.httpClient.get('/api/v1/timeseries/metrics');
    return response.data;
  }

  /**
   * Describe a time-series metric
   * @param metric - Metric name
   */
  async tsDescribeMetric(metric: string): Promise<any> {
    this.checkConnection();
    const response = await this.httpClient.get(`/api/v1/timeseries/metrics/${metric}`);
    return response.data;
  }

  /**
   * Query time-series data points
   * @param metric - Metric name
   * @param params - Query parameters (start_time, end_time, resolution, tags, fields, limit)
   */
  async tsQuery(metric: string, params?: Record<string, any>): Promise<any[]> {
    this.checkConnection();
    const queryParams = new URLSearchParams();
    if (params) {
      for (const [key, val] of Object.entries(params)) {
        const v = typeof val === 'object' ? JSON.stringify(val) : String(val);
        queryParams.append(key, v);
      }
    }
    const qs = queryParams.toString();
    const response = await this.httpClient.get(`/api/v1/timeseries/${metric}/query${qs ? '?' + qs : ''}`);
    return response.data;
  }

  /**
   * Insert a single time-series data point
   * @param metric - Metric name
   * @param timestamp - Unix timestamp in milliseconds
   * @param fields - Field values (e.g. {cpu: 50.5})
   * @param tags - Optional tags (e.g. {host: "web1"})
   */
  async tsInsertPoint(metric: string, timestamp: number, fields: Record<string, number>, tags?: Record<string, string>): Promise<any> {
    this.checkConnection();
    const payload: Record<string, any> = { metric, timestamp, fields };
    if (tags) payload.tags = tags;
    const response = await this.httpClient.post('/api/v1/timeseries/insert', payload);
    return response.data;
  }

  /**
   * Aggregate time-series data
   * @param metric - Metric name
   * @param fn - Aggregation function (avg, min, max, sum, count, stddev, median, p50/p90/p95/p99, rate, delta, first, last)
   * @param params - Aggregation parameters (resolution, start_time, end_time, tags, fill_policy)
   */
  async tsAggregate(metric: string, fn: string, params?: Record<string, any>): Promise<any[]> {
    this.checkConnection();
    const payload: Record<string, any> = { metric, fn };
    if (params) {
      if (params.resolution) payload.resolution = params.resolution;
      if (params.start_time != null) payload.start_time = params.start_time;
      if (params.end_time != null) payload.end_time = params.end_time;
      if (params.tags) payload.tags = params.tags;
      if (params.fill_policy) payload.fill_policy = params.fill_policy;
    }
    const response = await this.httpClient.post(`/api/v1/timeseries/${metric}/aggregate`, payload);
    return response.data;
  }

  /**
   * Downsample time-series data to a lower resolution
   * @param metric - Metric name
   * @param targetResolution - Target resolution (1m, 5m, 15m, 1h, 1d)
   * @param sourceResolution - Source resolution (default: raw)
   */
  async tsDownsample(metric: string, targetResolution: string, sourceResolution: string = 'raw'): Promise<any> {
    this.checkConnection();
    const response = await this.httpClient.post(`/api/v1/timeseries/${metric}/downsample`, {
      metric,
      target_resolution: targetResolution,
      source_resolution: sourceResolution,
    });
    return response.data;
  }

  /**
   * Set retention policy for a time-series metric
   * @param metric - Metric name
   * @param retentionDays - Retention period in days
   */
  async tsSetRetention(metric: string, retentionDays: number): Promise<any> {
    this.checkConnection();
    const response = await this.httpClient.post(`/api/v1/timeseries/${metric}/retain`, {
      metric,
      retention_days: retentionDays,
    });
    return response.data;
  }

  /**
   * Add a rollup resolution to a time-series metric
   * @param metric - Metric name
   * @param resolution - Resolution string (1m, 5m, 15m, 1h, 1d)
   * @param retentionDays - Retention in days (0 = unlimited)
   * @param aggregationFn - Aggregation function (default: avg)
   */
  async tsAddResolution(metric: string, resolution: string, retentionDays: number = 0, aggregationFn: string = 'avg'): Promise<any> {
    this.checkConnection();
    const response = await this.httpClient.post(`/api/v1/timeseries/${metric}/resolution`, {
      metric,
      resolution,
      retention_days: retentionDays,
      aggregation_fn: aggregationFn,
    });
    return response.data;
  }

  /**
   * Get time-series engine statistics
   */
  async tsStats(): Promise<any> {
    this.checkConnection();
    const response = await this.httpClient.get('/api/v1/timeseries/stats');
    return response.data;
  }

  /**
   * Get federation metrics
   */
  async federationMetrics(): Promise<any> {
    this.checkConnection();
    try {
      const response = await this.httpClient.get('/api/v1/federation/metrics');
      return response.data;
    } catch (error) {
      throw new Error(`Federation metrics failed: ${error}`);
    }
  }
}

// Export default
export default PrimusDB;