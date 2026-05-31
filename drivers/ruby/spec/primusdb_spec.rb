require 'primusdb'
require 'webmock/rspec'

RSpec.configure do |config|
  config.order = :random
end

# ==================== StorageType ====================

RSpec.describe PrimusDB::Client::StorageType do
  it 'defines all storage types' do
    expect(described_class::COLUMNAR).to eq('columnar')
    expect(described_class::VECTOR).to eq('vector')
    expect(described_class::DOCUMENT).to eq('document')
    expect(described_class::RELATIONAL).to eq('relational')
    expect(described_class::KEYVALUE).to eq('keyvalue')
  end
end

# ==================== Config ====================

RSpec.describe PrimusDB::Client::Config do
  subject(:config) { described_class.new }

  it 'has default values' do
    expect(config.host).to eq('localhost')
    expect(config.port).to eq(8080)
    expect(config.timeout).to eq(30)
    expect(config.max_connections).to eq(10)
  end

  it 'accepts custom values' do
    c = described_class.new(host: '10.0.0.1', port: 9090, timeout: 15, max_connections: 5)
    expect(c.host).to eq('10.0.0.1')
    expect(c.port).to eq(9090)
    expect(c.timeout).to eq(15)
    expect(c.max_connections).to eq(5)
  end
end

# ==================== Client ====================

