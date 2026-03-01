/**
 * PrimusDB Node.js Driver
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: MIT - See LICENSE file for details
 * Version: 1.2.0-alpha - Added: Auth, Token, Encryption, Transaction methods
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
   */
  async truncateTable(storageType: string, table: string): Promise<void> {
    this.checkConnection();

    try {
      const response = await this.httpClient.post(`/api/v1/crud/${storageType}/${table}/truncate`);
      if (response.status !== 200) {
        throw new Error(`Failed to truncate table: ${response.statusText}`);
      }
    } catch (error) {
      throw new Error(`Truncate table failed: ${error}`);
    }
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
}

// Export default
export default PrimusDB;