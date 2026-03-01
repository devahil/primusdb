# frozen_string_literal: true

# PrimusDB Ruby Driver
# Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
# License: MIT - See LICENSE file for details
# Version: 1.2.0-alpha - Full CRUD + AI/ML + Key-Value support

require 'faraday'
require 'json'
require 'concurrent'

module PrimusDB
  # PrimusDB Ruby Client
  class Client
    # Storage engine types
    module StorageType
      COLUMNAR = 'columnar'
      VECTOR = 'vector'
      DOCUMENT = 'document'
      RELATIONAL = 'relational'
      KEYVALUE = 'keyvalue'
    end

    # Connection configuration
    class Config
      attr_accessor :host, :port, :timeout, :max_connections

      def initialize(host: 'localhost', port: 8080, timeout: 30, max_connections: 10)
        @host = host
        @port = port
        @timeout = timeout
        @max_connections = max_connections
      end
    end

    attr_reader :config

    # Initialize client
    #
    # @param config [Config] Connection configuration
    def initialize(config = nil)
      @config = config || Config.new
      @connected = false
      @http_client = nil
    end

    # Connect to PrimusDB server
    #
    # @return [void]
    def connect
      @http_client = Faraday.new(url: base_url) do |faraday|
        faraday.request :json
        faraday.response :json
        faraday.adapter Faraday.default_adapter
        faraday.options.timeout = @config.timeout
      end
      @connected = true
    end

    # Close connection
    #
    # @return [void]
    def close
      @http_client = nil
      @connected = false
    end

    # Check if connected
    #
    # @return [Boolean]
    def connected?
      @connected
    end

    # Create table/collection
    #
    # @param storage_type [String] Storage engine type
    # @param table [String] Table/collection name
    # @param schema [Hash] Schema definition
    # @return [void]
    def create_table(storage_type, table, schema)
      check_connection
      response = @http_client.post("table/#{storage_type}/#{table}") do |req|
        req.body = { schema: schema }
      end
      handle_response(response)
    end

    # Insert data
    #
    # @param storage_type [String] Storage engine type
    # @param table [String] Table/collection name
    # @param data [Hash] Data to insert
    # @return [Integer] Number of records inserted
    def insert(storage_type, table, data)
      check_connection
      response = @http_client.post("crud/#{storage_type}/#{table}") do |req|
        req.body = { data: data }
      end
      result = handle_response(response)
      result['count'] || 0
    end

    # Select data
    #
    # @param storage_type [String] Storage engine type
    # @param table [String] Table/collection name
    # @param conditions [Hash, nil] Query conditions
    # @param limit [Integer, nil] Maximum results
    # @param offset [Integer, nil] Results offset
    # @return [Array<Hash>] Query results
    def select(storage_type, table, conditions: nil, limit: nil, offset: nil)
      check_connection

      params = {}
      params[:conditions] = conditions.to_json if conditions
      params[:limit] = limit if limit
      params[:offset] = offset if offset

      query_string = URI.encode_www_form(params) unless params.empty?

      endpoint = "crud/#{storage_type}/#{table}"
      endpoint += "?#{query_string}" if query_string

      response = @http_client.get(endpoint)
      handle_response(response)
    end

    # Update data
    #
    # @param storage_type [String] Storage engine type
    # @param table [String] Table/collection name
    # @param conditions [Hash, nil] Update conditions
    # @param data [Hash] New data
    # @return [Integer] Number of records updated
    def update(storage_type, table, conditions, data)
      check_connection
      response = @http_client.put("crud/#{storage_type}/#{table}") do |req|
        req.body = { conditions: conditions, data: data }
      end
      result = handle_response(response)
      result['count'] || 0
    end

    # Delete data
    #
    # @param storage_type [String] Storage engine type
    # @param table [String] Table/collection name
    # @param conditions [Hash, nil] Delete conditions
    # @return [Integer] Number of records deleted
    def delete(storage_type, table, conditions = nil)
      check_connection

      params = {}
      params[:conditions] = conditions.to_json if conditions

      query_string = URI.encode_www_form(params) unless params.empty?

      endpoint = "crud/#{storage_type}/#{table}"
      endpoint += "?#{query_string}" if query_string

      response = @http_client.delete(endpoint)
      result = handle_response(response)
      result['count'] || 0
    end

    # Analyze data
    #
    # @param storage_type [String] Storage engine type
    # @param table [String] Table/collection name
    # @param conditions [Hash, nil] Analysis conditions
    # @return [Hash] Analysis results
    def analyze(storage_type, table, conditions = nil)
      check_connection
      response = @http_client.post("advanced/analyze/#{storage_type}/#{table}") do |req|
        req.body = { conditions: conditions } if conditions
      end
      handle_response(response)
    end

    # Make AI predictions
    #
    # @param storage_type [String] Storage engine type
    # @param table [String] Table/collection name
    # @param data [Hash] Input data
    # @param prediction_type [String] Prediction algorithm
    # @return [Hash] Prediction results
    def predict(storage_type, table, data, prediction_type = 'linear_regression')
      check_connection
      response = @http_client.post("advanced/predict/#{storage_type}/#{table}") do |req|
        req.body = {
          data: data,
          prediction_type: prediction_type
        }
      end
      handle_response(response)
    end

    # Vector similarity search
    #
    # @param table [String] Vector table name
    # @param query_vector [Array<Float>] Query vector
    # @param limit [Integer] Maximum results
    # @return [Array<Hash>] Similar vectors
    def vector_search(table, query_vector, limit = 10)
      check_connection
      response = @http_client.post("advanced/vector-search/#{table}") do |req|
        req.body = {
          query_vector: query_vector,
          limit: limit
        }
      end
      handle_response(response)
    end

    # Data clustering
    #
    # @param storage_type [String] Storage engine type
    # @param table [String] Table name
    # @param params [Hash, nil] Clustering parameters
    # @return [Hash] Clustering results
    def cluster(storage_type, table, params = nil)
      check_connection
      response = @http_client.post("advanced/cluster/#{storage_type}/#{table}") do |req|
        req.body = params || { algorithm: 'kmeans', clusters: 5 }
      end
      handle_response(response)
    end

    private

    def base_url
      "http://#{@config.host}:#{@config.port}/api/v1"
    end

    def check_connection
      raise 'Not connected to PrimusDB server' unless connected?
    end

    def handle_response(response)
      raise "PrimusDB API error: #{response.status} - #{response.body}" unless response.success?

      body = response.body
      return body['data'] if body.is_a?(Hash) && body.key?('data')

      body
    end
  end

  # Collection abstraction for easier data operations
  class Collection
    attr_reader :client, :storage_type, :name

    def initialize(client, storage_type, name)
      @client = client
      @storage_type = storage_type
      @name = name
    end

    def insert_one(data)
      @client.insert(@storage_type, @name, data)
    end

    def find(conditions = nil, limit: nil, offset: nil)
      @client.select(@storage_type, @name, conditions: conditions, limit: limit, offset: offset)
    end

    def update_one(conditions, data)
      @client.update(@storage_type, @name, conditions, data)
    end

    def delete_one(conditions)
      @client.delete(@storage_type, @name, conditions)
    end

    def count(conditions = nil)
      results = find(conditions, limit: 1_000_000)
      results.size
    end
  end

  # Rails ActiveRecord-style adapter
  module Rails
    class Adapter
      attr_reader :config

      def initialize(config)
        @config = config
        @client = nil
      end

      def establish_connection
        @client = Client.new(@config)
        @client.connect
      end

      def execute(sql)
        # Convert Rails SQL-like queries to PrimusDB operations
        # This is a simplified implementation
        case sql
        when /^SELECT/i
          # Parse SELECT query and convert to PrimusDB format
          parse_select_query(sql)
        when /^INSERT/i
          # Parse INSERT query and convert to PrimusDB format
          parse_insert_query(sql)
        when /^UPDATE/i
          # Parse UPDATE query and convert to PrimusDB format
          parse_update_query(sql)
        when /^DELETE/i
          # Parse DELETE query and convert to PrimusDB format
          parse_delete_query(sql)
        else
          raise "Unsupported SQL operation: #{sql}"
        end
      end

      private

      def parse_select_query(sql)
        # Simplified SQL parsing - in real implementation would use proper parser
        raise "Invalid SELECT query: #{sql}" unless sql =~ /FROM\s+(\w+)/i

        table = ::Regexp.last_match(1)
        # Determine storage type from table name or configuration
        storage_type = infer_storage_type(table)
        @client.select(storage_type, table)
      end

      def parse_insert_query(sql)
        # Simplified INSERT parsing
        raise "Invalid INSERT query: #{sql}" unless sql =~ /INTO\s+(\w+)/i

        table = ::Regexp.last_match(1)
        storage_type = infer_storage_type(table)
        # Extract values - simplified implementation
        values = extract_values_from_sql(sql)
        @client.insert(storage_type, table, values)
      end

      def parse_update_query(sql)
        # Simplified UPDATE parsing
        raise "Invalid UPDATE query: #{sql}" unless sql =~ /UPDATE\s+(\w+)/i

        table = ::Regexp.last_match(1)
        storage_type = infer_storage_type(table)
        # Extract SET and WHERE clauses - simplified
        set_clause, where_clause = extract_update_clauses(sql)
        @client.update(storage_type, table, where_clause, set_clause)
      end

      def parse_delete_query(sql)
        # Simplified DELETE parsing
        raise "Invalid DELETE query: #{sql}" unless sql =~ /FROM\s+(\w+)/i

        table = ::Regexp.last_match(1)
        storage_type = infer_storage_type(table)
        where_clause = extract_where_clause(sql)
        @client.delete(storage_type, table, where_clause)
      end

      def infer_storage_type(table)
        # Simple heuristic - in real implementation would use configuration
        case table
        when /vector/i
          StorageType::VECTOR
        when /document/i
          StorageType::DOCUMENT
        when /column/i
          StorageType::COLUMNAR
        else
          StorageType::RELATIONAL
        end
      end

      def extract_values_from_sql(_sql)
        # Simplified value extraction - real implementation would parse properly
        {}
      end

      def extract_update_clauses(_sql)
        # Simplified clause extraction
        [{}, {}]
      end

      def extract_where_clause(_sql)
        # Simplified WHERE clause extraction
        {}
      end
    end
  end

  # Key-Value Database operations (CouchDB-compatible)
  module KeyValue
    # Get database information
    def kv_get_db_info(database)
      check_connection
      response = @http_client.get("kv/#{database}")
      handle_response(response)
    end

    # Create a Key-Value database
    def kv_create_database(database)
      check_connection
      response = @http_client.put("kv/#{database}") do |req|
        req.body = {}.to_json
      end
      handle_response(response)
    end

    # Delete a Key-Value database
    def kv_delete_database(database)
      check_connection
      response = @http_client.delete("kv/#{database}")
      handle_response(response)
    end

    # Get all documents
    def kv_all_docs(database, include_docs: false, limit: nil, skip: nil)
      check_connection
      params = { include_docs: include_docs }
      params[:limit] = limit if limit
      params[:skip] = skip if skip

      query_string = URI.encode_www_form(params) unless params.empty?
      endpoint = "kv/#{database}/_all_docs"
      endpoint += "?#{query_string}" if query_string

      response = @http_client.get(endpoint)
      handle_response(response)
    end

    # Get a document by ID
    def kv_get_document(database, doc_id)
      check_connection
      response = @http_client.get("kv/#{database}/#{doc_id}")
      handle_response(response)
    end

    # Create or update a document
    def kv_put_document(database, doc_id, data)
      check_connection
      response = @http_client.put("kv/#{database}/#{doc_id}") do |req|
        req.body = data.to_json
      end
      handle_response(response)
    end

    # Delete a document
    def kv_delete_document(database, doc_id, rev)
      check_connection
      response = @http_client.delete("kv/#{database}/#{doc_id}?rev=#{rev}")
      handle_response(response)
    end

    # Bulk document operations
    def kv_bulk_docs(database, docs, all_or_nothing: false)
      check_connection
      response = @http_client.post("kv/#{database}/_bulk_docs") do |req|
        req.body = { docs: docs, all_or_nothing: all_or_nothing }.to_json
      end
      handle_response(response)
    end

    # Find documents using Mango query
    def kv_find(database, selector, limit: nil, skip: nil)
      check_connection
      body = { selector: selector }
      body[:limit] = limit if limit
      body[:skip] = skip if skip

      response = @http_client.post("kv/#{database}/_find") do |req|
        req.body = body.to_json
      end
      handle_response(response)
    end

    # Create an index
    def kv_create_index(database, name, fields)
      check_connection
      response = @http_client.post("kv/#{database}/_index") do |req|
        req.body = { index: { fields: fields }, name: name }.to_json
      end
      handle_response(response)
    end

    # Compact database
    def kv_compact(database)
      check_connection
      response = @http_client.post("kv/#{database}/_compact")
      handle_response(response)
    end

    private

    def kv_base_url
      "http://#{@config.host}:#{@config.port}/api/v1"
    end
  end
end
