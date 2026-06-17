require "net/http"
require "json"
require "uri"

BASE_URL = "http://localhost:8080"

puts "=== PrimusDB Ruby Example ==="

# Check health
begin
  uri = URI("#{BASE_URL}/health")
  resp = Net::HTTP.get_response(uri)
  puts "Health: #{resp.body}"
rescue StandardError => e
  puts "Error connecting to PrimusDB: #{e.message}"
  exit 1
end

# Get version
begin
  uri = URI("#{BASE_URL}/version")
  resp = Net::HTTP.get_response(uri)
  puts "Version: #{resp.body}"
rescue StandardError => e
  puts "Error fetching version: #{e.message}"
end

# Create a record
begin
  uri = URI("#{BASE_URL}/records")
  record = {
    collection: "users",
    data: {
      name: "Alice",
      email: "alice@example.com"
    }
  }
  http = Net::HTTP.new(uri.host, uri.port)
  req = Net::HTTP::Post.new(uri.path)
  req["Content-Type"] = "application/json"
  req.body = record.to_json
  resp = http.request(req)
  puts "Created record: #{resp.body}"
rescue StandardError => e
  puts "Error creating record: #{e.message}"
end

# Query records
begin
  uri = URI("#{BASE_URL}/records/users")
  resp = Net::HTTP.get_response(uri)
  puts "Records: #{resp.body}"
rescue StandardError => e
  puts "Error querying records: #{e.message}"
end