RSpec.describe PrimusDB::Client do
  subject(:client) { described_class.new }

  let(:base_url) { 'http://localhost:8080/api/v1' }

  before do
    stub_request(:any, /localhost/)
    client.connect
  end

  describe '#initialize' do
    it 'uses default config' do
      expect(client.config).to be_a(PrimusDB::Client::Config)
    end

    it 'is not connected initially' do
      c = described_class.new
      expect(c).not_to be_connected
    end
  end

  describe '#connect' do
    it 'marks as connected' do
      expect(client).to be_connected
    end
  end

  describe '#close' do
    it 'marks as disconnected' do
      client.close
      expect(client).not_to be_connected
    end
  end

  describe '#create_table' do
    it 'sends a POST request' do
      client.create_table('relational', 'users', { id: 'int' })
      expect(WebMock).to have_requested(:post, "#{base_url}/table/relational/users")
        .with(body: { schema: { id: 'int' } }.to_json)
    end
  end

  describe '#insert' do
    let(:json_headers) { { 'Content-Type' => 'application/json' } }

    it 'sends a POST request and returns count' do
      stub_request(:post, "#{base_url}/crud/relational/users")
        .to_return(status: 200, body: { data: { count: 1 } }.to_json, headers: json_headers)

      count = client.insert('relational', 'users', { name: 'Alice' })
      expect(count).to eq(1)
    end

    it 'returns 0 when count is missing' do
      stub_request(:post, "#{base_url}/crud/relational/users")
        .to_return(status: 200, body: { data: {} }.to_json, headers: json_headers)

      count = client.insert('relational', 'users', { name: 'Alice' })
      expect(count).to eq(0)
    end
  end

  describe '#select' do
    let(:json_headers) { { 'Content-Type' => 'application/json' } }

    it 'sends a GET request' do
      stub_request(:get, "#{base_url}/crud/relational/users")
        .to_return(status: 200, body: { data: [{ id: 1 }] }.to_json, headers: json_headers)

      results = client.select('relational', 'users')
      expect(results).to eq([{ 'id' => 1 }])
    end

    it 'includes query params when given' do
      client.select('relational', 'users', conditions: { name: 'Alice' }, limit: 10)
      expect(WebMock).to have_requested(:get, /conditions=/)
    end
  end

  describe '#update' do
    let(:json_headers) { { 'Content-Type' => 'application/json' } }

    it 'sends a PUT request and returns count' do
      stub_request(:put, "#{base_url}/crud/relational/users")
        .to_return(status: 200, body: { data: { count: 2 } }.to_json, headers: json_headers)

      count = client.update('relational', 'users', { status: 'draft' }, { status: 'published' })
      expect(count).to eq(2)
    end
  end

  describe '#delete' do
    let(:json_headers) { { 'Content-Type' => 'application/json' } }

    it 'sends a DELETE request and returns count' do
      stub_request(:delete, /#{base_url}\/crud\/vector\/vectors/)
        .to_return(status: 200, body: { data: { count: 3 } }.to_json, headers: json_headers)

      count = client.delete('vector', 'vectors', { category: 'test' })
      expect(count).to eq(3)
    end
  end

  # ==================== Advanced Operations ====================

  describe '#analyze' do
    it 'sends a POST request' do
      client.analyze('columnar', 'sales')
      expect(WebMock).to have_requested(:post, "#{base_url}/advanced/analyze/columnar/sales")
    end
  end

  describe '#predict' do
    it 'sends a POST request' do
      client.predict('columnar', 'sales', { features: [1, 2, 3] })
      expect(WebMock).to have_requested(:post, "#{base_url}/advanced/predict/columnar/sales")
    end
  end

  describe '#vector_search' do
    it 'sends a POST request' do
      client.vector_search('embeddings', [0.1, 0.2])
      expect(WebMock).to have_requested(:post, "#{base_url}/advanced/vector-search/embeddings")
    end
  end

  describe '#cluster' do
    it 'sends a POST request' do
      client.cluster('document', 'docs')
      expect(WebMock).to have_requested(:post, "#{base_url}/advanced/cluster/document/docs")
    end
  end

  # ==================== Cluster Gateway ====================

  describe '#cluster_status' do
    it 'sends a GET request' do
      client.cluster_status
      expect(WebMock).to have_requested(:get, "#{base_url}/cluster/status")
    end
  end

  describe '#cluster_nodes' do
    it 'sends a GET request' do
      client.cluster_nodes
      expect(WebMock).to have_requested(:get, "#{base_url}/cluster/nodes")
    end
  end

  describe '#route_request' do
    it 'sends a POST request' do
      client.route_request(shard_key: 'abc')
      expect(WebMock).to have_requested(:post, "#{base_url}/cluster/route")
    end
  end

  describe '#cluster_metrics' do
    it 'sends a GET request' do
      client.cluster_metrics
      expect(WebMock).to have_requested(:get, "#{base_url}/cluster/metrics")
    end
  end

  # ==================== Federation ====================

  describe '#federation_status' do
    it 'sends a GET request' do
      client.federation_status
      expect(WebMock).to have_requested(:get, "#{base_url}/federation/status")
    end
  end

  describe '#create_data_domain' do
    it 'sends a POST request' do
      client.create_data_domain('domain1')
      expect(WebMock).to have_requested(:post, "#{base_url}/federation/domains")
    end
  end

  # ==================== DDL ====================

  describe '#add_column' do
    it 'sends a POST request' do
      client.add_column('relational', 'users', { name: 'TEXT' })
      expect(WebMock).to have_requested(:post, "#{base_url}/ddl/relational/users/column/add")
    end
  end

  describe '#drop_column' do
    it 'sends a DELETE request' do
      client.drop_column('relational', 'users', 'age')
      expect(WebMock).to have_requested(:delete, "#{base_url}/ddl/relational/users/column/age")
    end
  end

  describe '#add_constraint' do
    it 'sends a POST request' do
      client.add_constraint('relational', 'users', { type: 'unique', field: 'email' })
      expect(WebMock).to have_requested(:post, "#{base_url}/ddl/relational/users/constraint")
    end
  end

  describe '#rename_table' do
    it 'sends a POST request' do
      client.rename_table('relational', 'old', 'new')
      expect(WebMock).to have_requested(:post, "#{base_url}/ddl/relational/old/rename")
    end
  end

  # ==================== Sequences ====================

  describe '#create_sequence' do
    it 'sends a POST request' do
      client.create_sequence('relational', { name: 'seq1' })
      expect(WebMock).to have_requested(:post, "#{base_url}/sequence/relational")
    end
  end

  describe '#nextval' do
    it 'sends a POST request' do
      client.nextval('relational', 'seq1')
      expect(WebMock).to have_requested(:post, "#{base_url}/sequence/relational/seq1/nextval")
    end
  end

  describe '#setval' do
    it 'sends a POST request' do
      client.setval('relational', 'seq1', 100)
      expect(WebMock).to have_requested(:post, "#{base_url}/sequence/relational/seq1/setval")
    end
  end

  # ==================== Views / Triggers / Info Schema ====================

  describe '#create_view' do
    it 'sends a POST request' do
      client.create_view('relational', { name: 'v1', query: 'SELECT * FROM users' })
      expect(WebMock).to have_requested(:post, "#{base_url}/view/relational")
    end
  end

  describe '#create_trigger' do
    it 'sends a POST request' do
      client.create_trigger('relational', 'users', { name: 'trg1' })
      expect(WebMock).to have_requested(:post, "#{base_url}/trigger/relational/users")
    end
  end

  describe '#info_schema_tables' do
    it 'sends a GET request' do
      client.info_schema_tables('relational')
      expect(WebMock).to have_requested(:get, "#{base_url}/info-schema/relational/tables")
    end
  end

  # ==================== SQL / UQL ====================

  describe '#execute_sql' do
    it 'sends a POST request' do
      client.execute_sql('SELECT * FROM users')
      expect(WebMock).to have_requested(:post, "#{base_url}/uql")
        .with(body: { query: 'SELECT * FROM users', language: 'sql' }.to_json)
    end
  end

  # ==================== Key-Value Operations ====================

  describe '#kv_get_db_info' do
    it 'sends a GET request' do
      client.kv_get_db_info('mydb')
      expect(WebMock).to have_requested(:get, "#{base_url}/kv/mydb")
    end
  end

  describe '#kv_create_database' do
    it 'sends a PUT request' do
      client.kv_create_database('mydb')
      expect(WebMock).to have_requested(:put, "#{base_url}/kv/mydb")
    end
  end

  describe '#kv_put_document' do
    it 'sends a PUT request' do
      client.kv_put_document('mydb', 'doc1', { title: 'Hello' })
      expect(WebMock).to have_requested(:put, "#{base_url}/kv/mydb/doc1")
    end
  end

  describe '#kv_get_document' do
    it 'sends a GET request' do
      client.kv_get_document('mydb', 'doc1')
      expect(WebMock).to have_requested(:get, "#{base_url}/kv/mydb/doc1")
    end
  end

  describe '#kv_bulk_docs' do
    it 'sends a POST request' do
      client.kv_bulk_docs('mydb', [{ _id: 'doc1', title: 'Hello' }])
      expect(WebMock).to have_requested(:post, "#{base_url}/kv/mydb/_bulk_docs")
    end
  end

  describe '#kv_find' do
    it 'sends a POST request' do
      client.kv_find('mydb', { name: 'Alice' })
      expect(WebMock).to have_requested(:post, "#{base_url}/kv/mydb/_find")
    end
  end

  # ==================== Error Handling ====================

  describe 'when not connected' do
    it 'raises an error on any operation' do
      c = described_class.new
      expect { c.create_table('r', 't', {}) }.to raise_error('Not connected to PrimusDB server')
    end
  end

  describe 'API error' do
    it 'raises on non-success status' do
      stub_request(:get, "#{base_url}/cluster/status")
        .to_return(status: 500, body: { error: 'Internal error' }.to_json)

      expect { client.cluster_status }.to raise_error(/PrimusDB API error/)
    end
  end
end

# ==================== Collection ====================

RSpec.describe PrimusDB::Collection do
  subject(:collection) { described_class.new(client, 'relational', 'users') }

  let(:client) { PrimusDB::Client.new }
  let(:base_url) { 'http://localhost:8080/api/v1' }

  before do
    stub_request(:any, /localhost/)
    client.connect
  end

  describe '#insert_one' do
    let(:json_headers) { { 'Content-Type' => 'application/json' } }

    it 'delegates to client.insert' do
      stub_request(:post, "#{base_url}/crud/relational/users")
        .to_return(status: 200, body: { data: { count: 1 } }.to_json, headers: json_headers)

      expect(collection.insert_one({ name: 'Alice' })).to eq(1)
    end
  end

  describe '#find' do
    let(:json_headers) { { 'Content-Type' => 'application/json' } }

    it 'delegates to client.select' do
      stub_request(:get, "#{base_url}/crud/relational/users")
        .to_return(status: 200, body: { data: [{ id: 1 }] }.to_json, headers: json_headers)

      expect(collection.find).to eq([{ 'id' => 1 }])
    end
  end

  describe '#count' do
    let(:json_headers) { { 'Content-Type' => 'application/json' } }

    it 'returns the size of find results' do
      stub_request(:get, /#{base_url}\/crud\/relational\/users/)
        .to_return(status: 200, body: { data: [{ id: 1 }, { id: 2 }] }.to_json, headers: json_headers)

      expect(collection.count).to eq(2)
    end
  end
end
